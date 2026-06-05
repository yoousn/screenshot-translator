use crate::*;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f64,
    pub box_coords: Vec<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct RapidOcrRunnerOutput {
    pub status: String,
    pub engine: Option<String>,
    #[serde(rename = "modelVersion")]
    pub model_version: Option<String>,
    #[serde(rename = "selectedLang")]
    pub selected_lang: Option<String>,
    #[serde(rename = "selectedVariant")]
    pub selected_variant: Option<String>,
    pub blocks: Option<Vec<OcrBlock>>,
    pub timings: Option<serde_json::Value>,
    pub candidates: Option<Vec<serde_json::Value>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RapidOcrCommandSpec {
    pub program: PathBuf,
    pub args_prefix: Vec<String>,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct RapidOcrWorkerEnvelope {
    pub id: u64,
    pub ok: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub struct RapidOcrWorkerProcess {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: BufReader<ChildStdout>,
    pub request_id: u64,
    pub spec: RapidOcrCommandSpec,
    pub last_error: Option<String>,
}

pub static RAPID_OCR_WORKER: OnceLock<Mutex<Option<RapidOcrWorkerProcess>>> = OnceLock::new();

pub fn rapid_ocr_worker_state() -> &'static Mutex<Option<RapidOcrWorkerProcess>> {
    RAPID_OCR_WORKER.get_or_init(|| Mutex::new(None))
}

#[tauri::command]
pub async fn prewarm_local_ocr_models(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let model_version = rapid_ocr_model_version();
        if rapid_ocr_worker_enabled() {
            start_rapid_ocr_worker_sync(&app, true)?;
            Ok(vec![format!("rapidocr-{model_version}-worker")])
        } else {
            run_rapidocr_probe(&app, &model_version)?;
            Ok(vec![format!("rapidocr-{model_version}")])
        }
    })
    .await
    .map_err(|error| format!("RapidOCR prewarm task failed: {error}"))?
}

#[tauri::command]
pub async fn run_local_ocr(
    app: tauri::AppHandle,
    image_base64: String,
    executable_path: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<Vec<OcrBlock>, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15000).clamp(500, 60000));
    let task =
        tokio::task::spawn_blocking(move || run_local_ocr_sync(app, image_base64, executable_path));
    match tokio::time::timeout(timeout, task).await {
        Ok(joined) => joined.map_err(|e| format!("Local OCR task failed: {}", e))?,
        Err(_) => Err(format!("Local OCR timed out ({} ms)", timeout.as_millis())),
    }
}

pub fn run_local_ocr_sync(
    app: tauri::AppHandle,
    image_base64: String,
    _executable_path: Option<String>,
) -> Result<Vec<OcrBlock>, String> {
    match run_rapidocr_sync(&app, &image_base64) {
        Ok(blocks) if !blocks.is_empty() => return Ok(blocks),
        Ok(_) => {
            return Err(
                "\u{672c}\u{5730}\u{622a}\u{56fe}\u{7ffb}\u{8bd1}\u{672a}\u{8bc6}\u{522b}\u{5230}\u{6587}\u{5b57}\u{3002}\u{8bf7}\u{91cd}\u{65b0}\u{6846}\u{9009}\u{66f4}\u{6e05}\u{6670}\u{3001}\u{66f4}\u{5b8c}\u{6574}\u{7684}\u{6587}\u{5b57}\u{533a}\u{57df}\u{3002}".to_string(),
            );
        }
        Err(error) => return Err(error),
    }
}

pub fn run_rapidocr_sync(app: &tauri::AppHandle, image_base64: &str) -> Result<Vec<OcrBlock>, String> {
    let total_started = Instant::now();
    let image_bytes = BASE64_STANDARD
        .decode(image_base64)
        .map_err(|error| format!("Decode RapidOCR image failed: {error}"))?;
    let temp_path = write_rapidocr_temp_image(&image_bytes)?;
    let model_version = rapid_ocr_model_version();
    let mode = rapid_ocr_mode();
    let model_root = rapid_ocr_model_root(app);
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    if !missing_models.is_empty() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "RapidOCR model files are missing from {}: {}",
            model_root.display(),
            missing_models.join(", ")
        ));
    }
    let args = vec![
        "--image".to_string(),
        temp_path.to_string_lossy().to_string(),
        "--model-version".to_string(),
        model_version.clone(),
        "--mode".to_string(),
        mode.clone(),
        "--model-root".to_string(),
        model_root.to_string_lossy().to_string(),
    ];
    let result = if rapid_ocr_worker_enabled() {
        match run_rapidocr_worker_ocr(app, &temp_path, &model_version, &mode, &model_root) {
            Ok(output) => Ok(output),
            Err(error) => {
                eprintln!(
                    "[local-screenshot-translate] rapidocr worker failed, falling back to one-shot runner: {error}"
                );
                run_rapidocr_json(app, args)
            }
        }
    } else {
        run_rapidocr_json(app, args)
    };
    let _ = fs::remove_file(&temp_path);
    let output = result?;
    if output.status != "success" {
        return Err(output
            .error
            .unwrap_or_else(|| "RapidOCR returned a failed status.".to_string()));
    }
    let blocks = output.blocks.unwrap_or_default();
    eprintln!(
        "[local-screenshot-translate] rapidocr total={}ms runner={} model={} lang={} variant={} blocks={} timings={}",
        total_started.elapsed().as_millis(),
        output.engine.as_deref().unwrap_or("rapidocr"),
        output.model_version.as_deref().unwrap_or(&model_version),
        output.selected_lang.as_deref().unwrap_or("auto"),
        output.selected_variant.as_deref().unwrap_or("original"),
        blocks.len(),
        serde_json::to_string(&output.timings).unwrap_or_else(|_| "null".to_string())
    );
    if let Some(candidates) = output.candidates {
        eprintln!(
            "[local-screenshot-translate] rapidocr candidates {}",
            serde_json::to_string(&candidates).unwrap_or_else(|_| "[]".to_string())
        );
    }
    Ok(blocks)
}

pub fn rapid_ocr_model_version() -> String {
    match config_value_string("rapidOcrModelVersion")
        .unwrap_or_else(|| "v5".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "v4" => "v4".to_string(),
        _ => "v5".to_string(),
    }
}

pub fn rapid_ocr_mode() -> String {
    match config_value_string("rapidOcrMode")
        .unwrap_or_else(|| "auto".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "full" => "full".to_string(),
        "latin" => "latin".to_string(),
        _ => "auto".to_string(),
    }
}

pub fn rapid_ocr_worker_enabled() -> bool {
    config_value_bool("rapidOcrWorkerEnabled").unwrap_or(true)
}

pub fn push_unique_path(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.iter().any(|candidate| candidate == &path) {
        candidates.push(path);
    }
}

pub fn push_rapid_ocr_model_candidates_from_base(candidates: &mut Vec<PathBuf>, base: &Path) {
    push_unique_path(candidates, base.join("models").join("rapidocr"));
    push_unique_path(
        candidates,
        base.join("resources").join("models").join("rapidocr"),
    );
    push_unique_path(
        candidates,
        base.join("resources")
            .join("_up_")
            .join("_up_")
            .join("models")
            .join("rapidocr"),
    );
    push_unique_path(
        candidates,
        base.join("_up_")
            .join("_up_")
            .join("models")
            .join("rapidocr"),
    );
}

pub fn rapid_ocr_model_root_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = config_value_string("rapidOcrModelRoot")
        .or_else(|| std::env::var("YSN_RAPIDOCR_MODEL_ROOT").ok())
        .map(PathBuf::from)
    {
        push_unique_path(&mut candidates, path);
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            push_rapid_ocr_model_candidates_from_base(&mut candidates, exe_dir);
            if let Some(parent) = exe_dir.parent() {
                push_rapid_ocr_model_candidates_from_base(&mut candidates, parent);
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        push_rapid_ocr_model_candidates_from_base(&mut candidates, &resource_dir);
    }

    use tauri::path::BaseDirectory;
    for resource_path in ["models/rapidocr", "../../models/rapidocr"] {
        if let Ok(path) = app.path().resolve(resource_path, BaseDirectory::Resource) {
            push_unique_path(&mut candidates, path);
        }
    }

    push_unique_path(
        &mut candidates,
        repo_root_from_manifest().join("models").join("rapidocr"),
    );
    candidates
}

pub fn rapid_ocr_model_root(app: &tauri::AppHandle) -> PathBuf {
    rapid_ocr_model_root_candidates(app)
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| repo_root_from_manifest().join("models").join("rapidocr"))
}

pub fn rapid_ocr_required_model_files(model_version: &str) -> Vec<&'static str> {
    let mut files = vec![
        "ch_PP-LCNet_x0_25_textline_ori_cls_mobile.onnx",
        "ppocr_keys_v1.txt",
        "ppocrv5_dict.txt",
    ];
    if model_version == "v4" {
        files.extend([
            "ch_PP-OCRv4_det_mobile.onnx",
            "ch_PP-OCRv4_rec_mobile.onnx",
            "latin_PP-OCRv3_rec_mobile.onnx",
        ]);
    } else {
        files.extend([
            "ch_PP-OCRv5_det_mobile.onnx",
            "ch_PP-OCRv5_rec_mobile.onnx",
            "latin_PP-OCRv5_rec_mobile.onnx",
            "korean_PP-OCRv5_rec_mobile.onnx",
            "arabic_PP-OCRv5_rec_mobile.onnx",
            "cyrillic_PP-OCRv5_rec_mobile.onnx",
            "th_PP-OCRv5_rec_mobile.onnx",
        ]);
    }
    files
}

pub fn rapid_ocr_missing_model_files(model_root: &Path, model_version: &str) -> Vec<String> {
    rapid_ocr_required_model_files(model_version)
        .into_iter()
        .filter(|name| !model_root.join(name).is_file())
        .map(str::to_string)
        .collect()
}

pub fn write_rapidocr_temp_image(image_bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir()
        .join("ysn-screenshot-translator")
        .join("rapidocr");
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create RapidOCR temp directory {}: {error}",
            dir.display()
        )
    })?;
    let path = dir.join(format!(
        "ocr-{}-{}.png",
        std::process::id(),
        chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
    ));
    fs::write(&path, image_bytes).map_err(|error| {
        format!(
            "failed to write RapidOCR temp image {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

pub fn push_rapid_ocr_runner_candidates_from_base(candidates: &mut Vec<PathBuf>, base: &Path) {
    push_unique_path(candidates, base.join("rapidocr-runner.exe"));
    push_unique_path(
        candidates,
        base.join("rapidocr").join("rapidocr-runner.exe"),
    );
    push_unique_path(
        candidates,
        base.join("rapidocr")
            .join("rapidocr-runner")
            .join("rapidocr-runner.exe"),
    );
    push_unique_path(
        candidates,
        base.join("resources")
            .join("rapidocr")
            .join("rapidocr-runner.exe"),
    );
    push_unique_path(
        candidates,
        base.join("resources")
            .join("rapidocr")
            .join("rapidocr-runner")
            .join("rapidocr-runner.exe"),
    );
}

pub fn rapid_ocr_runner_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            push_rapid_ocr_runner_candidates_from_base(&mut candidates, exe_dir);
            push_rapid_ocr_runner_candidates_from_base(
                &mut candidates,
                &exe_dir.join("tauri-client").join("src-tauri"),
            );
            if let Some(parent) = exe_dir.parent() {
                push_rapid_ocr_runner_candidates_from_base(&mut candidates, parent);
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        push_rapid_ocr_runner_candidates_from_base(&mut candidates, &resource_dir);
    }

    use tauri::path::BaseDirectory;
    for resource_path in [
        "resources/rapidocr/rapidocr-runner/rapidocr-runner.exe",
        "resources/rapidocr/rapidocr-runner.exe",
        "rapidocr/rapidocr-runner/rapidocr-runner.exe",
        "rapidocr/rapidocr-runner.exe",
    ] {
        if let Ok(path) = app.path().resolve(resource_path, BaseDirectory::Resource) {
            push_unique_path(&mut candidates, path);
        }
    }

    push_rapid_ocr_runner_candidates_from_base(
        &mut candidates,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    );
    candidates
}

pub fn resolve_rapidocr_command(app: &tauri::AppHandle) -> Result<RapidOcrCommandSpec, String> {
    if let Some(path) = config_value_string("rapidOcrRunnerPath")
        .or_else(|| std::env::var("YSN_RAPIDOCR_RUNNER").ok())
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Ok(RapidOcrCommandSpec {
            program: path,
            args_prefix: Vec::new(),
            kind: "custom-runner".to_string(),
        });
    }

    if let Some(path) = rapid_ocr_runner_candidates(app)
        .into_iter()
        .find(|path| path.is_file())
    {
        return Ok(RapidOcrCommandSpec {
            program: path,
            args_prefix: Vec::new(),
            kind: "bundled-runner".to_string(),
        });
    }

    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("rapidocr")
        .join("rapidocr_runner.py");
    if script_path.exists() {
        return Ok(RapidOcrCommandSpec {
            program: PathBuf::from("python"),
            args_prefix: vec![script_path.to_string_lossy().to_string()],
            kind: "python-runner".to_string(),
        });
    }

    Err("RapidOCR runner was not found. Expected bundled rapidocr-runner.exe or src-tauri/rapidocr/rapidocr_runner.py.".to_string())
}

pub fn run_rapidocr_json(
    app: &tauri::AppHandle,
    args: Vec<String>,
) -> Result<RapidOcrRunnerOutput, String> {
    let spec = resolve_rapidocr_command(app)?;
    let mut command = Command::new(&spec.program);
    command.args(&spec.args_prefix);
    command.args(&args);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command.output().map_err(|error| {
        format!(
            "failed to start RapidOCR runner ({}): {error}",
            spec.program.display()
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!(
            "RapidOCR runner failed with status {}. stdout: {} stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| {
            format!(
                "RapidOCR runner did not return JSON. stderr: {}",
                stderr.trim()
            )
        })?;
    let mut parsed: RapidOcrRunnerOutput = serde_json::from_str(json_line.trim())
        .map_err(|error| format!("failed to parse RapidOCR JSON: {error}; output: {json_line}"))?;
    if parsed.engine.is_none() {
        parsed.engine = Some(spec.kind);
    }
    Ok(parsed)
}

pub fn spawn_rapid_ocr_worker_process(app: &tauri::AppHandle) -> Result<RapidOcrWorkerProcess, String> {
    let spec = resolve_rapidocr_command(app)?;
    let mut command = Command::new(&spec.program);
    command.args(&spec.args_prefix);
    command.arg("--worker");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);

    let mut child = command.spawn().map_err(|error| {
        format!(
            "failed to start RapidOCR worker ({}): {error}",
            spec.program.display()
        )
    })?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "RapidOCR worker stdin was not available.".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "RapidOCR worker stdout was not available.".to_string())?;

    let mut worker = RapidOcrWorkerProcess {
        child,
        stdin,
        stdout: BufReader::new(stdout),
        request_id: 0,
        spec,
        last_error: None,
    };
    let _ = rapid_ocr_worker_request_value(&mut worker, "ping", serde_json::json!({}))?;
    Ok(worker)
}

pub fn rapid_ocr_worker_request_value(
    worker: &mut RapidOcrWorkerProcess,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if let Some(status) = worker
        .child
        .try_wait()
        .map_err(|error| format!("RapidOCR worker status check failed: {error}"))?
    {
        return Err(format!(
            "RapidOCR worker already exited with status {status}."
        ));
    }

    worker.request_id = worker.request_id.saturating_add(1);
    let request_id = worker.request_id;
    let payload = serde_json::json!({
        "id": request_id,
        "method": method,
        "params": params,
    });
    writeln!(worker.stdin, "{payload}")
        .and_then(|_| worker.stdin.flush())
        .map_err(|error| format!("failed to send RapidOCR worker request: {error}"))?;

    let mut line = String::new();
    loop {
        line.clear();
        let read = worker
            .stdout
            .read_line(&mut line)
            .map_err(|error| format!("failed to read RapidOCR worker response: {error}"))?;
        if read == 0 {
            return Err("RapidOCR worker closed stdout before responding.".to_string());
        }
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        let envelope: RapidOcrWorkerEnvelope = serde_json::from_str(trimmed).map_err(|error| {
            format!("failed to parse RapidOCR worker response: {error}; output: {trimmed}")
        })?;
        if envelope.id != request_id {
            continue;
        }
        if envelope.ok {
            return Ok(envelope.result.unwrap_or_else(|| serde_json::json!({})));
        }
        return Err(envelope
            .error
            .unwrap_or_else(|| "RapidOCR worker returned an error.".to_string()));
    }
}

pub fn with_rapid_ocr_worker<T>(
    app: &tauri::AppHandle,
    action: impl FnOnce(&mut RapidOcrWorkerProcess) -> Result<T, String>,
) -> Result<T, String> {
    let state = rapid_ocr_worker_state();
    let mut guard = state
        .lock()
        .map_err(|error| format!("RapidOCR worker lock failed: {error}"))?;

    let exited_status = if let Some(worker) = guard.as_mut() {
        worker
            .child
            .try_wait()
            .map_err(|error| format!("RapidOCR worker status check failed: {error}"))?
    } else {
        None
    };
    if let Some(status) = exited_status {
        *guard = None;
        return Err(format!("RapidOCR worker exited with status {status}."));
    }

    if guard.is_none() {
        *guard = Some(spawn_rapid_ocr_worker_process(app)?);
    }

    let worker = guard
        .as_mut()
        .ok_or_else(|| "RapidOCR worker was not started.".to_string())?;
    match action(worker) {
        Ok(value) => {
            worker.last_error = None;
            Ok(value)
        }
        Err(error) => {
            worker.last_error = Some(error.clone());
            Err(error)
        }
    }
}

pub fn run_rapidocr_worker_ocr(
    app: &tauri::AppHandle,
    image_path: &Path,
    model_version: &str,
    mode: &str,
    model_root: &Path,
) -> Result<RapidOcrRunnerOutput, String> {
    let result = with_rapid_ocr_worker(app, |worker| {
        rapid_ocr_worker_request_value(
            worker,
            "ocr",
            serde_json::json!({
                "imagePath": image_path.to_string_lossy().to_string(),
                "modelVersion": model_version,
                "mode": mode,
                "modelRoot": model_root.to_string_lossy().to_string(),
                "smallTextRetry": true
            }),
        )
    })?;
    let mut parsed: RapidOcrRunnerOutput = serde_json::from_value(result)
        .map_err(|error| format!("failed to parse RapidOCR worker OCR JSON: {error}"))?;
    if parsed.engine.is_none() {
        parsed.engine = Some("rapidocr-worker".to_string());
    }
    Ok(parsed)
}

pub fn start_rapid_ocr_worker_sync(
    app: &tauri::AppHandle,
    warm_basic_models: bool,
) -> Result<serde_json::Value, String> {
    let model_version = rapid_ocr_model_version();
    let model_root = rapid_ocr_model_root(app);
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    if !missing_models.is_empty() {
        return Err(format!(
            "RapidOCR model files are missing from {}: {}",
            model_root.display(),
            missing_models.join(", ")
        ));
    }

    with_rapid_ocr_worker(app, |worker| {
        let mut warm_result = serde_json::json!(null);
        if warm_basic_models {
            warm_result = rapid_ocr_worker_request_value(
                worker,
                "warm",
                serde_json::json!({
                    "modelRoot": model_root.to_string_lossy().to_string(),
                    "modelVersion": model_version,
                    "langs": ["ch", "latin"]
                }),
            )?;
        }
        let status = rapid_ocr_worker_request_value(worker, "status", serde_json::json!({}))?;
        Ok(serde_json::json!({
            "running": true,
            "pid": worker.child.id(),
            "runnerKind": worker.spec.kind.clone(),
            "runnerPath": worker.spec.program.to_string_lossy().to_string(),
            "modelVersion": model_version,
            "modelRoot": model_root.to_string_lossy().to_string(),
            "warmResult": warm_result,
            "status": status
        }))
    })
}

pub fn stop_rapid_ocr_worker_internal(grace_ms: u64) -> Result<serde_json::Value, String> {
    let state = rapid_ocr_worker_state();
    let mut guard = state
        .lock()
        .map_err(|error| format!("RapidOCR worker lock failed: {error}"))?;
    let Some(mut worker) = guard.take() else {
        return Ok(serde_json::json!({
            "running": false,
            "stopped": false,
            "message": "RapidOCR worker was not running."
        }));
    };

    let _ = rapid_ocr_worker_request_value(&mut worker, "shutdown", serde_json::json!({}));
    let started = Instant::now();
    loop {
        if let Some(status) = worker
            .child
            .try_wait()
            .map_err(|error| format!("RapidOCR worker status check failed: {error}"))?
        {
            return Ok(serde_json::json!({
                "running": false,
                "stopped": true,
                "exitStatus": status.to_string()
            }));
        }
        if started.elapsed() >= Duration::from_millis(grace_ms) {
            let _ = worker.child.kill();
            let _ = worker.child.wait();
            return Ok(serde_json::json!({
                "running": false,
                "stopped": true,
                "forced": true
            }));
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[tauri::command]
pub async fn restart_rapid_ocr_worker(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || {
        let _ = stop_rapid_ocr_worker_internal(1500);
        start_rapid_ocr_worker_sync(&app, true)
    })
    .await
    .map_err(|error| format!("RapidOCR worker restart task failed: {error}"))?
}

#[tauri::command]
pub fn stop_rapid_ocr_worker() -> Result<serde_json::Value, String> {
    stop_rapid_ocr_worker_internal(1500)
}

#[tauri::command]
pub fn get_rapid_ocr_worker_status() -> Result<serde_json::Value, String> {
    Ok(rapid_ocr_worker_status_value())
}

#[tauri::command]
pub fn get_rapid_ocr_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let model_version = rapid_ocr_model_version();
    let model_root = rapid_ocr_model_root(&app);
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    let runner = resolve_rapidocr_command(&app);
    let worker_enabled = rapid_ocr_worker_enabled();
    let worker_status = rapid_ocr_worker_status_value();
    let worker_running = worker_status
        .get("running")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let mut last_error: Option<String> = None;
    let mut probe_timings = serde_json::json!(null);
    let mut probe_ok = false;
    if !missing_models.is_empty() {
        last_error = Some(format!(
            "RapidOCR model files are missing from {}: {}",
            model_root.display(),
            missing_models.join(", ")
        ));
    } else if worker_enabled {
        probe_ok = runner.is_ok() && missing_models.is_empty();
        probe_timings = worker_status
            .get("status")
            .and_then(|status| status.get("timings"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!(null));
        if let Some(error) = worker_status
            .get("lastError")
            .and_then(|value| value.as_str())
        {
            if !error.trim().is_empty() {
                last_error = Some(error.to_string());
            }
        }
    } else if runner.is_ok() {
        match run_rapidocr_probe(&app, &model_version) {
            Ok(output) if output.status == "success" => {
                probe_ok = true;
                probe_timings = output.timings.unwrap_or_else(|| serde_json::json!(null));
            }
            Ok(output) => {
                last_error = Some(
                    output
                        .error
                        .unwrap_or_else(|| "RapidOCR probe failed.".to_string()),
                );
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    } else if let Err(error) = &runner {
        last_error = Some(error.clone());
    }
    let runner_kind = runner
        .as_ref()
        .map(|spec| spec.kind.clone())
        .unwrap_or_else(|_| "missing".to_string());
    let runner_path = runner
        .as_ref()
        .map(|spec| spec.program.to_string_lossy().to_string())
        .unwrap_or_default();
    let models_ready = missing_models.is_empty();
    let ready = runner.is_ok() && models_ready && probe_ok;
    Ok(serde_json::json!({
        "ready": ready,
        "runnerReady": runner.is_ok(),
        "runtimeInferenceReady": ready,
        "modelPacksReady": models_ready,
        "activeModelsReady": models_ready,
        "selfTestReady": probe_ok,
        "runtime": "rapidocr",
        "engine": "rapidocr",
        "runnerKind": runner_kind,
        "runnerPath": runner_path,
        "workerEnabled": worker_enabled,
        "workerRunning": worker_running,
        "worker": worker_status,
        "runtimeVersion": "rapidocr-python-3.x",
        "modelSetVersion": format!("rapidocr-{}", model_version),
        "rapidOcrModelVersion": model_version,
        "modelDir": model_root.to_string_lossy().to_string(),
        "modelRoot": model_root.to_string_lossy().to_string(),
        "missingModelFiles": missing_models,
        "defaultSourceLanguage": "auto",
        "defaultProfile": "balanced",
        "lastError": last_error,
        "probeTimings": probe_timings,
        "supportedModelVersions": ["v5", "v4"],
        "readinessSteps": [
            {
                "id": "rapidocr-runner",
                "ready": runner.is_ok(),
                "severity": if runner.is_ok() { "success" } else { "error" },
                "label": "RapidOCR runner",
                "description": "RapidOCR runner executable or development Python runner is available.",
                "nextAction": if runner.is_ok() { "run-ocr-self-test" } else { "install-rapidocr-runner" }
            },
            {
                "id": "rapidocr-worker",
                "ready": if worker_enabled { worker_running } else { true },
                "severity": if !worker_enabled || worker_running { "success" } else { "warning" },
                "label": "RapidOCR worker",
                "description": if worker_enabled { "RapidOCR resident worker is enabled; it starts lazily or from the panel." } else { "RapidOCR resident worker is disabled; OCR uses the one-shot runner fallback." },
                "nextAction": if worker_enabled && !worker_running { "start-rapidocr-worker" } else { "ready" }
            },
            {
                "id": "rapidocr-probe",
                "ready": probe_ok,
                "severity": if probe_ok { "success" } else { "warning" },
                "label": "RapidOCR probe",
                "description": if worker_enabled { "RapidOCR dependencies are available; self-test warms the resident worker." } else { "RapidOCR can initialize the configured PP-OCR model version." },
                "nextAction": if probe_ok { "ready" } else { "run-ocr-self-test" }
            },
            {
                "id": "rapidocr-root-models",
                "ready": models_ready,
                "severity": if models_ready { "success" } else { "error" },
                "label": "RapidOCR root models",
                "description": "RapidOCR model files are present under the repository or app root models/rapidocr directory.",
                "nextAction": if models_ready { "ready" } else { "restore-root-models" }
            }
        ]
    }))
}

#[tauri::command]
pub fn run_rapid_ocr_self_test(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let tested_at = chrono::Local::now().to_rfc3339();
    let model_version = rapid_ocr_model_version();
    let model_root = rapid_ocr_model_root(&app);
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    if !missing_models.is_empty() {
        return Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "modelRoot": model_root.to_string_lossy().to_string(),
            "message": format!("RapidOCR model files are missing from {}: {}", model_root.display(), missing_models.join(", ")),
            "samples": []
        }));
    }
    if rapid_ocr_worker_enabled() {
        return match start_rapid_ocr_worker_sync(&app, true) {
            Ok(output) => Ok(serde_json::json!({
                "ok": true,
                "testedAt": tested_at,
                "runtime": "rapidocr",
                "modelVersion": model_version,
                "modelRoot": model_root.to_string_lossy().to_string(),
                "message": "RapidOCR worker started and warmed.",
                "timings": output.get("warmResult").and_then(|value| value.get("timings")).cloned().unwrap_or_else(|| serde_json::json!(null)),
                "samples": [
                    { "id": "worker-warm", "ok": true, "confidence": 1.0, "modelId": format!("rapidocr-{}-worker", model_version) }
                ],
                "worker": output
            })),
            Err(error) => Ok(serde_json::json!({
                "ok": false,
                "testedAt": tested_at,
                "runtime": "rapidocr",
                "modelVersion": model_version,
                "modelRoot": model_root.to_string_lossy().to_string(),
                "message": error,
                "samples": [
                    { "id": "worker-warm", "ok": false, "confidence": 0.0, "modelId": format!("rapidocr-{}-worker", model_version) }
                ]
            })),
        };
    }

    match run_rapidocr_probe(&app, &model_version) {
        Ok(output) if output.status == "success" => Ok(serde_json::json!({
            "ok": true,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "modelRoot": model_root.to_string_lossy().to_string(),
            "message": "RapidOCR probe passed.",
            "timings": output.timings,
            "samples": [
                { "id": "engine-init", "ok": true, "confidence": 1.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
        Ok(output) => Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "message": output.error.unwrap_or_else(|| "RapidOCR probe failed.".to_string()),
            "samples": [
                { "id": "engine-init", "ok": false, "confidence": 0.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
        Err(error) => Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "message": error,
            "samples": [
                { "id": "engine-init", "ok": false, "confidence": 0.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
    }
}


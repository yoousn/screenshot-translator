use crate::*;
use base64::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::worker::{rapid_ocr_worker_enabled, run_rapidocr_worker_ocr};

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

pub fn run_local_ocr_sync(
    app: tauri::AppHandle,
    image_base64: String,
    _executable_path: Option<String>,
    small_text_retry: Option<bool>,
    worker_timeout: Option<Duration>,
) -> Result<Vec<OcrBlock>, String> {
    match run_rapidocr_sync(&app, &image_base64, small_text_retry, worker_timeout) {
        Ok(blocks) if !blocks.is_empty() => Ok(blocks),
        Ok(_) => {
            Err(
                "\u{672c}\u{5730}\u{622a}\u{56fe}\u{7ffb}\u{8bd1}\u{672a}\u{8bc6}\u{522b}\u{5230}\u{6587}\u{5b57}\u{3002}\u{8bf7}\u{91cd}\u{65b0}\u{6846}\u{9009}\u{66f4}\u{6e05}\u{6670}\u{3001}\u{66f4}\u{5b8c}\u{6574}\u{7684}\u{6587}\u{5b57}\u{533a}\u{57df}\u{3002}".to_string(),
            )
        }
        Err(error) => Err(error),
    }
}

pub fn run_rapidocr_sync(
    app: &tauri::AppHandle,
    image_base64: &str,
    small_text_retry: Option<bool>,
    worker_timeout: Option<Duration>,
) -> Result<Vec<OcrBlock>, String> {
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

    let final_small_text_retry = small_text_retry
        .or_else(|| config_value_bool("rapidOcrSmallTextRetry"))
        .unwrap_or(true);

    let mut args = vec![
        "--image".to_string(),
        temp_path.to_string_lossy().to_string(),
        "--model-version".to_string(),
        model_version.clone(),
        "--mode".to_string(),
        mode.clone(),
        "--model-root".to_string(),
        model_root.to_string_lossy().to_string(),
    ];
    if !final_small_text_retry {
        args.push("--no-small-text-retry".to_string());
    }

    let result = if rapid_ocr_worker_enabled() {
        match run_rapidocr_worker_ocr(
            app,
            &temp_path,
            &model_version,
            &mode,
            &model_root,
            final_small_text_retry,
            worker_timeout.unwrap_or_else(|| Duration::from_millis(60_000)),
        ) {
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
        .unwrap_or_else(|| rapid_ocr_model_install_root(app))
}

pub fn rapid_ocr_model_install_root(app: &tauri::AppHandle) -> PathBuf {
    if let Some(path) = config_value_string("rapidOcrModelRoot")
        .or_else(|| std::env::var("YSN_RAPIDOCR_MODEL_ROOT").ok())
        .map(PathBuf::from)
    {
        return path;
    }

    let repo_root = repo_root_from_manifest().join("models").join("rapidocr");
    if cfg!(debug_assertions) || repo_root.exists() {
        return repo_root;
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join("models").join("rapidocr");
        }
    }

    app.path()
        .resource_dir()
        .map(|path| path.join("models").join("rapidocr"))
        .unwrap_or(repo_root)
}

pub fn rapid_ocr_required_model_files(model_version: &str) -> Vec<&'static str> {
    if model_version == "v4" {
        vec![
            "ch_ppocr_mobile_v2.0_cls_mobile.onnx",
            "ch_PP-OCRv4_det_mobile.onnx",
            "ch_PP-OCRv4_rec_mobile.onnx",
            "latin_PP-OCRv3_rec_mobile.onnx",
            "korean_PP-OCRv4_rec_mobile.onnx",
            "arabic_PP-OCRv4_rec_mobile.onnx",
            "cyrillic_PP-OCRv3_rec_mobile.onnx",
        ]
    } else {
        vec![
            "ch_PP-LCNet_x0_25_textline_ori_cls_mobile.onnx",
            "ch_PP-OCRv5_det_mobile.onnx",
            "ch_PP-OCRv5_rec_mobile.onnx",
            "latin_PP-OCRv5_rec_mobile.onnx",
            "korean_PP-OCRv5_rec_mobile.onnx",
            "arabic_PP-OCRv5_rec_mobile.onnx",
            "cyrillic_PP-OCRv5_rec_mobile.onnx",
            "th_PP-OCRv5_rec_mobile.onnx",
        ]
    }
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
    let (value, runner_kind) = run_rapidocr_value_with_runner_kind(app, args)?;
    let mut parsed: RapidOcrRunnerOutput = serde_json::from_value(value)
        .map_err(|error| format!("failed to parse RapidOCR JSON: {error}"))?;
    if parsed.engine.is_none() {
        parsed.engine = Some(runner_kind);
    }
    Ok(parsed)
}

pub fn run_rapidocr_command_value(
    app: &tauri::AppHandle,
    args: Vec<String>,
) -> Result<serde_json::Value, String> {
    run_rapidocr_value_with_runner_kind(app, args).map(|(value, _)| value)
}

fn run_rapidocr_value_with_runner_kind(
    app: &tauri::AppHandle,
    args: Vec<String>,
) -> Result<(serde_json::Value, String), String> {
    let spec = resolve_rapidocr_command(app)?;
    let mut command = Command::new(&spec.program);
    command.args(&spec.args_prefix);
    command.args(&args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
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
    let parsed: serde_json::Value = serde_json::from_str(json_line.trim())
        .map_err(|error| format!("failed to parse RapidOCR JSON: {error}; output: {json_line}"))?;
    Ok((parsed, spec.kind))
}

pub fn run_rapidocr_probe(
    app: &tauri::AppHandle,
    model_version: &str,
) -> Result<RapidOcrRunnerOutput, String> {
    let model_root = rapid_ocr_model_root(app);
    run_rapidocr_json(
        app,
        vec![
            "--probe".to_string(),
            "--model-version".to_string(),
            model_version.to_string(),
            "--model-root".to_string(),
            model_root.to_string_lossy().to_string(),
        ],
    )
}

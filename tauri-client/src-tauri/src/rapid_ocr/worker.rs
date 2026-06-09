use crate::*;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::runner::{
    rapid_ocr_missing_model_files, rapid_ocr_model_root, rapid_ocr_model_version,
    resolve_rapidocr_command, RapidOcrCommandSpec, RapidOcrRunnerOutput,
};

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

pub fn rapid_ocr_worker_enabled() -> bool {
    config_value_bool("rapidOcrWorkerEnabled").unwrap_or(true)
}

pub fn spawn_rapid_ocr_worker_process(
    app: &tauri::AppHandle,
) -> Result<RapidOcrWorkerProcess, String> {
    let spec = resolve_rapidocr_command(app)?;
    let mut command = std::process::Command::new(&spec.program);
    command.args(&spec.args_prefix);
    command.arg("--worker");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

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
    small_text_retry: bool,
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
                "smallTextRetry": small_text_retry
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

pub fn rapid_ocr_worker_status_value() -> serde_json::Value {
    let state = rapid_ocr_worker_state();
    let Ok(mut guard) = state.lock() else {
        return serde_json::json!({
            "enabled": rapid_ocr_worker_enabled(),
            "running": false,
            "lastError": "RapidOCR worker lock failed."
        });
    };
    let Some(worker) = guard.as_mut() else {
        return serde_json::json!({
            "enabled": rapid_ocr_worker_enabled(),
            "running": false,
            "lastError": null
        });
    };

    match worker.child.try_wait() {
        Ok(Some(status)) => {
            *guard = None;
            serde_json::json!({
                "enabled": rapid_ocr_worker_enabled(),
                "running": false,
                "lastError": format!("RapidOCR worker exited with status {status}.")
            })
        }
        Ok(None) => {
            let pid = worker.child.id();
            match rapid_ocr_worker_request_value(worker, "status", serde_json::json!({})) {
                Ok(status) => serde_json::json!({
                    "enabled": rapid_ocr_worker_enabled(),
                    "running": true,
                    "pid": pid,
                    "runnerKind": worker.spec.kind.clone(),
                    "runnerPath": worker.spec.program.to_string_lossy().to_string(),
                    "lastError": worker.last_error,
                    "status": status,
                    "cachedEngines": status.get("cachedEngines").cloned().unwrap_or_else(|| serde_json::json!([]))
                }),
                Err(error) => {
                    worker.last_error = Some(error.clone());
                    serde_json::json!({
                        "enabled": rapid_ocr_worker_enabled(),
                        "running": true,
                        "pid": pid,
                        "runnerKind": worker.spec.kind.clone(),
                        "runnerPath": worker.spec.program.to_string_lossy().to_string(),
                        "lastError": error
                    })
                }
            }
        }
        Err(error) => serde_json::json!({
            "enabled": rapid_ocr_worker_enabled(),
            "running": false,
            "lastError": format!("RapidOCR worker status check failed: {error}")
        }),
    }
}

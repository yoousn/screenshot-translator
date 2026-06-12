pub mod runner;
pub mod worker;

pub use runner::*;
pub use worker::*;

use std::fs;
use std::time::{Duration, Instant};
use tauri::Emitter;

const RAPIDOCR_DOCS_MODEL_LIST_URL: &str =
    "https://rapidai.github.io/RapidOCRDocs/main/model_list/";
const RAPIDOCR_MODELSCOPE_URL: &str = "https://www.modelscope.cn/models/RapidAI/RapidOCR";

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
    small_text_retry: Option<bool>,
) -> Result<Vec<OcrBlock>, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15000).clamp(500, 60000));
    let worker_timeout = timeout
        .checked_sub(Duration::from_millis(250))
        .filter(|value| *value >= Duration::from_millis(500))
        .unwrap_or(timeout);
    let task = tokio::task::spawn_blocking(move || {
        run_local_ocr_sync(
            app,
            image_base64,
            executable_path,
            small_text_retry,
            Some(worker_timeout),
        )
    });
    match tokio::time::timeout(timeout, task).await {
        Ok(joined) => joined.map_err(|e| format!("Local OCR task failed: {}", e))?,
        Err(_) => Err(format!("Local OCR timed out ({} ms)", timeout.as_millis())),
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
pub async fn install_rapid_ocr_models(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    emit_rapid_ocr_install_progress(
        &app,
        "准备安装",
        "正在检查模型目录和 RapidOCR runner。",
        5,
        "active",
    );
    let task_app = app.clone();
    let result = tokio::task::spawn_blocking(move || install_rapid_ocr_models_sync(&task_app))
        .await
        .map_err(|error| format!("RapidOCR model install task failed: {error}"))?;
    if let Err(error) = &result {
        emit_rapid_ocr_install_progress(&app, "安装失败", error, 100, "exception");
    }
    result
}

fn install_rapid_ocr_models_sync(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    let model_root = rapid_ocr_model_install_root(app);
    fs::create_dir_all(&model_root).map_err(|error| {
        format!(
            "failed to create RapidOCR model directory {}: {error}",
            model_root.display()
        )
    })?;

    let _ = stop_rapid_ocr_worker_internal(1500);
    let model_root_arg = model_root.to_string_lossy().to_string();
    emit_rapid_ocr_install_progress(
        app,
        "下载并初始化模型",
        "RapidOCR 正在从官方 ModelScope 源补齐缺失的 ONNX 模型。",
        15,
        "active",
    );
    let warm_result = run_rapidocr_command_value(
        app,
        vec![
            "--warm-models".to_string(),
            "--model-root".to_string(),
            model_root_arg.clone(),
        ],
    )?;
    emit_rapid_ocr_install_progress(
        app,
        "验证 Rapid OCR V5",
        "模型下载阶段已完成，正在验证默认 V5 模型。",
        80,
        "active",
    );
    let probe_v5 = run_rapidocr_command_value(
        app,
        vec![
            "--probe".to_string(),
            "--model-version".to_string(),
            "v5".to_string(),
            "--model-root".to_string(),
            model_root_arg.clone(),
        ],
    )?;
    emit_rapid_ocr_install_progress(
        app,
        "验证 Rapid OCR V4",
        "V5 已完成，正在验证兼容 V4 模型。",
        90,
        "active",
    );
    let probe_v4 = run_rapidocr_command_value(
        app,
        vec![
            "--probe".to_string(),
            "--model-version".to_string(),
            "v4".to_string(),
            "--model-root".to_string(),
            model_root_arg,
        ],
    )?;

    let missing_v5 = rapid_ocr_missing_model_files(&model_root, "v5");
    let missing_v4 = rapid_ocr_missing_model_files(&model_root, "v4");
    let ok = missing_v5.is_empty()
        && missing_v4.is_empty()
        && probe_v5
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            == "success"
        && probe_v4
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            == "success";

    let result = serde_json::json!({
        "ok": ok,
        "modelRoot": model_root.to_string_lossy().to_string(),
        "source": {
            "name": "RapidOCR official ModelScope model repository",
            "docsUrl": RAPIDOCR_DOCS_MODEL_LIST_URL,
            "modelRepositoryUrl": RAPIDOCR_MODELSCOPE_URL,
            "package": "rapidocr==3.8.1"
        },
        "warmResult": warm_result,
        "probeResults": {
            "v5": probe_v5,
            "v4": probe_v4
        },
        "missingModelFiles": {
            "v5": missing_v5,
            "v4": missing_v4
        },
        "elapsedMs": started.elapsed().as_millis()
    });
    if ok {
        emit_rapid_ocr_install_progress(
            app,
            "安装完成",
            "Rapid OCR V5 / V4 模型均已通过验证。",
            100,
            "success",
        );
    } else {
        emit_rapid_ocr_install_progress(
            app,
            "文件检查未通过",
            "下载已结束，但仍有模型文件缺失或模型验证失败。",
            100,
            "exception",
        );
    }
    Ok(result)
}

fn emit_rapid_ocr_install_progress(
    app: &tauri::AppHandle,
    phase: &str,
    detail: &str,
    percent: u8,
    status: &str,
) {
    let _ = app.emit(
        "rapidocr-model-install-progress",
        serde_json::json!({
            "phase": phase,
            "detail": detail,
            "percent": percent,
            "status": status,
        }),
    );
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

#[tauri::command]
pub async fn start_rapid_ocr_worker(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || start_rapid_ocr_worker_sync(&app, true))
        .await
        .map_err(|error| format!("RapidOCR worker start task failed: {error}"))?
}

use crate::*;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub static STARTUP_READINESS_CACHE: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();

pub fn get_startup_readiness_cache() -> &'static Mutex<Option<serde_json::Value>> {
    STARTUP_READINESS_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn cache_startup_readiness_snapshot(snapshot: serde_json::Value) {
    if let Ok(mut guard) = get_startup_readiness_cache().lock() {
        *guard = Some(snapshot);
    }
}

pub static LAST_TRANSLATION_DIAGNOSTICS: OnceLock<Mutex<Option<serde_json::Value>>> =
    OnceLock::new();

pub fn get_last_translation_diagnostics() -> &'static Mutex<Option<serde_json::Value>> {
    LAST_TRANSLATION_DIAGNOSTICS.get_or_init(|| Mutex::new(None))
}

#[tauri::command]
pub fn set_last_translation_diagnostics(payload: serde_json::Value) -> Result<(), String> {
    if let Ok(mut guard) = get_last_translation_diagnostics().lock() {
        *guard = Some(payload);
    }
    Ok(())
}

pub fn startup_diagnostics_probe_path() -> PathBuf {
    std::env::temp_dir()
        .join("ysn_screenshot_translator")
        .join("startup_status.json")
}

pub fn write_startup_diagnostics_probe(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let path = startup_diagnostics_probe_path();
    let parent = path
        .parent()
        .ok_or_else(|| "failed to resolve startup diagnostics directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("create startup diagnostics directory failed: {}", e))?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unavailable".to_string());
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "generatedAt": chrono::Local::now().to_rfc3339(),
        "processId": std::process::id(),
        "app": {
            "name": "YSN Screenshot Translator",
            "version": env!("CARGO_PKG_VERSION"),
            "appDataDir": app_data_dir,
        },
        "diagnostics": {
            "pending": true,
            "message": "Full diagnostics are generated asynchronously from the app diagnostics panel."
        },
    });
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize startup diagnostics failed: {}", e))?;
    fs::write(&path, body).map_err(|e| format!("write startup diagnostics failed: {}", e))?;
    Ok(path)
}

pub(crate) fn build_diagnostic_readiness_by_module(
    ocr_runtime: &serde_json::Value,
    recording: &serde_json::Value,
) -> serde_json::Value {
    let ocr_steps = ocr_runtime["readinessSteps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let ocr_ready_steps = ocr_steps
        .iter()
        .filter(|step| step["ready"].as_bool().unwrap_or(false))
        .count();
    let first_blocked_ocr_step = ocr_steps
        .iter()
        .find(|step| !step["ready"].as_bool().unwrap_or(false))
        .cloned()
        .unwrap_or_else(|| serde_json::json!(null));
    let ffmpeg_ready = recording["ffmpegFound"].as_bool().unwrap_or(false);
    let audio_ready = recording["audioDevices"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let recording_steps = serde_json::json!([
        {
            "id": "ffmpeg",
            "ready": ffmpeg_ready,
            "label": "FFmpeg executable",
            "nextAction": if ffmpeg_ready { "detect-audio-devices" } else { "download-or-choose-ffmpeg" }
        },
        {
            "id": "audio-devices",
            "ready": audio_ready,
            "label": "Recording audio devices",
            "nextAction": if audio_ready { "ready" } else { "recheck-recording-audio-devices" }
        }
    ]);
    let recording_ready_steps = recording_steps
        .as_array()
        .map(|steps| {
            steps
                .iter()
                .filter(|step| step["ready"].as_bool().unwrap_or(false))
                .count()
        })
        .unwrap_or(0);
    let first_blocked_recording_step = recording_steps
        .as_array()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| !step["ready"].as_bool().unwrap_or(false))
                .cloned()
        })
        .unwrap_or_else(|| serde_json::json!(null));

    serde_json::json!({
        "ocrRuntime": {
            "ready": ocr_runtime["ready"].as_bool().unwrap_or(false),
            "readySteps": ocr_ready_steps,
            "totalSteps": ocr_steps.len(),
            "firstBlockedStep": first_blocked_ocr_step,
            "steps": ocr_steps,
        },
        "recording": {
            "ready": ffmpeg_ready && audio_ready,
            "readySteps": recording_ready_steps,
            "totalSteps": recording_steps.as_array().map(|steps| steps.len()).unwrap_or(0),
            "firstBlockedStep": first_blocked_recording_step,
            "steps": recording_steps,
        }
    })
}

pub(crate) fn build_startup_readiness_snapshot(app: tauri::AppHandle) -> serde_json::Value {
    let checked_at = chrono::Local::now().to_rfc3339();
    let rapid_ocr = get_rapid_ocr_status(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ready": false,
            "runtime": "rapidocr",
            "error": error,
        })
    });
    let recording = get_recording_info_sync(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ready": false,
            "error": error,
        })
    });
    let ocr_ready = rapid_ocr["ready"].as_bool().unwrap_or(false);
    let ffmpeg_ready = recording["ffmpegFound"].as_bool().unwrap_or(false);
    let audio_ready = recording["audioDevices"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let recording_ready = ffmpeg_ready && audio_ready;
    let ready = ocr_ready && recording_ready;

    serde_json::json!({
        "checkedAt": checked_at,
        "ready": ready,
        "rapidOcr": rapid_ocr,
        "recording": recording,
        "pending": false,
    })
}

pub fn get_diagnostics_report_sync(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let generated_at = chrono::Local::now().to_rfc3339();
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unavailable".to_string());
    let startup_probe_path = startup_diagnostics_probe_path()
        .to_string_lossy()
        .to_string();
    let recording = get_recording_info_sync(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ok": false,
            "error": error,
        })
    });
    let ocr_runtime = get_rapid_ocr_status(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ready": false,
            "error": error,
        })
    });
    let shortcut_status = serde_json::json!({
        "registered": true,
        "note": "Shortcut registration errors are surfaced during app startup; detailed shortcut state is managed in AppShortcutStatus."
    });

    let last_translation = get_last_translation_diagnostics()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(None)
        .unwrap_or_else(|| serde_json::json!(null));

    let mut issues = Vec::new();
    if !ocr_runtime["ready"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "ocrRuntime",
            "code": "rapidocr-not-ready",
            "message": "RapidOCR text recognition is not ready.",
            "nextAction": "Open the text recognition panel and run the RapidOCR check."
        }));
    }
    if !ocr_runtime["runnerReady"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "warning",
            "module": "ocrRuntime",
            "code": "rapidocr-runner-missing",
            "message": "RapidOCR runner is not available.",
            "nextAction": "Install the RapidOCR package for development or bundle rapidocr-runner.exe for release."
        }));
    }
    if ocr_runtime["lastError"].as_str().is_some() {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "ocrRuntime",
            "code": "rapidocr-probe-failed",
            "message": "RapidOCR probe failed.",
            "nextAction": "Initialize local OCR and reinstall the model/runtime package if needed."
        }));
    }
    if !recording["ffmpegFound"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "recording",
            "code": "ffmpeg-not-found",
            "message": "FFmpeg was not found, so video recording cannot be fully ready.",
            "nextAction": "Download FFmpeg from the video recording dependency panel or choose ffmpeg.exe manually."
        }));
    }
    if recording["audioDevices"]
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        issues.push(serde_json::json!({
            "severity": "warning",
            "module": "recording",
            "code": "audio-devices-empty",
            "message": "No FFmpeg audio devices were detected.",
            "nextAction": "Re-check recording dependency after FFmpeg is installed; verify Windows audio devices if needed."
        }));
    }

    let critical_count = issues
        .iter()
        .filter(|issue| issue["severity"].as_str() == Some("error"))
        .count();
    let mut issues_by_module = std::collections::BTreeMap::<String, usize>::new();
    for issue in &issues {
        if let Some(module) = issue["module"].as_str() {
            *issues_by_module.entry(module.to_string()).or_insert(0) += 1;
        }
    }
    let readiness_by_module = build_diagnostic_readiness_by_module(&ocr_runtime, &recording);

    Ok(serde_json::json!({
        "schemaVersion": 2,
        "generatedAt": generated_at,
        "app": {
            "name": "YSN Screenshot Translator",
            "version": env!("CARGO_PKG_VERSION"),
            "appDataDir": app_data_dir,
            "startupProbePath": startup_probe_path,
        },
        "health": {
            "ready": critical_count == 0,
            "criticalCount": critical_count,
            "issueCount": issues.len(),
            "issuesByModule": issues_by_module,
            "readinessByModule": readiness_by_module,
            "issues": issues,
        },
        "ocrRuntime": ocr_runtime,
        "recording": recording,
        "shortcuts": shortcut_status,
        "lastTranslation": last_translation,
        "recovery": {
            "ocr": "Open the text recognition panel, choose the local OCR model, then initialize and apply it.",
            "recording": "Install or choose ffmpeg.exe, then re-check video recording dependency.",
            "shortcuts": "If global shortcuts fail, restart the app or change conflicting hotkeys in settings."
        }
    }))
}

#[tauri::command]
pub async fn get_diagnostics_report(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || get_diagnostics_report_sync(app))
        .await
        .map_err(|error| format!("diagnostics report task failed: {error}"))?
}

#[tauri::command]
pub fn get_startup_diagnostics_probe_path() -> Result<String, String> {
    Ok(startup_diagnostics_probe_path()
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn get_startup_readiness_snapshot() -> Result<serde_json::Value, String> {
    let snapshot = get_startup_readiness_cache()
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    Ok(snapshot.unwrap_or_else(|| {
        serde_json::json!({
            "checkedAt": null,
            "rapidOcr": null,
            "recording": null,
            "pending": true,
        })
    }))
}

#[tauri::command]
pub async fn run_startup_readiness_probe(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let snapshot = tokio::task::spawn_blocking(move || build_startup_readiness_snapshot(app))
        .await
        .map_err(|error| format!("startup readiness probe task failed: {error}"))?;
    cache_startup_readiness_snapshot(snapshot.clone());
    Ok(snapshot)
}

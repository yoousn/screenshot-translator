use crate::*;
use std::fs;
use std::path::{Path, PathBuf};

pub fn app_data_dir() -> PathBuf {
    let base_dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|_| dirs::data_local_dir().ok_or(()))
        .or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|p| PathBuf::from(p).join("AppData").join("Local"))
        })
        .unwrap_or_else(|_| {
            eprintln!("Warning: Failed to resolve local app data directory, falling back to current directory");
            PathBuf::from(".")
        });
    base_dir.join("ScreenshotTranslator")
}

pub fn cleanup_temp_files() {
    let _ = stop_recording_internal(1500, true);
    let _ = stop_rapid_ocr_worker_internal(1500);
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    let mut legacy_path = app_data_dir();
    legacy_path.push("fullscreen_temp.jpg");
    if legacy_path.exists() {
        let _ = fs::remove_file(&legacy_path);
    }
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    if cropped_path.exists() {
        let _ = fs::remove_file(&cropped_path);
    }
}

pub fn sanitize_tag(tag: &str) -> String {
    let safe: String = tag
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "latest".to_string()
    } else {
        safe
    }
}

pub fn ensure_writable_dir(preferred: PathBuf) -> PathBuf {
    if fs::create_dir_all(&preferred).is_ok() {
        let probe = preferred.join(".write-test");
        if fs::write(&probe, b"ok").is_ok() {
            let _ = fs::remove_file(probe);
            return preferred;
        }
    }
    let mut fallback = app_data_dir();
    fallback.push("ffmpeg");
    fallback
}

pub fn repo_root_from_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|client_root| client_root.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

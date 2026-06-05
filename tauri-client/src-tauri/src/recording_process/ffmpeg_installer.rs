use std::sync::{Mutex, OnceLock};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Cursor;
use tauri::Manager;
use tauri::Emitter;
use futures_util::StreamExt;
use crate::*;

use super::device_detector::hidden_ffmpeg_command;

pub fn ffmpeg_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    use tauri::path::BaseDirectory;
    let mut candidates = Vec::new();

    if let Some(path) = config_value_string("recordingFfmpegPath") {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(path) = std::env::var("FFMPEG_PATH") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path.trim()));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("tools").join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("plugins").join("ffmpeg").join("ffmpeg.exe"));
        }
    }

    if let Ok(path) = app
        .path()
        .resolve("resources/ffmpeg/ffmpeg.exe", BaseDirectory::Resource)
    {
        candidates.push(path);
    }

    let mut app_ffmpeg = app_data_dir();
    app_ffmpeg.push("ffmpeg");
    app_ffmpeg.push("ffmpeg.exe");
    candidates.push(app_ffmpeg);
    candidates.push(PathBuf::from("ffmpeg"));
    candidates
}

pub fn emit_ffmpeg_progress(
    app: &tauri::AppHandle,
    phase: &str,
    downloaded: u64,
    total: Option<u64>,
    percent: u8,
) {
    let _ = app.emit(
        "ffmpeg-download-progress",
        serde_json::json!({
            "phase": phase,
            "downloaded": downloaded,
            "total": total,
            "percent": percent,
        }),
    );
}

pub fn default_ffmpeg_install_dir() -> PathBuf {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            return dir.join("ffmpeg");
        }
    }
    let mut dir = app_data_dir();
    dir.push("ffmpeg");
    dir
}

pub fn extract_ffmpeg_exe_from_zip(
    bytes: &[u8],
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Read ffmpeg archive failed: {}", e))?;
    fs::create_dir_all(install_dir)
        .map_err(|e| format!("Create ffmpeg directory failed: {}", e))?;
    let target = install_dir.join("ffmpeg.exe");
    let mut found = false;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Read ffmpeg archive entry failed: {}", e))?;
        if !file
            .name()
            .replace('\\', "/")
            .to_ascii_lowercase()
            .ends_with("/bin/ffmpeg.exe")
            && !file.name().eq_ignore_ascii_case("ffmpeg.exe")
        {
            continue;
        }
        let mut out =
            fs::File::create(&target).map_err(|e| format!("Create ffmpeg.exe failed: {}", e))?;
        std::io::copy(&mut file, &mut out)
            .map_err(|e| format!("Extract ffmpeg.exe failed: {}", e))?;
        found = true;
        break;
    }
    if !found {
        return Err("ffmpeg.exe was not found in the archive".to_string());
    }
    Ok(target)
}

pub fn find_ffmpeg_executable(app: &tauri::AppHandle) -> Option<PathBuf> {
    for candidate in ffmpeg_candidates(app) {
        if candidate.to_string_lossy().eq_ignore_ascii_case("ffmpeg") {
            if hidden_ffmpeg_command(Path::new("ffmpeg"))
                .arg("-version")
                .output()
                .is_ok()
            {
                return Some(candidate);
            }
        } else if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[tauri::command]
pub async fn get_ffmpeg_release_info() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("Create request client failed: {}", e))?;
    let release = client
        .get("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Check ffmpeg release failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Read ffmpeg release response failed: {}", e))?
        .json::<GithubReleaseInfo>()
        .await
        .map_err(|e| format!("Parse ffmpeg release failed: {}", e))?;

    let asset = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".zip")
                && name.contains("win64")
                && name.contains("gpl")
                && !name.contains("shared")
        })
        .or_else(|| {
            release.assets.iter().find(|asset| {
                let name = asset.name.to_ascii_lowercase();
                name.ends_with(".zip") && name.contains("win64") && !name.contains("shared")
            })
        })
        .ok_or_else(|| {
            "No Windows x64 ffmpeg zip asset found in the official release".to_string()
        })?;

    Ok(serde_json::json!({
        "tag": release.tag_name,
        "pageUrl": release.html_url,
        "assetName": asset.name,
        "downloadUrl": asset.browser_download_url,
        "size": asset.size,
        "installDir": default_ffmpeg_install_dir().to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn download_ffmpeg_release(
    app: tauri::AppHandle,
    url: String,
    tag: String,
) -> Result<serde_json::Value, String> {
    let allowed = [
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/",
        "https://objects.githubusercontent.com/github-production-release-asset-",
    ];
    if !allowed.iter().any(|prefix| url.starts_with(prefix))
        || !url.to_ascii_lowercase().ends_with(".zip")
    {
        return Err(
            "Please choose an official Windows zip from BtbN/FFmpeg-Builds GitHub Releases"
                .to_string(),
        );
    }

    emit_ffmpeg_progress(&app, "Preparing", 0, None, 1);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("Create download client failed: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download ffmpeg failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Download ffmpeg failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read ffmpeg download stream failed: {}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let percent = total
            .map(|value| ((downloaded as f64 / value.max(1) as f64) * 80.0).round() as u8)
            .unwrap_or(10)
            .clamp(1, 80);
        emit_ffmpeg_progress(&app, "Downloading", downloaded, total, percent);
    }

    let safe_tag = sanitize_tag(&tag);
    let mut download_dir = app_data_dir();
    download_dir.push("ffmpeg");
    download_dir.push("downloads");
    fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Create ffmpeg download directory failed: {}", e))?;
    let archive_path = download_dir.join(format!("ffmpeg-{}.zip", safe_tag));
    fs::write(&archive_path, &bytes).map_err(|e| format!("Save ffmpeg archive failed: {}", e))?;

    emit_ffmpeg_progress(&app, "Installing", downloaded, total, 85);
    let install_dir = ensure_writable_dir(default_ffmpeg_install_dir());
    let exe_path = extract_ffmpeg_exe_from_zip(&bytes, &install_dir)?;
    let _ = fs::remove_file(&archive_path);
    emit_ffmpeg_progress(&app, "完成", downloaded, total, 100);

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": install_dir.to_string_lossy().to_string(),
        "bytes": bytes.len(),
    }))
}

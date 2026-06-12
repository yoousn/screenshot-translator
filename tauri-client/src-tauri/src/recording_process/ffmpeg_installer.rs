use crate::*;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tauri::Manager;

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

fn extract_ffmpeg_exe_from_zip_reader<R: Read + Seek>(
    reader: R,
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
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

pub fn extract_ffmpeg_exe_from_zip(
    bytes: &[u8],
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    extract_ffmpeg_exe_from_zip_reader(Cursor::new(bytes), install_dir)
}

pub fn extract_ffmpeg_exe_from_zip_file(
    archive_path: &std::path::Path,
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let file =
        fs::File::open(archive_path).map_err(|e| format!("Open ffmpeg archive failed: {}", e))?;
    extract_ffmpeg_exe_from_zip_reader(file, install_dir)
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

pub fn ffmpeg_asset_name_from_url(url: &str) -> Option<String> {
    let clean_url = url.split(['?', '#']).next().unwrap_or(url);
    clean_url
        .rsplit('/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn ffmpeg_checksum_url_for_download(download_url: &str) -> Option<String> {
    let clean_url = download_url
        .split(['?', '#'])
        .next()
        .unwrap_or(download_url);
    let marker = "/releases/download/";
    if !clean_url.starts_with("https://github.com/BtbN/FFmpeg-Builds/releases/download/")
        || !clean_url.contains(marker)
    {
        return None;
    }
    let base = clean_url.rsplit_once('/')?.0;
    Some(format!("{}/checksums.sha256", base))
}

pub fn parse_ffmpeg_sha256_manifest(manifest: &str, asset_name: &str) -> Option<String> {
    let expected_name = asset_name.trim();
    manifest.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let mut parts = trimmed.split_whitespace();
        let hash = parts.next()?.trim_start_matches('*');
        let file_name = parts.next()?.trim_start_matches('*');
        if hash.len() == 64
            && hash.chars().all(|ch| ch.is_ascii_hexdigit())
            && file_name.eq_ignore_ascii_case(expected_name)
        {
            Some(hash.to_ascii_lowercase())
        } else {
            None
        }
    })
}

fn ffmpeg_checksum_lock_path() -> PathBuf {
    let mut path = app_data_dir();
    path.push("ffmpeg");
    path.push("ffmpeg_release.lock");
    path
}

fn read_ffmpeg_checksum_lock(asset_name: &str) -> Option<String> {
    let content = fs::read_to_string(ffmpeg_checksum_lock_path()).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let locked_asset = value.get("assetName")?.as_str()?;
    let locked_sha = value.get("sha256")?.as_str()?;
    if locked_asset.eq_ignore_ascii_case(asset_name)
        && locked_sha.len() == 64
        && locked_sha.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        Some(locked_sha.to_ascii_lowercase())
    } else {
        None
    }
}

fn write_ffmpeg_checksum_lock(tag: &str, asset_name: &str, download_url: &str, sha256: &str) {
    let path = ffmpeg_checksum_lock_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        path,
        serde_json::to_string_pretty(&serde_json::json!({
            "tag": tag,
            "assetName": asset_name,
            "downloadUrl": download_url,
            "sha256": sha256,
            "verifiedAt": chrono::Utc::now().to_rfc3339(),
        }))
        .unwrap_or_default(),
    );
}

async fn fetch_text(client: &reqwest::Client, url: &str, label: &str) -> Result<String, String> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download {} failed: {}", label, e))?
        .error_for_status()
        .map_err(|e| format!("Read {} response failed: {}", label, e))?
        .text()
        .await
        .map_err(|e| format!("Read {} body failed: {}", label, e))
}

async fn resolve_expected_ffmpeg_sha256(
    client: &reqwest::Client,
    download_url: &str,
    asset_name: &str,
) -> Result<(String, Option<String>), String> {
    if let Some(checksum_url) = ffmpeg_checksum_url_for_download(download_url) {
        let manifest = fetch_text(client, &checksum_url, "ffmpeg checksum").await?;
        if let Some(sha256) = parse_ffmpeg_sha256_manifest(&manifest, asset_name) {
            return Ok((sha256, Some(checksum_url)));
        }
        return Err(format!(
            "Official ffmpeg checksum manifest does not contain {}",
            asset_name
        ));
    }
    if let Some(sha256) = read_ffmpeg_checksum_lock(asset_name) {
        return Ok((sha256, None));
    }
    Err("FFmpeg checksum was unavailable and no trusted local lock exists.".to_string())
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
    let (sha256, checksum_url) =
        resolve_expected_ffmpeg_sha256(&client, &asset.browser_download_url, &asset.name).await?;

    Ok(serde_json::json!({
        "tag": release.tag_name,
        "pageUrl": release.html_url,
        "assetName": asset.name,
        "downloadUrl": asset.browser_download_url,
        "checksumUrl": checksum_url,
        "sha256": sha256,
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
    let asset_name = ffmpeg_asset_name_from_url(&url)
        .ok_or_else(|| "FFmpeg download URL does not contain an asset name".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("Create download client failed: {}", e))?;
    let (expected_sha256, _checksum_url) =
        resolve_expected_ffmpeg_sha256(&client, &url, &asset_name).await?;

    let safe_tag = sanitize_tag(&tag);
    let mut download_dir = app_data_dir();
    download_dir.push("ffmpeg");
    download_dir.push("downloads");
    fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Create ffmpeg download directory failed: {}", e))?;
    let archive_path = download_dir.join(format!("ffmpeg-{}.zip", safe_tag));
    let temp_archive_path = archive_path.with_extension("zip.part");
    let mut archive_file = fs::File::create(&temp_archive_path)
        .map_err(|e| format!("Create temporary ffmpeg archive failed: {}", e))?;

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
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read ffmpeg download stream failed: {}", e))?;
        downloaded += chunk.len() as u64;
        hasher.update(&chunk);
        archive_file
            .write_all(&chunk)
            .map_err(|e| format!("Write ffmpeg archive stream failed: {}", e))?;
        let percent = total
            .map(|value| ((downloaded as f64 / value.max(1) as f64) * 80.0).round() as u8)
            .unwrap_or(10)
            .clamp(1, 80);
        emit_ffmpeg_progress(&app, "Downloading", downloaded, total, percent);
    }
    archive_file
        .flush()
        .map_err(|e| format!("Flush ffmpeg archive failed: {}", e))?;
    drop(archive_file);

    let actual_sha256 = format!("{:x}", hasher.finalize());
    if actual_sha256 != expected_sha256 {
        let _ = fs::remove_file(&temp_archive_path);
        return Err(format!(
            "FFmpeg checksum mismatch for {}: expected {}, got {}",
            asset_name, expected_sha256, actual_sha256
        ));
    }
    let _ = fs::remove_file(&archive_path);
    fs::rename(&temp_archive_path, &archive_path)
        .map_err(|e| format!("Save verified ffmpeg archive failed: {}", e))?;
    write_ffmpeg_checksum_lock(&tag, &asset_name, &url, &actual_sha256);

    emit_ffmpeg_progress(&app, "Installing", downloaded, total, 85);
    let install_dir = ensure_writable_dir(default_ffmpeg_install_dir());
    let exe_path = extract_ffmpeg_exe_from_zip_file(&archive_path, &install_dir)?;
    let _ = fs::remove_file(&archive_path);
    emit_ffmpeg_progress(&app, "完成", downloaded, total, 100);

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": install_dir.to_string_lossy().to_string(),
        "bytes": downloaded,
        "sha256": actual_sha256,
    }))
}

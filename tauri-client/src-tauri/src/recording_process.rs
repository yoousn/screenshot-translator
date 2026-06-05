use std::sync::{Mutex, OnceLock};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use tauri::Manager;
use tauri::Emitter;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use crate::*;

#[derive(Debug, Deserialize)]
pub struct RecordingOptions {
    fps: Option<u32>,
    resolution: Option<String>,
    audio_mode: Option<String>,
    mic_device: Option<String>,
    system_audio_device: Option<String>,
    output_dir: Option<String>,
    region_x: Option<i32>,
    region_y: Option<i32>,
    region_w: Option<i32>,
    region_h: Option<i32>,
}
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
pub fn parse_quoted_audio_devices(
    output: &str,
    audio_marker_required: bool,
    prefix: Option<&str>,
) -> Vec<String> {
    let mut devices = Vec::new();
    for line in output.lines() {
        if audio_marker_required && !line.contains("(audio)") {
            continue;
        }
        if let Some(first_quote) = line.find('"') {
            if let Some(second_quote) = line[first_quote + 1..].find('"') {
                let name = line[first_quote + 1..first_quote + 1 + second_quote].trim();
                if !name.is_empty() {
                    let value = match prefix {
                        Some(prefix) => format!("{}{}", prefix, name),
                        None => name.to_string(),
                    };
                    if !devices.contains(&value) {
                        devices.push(value);
                    }
                }
            }
        }
    }
    devices
}
pub fn ffmpeg_supports_input_format(formats_output: &str, format_name: &str) -> bool {
    formats_output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("D") && trimmed.split_whitespace().nth(1) == Some(format_name)
    })
}
pub fn hidden_ffmpeg_command(ffmpeg_path: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
pub fn ffmpeg_input_formats(ffmpeg_path: &Path) -> String {
    hidden_ffmpeg_command(ffmpeg_path)
        .args(["-hide_banner", "-formats"])
        .output()
        .map(|out| {
            format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
        .unwrap_or_default()
}
pub fn collect_ffmpeg_audio_devices(ffmpeg_path: &Path) -> Vec<String> {
    let mut devices = Vec::new();
    let input_formats = ffmpeg_input_formats(ffmpeg_path);
    if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
    {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        devices.extend(parse_quoted_audio_devices(&combined, true, None));
    }
    if ffmpeg_supports_input_format(&input_formats, "wasapi") {
        if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
            .args([
                "-hide_banner",
                "-list_devices",
                "true",
                "-f",
                "wasapi",
                "-i",
                "dummy",
            ])
            .output()
        {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            devices.extend(parse_quoted_audio_devices(
                &combined,
                false,
                Some("wasapi:"),
            ));
        }
        if !devices.contains(&"wasapi:default".to_string()) {
            devices.push("wasapi:default".to_string());
        }
    }
    devices
}
#[tauri::command]
pub fn get_recording_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let _ = cleanup_finished_recording_process()?;
    let ffmpeg = find_ffmpeg_executable(&app);
    let is_recording = get_recording_process()
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    let audio_devices = if let Some(ffmpeg_path) = &ffmpeg {
        collect_ffmpeg_audio_devices(ffmpeg_path)
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "ffmpegFound": ffmpeg.is_some(),
        "ffmpegPath": ffmpeg.map(|path| path.to_string_lossy().to_string()),
        "isRecording": is_recording,
        "audioDevices": audio_devices,
    }))
}
pub fn recording_temp_dir() -> PathBuf {
    let mut dir = app_data_dir();
    dir.push("recordings");
    dir
}
pub fn default_recording_output_dir() -> PathBuf {
    dirs::video_dir().unwrap_or_else(app_data_dir).join("YSN")
}
pub fn timestamped_recording_file_name() -> String {
    let now = chrono::Local::now();
    format!("YSN_{}.mp4", now.format("%Y%m%d_%H%M%S"))
}
pub fn unique_recording_output_path() -> Result<PathBuf, String> {
    let dir = default_recording_output_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create recording directory failed: {}", e))?;
    let base = timestamped_recording_file_name();
    let path = dir.join(&base);
    if !path.exists() {
        return Ok(path);
    }
    let stem = base.trim_end_matches(".mp4");
    for index in 2..1000 {
        let candidate = dir.join(format!("{}_{}.mp4", stem, index));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("failed to create unique recording filename".to_string())
}
pub fn recording_output_path(output_dir: Option<String>) -> Result<PathBuf, String> {
    let dir = output_dir
        .filter(|value| !value.trim().is_empty())
        .map(|value| PathBuf::from(value.trim()))
        .unwrap_or_else(recording_temp_dir);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(dir.join(format!("recording_{}.mp4", millis)))
}
#[tauri::command]
pub fn open_path_in_file_manager(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is empty".to_string());
    }
    let input_path = PathBuf::from(trimmed);
    let target_path = if input_path.exists() {
        input_path
    } else {
        fs::create_dir_all(&input_path)
            .map_err(|e| format!("create directory before opening failed: {}", e))?;
        input_path
    };

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer.exe");
        if target_path.is_file() {
            command.arg(format!("/select,{}", target_path.to_string_lossy()));
        } else {
            command.arg(target_path.to_string_lossy().to_string());
        }
        command
            .spawn()
            .map_err(|e| format!("open path with Explorer failed: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&target_path)
            .spawn()
            .map_err(|e| format!("open path failed: {}", e))?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&target_path)
            .spawn()
            .map_err(|e| format!("open path failed: {}", e))?;
        Ok(())
    }
}
pub fn resolution_scale_filter(resolution: &str) -> Option<&'static str> {
    match resolution {
        "480p" => Some("scale=-2:480"),
        "720p" => Some("scale=-2:720"),
        "1080p" => Some("scale=-2:1080"),
        "original" => None,
        _ => Some("scale=-2:1080"),
    }
}
pub fn push_recording_audio_input(
    device: Option<&str>,
    label: &str,
    args: &mut Vec<String>,
) -> Result<(), String> {
    let name = device
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Please choose {} audio device", label))?;
    let trimmed = name.trim();
    if let Some(wasapi_device) = trimmed.strip_prefix("wasapi:") {
        args.extend([
            "-f".to_string(),
            "wasapi".to_string(),
            "-i".to_string(),
            wasapi_device.trim().to_string(),
        ]);
    } else {
        let dshow_device = trimmed.strip_prefix("dshow:").unwrap_or(trimmed);
        args.extend([
            "-f".to_string(),
            "dshow".to_string(),
            "-i".to_string(),
            format!("audio={}", dshow_device.trim()),
        ]);
    }
    Ok(())
}
pub fn build_recording_args(
    options: &RecordingOptions,
    output_path: &Path,
) -> Result<Vec<String>, String> {
    let fps = options.fps.unwrap_or(30).clamp(1, 60).to_string();
    let resolution = options.resolution.as_deref().unwrap_or("1080p");
    let audio_mode = options.audio_mode.as_deref().unwrap_or("none");

    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-f".to_string(),
        "gdigrab".to_string(),
        "-framerate".to_string(),
        fps.clone(),
    ];
    if let (Some(x), Some(y), Some(w), Some(h)) = (
        options.region_x,
        options.region_y,
        options.region_w,
        options.region_h,
    ) {
        if w <= 0 || h <= 0 {
            return Err("Invalid recording region size".to_string());
        }
        args.extend([
            "-offset_x".to_string(),
            x.to_string(),
            "-offset_y".to_string(),
            y.to_string(),
            "-video_size".to_string(),
            format!("{}x{}", w, h),
        ]);
    }
    args.extend(["-i".to_string(), "desktop".to_string()]);

    let audio_inputs = match audio_mode {
        "none" => 0,
        "mic" => {
            push_recording_audio_input(options.mic_device.as_deref(), "microphone", &mut args)?;
            1
        }
        "system" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "绯荤粺澹伴煶",
                &mut args,
            )?;
            1
        }
        "system_mic" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "绯荤粺澹伴煶",
                &mut args,
            )?;
            push_recording_audio_input(options.mic_device.as_deref(), "microphone", &mut args)?;
            2
        }
        _ => return Err("Unknown recording audio mode".to_string()),
    };

    args.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-r".to_string(),
        fps,
    ]);
    if let Some(filter) = resolution_scale_filter(resolution) {
        args.extend(["-vf".to_string(), filter.to_string()]);
    }

    match audio_inputs {
        0 => args.push("-an".to_string()),
        1 => args.extend([
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "1:a".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "160k".to_string(),
        ]),
        2 => args.extend([
            "-filter_complex".to_string(),
            "[1:a][2:a]amix=inputs=2:duration=longest[aout]".to_string(),
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "[aout]".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "160k".to_string(),
        ]),
        _ => {}
    }
    args.push(output_path.to_string_lossy().to_string());
    Ok(args)
}
#[tauri::command]
pub async fn get_ffmpeg_release_info() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
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
        .timeout(Duration::from_secs(600))
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
    emit_ffmpeg_progress(&app, "瀹屾垚", downloaded, total, 100);

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": install_dir.to_string_lossy().to_string(),
        "bytes": bytes.len(),
    }))
}
#[tauri::command]
pub fn choose_ffmpeg_executable(current_path: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Choose ffmpeg.exe")
        .add_filter("ffmpeg", &["exe"]);
    if let Some(path) = current_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            let path_buf = PathBuf::from(trimmed);
            if let Some(parent) = path_buf.parent() {
                dialog = dialog.set_directory(parent);
            }
        }
    }
    Ok(dialog
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}
#[tauri::command]
pub fn choose_recording_output_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("Choose recording output directory");
    if let Some(dir) = current_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            dialog = dialog.set_directory(trimmed);
        }
    }
    Ok(dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}
#[tauri::command]
pub fn start_recording(app: tauri::AppHandle, options: RecordingOptions) -> Result<String, String> {
    println!("[window-trace] enter start_recording");
    let _ = cleanup_finished_recording_process()?;
    {
        let guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("Recording is already running".to_string());
        }
    }

    let ffmpeg = find_ffmpeg_executable(&app).ok_or_else(|| {
        "ffmpeg.exe was not found. Put ffmpeg.exe next to the app or choose ffmpeg.exe in settings.".to_string()
    })?;
    println!("[window-trace] start_recording ffmpeg_path={:?}", ffmpeg);
    let output_path = recording_output_path(options.output_dir.clone())?;
    println!("[window-trace] start_recording output_path={:?}", output_path);
    let args = build_recording_args(&options, &output_path)?;
    println!("[window-trace] start_recording build args finish");

    let mut cmd = hidden_ffmpeg_command(&ffmpeg);
    cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    println!("[window-trace] start_recording spawn ffmpeg before");
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg recording: {}", e))?;
    println!("[window-trace] start_recording spawn ffmpeg after");
    if let Some(status) = child
        .try_wait()
        .map_err(|e| format!("Failed to inspect ffmpeg recording process: {}", e))?
    {
        println!("[window-trace] start_recording try_wait result: exited with {}", status);
        return Err(format!("ffmpeg recording exited immediately with status {}. Check recording options, audio device, or ffmpeg version.", status));
    }
    println!("[window-trace] start_recording try_wait result: still running");
    let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Recording is already running".to_string());
    }
    *guard = Some(child);
    println!("[window-trace] start_recording RECORDING_PROCESS set Some");
    Ok(output_path.to_string_lossy().to_string())
}
pub fn stop_recording_internal(grace_ms: u64, kill_on_timeout: bool) -> Result<(), String> {
    let child = {
        let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(mut child) = child {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(b"q\n");
            let _ = stdin.flush();
        }
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(grace_ms);
        let mut exited = false;
        while start.elapsed() < timeout {
            if child
                .try_wait()
                .map_err(|e| format!("Failed to stop recording process: {}", e))?
                .is_some()
            {
                exited = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if !exited {
            if kill_on_timeout {
                let _ = child.kill();
            } else {
                let _ = child.kill();
                let _ = child.wait();
                return Err("ffmpeg did not finalize the recording segment in time".to_string());
            }
        }
        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for recording process: {}", e))?;
        if !status.success() {
            eprintln!("ffmpeg recording stopped with status {}", status);
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn stop_recording() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| stop_recording_internal(15000, false))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cancel_recording_process() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| stop_recording_internal(350, true))
        .await
        .map_err(|e| e.to_string())?
}
pub fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "\\'")
}
pub fn ffmpeg_stderr_excerpt(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let excerpt = text
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    if excerpt.trim().is_empty() {
        "(no ffmpeg stderr)".to_string()
    } else {
        excerpt
    }
}
pub fn run_ffmpeg_merge(ffmpeg: &Path, args: &[String]) -> Result<(), String> {
    let output = hidden_ffmpeg_command(ffmpeg)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to start ffmpeg merge: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{}\n{}",
            output.status,
            ffmpeg_stderr_excerpt(&output.stderr)
        ))
    }
}
#[tauri::command]
pub fn concat_recording_segments(
    app: tauri::AppHandle,
    segment_paths: Vec<String>,
) -> Result<String, String> {
    if segment_paths.is_empty() {
        return Err("no recording segments to merge".to_string());
    }
    let mut existing_segments: Vec<PathBuf> = Vec::new();
    for raw_path in &segment_paths {
        let path = PathBuf::from(raw_path.trim());
        if !path.is_file() {
            return Err(format!("recording segment does not exist: {}", raw_path));
        }
        let size = fs::metadata(&path)
            .map_err(|e| format!("read recording segment metadata failed: {}", e))?
            .len();
        if size == 0 {
            return Err(format!("recording segment is empty: {}", path.to_string_lossy()));
        }
        existing_segments.push(path);
    }
    if existing_segments.is_empty() {
        return Err("video file does not exist".to_string());
    }

    let save_path = unique_recording_output_path()?;
    if existing_segments.len() == 1 {
        fs::copy(&existing_segments[0], &save_path)
            .map_err(|e| format!("save recording failed: {}", e))?;
        return Ok(save_path.to_string_lossy().to_string());
    }

    let ffmpeg = find_ffmpeg_executable(&app)
        .ok_or_else(|| "ffmpeg.exe not found, cannot merge recording segments".to_string())?;
    let mut list_path = recording_temp_dir();
    fs::create_dir_all(&list_path)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    list_path.push(format!("concat_{}.txt", millis));
    let list_body = existing_segments
        .iter()
        .map(|path| format!("file '{}'", escape_concat_path(path)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&list_path, list_body)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;

    let copy_args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "0".to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        save_path.to_string_lossy().to_string(),
    ];
    let copy_error = match run_ffmpeg_merge(&ffmpeg, &copy_args) {
        Ok(()) => {
            let _ = fs::remove_file(&list_path);
            return Ok(save_path.to_string_lossy().to_string());
        }
        Err(error) => error,
    };
    let _ = fs::remove_file(&save_path);

    let transcode_args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        "22".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "160k".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        save_path.to_string_lossy().to_string(),
    ];
    if let Err(transcode_error) = run_ffmpeg_merge(&ffmpeg, &transcode_args) {
        let _ = fs::remove_file(&list_path);
        return Err(format!(
            "ffmpeg failed to merge recording segments.\ncopy attempt:\n{}\ntranscode fallback:\n{}",
            copy_error, transcode_error
        ));
    }
    let _ = fs::remove_file(&list_path);
    Ok(save_path.to_string_lossy().to_string())
}
#[tauri::command]
pub fn copy_file_to_clipboard(path: String) -> Result<(), String> {
    let file_path = PathBuf::from(path.trim());
    if !file_path.is_file() {
        return Err("video file does not exist".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Set-Clipboard -LiteralPath {}",
            shell_escape_powershell_single(&file_path.to_string_lossy())
        );
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .status()
            .map_err(|e| format!("failed to start clipboard command: {}", e))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!(
            "failed to copy video file to clipboard: {}",
            status
        ));
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("copying video files is not supported on this platform".to_string())
    }
}
pub fn shell_escape_powershell_single(value: &str) -> String {
    format!("'{}'", value.replace("'", "''"))
}
pub fn is_recording_temp_file(path: &Path, temp_dir: &Path) -> bool {
    let Ok(canonical_path) = fs::canonicalize(path) else {
        return false;
    };
    let Ok(canonical_temp_dir) = fs::canonicalize(temp_dir) else {
        return false;
    };
    canonical_path.starts_with(canonical_temp_dir)
        && canonical_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("mp4"))
            .unwrap_or(false)
}
#[tauri::command]
pub fn cleanup_recording_files(paths: Vec<String>) -> Result<(), String> {
    let temp_dir = recording_temp_dir();
    for path in paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path_buf = PathBuf::from(trimmed);
        if path_buf.exists() && is_recording_temp_file(&path_buf, &temp_dir) {
            let _ = fs::remove_file(path_buf);
        }
    }
    Ok(())
}
pub static RECORDING_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
pub fn get_recording_process() -> &'static Mutex<Option<Child>> {
    RECORDING_PROCESS.get_or_init(|| Mutex::new(None))
}

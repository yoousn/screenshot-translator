#[cfg(windows)]
use std::os::windows::process::CommandExt;

use arboard::{Clipboard, ImageData};
use base64::{prelude::BASE64_STANDARD, Engine};
use futures_util::StreamExt;
use screenshots::Screen;
use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tokio::time::Duration;

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
static CAPTURING: AtomicBool = AtomicBool::new(false);
static RECORDING_OVERLAY: OnceLock<Mutex<Option<NativeRecordingOverlay>>> = OnceLock::new();

static SCREENSHOT_JPEG: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
fn get_screenshot_jpeg() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_JPEG.get_or_init(|| Mutex::new(None))
}

#[derive(Clone, Copy)]
struct NativeRecordingOverlay {
    hwnd: isize,
}

unsafe impl Send for NativeRecordingOverlay {}

fn get_recording_overlay() -> &'static Mutex<Option<NativeRecordingOverlay>> {
    RECORDING_OVERLAY.get_or_init(|| Mutex::new(None))
}

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);

const DEFAULT_SCREENSHOT_HOTKEY: &str = "Alt+A";
const TRANSLATE_HOTKEY_LABEL: &str = "Alt+T";
const RECORDING_HOTKEY_LABEL: &str = "Alt+R";

fn normalize_key_code(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.len() == 1 {
        let ch = trimmed.chars().next()?.to_ascii_uppercase();
        if ch.is_ascii_alphabetic() {
            return Some(format!("Key{}", ch));
        }
        if ch.is_ascii_digit() {
            return Some(format!("Digit{}", ch));
        }
    }

    let lowered = trimmed.to_ascii_lowercase();
    let code = match lowered.as_str() {
        "esc" | "escape" => "Escape",
        "space" | "spacebar" => "Space",
        "enter" | "return" => "Enter",
        "tab" => "Tab",
        "backspace" => "Backspace",
        "delete" | "del" => "Delete",
        "up" | "arrowup" => "ArrowUp",
        "down" | "arrowdown" => "ArrowDown",
        "left" | "arrowleft" => "ArrowLeft",
        "right" | "arrowright" => "ArrowRight",
        "minus" | "-" => "Minus",
        "equal" | "=" => "Equal",
        "comma" | "," => "Comma",
        "period" | "." => "Period",
        "slash" | "/" => "Slash",
        "backslash" | "\\" => "Backslash",
        "quote" | "'" => "Quote",
        "semicolon" | ";" => "Semicolon",
        "backquote" | "`" => "Backquote",
        _ if lowered.starts_with('f') && lowered[1..].parse::<u8>().is_ok() => trimmed,
        _ => return None,
    };
    Some(code.to_string())
}

fn parse_hotkey(hotkey: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = hotkey
        .split(|ch| ch == '+' || ch == '-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err("快捷键至少需要一个修饰键，例如 Alt+A".to_string());
    }

    let mut modifiers = Modifiers::empty();
    for part in &parts[..parts.len() - 1] {
        match part.to_ascii_lowercase().as_str() {
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "cmd" | "command" | "meta" | "win" | "windows" | "super" => {
                modifiers |= Modifiers::META
            }
            other => return Err(format!("不支持的修饰键: {}", other)),
        }
    }
    if modifiers.is_empty() {
        return Err("快捷键至少需要 Alt/Ctrl/Shift/Win 中的一个修饰键".to_string());
    }

    let key_part = parts.last().copied().unwrap_or_default();
    let code_name =
        normalize_key_code(key_part).ok_or_else(|| format!("不支持的按键: {}", key_part))?;
    let code = Code::from_str(&code_name).map_err(|_| format!("不支持的按键: {}", key_part))?;
    Ok(Shortcut::new(Some(modifiers), code))
}

fn read_configured_hotkeys() -> (String, String) {
    let mut path = app_data_dir();
    path.push("config.json");
    let Ok(config_str) = fs::read_to_string(path) else {
        return (
            DEFAULT_SCREENSHOT_HOTKEY.to_string(),
            TRANSLATE_HOTKEY_LABEL.to_string(),
        );
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) else {
        return (
            DEFAULT_SCREENSHOT_HOTKEY.to_string(),
            TRANSLATE_HOTKEY_LABEL.to_string(),
        );
    };
    let screenshot = config
        .get("hotkey")
        .and_then(|value| value.as_str())
        .unwrap_or(DEFAULT_SCREENSHOT_HOTKEY)
        .trim()
        .to_string();
    let translate = config
        .get("translateHotkey")
        .and_then(|value| value.as_str())
        .unwrap_or(TRANSLATE_HOTKEY_LABEL)
        .trim()
        .to_string();
    (screenshot, translate)
}

fn register_global_shortcuts(
    app: &tauri::AppHandle,
    screenshot_hotkey: &str,
    translate_hotkey: &str,
) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    if !screenshot_hotkey.trim().is_empty() {
        match parse_hotkey(screenshot_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                let app_h = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = start_screenshot(app_h, None).await {
                                        eprintln!("Failed to start screenshot: {}", e);
                                    }
                                });
                            }
                        })
                {
                    errors.push(format!("{}: {}", screenshot_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", screenshot_hotkey, e)),
        }
    }

    if !translate_hotkey.trim().is_empty() {
        match parse_hotkey(translate_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                let app_h = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) =
                                        start_screenshot(app_h, Some("translate".to_string())).await
                                    {
                                        eprintln!("Failed to start translate screenshot: {}", e);
                                    }
                                });
                            }
                        })
                {
                    errors.push(format!("{}: {}", translate_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", translate_hotkey, e)),
        }
    }

    match parse_hotkey(RECORDING_HOTKEY_LABEL) {
        Ok(shortcut) => {
            if let Err(e) =
                app.global_shortcut()
                    .on_shortcut(shortcut, move |app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            let app_h = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) =
                                    start_screenshot(app_h, Some("record".to_string())).await
                                {
                                    eprintln!("Failed to start recording selection: {}", e);
                                }
                            });
                        }
                    })
            {
                errors.push(format!("{}: {}", RECORDING_HOTKEY_LABEL, e));
            }
        }
        Err(e) => errors.push(format!("{}: {}", RECORDING_HOTKEY_LABEL, e)),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[tauri::command]
fn re_register_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppShortcutStatus>,
    hotkey: String,
    translate_hotkey: Option<String>,
) -> Result<(), String> {
    let translate = translate_hotkey.unwrap_or_else(|| TRANSLATE_HOTKEY_LABEL.to_string());
    let status = register_global_shortcuts(&app, hotkey.trim(), translate.trim());
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = status.clone();
    status
}

#[tauri::command]
fn get_shortcut_status(state: tauri::State<'_, AppShortcutStatus>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match &*guard {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

fn app_data_dir() -> PathBuf {
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

fn cleanup_temp_files() {
    let _ = stop_recording_internal(1500);
    let mut path = app_data_dir();
    path.push("fullscreen_temp.jpg");
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    if cropped_path.exists() {
        let _ = fs::remove_file(&cropped_path);
    }
}

#[tauri::command]
fn get_config() -> Result<String, String> {
    let mut path = app_data_dir();
    path.push("config.json");
    if !path.exists() {
        return Ok("{}".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config_str: String) -> Result<(), String> {
    let mut path = app_data_dir();
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    path.push("config.json");
    fs::write(path, config_str).map_err(|e| e.to_string())
}

fn config_value_string(key: &str) -> Option<String> {
    let mut path = app_data_dir();
    path.push("config.json");
    let content = fs::read_to_string(path).ok()?;
    let config = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    config
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn sanitize_tag(tag: &str) -> String {
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

fn find_paddleocr_json_exe(dir: &std::path::Path) -> Option<PathBuf> {
    if !dir.exists() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("PaddleOCR-json.exe"))
                .unwrap_or(false)
        {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_paddleocr_json_exe(&path) {
                return Some(found);
            }
        }
    }
    None
}

fn portable_ocr_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("ocr"))
}

fn default_ocr_install_dir() -> PathBuf {
    if let Some(dir) = portable_ocr_dir() {
        return dir;
    }
    let mut dir = app_data_dir();
    dir.push("ocr");
    dir.push("runtime");
    dir
}
fn resolve_paddleocr_json_exe_from_path(path: &std::path::Path) -> Option<PathBuf> {
    resolve_ocr_executable_from_path(path)
}

fn ocr_runtime_manifest_path(exe_path: &std::path::Path) -> Option<PathBuf> {
    exe_path.parent().map(|dir| dir.join("ocr-runtime.json"))
}

fn read_ocr_runtime_manifest(exe_path: &std::path::Path) -> Option<serde_json::Value> {
    let manifest_path = ocr_runtime_manifest_path(exe_path)?;
    let content = fs::read_to_string(manifest_path).ok()?;
    serde_json::from_str(&content).ok()
}

fn resolve_manifest_entry_from_dir(dir: &std::path::Path) -> Option<PathBuf> {
    let manifest_path = dir.join("ocr-runtime.json");
    let content = fs::read_to_string(manifest_path).ok()?;
    let manifest = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let entry = manifest
        .get("entry")
        .and_then(|value| value.as_str())?
        .trim();
    if entry.is_empty() {
        return None;
    }
    let entry_path = dir.join(entry);
    if entry_path.is_file() {
        Some(entry_path)
    } else {
        None
    }
}

fn resolve_ocr_executable_from_path(path: &std::path::Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if path.is_dir() {
        return resolve_manifest_entry_from_dir(path).or_else(|| find_paddleocr_json_exe(path));
    }
    None
}

fn ocr_runtime_protocol(exe_path: &std::path::Path) -> String {
    read_ocr_runtime_manifest(exe_path)
        .and_then(|manifest| {
            manifest
                .get("protocol")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| "paddleocr-json-stdin".to_string())
}

fn write_paddleocr_runtime_manifest(
    install_dir: &std::path::Path,
    tag: &str,
    exe_path: &std::path::Path,
) -> Result<(), String> {
    let entry = exe_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("PaddleOCR-json.exe");
    let manifest = serde_json::json!({
        "id": "paddleocr-json",
        "name": "PaddleOCR-json",
        "engine": "paddleocr-json",
        "version": tag,
        "entry": entry,
        "protocol": "paddleocr-json-stdin",
        "languages": ["zh", "en", "ja", "ko"],
        "outputAdapter": "paddleocr-json"
    });
    let manifest_path = install_dir.join("ocr-runtime.json");
    let content = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to build OCR runtime manifest: {}", e))?;
    fs::write(&manifest_path, content)
        .map_err(|e| format!("Failed to write OCR runtime manifest: {}", e))
}

fn resolve_local_ocr_executable(
    app: &tauri::AppHandle,
    executable_path: Option<String>,
) -> Result<PathBuf, String> {
    use tauri::path::BaseDirectory;

    if let Some(path) = executable_path.filter(|path| !path.trim().is_empty()) {
        let raw_path = PathBuf::from(path.trim());
        return resolve_paddleocr_json_exe_from_path(&raw_path).ok_or_else(|| {
            format!(
                "未在指定 OCR 路径找到 PaddleOCR-json.exe：{}",
                raw_path.to_string_lossy()
            )
        });
    }

    if let Some(portable_dir) = portable_ocr_dir() {
        if let Some(path) = find_paddleocr_json_exe(&portable_dir) {
            return Ok(path);
        }
    }

    let install_dir = default_ocr_install_dir();
    if let Some(path) = find_paddleocr_json_exe(&install_dir) {
        return Ok(path);
    }

    let resource_path = app
        .path()
        .resolve("resources/ocr/PaddleOCR-json.exe", BaseDirectory::Resource)
        .map_err(|e| format!("解析 OCR 资源路径失败：{}", e))?;
    if resource_path.is_file() {
        return Ok(resource_path);
    }

    Err("未找到 OCR 运行入口。默认请放在软件同级 ocr\\PaddleOCR-json.exe，或在模型/视频配置中选择运行包目录。".to_string())
}

fn emit_ocr_progress(
    app: &tauri::AppHandle,
    phase: &str,
    downloaded: u64,
    total: Option<u64>,
    percent: u8,
) {
    let _ = app.emit(
        "ocr-download-progress",
        serde_json::json!({
            "phase": phase,
            "downloaded": downloaded,
            "total": total,
            "percent": percent,
        }),
    );
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| {
        format!(
            "\u{521b}\u{5efa}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    for entry in fs::read_dir(src).map_err(|e| {
        format!(
            "\u{8bfb}\u{53d6}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })? {
        let entry = entry.map_err(|e| {
            format!(
                "\u{8bfb}\u{53d6}\u{76ee}\u{5f55}\u{9879}\u{5931}\u{8d25}\u{ff1a}{}",
                e
            )
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "\u{590d}\u{5236}\u{6587}\u{4ef6}\u{5931}\u{8d25}\u{ff1a}{}",
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn stop_ocr_process() {
    let manager = get_ocr_manager();
    if let Ok(mut guard) = manager.lock() {
        if let Some(mut proc) = guard.process.take() {
            let _ = proc.child.kill();
        }
    };
}

#[tauri::command]
async fn download_paddleocr_release(
    app: tauri::AppHandle,
    url: String,
    tag: String,
    install_dir: Option<String>,
) -> Result<serde_json::Value, String> {
    let allowed = [
        "https://github.com/hiroi-sora/PaddleOCR-json/releases/download/",
        "https://objects.githubusercontent.com/github-production-release-asset-",
    ];
    if !allowed.iter().any(|prefix| url.starts_with(prefix))
        || !url.to_ascii_lowercase().ends_with(".7z")
    {
        return Err("\u{8bf7}\u{9009}\u{62e9} PaddleOCR-json \u{5b98}\u{65b9} GitHub Release \u{7684} Windows .7z \u{6587}\u{4ef6}".to_string());
    }

    stop_ocr_process();
    emit_ocr_progress(&app, "准备下载", 0, None, 1);

    let safe_tag = sanitize_tag(&tag);
    let filename = format!("PaddleOCR-json-{}.7z", safe_tag);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("\u{521b}\u{5efa}\u{4e0b}\u{8f7d}\u{5ba2}\u{6237}\u{7aef}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    let resp = client.get(&url).send().await.map_err(|e| {
        format!(
            "\u{4e0b}\u{8f7d} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    if !resp.status().is_success() {
        return Err(format!(
            "\u{4e0b}\u{8f7d} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}HTTP {}",
            resp.status()
        ));
    }

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("\u{8bfb}\u{53d6} PaddleOCR-json \u{4e0b}\u{8f7d}\u{6570}\u{636e}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let percent = total
            .map(|value| ((downloaded as f64 / value.max(1) as f64) * 70.0).round() as u8)
            .unwrap_or(10)
            .clamp(1, 70);
        emit_ocr_progress(&app, "下载中", downloaded, total, percent);
    }

    let mut download_dir = app_data_dir();
    download_dir.push("ocr");
    download_dir.push("downloads");
    fs::create_dir_all(&download_dir).map_err(|e| {
        format!(
            "\u{521b}\u{5efa}\u{4e0b}\u{8f7d}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    let archive_path = download_dir.join(filename);
    fs::write(&archive_path, &bytes).map_err(|e| {
        format!(
            "\u{4fdd}\u{5b58} PaddleOCR-json \u{538b}\u{7f29}\u{5305}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;

    emit_ocr_progress(&app, "解压中", downloaded, total, 75);
    let install_dir = install_dir
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| ensure_writable_dir(default_ocr_install_dir()));
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| {
            format!(
                "\u{6e05}\u{7406} OCR \u{5b89}\u{88c5}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
                e
            )
        })?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| {
        format!(
            "\u{521b}\u{5efa} OCR \u{5b89}\u{88c5}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;

    sevenz_rust::decompress_file(&archive_path, &install_dir).map_err(|e| {
        format!(
            "\u{89e3}\u{538b} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    let _ = fs::remove_file(&archive_path);
    emit_ocr_progress(&app, "检查可执行文件", downloaded, total, 95);

    let exe_path = find_paddleocr_json_exe(&install_dir).ok_or_else(|| {
        "\u{89e3}\u{538b}\u{540e}\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe".to_string()
    })?;
    write_paddleocr_runtime_manifest(&install_dir, &tag, &exe_path)?;
    emit_ocr_progress(&app, "完成", downloaded, total, 100);

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": install_dir.to_string_lossy().to_string(),
        "bytes": bytes.len(),
    }))
}

#[tauri::command]
fn choose_ocr_install_dir() -> Result<Option<String>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("\u{9009}\u{62e9} OCR \u{5b89}\u{88c5}\u{76ee}\u{5f55}")
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn choose_ocr_runtime_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("选择 OCR 运行包目录");
    if let Some(dir) = current_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_dir() {
                dialog = dialog.set_directory(path);
            } else if let Some(parent) = path.parent() {
                dialog = dialog.set_directory(parent);
            }
        }
    }
    Ok(dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn move_ocr_runtime(
    target_dir: String,
    executable_path: Option<String>,
) -> Result<serde_json::Value, String> {
    let target_dir = PathBuf::from(target_dir);
    if target_dir.as_os_str().is_empty() {
        return Err("\u{8bf7}\u{9009}\u{62e9}\u{76ee}\u{6807}\u{76ee}\u{5f55}".to_string());
    }
    stop_ocr_process();

    let source_exe = executable_path
        .filter(|path| !path.trim().is_empty())
        .and_then(|path| resolve_paddleocr_json_exe_from_path(&PathBuf::from(path.trim())))
        .or_else(|| portable_ocr_dir().and_then(|dir| find_paddleocr_json_exe(&dir)))
        .or_else(|| find_paddleocr_json_exe(&default_ocr_install_dir()))
        .ok_or_else(|| "\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe\u{ff0c}\u{8bf7}\u{5148}\u{4e0b}\u{8f7d}\u{6216}\u{9009}\u{62e9} OCR \u{76ee}\u{5f55}".to_string())?;
    let source_dir = source_exe
        .parent()
        .ok_or_else(|| "\u{65e0}\u{6cd5}\u{89e3}\u{6790} OCR \u{76ee}\u{5f55}".to_string())?
        .to_path_buf();

    let source_canon = fs::canonicalize(&source_dir).map_err(|e| {
        format!(
            "\u{8bfb}\u{53d6} OCR \u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    fs::create_dir_all(&target_dir).map_err(|e| {
        format!(
            "\u{521b}\u{5efa}\u{76ee}\u{6807}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    let target_canon = fs::canonicalize(&target_dir).map_err(|e| {
        format!(
            "\u{89e3}\u{6790}\u{76ee}\u{6807}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}",
            e
        )
    })?;
    if source_canon == target_canon {
        return Ok(serde_json::json!({
            "path": source_exe.to_string_lossy().to_string(),
            "installDir": source_dir.to_string_lossy().to_string(),
        }));
    }

    copy_dir_recursive(&source_dir, &target_dir)?;
    let exe_path = find_paddleocr_json_exe(&target_dir).ok_or_else(|| {
        "\u{79fb}\u{52a8}\u{5b8c}\u{6210}\u{540e}\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe"
            .to_string()
    })?;
    if !target_dir.join("ocr-runtime.json").exists() {
        let _ = write_paddleocr_runtime_manifest(&target_dir, "custom", &exe_path);
    }

    if !target_canon.starts_with(&source_canon) && !source_canon.starts_with(&target_canon) {
        let _ = fs::remove_dir_all(&source_dir);
    }

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": target_dir.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
fn check_local_ocr_status(
    app: tauri::AppHandle,
    executable_path: Option<String>,
) -> Result<serde_json::Value, String> {
    let exe_path = resolve_local_ocr_executable(&app, executable_path)?;
    let exists = exe_path.exists();
    let is_file = exe_path.is_file();
    let parent_exists = exe_path.parent().map(|path| path.exists()).unwrap_or(false);
    Ok(serde_json::json!({
        "ok": exists && is_file,
        "path": exe_path.to_string_lossy().to_string(),
        "exists": exists,
        "isFile": is_file,
        "parentExists": parent_exists,
        "runtimeManifest": read_ocr_runtime_manifest(&exe_path),
    }))
}

#[tauri::command]
fn is_autostart_enabled() -> bool {
    let output = Command::new("reg")
        .args([
            "query",
            "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            "ScreenshotTranslator",
        ])
        .output();
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let current_exe_str = current_exe.to_string_lossy();
        let status = Command::new("reg")
            .args([
                "add",
                "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "ScreenshotTranslator",
                "/t",
                "REG_SZ",
                "/d",
                &format!("\"{}\"", current_exe_str),
                "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to execute reg command: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err("reg add command returned non-zero exit code".to_string())
        }
    } else {
        let _ = Command::new("reg")
            .args([
                "delete",
                "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "ScreenshotTranslator",
                "/f",
            ])
            .status();
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct RECT {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct PAINTSTRUCT {
        pub hdc: isize,
        pub f_erase: i32,
        pub rc_paint: RECT,
        pub f_restore: i32,
        pub f_inc_update: i32,
        pub rgb_reserved: [u8; 32],
    }
    #[repr(C)]
    pub struct WNDCLASSW {
        pub style: u32,
        pub lpfn_wnd_proc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
        pub cb_cls_extra: i32,
        pub cb_wnd_extra: i32,
        pub h_instance: isize,
        pub h_icon: isize,
        pub h_cursor: isize,
        pub hbr_background: isize,
        pub lpsz_menu_name: *const u16,
        pub lpsz_class_name: *const u16,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct MSG {
        pub hwnd: isize,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt: POINT,
    }
    pub type EnumWindowsProc = Option<unsafe extern "system" fn(isize, isize) -> i32>;
    extern "system" {
        pub fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
        pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
        pub fn CreateWindowExW(
            dwExStyle: u32,
            lpClassName: *const u16,
            lpWindowName: *const u16,
            dwStyle: u32,
            X: i32,
            Y: i32,
            nWidth: i32,
            nHeight: i32,
            hWndParent: isize,
            hMenu: isize,
            hInstance: isize,
            lpParam: *mut c_void,
        ) -> isize;
        pub fn DefWindowProcW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> isize;
        pub fn DestroyWindow(hWnd: isize) -> i32;
        pub fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
        pub fn UpdateWindow(hWnd: isize) -> i32;
        pub fn PostMessageW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> i32;
        pub fn PostQuitMessage(nExitCode: i32);
        pub fn GetMessageW(
            lpMsg: *mut MSG,
            hWnd: isize,
            wMsgFilterMin: u32,
            wMsgFilterMax: u32,
        ) -> i32;
        pub fn TranslateMessage(lpMsg: *const MSG) -> i32;
        pub fn DispatchMessageW(lpMsg: *const MSG) -> isize;
        pub fn BeginPaint(hWnd: isize, lpPaint: *mut PAINTSTRUCT) -> isize;
        pub fn EndPaint(hWnd: isize, lpPaint: *const PAINTSTRUCT) -> i32;
        pub fn FillRect(hDC: isize, lprc: *const RECT, hbr: isize) -> i32;
        pub fn CreateSolidBrush(color: u32) -> isize;
        pub fn DeleteObject(ho: isize) -> i32;
        pub fn SetLayeredWindowAttributes(hwnd: isize, crKey: u32, bAlpha: u8, dwFlags: u32)
            -> i32;
        pub fn SetWindowDisplayAffinity(hWnd: isize, dwAffinity: u32) -> i32;
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn GetWindowTextLengthW(hWnd: isize) -> i32;
        pub fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
        pub fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
        pub fn EnumChildWindows(
            hWndParent: isize,
            lpEnumFunc: EnumWindowsProc,
            lParam: isize,
        ) -> i32;
        pub fn IsWindowVisible(hWnd: isize) -> i32;
        pub fn SetCursorPos(X: i32, Y: i32) -> i32;
        pub fn mouse_event(dwFlags: u32, dx: u32, dy: u32, dwData: u32, dwExtraInfo: usize);
    }
    #[link(name = "dwmapi")]
    extern "system" {
        pub fn DwmSetWindowAttribute(
            hwnd: isize,
            dwAttribute: u32,
            pvAttribute: *const std::ffi::c_void,
            cbAttribute: u32,
        ) -> i32;
        pub fn DwmGetWindowAttribute(
            hwnd: isize,
            dwAttribute: u32,
            pvAttribute: *mut std::ffi::c_void,
            cbAttribute: u32,
        ) -> i32;
    }
}

#[tauri::command]
fn set_window_capture_excluded(
    app: tauri::AppHandle,
    label: String,
    excluded: bool,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const WDA_NONE: u32 = 0x00000000;
        const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;
        let window = app
            .get_webview_window(&label)
            .ok_or_else(|| format!("window not found: {}", label))?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        let affinity = if excluded {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        let ok = unsafe { win32::SetWindowDisplayAffinity(hwnd, affinity) };
        if ok == 0 {
            return Err("SetWindowDisplayAffinity failed".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, label, excluded);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(i32, i32)> {
    let mut point = win32::POINT { x: 0, y: 0 };
    // SAFETY: Calling Win32 API GetCursorPos with a valid mutable pointer to a POINT struct.
    unsafe {
        if win32::GetCursorPos(&mut point) != 0 {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> Option<(i32, i32)> {
    None
}

fn disable_windows_transition<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let value: i32 = 1;
        // SAFETY: Calling Dwmapi function DwmSetWindowAttribute with valid hwnd and parameters.
        unsafe {
            let _ = win32::DwmSetWindowAttribute(
                hwnd.0 as isize,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &value as *const i32 as *const std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

#[cfg(target_os = "windows")]
const RECORDING_OVERLAY_CLASS: &str = "YSNRecordingOverlayNative";
#[cfg(target_os = "windows")]
const WM_PAINT: u32 = 0x000F;
#[cfg(target_os = "windows")]
const WM_DESTROY: u32 = 0x0002;
#[cfg(target_os = "windows")]
const WM_CLOSE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const WM_NCHITTEST: u32 = 0x0084;
#[cfg(target_os = "windows")]
const HTTRANSPARENT: isize = -1;
#[cfg(target_os = "windows")]
const WS_POPUP: u32 = 0x80000000;
#[cfg(target_os = "windows")]
const WS_EX_TOPMOST: u32 = 0x00000008;
#[cfg(target_os = "windows")]
const WS_EX_TRANSPARENT: u32 = 0x00000020;
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: u32 = 0x00000080;
#[cfg(target_os = "windows")]
const WS_EX_LAYERED: u32 = 0x00080000;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const LWA_COLORKEY: u32 = 0x00000001;
#[cfg(target_os = "windows")]
const TRANSPARENT_COLOR_KEY: u32 = 0x000000;
#[cfg(target_os = "windows")]
const RECORDING_BORDER_COLORREF: u32 = 0x4f3bff;
#[cfg(target_os = "windows")]
const RECORDING_BORDER_THICKNESS: i32 = 1;

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn recording_overlay_wnd_proc(
    hwnd: isize,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    match msg {
        WM_NCHITTEST => HTTRANSPARENT,
        WM_PAINT => {
            let mut ps = win32::PAINTSTRUCT {
                hdc: 0,
                f_erase: 0,
                rc_paint: win32::RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                f_restore: 0,
                f_inc_update: 0,
                rgb_reserved: [0; 32],
            };
            let hdc = win32::BeginPaint(hwnd, &mut ps);
            let width = ps.rc_paint.right.max(1);
            let height = ps.rc_paint.bottom.max(1);
            let transparent_brush = win32::CreateSolidBrush(TRANSPARENT_COLOR_KEY);
            let red_brush = win32::CreateSolidBrush(RECORDING_BORDER_COLORREF);
            let full = win32::RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            let top = win32::RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: RECORDING_BORDER_THICKNESS.min(height),
            };
            let bottom = win32::RECT {
                left: 0,
                top: (height - RECORDING_BORDER_THICKNESS).max(0),
                right: width,
                bottom: height,
            };
            let left = win32::RECT {
                left: 0,
                top: 0,
                right: RECORDING_BORDER_THICKNESS.min(width),
                bottom: height,
            };
            let right = win32::RECT {
                left: (width - RECORDING_BORDER_THICKNESS).max(0),
                top: 0,
                right: width,
                bottom: height,
            };
            win32::FillRect(hdc, &full, transparent_brush);
            win32::FillRect(hdc, &top, red_brush);
            win32::FillRect(hdc, &bottom, red_brush);
            win32::FillRect(hdc, &left, red_brush);
            win32::FillRect(hdc, &right, red_brush);
            let _ = win32::DeleteObject(transparent_brush);
            let _ = win32::DeleteObject(red_brush);
            win32::EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE => {
            win32::DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            win32::PostQuitMessage(0);
            0
        }
        _ => win32::DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

#[cfg(target_os = "windows")]
fn hide_recording_overlay_internal() {
    let overlay = get_recording_overlay()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(overlay) = overlay {
        unsafe {
            let _ = win32::PostMessageW(overlay.hwnd, WM_CLOSE, 0, 0);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_recording_overlay_internal() {}

#[tauri::command]
fn hide_recording_overlay() -> Result<(), String> {
    hide_recording_overlay_internal();
    Ok(())
}

#[tauri::command]
fn show_recording_overlay(x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    if w <= 0 || h <= 0 {
        return Err("录制区域尺寸无效".to_string());
    }
    hide_recording_overlay_internal();
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = mpsc::channel::<Result<isize, String>>();
        std::thread::spawn(move || {
            let result = unsafe {
                let class_name = wide_null(RECORDING_OVERLAY_CLASS);
                let title = wide_null("YSN Recording Border");
                let h_instance = win32::GetModuleHandleW(std::ptr::null());
                let wnd_class = win32::WNDCLASSW {
                    style: 0,
                    lpfn_wnd_proc: Some(recording_overlay_wnd_proc),
                    cb_cls_extra: 0,
                    cb_wnd_extra: 0,
                    h_instance,
                    h_icon: 0,
                    h_cursor: 0,
                    hbr_background: 0,
                    lpsz_menu_name: std::ptr::null(),
                    lpsz_class_name: class_name.as_ptr(),
                };
                let _ = win32::RegisterClassW(&wnd_class);
                let hwnd = win32::CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                    class_name.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP,
                    x,
                    y,
                    w,
                    h,
                    0,
                    0,
                    h_instance,
                    std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    Err("创建原生录制边框失败".to_string())
                } else {
                    let _ = win32::SetLayeredWindowAttributes(
                        hwnd,
                        TRANSPARENT_COLOR_KEY,
                        255,
                        LWA_COLORKEY,
                    );
                    let _ = win32::ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let _ = win32::UpdateWindow(hwnd);
                    Ok(hwnd)
                }
            };
            let hwnd = match result {
                Ok(hwnd) => {
                    let _ = tx.send(Ok(hwnd));
                    hwnd
                }
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            };
            let mut msg = win32::MSG {
                hwnd: 0,
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt: win32::POINT { x: 0, y: 0 },
            };
            unsafe {
                while win32::GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    let _ = win32::TranslateMessage(&msg);
                    let _ = win32::DispatchMessageW(&msg);
                }
            }
            if let Ok(mut guard) = get_recording_overlay().lock() {
                if guard.map(|value| value.hwnd) == Some(hwnd) {
                    *guard = None;
                }
            }
        });
        let hwnd = rx
            .recv_timeout(std::time::Duration::from_millis(1000))
            .map_err(|_| "创建原生录制边框超时".to_string())??;
        *get_recording_overlay().lock().map_err(|e| e.to_string())? =
            Some(NativeRecordingOverlay { hwnd });
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

fn close_screenshot_windows(app: &tauri::AppHandle, include_primary: bool) {
    for (label, window) in app.webview_windows() {
        if label == "screenshot" && include_primary {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
        } else if label.starts_with("screenshot_") {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
            let _ = window.close();
        } else if label == "recording_border" || label.starts_with("recording_border_") {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
            let _ = window.close();
        }
    }
}

async fn start_screenshot_impl(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());
    let capture_visible_overlay = app
        .get_webview_window("screenshot")
        .and_then(|win| win.is_visible().ok())
        .unwrap_or(false);

    // Hide app windows before capture. If the screenshot overlay is already visible,
    // keep it visible so a second hotkey can intentionally capture the current box/tools UI.
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }
    if !capture_visible_overlay {
        if let Some(screenshot_win) = app.get_webview_window("screenshot") {
            let _ = screenshot_win.set_always_on_top(false);
            let _ = screenshot_win.hide();
        }
    }
    close_screenshot_windows(&app, false);

    // Capture and encode on a blocking thread to avoid blocking the async runtime
    let (jpeg_bytes, base64_data, screen_info) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<u8>, String, (i32, i32, u32, u32)), String> {
            let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
            if screens.is_empty() {
                return Err("未检测到显示器".to_string());
            }
            let screen = if let Some((cx, cy)) = get_cursor_position() {
                Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
            } else {
                screens[0]
            };
            let info = screen.display_info;
            let screen_info = (info.x, info.y, info.width, info.height);

            let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
            let mut buffer = std::io::Cursor::new(Vec::new());
            let encoder =
                screenshots::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
            image
                .write_with_encoder(encoder)
                .map_err(|e| format!("生成JPEG字节流失败：{}", e))?;
            let jpeg_bytes = buffer.into_inner();
            let base64_data = BASE64_STANDARD.encode(&jpeg_bytes);
            Ok((jpeg_bytes, base64_data, screen_info))
        },
    )
    .await
    .map_err(|e| format!("截屏任务执行失败：{}", e))??;

    // Store JPEG bytes in memory for capture_region (avoids disk read on the critical path)
    if let Ok(mut guard) = get_screenshot_jpeg().lock() {
        *guard = Some(jpeg_bytes.clone());
    }

    // Write to disk asynchronously (non-blocking) — only needed as a backup
    let write_dir = app_data_dir();
    let write_path = write_dir.join("fullscreen_temp.jpg");
    let jpeg_for_write = jpeg_bytes.clone();
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = write_path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        let _ = fs::write(&write_path, &jpeg_for_write);
    });

    let screenshot_win = if let Some(win) = app.get_webview_window("screenshot") {
        win
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "screenshot",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("YSN 截图辅助窗口")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .build()
        .map_err(|e| format!("创建截图窗口失败：{}", e))?
    };

    // Disable transition animation to avoid windows rendering delay/flicker
    disable_windows_transition(&screenshot_win);

    let (x, y, width, height) = screen_info;

    // Position and configure the window while still hidden
    let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
    let _ = screenshot_win.set_always_on_top(true);

    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let _ = screenshot_win.emit("screenshot-updated", base64_data);

    Ok(())
}

#[tauri::command]
async fn overlay_ready_to_show(app: tauri::AppHandle, label: Option<String>) -> Result<(), String> {
    let target_label = label.unwrap_or_else(|| "screenshot".to_string());
    if target_label != "screenshot" && !target_label.starts_with("screenshot_") {
        return Ok(());
    }
    if let Some(screenshot_win) = app.get_webview_window(&target_label) {
        let _ = screenshot_win.show();
        let _ = screenshot_win.set_focus();
        let _ = screenshot_win.set_always_on_top(true);
    }
    Ok(())
}

#[tauri::command]
async fn start_screenshot(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    // Allow re-entry: pressing hotkey again while capturing restarts the session
    CAPTURING.store(true, Ordering::SeqCst);

    match start_screenshot_impl(app, mode).await {
        Ok(()) => Ok(()),
        Err(e) => {
            CAPTURING.store(false, Ordering::SeqCst);
            Err(e)
        }
    }
}

#[tauri::command]
async fn force_close_screenshots(app: tauri::AppHandle) -> Result<(), String> {
    hide_recording_overlay_internal();
    close_screenshot_windows(&app, true);
    CAPTURING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn quick_fullscreen_capture() -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() {
        return Err("未检测到显示器".to_string());
    }
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
    } else {
        screens[0]
    };
    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let (width, height) = image.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(image.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn current_screen_origin() -> (i32, i32, i32, i32) {
    if let Some((cx, cy)) = get_cursor_position() {
        if let Ok(screen) = Screen::from_point(cx, cy) {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
    }
    if let Ok(screens) = Screen::all() {
        if let Some(screen) = screens.first() {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
    }
    (0, 0, i32::MAX, i32::MAX)
}

#[cfg(target_os = "windows")]
fn hwnd_rect(hwnd: isize, prefer_dwm_bounds: bool) -> Option<win32::RECT> {
    if hwnd == 0 {
        return None;
    }
    if prefer_dwm_bounds {
        let mut rect = win32::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: DwmGetWindowAttribute is called with a valid HWND and RECT buffer.
        let hr = unsafe {
            win32::DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut win32::RECT as *mut std::ffi::c_void,
                std::mem::size_of::<win32::RECT>() as u32,
            )
        };
        if hr == 0 && rect.right > rect.left && rect.bottom > rect.top {
            return Some(rect);
        }
    }
    let mut rect = win32::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: GetWindowRect is called with a valid HWND and RECT buffer.
    let ok = unsafe { win32::GetWindowRect(hwnd, &mut rect) };
    if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
        Some(rect)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn push_rect_candidate(
    rects: &mut Vec<serde_json::Value>,
    rect: win32::RECT,
    kind: &str,
    screen: (i32, i32, i32, i32),
    min_size: i32,
) {
    let (screen_x, screen_y, screen_w, screen_h) = screen;
    let left = rect.left.max(screen_x);
    let top = rect.top.max(screen_y);
    let right = rect.right.min(screen_x + screen_w);
    let bottom = rect.bottom.min(screen_y + screen_h);
    let w = right - left;
    let h = bottom - top;
    if w < min_size || h < min_size {
        return;
    }
    let json_rect = serde_json::json!({
        "x": left - screen_x,
        "y": top - screen_y,
        "w": w,
        "h": h,
        "kind": kind,
    });
    let duplicate = rects.iter().any(|item| {
        item.get("x") == json_rect.get("x")
            && item.get("y") == json_rect.get("y")
            && item.get("w") == json_rect.get("w")
            && item.get("h") == json_rect.get("h")
    });
    if !duplicate {
        rects.push(json_rect);
    }
}

#[cfg(target_os = "windows")]
struct WindowSearchContext {
    cursor_x: i32,
    cursor_y: i32,
    excluded_hwnds: Vec<isize>,
    matches: Vec<isize>,
    min_size: i32,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || ctx.excluded_hwnds.contains(&hwnd) || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, true) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let contains_cursor = ctx.cursor_x >= rect.left
            && ctx.cursor_x <= rect.right
            && ctx.cursor_y >= rect.top
            && ctx.cursor_y <= rect.bottom;
        if contains_cursor && w >= ctx.min_size && h >= ctx.min_size {
            ctx.matches.push(hwnd);
        }
    }
    1
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_child_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, false) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let contains_cursor = ctx.cursor_x >= rect.left
            && ctx.cursor_x <= rect.right
            && ctx.cursor_y >= rect.top
            && ctx.cursor_y <= rect.bottom;
        if contains_cursor && w >= ctx.min_size && h >= ctx.min_size {
            ctx.matches.push(hwnd);
        }
    }
    1
}

#[cfg(target_os = "windows")]
fn excluded_app_hwnds(app: &tauri::AppHandle) -> Vec<isize> {
    let mut excluded = Vec::new();
    for label in ["screenshot", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            if let Ok(hwnd) = window.hwnd() {
                excluded.push(hwnd.0 as isize);
            }
        }
    }
    excluded
}

#[cfg(target_os = "windows")]
fn top_level_windows_at_cursor(
    cursor_x: i32,
    cursor_y: i32,
    excluded_hwnds: Vec<isize>,
) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds,
        matches: Vec::new(),
        min_size: 50,
    };
    // SAFETY: EnumWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumWindows(
            Some(enum_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[cfg(target_os = "windows")]
fn child_windows_at_cursor(root: isize, cursor_x: i32, cursor_y: i32) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds: Vec::new(),
        matches: Vec::new(),
        min_size: 12,
    };
    // SAFETY: EnumChildWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumChildWindows(
            root,
            Some(enum_child_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[tauri::command]
fn get_window_rects(
    app: tauri::AppHandle,
    include_controls: Option<bool>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut rects: Vec<serde_json::Value> = Vec::new();
        let screen = current_screen_origin();
        let include_controls = include_controls.unwrap_or(false);
        if let Some((cx, cy)) = get_cursor_position() {
            let excluded_hwnds = excluded_app_hwnds(&app);
            let windows = top_level_windows_at_cursor(cx, cy, excluded_hwnds);
            if let Some(hwnd) = windows.first().copied() {
                if include_controls {
                    for child in child_windows_at_cursor(hwnd, cx, cy)
                        .into_iter()
                        .rev()
                        .take(1)
                    {
                        if let Some(rect) = hwnd_rect(child, false) {
                            push_rect_candidate(&mut rects, rect, "control", screen, 12);
                        }
                    }
                }
                if let Some(rect) = hwnd_rect(hwnd, true) {
                    push_rect_candidate(&mut rects, rect, "window", screen, 50);
                }
            }
        }
        Ok(serde_json::to_string(&rects).unwrap_or_else(|_| "[]".to_string()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok("[]".to_string())
    }
}

#[tauri::command]
async fn cancel_screenshot(app: tauri::AppHandle, label: Option<String>) -> Result<(), String> {
    if let Some(target_label) = label {
        if target_label == "screenshot" || target_label.starts_with("screenshot_") {
            if let Some(screenshot_win) = app.get_webview_window(&target_label) {
                let _ = screenshot_win.set_always_on_top(false);
                let _ = screenshot_win.hide();
            }
            close_screenshot_windows(&app, false);
        }
    } else {
        close_screenshot_windows(&app, true);
    }
    CAPTURING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_fullscreen_image() -> Result<String, String> {
    // Try memory first (fast), fall back to disk
    if let Ok(guard) = get_screenshot_jpeg().lock() {
        if let Some(ref bytes) = *guard {
            return Ok(BASE64_STANDARD.encode(bytes));
        }
    }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.jpg");
    if !path.exists() {
        return Err("没有可用的全屏截图".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取全屏图失败：{}", e))?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("选区范围无效".to_string());
    }

    // Try memory first (fast), fall back to disk
    let jpeg_bytes = {
        let guard = get_screenshot_jpeg().lock().map_err(|e| e.to_string())?;
        if let Some(ref bytes) = *guard {
            bytes.clone()
        } else {
            let mut path = app_data_dir();
            path.push("fullscreen_temp.jpg");
            if !path.exists() {
                return Err("原始截图文件不存在".to_string());
            }
            fs::read(&path).map_err(|e| format!("读取全屏图失败：{}", e))?
        }
    };

    let img = screenshots::image::load_from_memory_with_format(
        &jpeg_bytes,
        screenshots::image::ImageFormat::Jpeg,
    )
    .map_err(|e| format!("加载全屏图失败：{}", e))?;
    let iw = img.width() as i32;
    let ih = img.height() as i32;
    let sx = x.clamp(0, iw.saturating_sub(1));
    let sy = y.clamp(0, ih.saturating_sub(1));
    let sw = w.clamp(1, iw - sx);
    let sh = h.clamp(1, ih - sy);
    let cropped = img.crop_imm(sx as u32, sy as u32, sw as u32, sh as u32);
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("图片编码 PNG 失败：{}", e))?;
    let bytes = buffer.into_inner();
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    let _ = fs::write(&cropped_path, &bytes);
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_live_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection area".to_string());
    }
    let (origin_x, origin_y, _, _) = current_screen_origin();
    let global_x = origin_x + x;
    let global_y = origin_y + y;
    let center_x = global_x + w / 2;
    let center_y = global_y + h / 2;
    let screen = Screen::from_point(center_x, center_y)
        .map_err(|e| format!("Failed to locate screen for scroll capture: {}", e))?;
    let rel_x = global_x - screen.display_info.x;
    let rel_y = global_y - screen.display_info.y;
    let image = screen
        .capture_area(rel_x, rel_y, w as u32, h as u32)
        .map_err(|e| format!("Failed to capture live region: {}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    screenshots::image::DynamicImage::ImageRgba8(image)
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    Ok(BASE64_STANDARD.encode(buffer.into_inner()))
}

#[tauri::command]
fn scroll_mouse_at(x: i32, y: i32, delta: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const MOUSEEVENTF_WHEEL: u32 = 0x0800;
        let (origin_x, origin_y, _, _) = current_screen_origin();
        let global_x = origin_x + x;
        let global_y = origin_y + y;
        unsafe {
            let _ = win32::SetCursorPos(global_x, global_y);
            win32::mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta as u32, 0);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (x, y, delta);
        Err("Automatic scrolling is not supported on this platform".to_string())
    }
}

#[tauri::command]
fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Base64解码失败：{}", e))?;
    let img = screenshots::image::load_from_memory_with_format(
        &bytes,
        screenshots::image::ImageFormat::Png,
    )
    .map_err(|e| format!("解析裁剪图像数据失败：{}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}

#[tauri::command]
async fn save_image_to_file(image_base64: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Base64解码失败：{}", e))?;
    let file_path = rfd::AsyncFileDialog::new()
        .add_filter("PNG 图像", &["png"])
        .set_file_name("screenshot.png")
        .save_file()
        .await;
    if let Some(file_handle) = file_path {
        let path = file_handle.path();
        fs::write(path, &bytes).map_err(|e| format!("写入文件失败：{}", e))?;
        if !path.exists() {
            return Err("文件未成功写入磁盘".to_string());
        }
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("用户取消了保存".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct RecordingOptions {
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

fn ffmpeg_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
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

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseInfo {
    tag_name: String,
    html_url: Option<String>,
    assets: Vec<GithubReleaseAsset>,
}

fn emit_ffmpeg_progress(
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

fn default_ffmpeg_install_dir() -> PathBuf {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            return dir.join("ffmpeg");
        }
    }
    let mut dir = app_data_dir();
    dir.push("ffmpeg");
    dir
}

fn ensure_writable_dir(preferred: PathBuf) -> PathBuf {
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

fn extract_ffmpeg_exe_from_zip(
    bytes: &[u8],
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("读取 ffmpeg 压缩包失败：{}", e))?;
    fs::create_dir_all(install_dir).map_err(|e| format!("创建 ffmpeg 目录失败：{}", e))?;
    let target = install_dir.join("ffmpeg.exe");
    let mut found = false;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("读取 ffmpeg 压缩包文件失败：{}", e))?;
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
            fs::File::create(&target).map_err(|e| format!("写入 ffmpeg.exe 失败：{}", e))?;
        std::io::copy(&mut file, &mut out).map_err(|e| format!("解压 ffmpeg.exe 失败：{}", e))?;
        found = true;
        break;
    }
    if !found {
        return Err("压缩包内未找到 ffmpeg.exe".to_string());
    }
    Ok(target)
}

fn cleanup_finished_recording_process() -> Result<bool, String> {
    let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
    let finished = if let Some(child) = guard.as_mut() {
        child
            .try_wait()
            .map_err(|e| format!("读取录屏进程状态失败：{}", e))?
            .is_some()
    } else {
        false
    };
    if finished {
        *guard = None;
    }
    Ok(finished)
}

fn find_ffmpeg_executable(app: &tauri::AppHandle) -> Option<PathBuf> {
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

fn parse_quoted_audio_devices(
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

fn ffmpeg_supports_input_format(formats_output: &str, format_name: &str) -> bool {
    formats_output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("D") && trimmed.split_whitespace().nth(1) == Some(format_name)
    })
}

fn hidden_ffmpeg_command(ffmpeg_path: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn ffmpeg_input_formats(ffmpeg_path: &Path) -> String {
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

fn collect_ffmpeg_audio_devices(ffmpeg_path: &Path) -> Vec<String> {
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

#[cfg(target_os = "windows")]
struct RecordingWindowListContext {
    excluded_hwnds: Vec<isize>,
    windows: Vec<serde_json::Value>,
}

#[cfg(target_os = "windows")]
fn window_title(hwnd: isize) -> String {
    let len = unsafe { win32::GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; (len + 1) as usize];
    let copied = unsafe { win32::GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_recording_windows(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut RecordingWindowListContext);
    if hwnd == 0 || ctx.excluded_hwnds.contains(&hwnd) || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let title = window_title(hwnd);
    if title.is_empty() {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, true) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w >= 120 && h >= 80 {
            ctx.windows.push(serde_json::json!({
                "id": hwnd.to_string(),
                "title": title,
                "x": rect.left,
                "y": rect.top,
                "w": w,
                "h": h,
            }));
        }
    }
    1
}

#[tauri::command]
fn get_recording_targets(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let displays = Screen::all()
        .map_err(|e| format!("无法获取显示器：{}", e))?
        .into_iter()
        .enumerate()
        .map(|(index, screen)| {
            let info = screen.display_info;
            serde_json::json!({
                "id": index.to_string(),
                "title": format!("显示器 {} ({}x{})", index + 1, info.width, info.height),
                "x": info.x,
                "y": info.y,
                "w": info.width,
                "h": info.height,
            })
        })
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    let windows = {
        let mut ctx = RecordingWindowListContext {
            excluded_hwnds: excluded_app_hwnds(&app),
            windows: Vec::new(),
        };
        unsafe {
            win32::EnumWindows(
                Some(enum_recording_windows),
                &mut ctx as *mut RecordingWindowListContext as isize,
            );
        }
        ctx.windows
    };
    #[cfg(not(target_os = "windows"))]
    let windows: Vec<serde_json::Value> = Vec::new();

    Ok(serde_json::json!({
        "windows": windows,
        "displays": displays,
    }))
}

#[tauri::command]
fn get_recording_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
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

fn recording_output_path(output_dir: Option<String>) -> Result<PathBuf, String> {
    let dir = output_dir
        .filter(|value| !value.trim().is_empty())
        .map(|value| PathBuf::from(value.trim()))
        .unwrap_or_else(|| {
            let mut dir = app_data_dir();
            dir.push("recordings");
            dir
        });
    fs::create_dir_all(&dir).map_err(|e| format!("创建录屏目录失败：{}", e))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(dir.join(format!("recording_{}.mp4", millis)))
}

fn resolution_scale_filter(resolution: &str) -> Option<&'static str> {
    match resolution {
        "480p" => Some("scale=-2:480"),
        "720p" => Some("scale=-2:720"),
        "1080p" => Some("scale=-2:1080"),
        "original" => None,
        _ => Some("scale=-2:1080"),
    }
}

fn push_recording_audio_input(
    device: Option<&str>,
    label: &str,
    args: &mut Vec<String>,
) -> Result<(), String> {
    let name = device
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("请选择{}音频设备", label))?;
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

fn build_recording_args(
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
            return Err("录屏区域尺寸无效".to_string());
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
            push_recording_audio_input(options.mic_device.as_deref(), "麦克风", &mut args)?;
            1
        }
        "system" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "系统声音",
                &mut args,
            )?;
            1
        }
        "system_mic" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "系统声音",
                &mut args,
            )?;
            push_recording_audio_input(options.mic_device.as_deref(), "麦克风", &mut args)?;
            2
        }
        _ => return Err("未知录屏音频模式".to_string()),
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
async fn get_ffmpeg_release_info() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("创建请求客户端失败：{}", e))?;
    let release = client
        .get("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest")
        .send()
        .await
        .map_err(|e| format!("检查 ffmpeg 更新失败：{}", e))?
        .error_for_status()
        .map_err(|e| format!("检查 ffmpeg 更新失败：{}", e))?
        .json::<GithubReleaseInfo>()
        .await
        .map_err(|e| format!("解析 ffmpeg Release 失败：{}", e))?;

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
        .ok_or_else(|| "官方 Release 中未找到 Windows x64 ffmpeg zip 包".to_string())?;

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
async fn download_ffmpeg_release(
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
            "请选择 BtbN/FFmpeg-Builds 官方 GitHub Release 的 Windows zip 文件".to_string(),
        );
    }

    emit_ffmpeg_progress(&app, "准备下载", 0, None, 1);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("创建下载客户端失败：{}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("下载 ffmpeg 失败：{}", e))?;
    if !resp.status().is_success() {
        return Err(format!("下载 ffmpeg 失败：HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取 ffmpeg 下载数据失败：{}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let percent = total
            .map(|value| ((downloaded as f64 / value.max(1) as f64) * 80.0).round() as u8)
            .unwrap_or(10)
            .clamp(1, 80);
        emit_ffmpeg_progress(&app, "下载中", downloaded, total, percent);
    }

    let safe_tag = sanitize_tag(&tag);
    let mut download_dir = app_data_dir();
    download_dir.push("ffmpeg");
    download_dir.push("downloads");
    fs::create_dir_all(&download_dir).map_err(|e| format!("创建 ffmpeg 下载目录失败：{}", e))?;
    let archive_path = download_dir.join(format!("ffmpeg-{}.zip", safe_tag));
    fs::write(&archive_path, &bytes).map_err(|e| format!("保存 ffmpeg 压缩包失败：{}", e))?;

    emit_ffmpeg_progress(&app, "安装中", downloaded, total, 85);
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

#[tauri::command]
fn choose_ffmpeg_executable(current_path: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("选择 ffmpeg.exe")
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
fn choose_recording_output_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("选择录屏输出目录");
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
fn start_recording(app: tauri::AppHandle, options: RecordingOptions) -> Result<String, String> {
    let _ = cleanup_finished_recording_process()?;
    {
        let guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("录屏已在进行中".to_string());
        }
    }

    let ffmpeg = find_ffmpeg_executable(&app).ok_or_else(|| {
        "未找到 ffmpeg.exe。默认请放在软件同级 ffmpeg\\ffmpeg.exe，或在“模型/视频配置”里选择 ffmpeg.exe。".to_string()
    })?;
    let output_path = recording_output_path(options.output_dir.clone())?;
    let args = build_recording_args(&options, &output_path)?;

    let mut cmd = hidden_ffmpeg_command(&ffmpeg);
    cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg recording: {}", e))?;
    if let Some(status) = child
        .try_wait()
        .map_err(|e| format!("Failed to inspect ffmpeg recording process: {}", e))?
    {
        return Err(format!("ffmpeg recording exited immediately with status {}. Check recording options, audio device, or ffmpeg version.", status));
    }
    let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("录屏已在进行中".to_string());
    }
    *guard = Some(child);
    Ok(output_path.to_string_lossy().to_string())
}

fn stop_recording_internal(grace_ms: u64) -> Result<(), String> {
    let child = {
        let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(mut child) = child {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(b"q\n");
            let _ = stdin.flush();
        }
        let attempts = (grace_ms / 100).max(1);
        let mut exited = false;
        for attempt in 0..attempts {
            if child
                .try_wait()
                .map_err(|e| format!("Failed to stop recording process: {}", e))?
                .is_some()
            {
                exited = true;
                break;
            }
            if attempt + 1 < attempts {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        if !exited {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    Ok(())
}
#[tauri::command]
fn stop_recording() -> Result<(), String> {
    stop_recording_internal(1200)
}

#[tauri::command]
fn cancel_recording_process() -> Result<(), String> {
    stop_recording_internal(250)
}

fn default_recording_file_name() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("YSN_Recording_{}.mp4", seconds)
}

fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''")
}

#[tauri::command]
fn concat_recording_segments(
    app: tauri::AppHandle,
    segment_paths: Vec<String>,
) -> Result<String, String> {
    if segment_paths.is_empty() {
        return Err("没有可合并的录屏片段".to_string());
    }
    let existing_segments: Vec<PathBuf> = segment_paths
        .iter()
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| path.exists())
        .collect();
    if existing_segments.is_empty() {
        return Err("录屏片段不存在，无法保存".to_string());
    }

    let default_name = default_recording_file_name();
    let save_path = rfd::FileDialog::new()
        .set_title("保存区域录屏")
        .add_filter("MP4 视频", &["mp4"])
        .set_file_name(&default_name)
        .save_file()
        .ok_or_else(|| "用户取消了保存".to_string())?;

    if existing_segments.len() == 1 {
        fs::copy(&existing_segments[0], &save_path).map_err(|e| format!("保存录屏失败：{}", e))?;
        return Ok(save_path.to_string_lossy().to_string());
    }

    let ffmpeg = find_ffmpeg_executable(&app)
        .ok_or_else(|| "未找到 ffmpeg.exe，无法合并录屏片段".to_string())?;
    let mut list_path = app_data_dir();
    list_path.push("recordings");
    fs::create_dir_all(&list_path).map_err(|e| format!("创建录屏临时目录失败：{}", e))?;
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
    fs::write(&list_path, list_body).map_err(|e| format!("写入录屏合并列表失败：{}", e))?;

    let args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.to_string_lossy().to_string(),
        "-c".to_string(),
        "copy".to_string(),
        save_path.to_string_lossy().to_string(),
    ];
    let status = hidden_ffmpeg_command(&ffmpeg)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("启动 ffmpeg 合并失败：{}", e))?;
    let _ = fs::remove_file(&list_path);
    if !status.success() {
        return Err(format!("ffmpeg 合并录屏片段失败：{}", status));
    }
    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
fn cleanup_recording_files(paths: Vec<String>) -> Result<(), String> {
    for path in paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path_buf = PathBuf::from(trimmed);
        if path_buf.exists() {
            let _ = fs::remove_file(path_buf);
        }
    }
    Ok(())
}

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Arc;
use std::time::Instant;

static RECORDING_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
fn get_recording_process() -> &'static Mutex<Option<Child>> {
    RECORDING_PROCESS.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f64,
    pub box_coords: Vec<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
struct PaddleOcrOutput {
    code: i32,
    data: Option<serde_json::Value>,
    msg: Option<String>,
}

struct LocalOcrProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    config_key: String,
}

struct OcrManagerState {
    process: Option<LocalOcrProcess>,
    last_used: Instant,
}

static OCR_MANAGER: OnceLock<Arc<Mutex<OcrManagerState>>> = OnceLock::new();

fn get_ocr_manager() -> Arc<Mutex<OcrManagerState>> {
    OCR_MANAGER
        .get_or_init(|| {
            let state = Arc::new(Mutex::new(OcrManagerState {
                process: None,
                last_used: Instant::now(),
            }));

            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    let mut guard = state_clone.lock().unwrap();
                    let should_kill = if guard.process.is_some() {
                        guard.last_used.elapsed() > Duration::from_secs(300)
                    } else {
                        false
                    };
                    if should_kill {
                        println!("PaddleOCR-json idle timeout reached. Terminating process...");
                        if let Some(mut proc) = guard.process.take() {
                            let _ = proc.child.kill();
                        }
                    }
                }
            });

            state
        })
        .clone()
}

fn start_ocr_process(
    exe_path: &std::path::Path,
    config_key: &str,
) -> Result<LocalOcrProcess, String> {
    if !exe_path.is_file() {
        return Err(format!(
            "本地 OCR 执行文件无效：{}",
            exe_path.to_string_lossy()
        ));
    }

    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "无法获取可执行文件所在目录".to_string())?;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new(exe_path);
    cmd.current_dir(exe_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if !config_key.is_empty() {
        let config_path = format!("models/config_{}.txt", config_key);
        if exe_dir.join(&config_path).exists() {
            cmd.arg(format!("--config_path={}", config_path));
        }
    }

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 PaddleOCR 子进程失败: {}", e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or("无法打开 stdin 管道".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or("无法打开 stdout 管道".to_string())?;
    let mut reader = BufReader::new(stdout);

    // 同步等待初始化完成标志: "OCR init completed."
    let mut init_line = String::new();
    loop {
        init_line.clear();
        match reader.read_line(&mut init_line) {
            Ok(0) => return Err("PaddleOCR 进程在初始化完成前已关闭".to_string()),
            Ok(_) => {
                if init_line.contains("OCR init completed.") {
                    break;
                }
            }
            Err(e) => return Err(format!("读取 PaddleOCR 初始化输出失败: {}", e)),
        }
    }

    Ok(LocalOcrProcess {
        child,
        stdin,
        reader,
        config_key: config_key.to_string(),
    })
}

fn request_ocr_with_config(
    guard: &mut OcrManagerState,
    exe_path: &std::path::Path,
    image_path: &str,
    config_key: &str,
) -> Result<String, String> {
    let needs_restart = guard
        .process
        .as_ref()
        .map(|process| process.config_key.as_str() != config_key)
        .unwrap_or(true);
    if needs_restart {
        if let Some(mut proc) = guard.process.take() {
            let _ = proc.child.kill();
        }
        guard.process = Some(start_ocr_process(exe_path, config_key)?);
    }

    guard.last_used = Instant::now();
    let proc = guard.process.as_mut().unwrap();
    let req_payload = serde_json::json!({ "image_path": image_path });
    let req_line = format!("{}\n", req_payload.to_string());

    if let Err(e) = proc.stdin.write_all(req_line.as_bytes()) {
        guard.process = None;
        return Err(format!(
            "\u{5199}\u{5165} PaddleOCR-json \u{7ba1}\u{9053}\u{5931}\u{8d25}: {}",
            e
        ));
    }
    if let Err(e) = proc.stdin.flush() {
        guard.process = None;
        return Err(format!(
            "\u{5237}\u{65b0} PaddleOCR-json \u{7ba1}\u{9053}\u{5931}\u{8d25}: {}",
            e
        ));
    }

    let mut resp_line = String::new();
    match proc.reader.read_line(&mut resp_line) {
        Ok(0) => {
            guard.process = None;
            Err(
                "PaddleOCR \u{8fdb}\u{7a0b}\u{5f02}\u{5e38}\u{4e2d}\u{65ad}\u{9000}\u{51fa}"
                    .to_string(),
            )
        }
        Ok(_) => Ok(resp_line),
        Err(e) => {
            guard.process = None;
            Err(format!("\u{4ece} PaddleOCR \u{7ba1}\u{9053}\u{8bfb}\u{53d6}\u{6570}\u{636e}\u{53d1}\u{751f}\u{9519}\u{8bef}: {}", e))
        }
    }
}

fn parse_box_coords(item: &serde_json::Value) -> Vec<Vec<i32>> {
    let candidate = item
        .get("box")
        .or_else(|| item.get("box_coords"))
        .or_else(|| item.get("points"))
        .or_else(|| item.get("dt_boxes"));
    let mut box_coords = Vec::new();
    if let Some(arr) = candidate.and_then(|value| value.as_array()) {
        for point in arr {
            if let Some(pt) = point.as_array() {
                let x = pt
                    .get(0)
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0)
                    .round() as i32;
                let y = pt
                    .get(1)
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0)
                    .round() as i32;
                box_coords.push(vec![x, y]);
            }
        }
    }
    box_coords
}

fn parse_generic_ocr_blocks(value: &serde_json::Value) -> Vec<OcrBlock> {
    let array = value
        .as_array()
        .or_else(|| value.get("data").and_then(|data| data.as_array()))
        .or_else(|| value.get("result").and_then(|data| data.as_array()))
        .or_else(|| value.get("blocks").and_then(|data| data.as_array()))
        .or_else(|| value.get("results").and_then(|data| data.as_array()));
    let mut blocks = Vec::new();
    if let Some(items) = array {
        for item in items {
            let text = item
                .get("text")
                .or_else(|| item.get("txt"))
                .or_else(|| item.get("content"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            if text.is_empty() {
                continue;
            }
            let confidence = item
                .get("score")
                .or_else(|| item.get("confidence"))
                .or_else(|| item.get("conf"))
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0);
            blocks.push(OcrBlock {
                text,
                confidence,
                box_coords: parse_box_coords(item),
            });
        }
    }
    blocks
}

fn parse_cli_json_ocr_response(output: &str) -> Result<Vec<OcrBlock>, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("OCR CLI returned empty output".to_string());
    }
    let parsed: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
        format!(
            "Failed to parse OCR CLI JSON output: {} (Raw: {})",
            e, trimmed
        )
    })?;
    let blocks = parse_generic_ocr_blocks(&parsed);
    Ok(blocks)
}

fn run_cli_json_file_ocr(
    exe_path: &std::path::Path,
    image_path: &str,
    manifest: Option<serde_json::Value>,
) -> Result<Vec<OcrBlock>, String> {
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Cannot resolve OCR runtime directory".to_string())?;
    let mut cmd = Command::new(exe_path);
    cmd.current_dir(exe_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let args = manifest
        .as_ref()
        .and_then(|manifest| manifest.get("args"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_else(|| vec![serde_json::Value::String("{image}".to_string())]);
    for arg in args {
        if let Some(text) = arg.as_str() {
            cmd.arg(text.replace("{image}", image_path));
        }
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run OCR CLI runtime: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "OCR CLI runtime exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_cli_json_ocr_response(&stdout)
}

fn parse_ocr_response(resp_line: &str, language_label: &str) -> Result<Vec<OcrBlock>, String> {
    let parsed: PaddleOcrOutput = serde_json::from_str(resp_line)
        .map_err(|e| format!("\u{89e3}\u{6790} PaddleOCR \u{8fd4}\u{56de}\u{7684} JSON \u{5931}\u{8d25}: {} (Raw: {})", e, resp_line))?;

    if parsed.code != 100 {
        let detail = parsed
            .msg
            .or_else(|| {
                parsed
                    .data
                    .as_ref()
                    .and_then(|value| value.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "\u{65e0}\u{8be6}\u{7ec6}\u{9519}\u{8bef}".to_string());
        return Err(format!("OCR \u{8bc6}\u{522b}\u{5931}\u{8d25}: PaddleOCR-json \u{8fd4}\u{56de} code={}, msg={}, \u{6a21}\u{578b}={}\u{3002}\u{5982}\u{679c}\u{6b63}\u{5728}\u{8bc6}\u{522b}\u{97e9}\u{6587}\u{ff0c}\u{7a0b}\u{5e8f}\u{4f1a}\u{81ea}\u{52a8}\u{5c1d}\u{8bd5}\u{97e9}\u{6587}\u{6a21}\u{578b}; \u{5426}\u{5219}\u{8bf7}\u{5728} OCR \u{914d}\u{7f6e}\u{9875}\u{66f4}\u{65b0}\u{8fd0}\u{884c}\u{5305}\u{6216}\u{66f4}\u{6362}\u{5bf9}\u{5e94}\u{8bed}\u{8a00}\u{6a21}\u{578b}\u{3002}", parsed.code, detail, language_label));
    }

    let mut ocr_blocks = Vec::new();
    if let Some(data) = parsed.data {
        if let Some(arr) = data.as_array() {
            for item in arr {
                let text = item
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string();
                let confidence = item.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
                let mut box_coords = Vec::new();
                if let Some(box_val) = item.get("box") {
                    if let Some(box_arr) = box_val.as_array() {
                        for point in box_arr {
                            if let Some(pt) = point.as_array() {
                                let x = pt.get(0).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                let y = pt.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                box_coords.push(vec![x, y]);
                            }
                        }
                    }
                }
                ocr_blocks.push(OcrBlock {
                    text,
                    confidence,
                    box_coords,
                });
            }
        }
    }
    Ok(ocr_blocks)
}

#[tauri::command]
async fn run_local_ocr(
    app: tauri::AppHandle,
    image_base64: String,
    executable_path: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<Vec<OcrBlock>, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15000).clamp(500, 60000));
    let task =
        tokio::task::spawn_blocking(move || run_local_ocr_sync(app, image_base64, executable_path));
    match tokio::time::timeout(timeout, task).await {
        Ok(joined) => joined.map_err(|e| format!("本地 OCR 任务执行失败: {}", e))?,
        Err(_) => {
            let manager = get_ocr_manager();
            if let Ok(mut guard) = manager.try_lock() {
                if let Some(mut proc) = guard.process.take() {
                    let _ = proc.child.kill();
                }
            }
            Err(format!("本地 OCR 超时 ({} ms)", timeout.as_millis()))
        }
    }
}

fn run_local_ocr_sync(
    app: tauri::AppHandle,
    image_base64: String,
    executable_path: Option<String>,
) -> Result<Vec<OcrBlock>, String> {
    let resolved_exe = resolve_local_ocr_executable(&app, executable_path)?;

    if !resolved_exe.is_file() {
        return Err(format!("本地 OCR 执行文件不存在于 {:?}", resolved_exe));
    }

    // 2. 解码并使用高精度微秒级时间戳作为唯一标识保存临时识别图片，防并发冲突
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("图片解码失败: {}", e))?;

    let rand_suffix: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let mut ocr_temp_path = std::env::temp_dir();
    ocr_temp_path.push(format!("ocr-{}.png", rand_suffix));
    fs::write(&ocr_temp_path, &bytes).map_err(|e| format!("保存临时识别图像失败: {}", e))?;

    let abs_image_path = ocr_temp_path.to_string_lossy().to_string();

    let manifest = read_ocr_runtime_manifest(&resolved_exe);
    let protocol = ocr_runtime_protocol(&resolved_exe);
    let result = if protocol == "cli-json-file" {
        run_cli_json_file_ocr(&resolved_exe, &abs_image_path, manifest)
    } else if protocol == "paddleocr-json-stdin" {
        let manager = get_ocr_manager();
        let mut guard = manager.lock().unwrap();
        let default_resp = request_ocr_with_config(&mut guard, &resolved_exe, &abs_image_path, "")?;
        let first_result = match parse_ocr_response(&default_resp, "default") {
            Ok(blocks) if !blocks.is_empty() => Ok(blocks),
            Ok(_) => Err("OCR default model recognized no text".to_string()),
            Err(error) => Err(error),
        };
        match first_result {
            Ok(blocks) => Ok(blocks),
            Err(first_error) => {
                let korean_config = resolved_exe
                    .parent()
                    .map(|dir| dir.join("models").join("config_korean.txt"))
                    .filter(|path| path.exists());
                if korean_config.is_some() {
                    match request_ocr_with_config(
                        &mut guard,
                        &resolved_exe,
                        &abs_image_path,
                        "korean",
                    )
                    .and_then(|resp| parse_ocr_response(&resp, "korean"))
                    {
                        Ok(blocks) if !blocks.is_empty() => Ok(blocks),
                        Ok(_) => Err(first_error),
                        Err(korean_error) => Err(format!(
                            "{}; Korean model retry failed: {}",
                            first_error, korean_error
                        )),
                    }
                } else {
                    Err(first_error)
                }
            }
        }
    } else {
        Err(format!("Unsupported OCR runtime protocol: {}", protocol))
    };

    let _ = fs::remove_file(&ocr_temp_path);
    result
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryRecord {
    pub id: String,
    pub time: String,
    pub filename: String,
    pub blocks: i32,
    pub channel: String,
    pub duration: String,
    pub status: String,
}

fn history_path_from_config() -> PathBuf {
    let mut config_path = app_data_dir();
    config_path.push("config.json");
    if let Ok(config_str) = fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
            if let Some(dir) = config.get("historyDir").and_then(|value| value.as_str()) {
                let trimmed = dir.trim();
                if !trimmed.is_empty() {
                    return PathBuf::from(trimmed).join("history.json");
                }
            }
        }
    }

    let mut path = app_data_dir();
    path.push("history.json");
    path
}

fn history_limits_from_config() -> (usize, u64) {
    let mut config_path = app_data_dir();
    config_path.push("config.json");
    let cfg: serde_json::Value = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let max_records = cfg
        .get("historyMaxRecords")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .clamp(10, 5000) as usize;
    let max_bytes = cfg
        .get("historyMaxBytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(2 * 1024 * 1024)
        .clamp(64 * 1024, 100 * 1024 * 1024);
    (max_records, max_bytes)
}

#[tauri::command]
fn get_history() -> Result<String, String> {
    let path = history_path_from_config();
    if !path.exists() {
        return Ok("[]".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_history(record: String) -> Result<(), String> {
    let path = history_path_from_config();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut history: Vec<serde_json::Value> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    if let Ok(new_record) = serde_json::from_str::<serde_json::Value>(&record) {
        history.insert(0, new_record); // Add to beginning
        let (max_records, max_bytes) = history_limits_from_config();
        if history.len() > max_records {
            history.truncate(max_records);
        }
        let mut json_str = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
        while json_str.as_bytes().len() as u64 > max_bytes && history.len() > 1 {
            history.pop();
            json_str = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
        }
        fs::write(path, json_str).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_history_info() -> Result<serde_json::Value, String> {
    let path = history_path_from_config();
    let (max_records, max_bytes) = history_limits_from_config();
    let bytes = if path.exists() {
        fs::metadata(&path).map_err(|e| e.to_string())?.len()
    } else {
        0
    };
    let count = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str::<Vec<serde_json::Value>>(&content)
            .map(|items| items.len())
            .unwrap_or(0)
    } else {
        0
    };
    let dir = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(serde_json::json!({
        "path": path.to_string_lossy().to_string(),
        "dir": dir,
        "bytes": bytes,
        "count": count,
        "maxRecords": max_records,
        "maxBytes": max_bytes,
    }))
}

#[tauri::command]
fn choose_history_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("\u{9009}\u{62e9}\u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{76ee}\u{5f55}");
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
fn clear_history() -> Result<(), String> {
    let path = history_path_from_config();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_shortcut_status,
            get_config,
            get_history,
            get_history_info,
            choose_history_dir,
            add_history,
            clear_history,
            get_recording_info,
            get_recording_targets,
            get_ffmpeg_release_info,
            download_ffmpeg_release,
            choose_ffmpeg_executable,
            choose_recording_output_dir,
            start_recording,
            stop_recording,
            cancel_recording_process,
            set_window_capture_excluded,
            show_recording_overlay,
            hide_recording_overlay,
            concat_recording_segments,
            cleanup_recording_files,
            save_config,
            download_paddleocr_release,
            choose_ocr_install_dir,
            choose_ocr_runtime_dir,
            move_ocr_runtime,
            check_local_ocr_status,
            is_autostart_enabled,
            set_autostart_enabled,
            start_screenshot,
            get_fullscreen_image,
            capture_region,
            copy_image_to_clipboard,
            save_image_to_file,
            quick_fullscreen_capture,
            capture_live_region,
            scroll_mouse_at,
            cancel_screenshot,
            force_close_screenshots,
            get_window_rects,
            overlay_ready_to_show,
            run_local_ocr,
            re_register_shortcut
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            if let Some(screenshot_win) = app.get_webview_window("screenshot") {
                disable_windows_transition(&screenshot_win);
            }

            let (configured_hotkey, configured_translate_hotkey) = read_configured_hotkeys();
            let shortcut_status = register_global_shortcuts(
                app.handle(),
                &configured_hotkey,
                &configured_translate_hotkey,
            );
            app.manage(AppShortcutStatus(std::sync::Mutex::new(shortcut_status)));

            let screenshot_item = tauri::menu::MenuItemBuilder::new("立即截图")
                .id("screenshot")
                .build(app)?;
            let show_item = tauri::menu::MenuItemBuilder::new("显示主窗口")
                .id("show")
                .build(app)?;
            let exit_item = tauri::menu::MenuItemBuilder::new("退出")
                .id("exit")
                .build(app)?;
            let tray_menu = tauri::menu::MenuBuilder::new(app)
                .item(&screenshot_item)
                .item(&show_item)
                .separator()
                .item(&exit_item)
                .build()?;
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(
                    tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).unwrap(),
                )
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "screenshot" => {
                        let app_h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_screenshot(app_h, None).await {
                                eprintln!("Failed to start screenshot: {}", e);
                            }
                        });
                    }
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "exit" => {
                        cleanup_temp_files();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    tauri::tray::TrayIconEvent::DoubleClick {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_screenshot(app, None).await {
                                eprintln!("Failed to start screenshot: {}", e);
                            }
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            if label == "screenshot" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let _ = window.set_always_on_top(false);
                        let _ = window.hide();
                        CAPTURING.store(false, Ordering::SeqCst);
                        api.prevent_close();
                    }
                    tauri::WindowEvent::Destroyed => {
                        CAPTURING.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            } else if label.starts_with("screenshot_") {
                if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed =
                    event
                {
                    CAPTURING.store(false, Ordering::SeqCst);
                }
            } else if label == "recording_border" || label.starts_with("recording_border_") {
                if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed =
                    event
                {
                    let _ = window.set_always_on_top(false);
                }
            } else if label == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct RawOcrBlock {
        text: String,
        score: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[derive(Debug, Serialize)]
    struct OcrBlock {
        text: String,
        confidence: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[test]
    fn test_raw_score_mapping() {
        let raw_json =
            r#"{"text": "Test OCR", "score": 0.975, "box_coords": [[0,0],[10,0],[10,5],[0,5]]}"#;
        let raw: RawOcrBlock = serde_json::from_str(raw_json).unwrap();
        let mapped = OcrBlock {
            text: raw.text,
            confidence: raw.score,
            box_coords: raw.box_coords,
        };
        assert_eq!(mapped.confidence, 0.975);
        assert_eq!(mapped.text, "Test OCR");
    }

    #[test]
    fn test_recording_resolution_filter_defaults_to_1080p() {
        assert_eq!(super::resolution_scale_filter("480p"), Some("scale=-2:480"));
        assert_eq!(super::resolution_scale_filter("720p"), Some("scale=-2:720"));
        assert_eq!(
            super::resolution_scale_filter("1080p"),
            Some("scale=-2:1080")
        );
        assert_eq!(super::resolution_scale_filter("original"), None);
        assert_eq!(
            super::resolution_scale_filter("unexpected"),
            Some("scale=-2:1080")
        );
    }

    fn recording_options(audio_mode: &str) -> super::RecordingOptions {
        super::RecordingOptions {
            fps: Some(60),
            resolution: Some("1080p".to_string()),
            audio_mode: Some(audio_mode.to_string()),
            mic_device: Some("dshow:Microphone Array".to_string()),
            system_audio_device: Some("wasapi:default".to_string()),
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        }
    }

    fn output_path() -> &'static std::path::Path {
        std::path::Path::new("recording_test.mp4")
    }

    #[test]
    fn test_recording_args_without_audio_use_default_1080p() {
        let options = super::RecordingOptions {
            fps: None,
            resolution: None,
            audio_mode: None,
            mic_device: None,
            system_audio_device: None,
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        };
        let args = super::build_recording_args(&options, output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-framerate", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-r", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-vf", "scale=-2:1080"]));
        assert!(args.contains(&"-an".to_string()));
        assert_eq!(args.last().unwrap(), "recording_test.mp4");
    }

    #[test]
    fn test_recording_args_original_resolution_omits_scale_filter() {
        let mut options = recording_options("none");
        options.resolution = Some("original".to_string());
        let args = super::build_recording_args(&options, output_path()).unwrap();
        assert!(!args.contains(&"-vf".to_string()));
    }

    #[test]
    fn test_recording_args_system_audio_uses_wasapi() {
        let args =
            super::build_recording_args(&recording_options("system"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "wasapi"]));
        assert!(args.windows(2).any(|pair| pair == ["-i", "default"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "1:a"]));
    }

    #[test]
    fn test_recording_args_microphone_uses_dshow() {
        let args = super::build_recording_args(&recording_options("mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "dshow"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-i", "audio=Microphone Array"]));
    }

    #[test]
    fn test_recording_args_system_and_microphone_mix_audio() {
        let args =
            super::build_recording_args(&recording_options("system_mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair
            == [
                "-filter_complex",
                "[1:a][2:a]amix=inputs=2:duration=longest[aout]"
            ]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[aout]"]));
    }

    #[test]
    fn test_recording_args_reject_missing_or_unknown_audio() {
        let mut missing_mic = recording_options("mic");
        missing_mic.mic_device = Some("  ".to_string());
        assert!(super::build_recording_args(&missing_mic, output_path())
            .unwrap_err()
            .contains("麦克风"));

        let unknown = recording_options("speaker_only");
        assert_eq!(
            super::build_recording_args(&unknown, output_path()).unwrap_err(),
            "未知录屏音频模式"
        );
    }

    #[test]
    fn test_audio_device_parser_deduplicates_dshow_devices() {
        let output = r#"
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "Stereo Mix" (audio)
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "USB Camera" (video)
"#;
        let devices = super::parse_quoted_audio_devices(output, true, None);
        assert_eq!(
            devices,
            vec!["Microphone Array".to_string(), "Stereo Mix".to_string()]
        );
    }

    #[test]
    fn test_audio_device_parser_prefixes_wasapi_devices() {
        let output = r#"
[wasapi @ 000] "default"
[wasapi @ 000] "Speakers (Realtek Audio)"
"#;
        let devices = super::parse_quoted_audio_devices(output, false, Some("wasapi:"));
        assert_eq!(
            devices,
            vec![
                "wasapi:default".to_string(),
                "wasapi:Speakers (Realtek Audio)".to_string()
            ]
        );
    }

    #[test]
    fn test_ffmpeg_input_format_detection() {
        let output = r#"
File formats:
 D  dshow           DirectShow capture
 DE gdigrab         GDI API Windows frame grabber
  E mp4             MP4 muxer
"#;
        assert!(super::ffmpeg_supports_input_format(output, "dshow"));
        assert!(super::ffmpeg_supports_input_format(output, "gdigrab"));
        assert!(!super::ffmpeg_supports_input_format(output, "wasapi"));
        assert!(!super::ffmpeg_supports_input_format(output, "mp4"));
    }

    #[test]
    fn test_sanitize_tag_keeps_release_names_filesystem_safe() {
        assert_eq!(super::sanitize_tag("v1.2.3"), "v1.2.3");
        assert_eq!(
            super::sanitize_tag("release/2026:01 beta"),
            "release_2026_01_beta"
        );
        assert_eq!(super::sanitize_tag("***"), "___");
    }
}

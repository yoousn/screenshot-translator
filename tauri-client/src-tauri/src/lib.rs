#[cfg(windows)]
use std::os::windows::process::CommandExt;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use screenshots::Screen;
use base64::{prelude::BASE64_STANDARD, Engine};
use tauri::Manager;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code, ShortcutState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::time::Duration;
use futures_util::StreamExt;
use tauri::Emitter;

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
static CAPTURING: AtomicBool = AtomicBool::new(false);

static SCREENSHOT_JPEG: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
fn get_screenshot_jpeg() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_JPEG.get_or_init(|| Mutex::new(None))
}

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);

const DEFAULT_SCREENSHOT_HOTKEY: &str = "Alt+A";
const TRANSLATE_HOTKEY_LABEL: &str = "Alt+T";


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
            "cmd" | "command" | "meta" | "win" | "windows" | "super" => modifiers |= Modifiers::META,
            other => return Err(format!("不支持的修饰键: {}", other)),
        }
    }
    if modifiers.is_empty() {
        return Err("快捷键至少需要 Alt/Ctrl/Shift/Win 中的一个修饰键".to_string());
    }

    let key_part = parts.last().copied().unwrap_or_default();
    let code_name = normalize_key_code(key_part).ok_or_else(|| format!("不支持的按键: {}", key_part))?;
    let code = Code::from_str(&code_name).map_err(|_| format!("不支持的按键: {}", key_part))?;
    Ok(Shortcut::new(Some(modifiers), code))
}

fn read_configured_hotkeys() -> (String, String) {
    let mut path = app_data_dir();
    path.push("config.json");
    let Ok(config_str) = fs::read_to_string(path) else {
        return (DEFAULT_SCREENSHOT_HOTKEY.to_string(), TRANSLATE_HOTKEY_LABEL.to_string());
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) else {
        return (DEFAULT_SCREENSHOT_HOTKEY.to_string(), TRANSLATE_HOTKEY_LABEL.to_string());
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

fn register_global_shortcuts(app: &tauri::AppHandle, screenshot_hotkey: &str, translate_hotkey: &str) -> Result<(), String> {
    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    if !screenshot_hotkey.trim().is_empty() {
        match parse_hotkey(screenshot_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app_h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_screenshot(app_h, None).await {
                                eprintln!("Failed to start screenshot: {}", e);
                            }
                        });
                    }
                }) {
                    errors.push(format!("{}: {}", screenshot_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", screenshot_hotkey, e)),
        }
    }

    if !translate_hotkey.trim().is_empty() {
        match parse_hotkey(translate_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app_h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_screenshot(app_h, Some("translate".to_string())).await {
                                eprintln!("Failed to start translate screenshot: {}", e);
                            }
                        });
                    }
                }) {
                    errors.push(format!("{}: {}", translate_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", translate_hotkey, e)),
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
}


#[tauri::command]
fn re_register_shortcut(app: tauri::AppHandle, state: tauri::State<'_, AppShortcutStatus>, hotkey: String, translate_hotkey: Option<String>) -> Result<(), String> {
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

fn sanitize_tag(tag: &str) -> String {
    let safe: String = tag
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') { ch } else { '_' })
        .collect();
    if safe.is_empty() { "latest".to_string() } else { safe }
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

fn default_ocr_install_dir() -> PathBuf {
    let mut dir = app_data_dir();
    dir.push("ocr");
    dir.push("runtime");
    dir
}

fn resolve_local_ocr_executable(app: &tauri::AppHandle, executable_path: Option<String>) -> Result<PathBuf, String> {
    use tauri::path::BaseDirectory;

    if let Some(path) = executable_path.filter(|path| !path.trim().is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let install_dir = default_ocr_install_dir();
    if let Some(path) = find_paddleocr_json_exe(&install_dir) {
        return Ok(path);
    }

    app.path()
        .resolve("resources/ocr/PaddleOCR-json.exe", BaseDirectory::Resource)
        .map_err(|e| format!("\u{89e3}\u{6790} OCR \u{8d44}\u{6e90}\u{8def}\u{5f84}\u{5931}\u{8d25}\u{ff1a}{}", e))
}

fn emit_ocr_progress(app: &tauri::AppHandle, phase: &str, downloaded: u64, total: Option<u64>, percent: u8) {
    let _ = app.emit("ocr-download-progress", serde_json::json!({
        "phase": phase,
        "downloaded": downloaded,
        "total": total,
        "percent": percent,
    }));
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("\u{521b}\u{5efa}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("\u{8bfb}\u{53d6}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))? {
        let entry = entry.map_err(|e| format!("\u{8bfb}\u{53d6}\u{76ee}\u{5f55}\u{9879}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("\u{590d}\u{5236}\u{6587}\u{4ef6}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
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
async fn download_paddleocr_release(app: tauri::AppHandle, url: String, tag: String, install_dir: Option<String>) -> Result<serde_json::Value, String> {
    let allowed = [
        "https://github.com/hiroi-sora/PaddleOCR-json/releases/download/",
        "https://objects.githubusercontent.com/github-production-release-asset-",
    ];
    if !allowed.iter().any(|prefix| url.starts_with(prefix)) || !url.to_ascii_lowercase().ends_with(".7z") {
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
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("\u{4e0b}\u{8f7d} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}{}", e))?;
    if !resp.status().is_success() {
        return Err(format!("\u{4e0b}\u{8f7d} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}HTTP {}", resp.status()));
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
    fs::create_dir_all(&download_dir).map_err(|e| format!("\u{521b}\u{5efa}\u{4e0b}\u{8f7d}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    let archive_path = download_dir.join(filename);
    fs::write(&archive_path, &bytes).map_err(|e| format!("\u{4fdd}\u{5b58} PaddleOCR-json \u{538b}\u{7f29}\u{5305}\u{5931}\u{8d25}\u{ff1a}{}", e))?;

    emit_ocr_progress(&app, "解压中", downloaded, total, 75);
    let install_dir = install_dir
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_ocr_install_dir);
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| format!("\u{6e05}\u{7406} OCR \u{5b89}\u{88c5}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| format!("\u{521b}\u{5efa} OCR \u{5b89}\u{88c5}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;

    sevenz_rust::decompress_file(&archive_path, &install_dir)
        .map_err(|e| format!("\u{89e3}\u{538b} PaddleOCR-json \u{5931}\u{8d25}\u{ff1a}{}", e))?;
    let _ = fs::remove_file(&archive_path);
    emit_ocr_progress(&app, "检查可执行文件", downloaded, total, 95);

    let exe_path = find_paddleocr_json_exe(&install_dir)
        .ok_or_else(|| "\u{89e3}\u{538b}\u{540e}\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe".to_string())?;
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
fn move_ocr_runtime(target_dir: String, executable_path: Option<String>) -> Result<serde_json::Value, String> {
    let target_dir = PathBuf::from(target_dir);
    if target_dir.as_os_str().is_empty() {
        return Err("\u{8bf7}\u{9009}\u{62e9}\u{76ee}\u{6807}\u{76ee}\u{5f55}".to_string());
    }
    stop_ocr_process();

    let source_exe = executable_path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| find_paddleocr_json_exe(&default_ocr_install_dir()))
        .ok_or_else(|| "\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe\u{ff0c}\u{8bf7}\u{5148}\u{4e0b}\u{8f7d}\u{6216}\u{9009}\u{62e9} OCR \u{76ee}\u{5f55}".to_string())?;
    let source_dir = source_exe
        .parent()
        .ok_or_else(|| "\u{65e0}\u{6cd5}\u{89e3}\u{6790} OCR \u{76ee}\u{5f55}".to_string())?
        .to_path_buf();

    let source_canon = fs::canonicalize(&source_dir).map_err(|e| format!("\u{8bfb}\u{53d6} OCR \u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    fs::create_dir_all(&target_dir).map_err(|e| format!("\u{521b}\u{5efa}\u{76ee}\u{6807}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    let target_canon = fs::canonicalize(&target_dir).map_err(|e| format!("\u{89e3}\u{6790}\u{76ee}\u{6807}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}{}", e))?;
    if source_canon == target_canon {
        return Ok(serde_json::json!({
            "path": source_exe.to_string_lossy().to_string(),
            "installDir": source_dir.to_string_lossy().to_string(),
        }));
    }

    copy_dir_recursive(&source_dir, &target_dir)?;
    let exe_path = find_paddleocr_json_exe(&target_dir)
        .ok_or_else(|| "\u{79fb}\u{52a8}\u{5b8c}\u{6210}\u{540e}\u{672a}\u{627e}\u{5230} PaddleOCR-json.exe".to_string())?;

    let default_dir = default_ocr_install_dir();
    if source_canon == fs::canonicalize(&default_dir).unwrap_or(default_dir) {
        let _ = fs::remove_dir_all(&source_dir);
    }

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": target_dir.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
fn check_local_ocr_status(app: tauri::AppHandle, executable_path: Option<String>) -> Result<serde_json::Value, String> {
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
        let current_exe = std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;
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
        if status.success() { Ok(()) } else { Err("reg add command returned non-zero exit code".to_string()) }
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
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT { pub x: i32, pub y: i32 }
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct RECT { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }
    pub type EnumWindowsProc = Option<unsafe extern "system" fn(isize, isize) -> i32>;
    extern "system" {
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
        pub fn EnumChildWindows(hWndParent: isize, lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
        pub fn IsWindowVisible(hWnd: isize) -> i32;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        pub fn DwmSetWindowAttribute(hwnd: isize, dwAttribute: u32, pvAttribute: *const std::ffi::c_void, cbAttribute: u32) -> i32;
        pub fn DwmGetWindowAttribute(hwnd: isize, dwAttribute: u32, pvAttribute: *mut std::ffi::c_void, cbAttribute: u32) -> i32;
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(i32, i32)> {
    let mut point = win32::POINT { x: 0, y: 0 };
    // SAFETY: Calling Win32 API GetCursorPos with a valid mutable pointer to a POINT struct.
    unsafe { if win32::GetCursorPos(&mut point) != 0 { Some((point.x, point.y)) } else { None } }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> Option<(i32, i32)> { None }

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


async fn start_screenshot_impl(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());

    // Hide main window before capture to prevent focus-steal that requires an extra click
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }

    // Capture and encode on a blocking thread to avoid blocking the async runtime
    let (jpeg_bytes, base64_data, screen_info) = tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, String, (i32, i32, u32, u32)), String> {
        let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
        if screens.is_empty() { return Err("未检测到显示器".to_string()); }
        let screen = if let Some((cx, cy)) = get_cursor_position() {
            Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
        } else {
            screens[0]
        };
        let info = screen.display_info;
        let screen_info = (info.x, info.y, info.width, info.height);

        let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
        let mut buffer = std::io::Cursor::new(Vec::new());
        let encoder = screenshots::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
        image.write_with_encoder(encoder).map_err(|e| format!("生成JPEG字节流失败：{}", e))?;
        let jpeg_bytes = buffer.into_inner();
        let base64_data = BASE64_STANDARD.encode(&jpeg_bytes);
        Ok((jpeg_bytes, base64_data, screen_info))
    }).await.map_err(|e| format!("截屏任务执行失败：{}", e))??;

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
            if !parent.exists() { let _ = fs::create_dir_all(parent); }
        }
        let _ = fs::write(&write_path, &jpeg_for_write);
    });

    // 2. 获取预置的静态截图遮罩窗口，或作为后备动态创建
    let screenshot_win = if let Some(win) = app.get_webview_window("screenshot") {
        win
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "screenshot",
            tauri::WebviewUrl::App("index.html".into())
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

    use tauri::Emitter;
    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let _ = screenshot_win.emit("screenshot-updated", base64_data);

    Ok(())
}

#[tauri::command]
async fn overlay_ready_to_show(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
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
fn quick_fullscreen_capture() -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() { return Err("未检测到显示器".to_string()); }
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
    } else {
        screens[0]
    };
    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let (width, height) = image.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData { width: width as usize, height: height as usize, bytes: Cow::Owned(image.into_raw()) };
    clipboard.set_image(img_data).map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
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
        let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
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
    let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
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
fn top_level_windows_at_cursor(cursor_x: i32, cursor_y: i32, excluded_hwnds: Vec<isize>) -> Vec<isize> {
    let mut ctx = WindowSearchContext { cursor_x, cursor_y, excluded_hwnds, matches: Vec::new(), min_size: 50 };
    // SAFETY: EnumWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumWindows(Some(enum_windows_for_cursor), &mut ctx as *mut WindowSearchContext as isize);
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
        win32::EnumChildWindows(root, Some(enum_child_windows_for_cursor), &mut ctx as *mut WindowSearchContext as isize);
    }
    ctx.matches
}

#[tauri::command]
fn get_window_rects(app: tauri::AppHandle, include_controls: Option<bool>) -> Result<String, String> {
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
                    for child in child_windows_at_cursor(hwnd, cx, cy).into_iter().rev().take(1) {
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
    { Ok("[]".to_string()) }
}

#[tauri::command]
async fn cancel_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let _ = screenshot_win.set_always_on_top(false);
        let _ = screenshot_win.hide();
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
    if !path.exists() { return Err("没有可用的全屏截图".to_string()); }
    let bytes = fs::read(&path).map_err(|e| format!("读取全屏图失败：{}", e))?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 { return Err("选区范围无效".to_string()); }

    // Try memory first (fast), fall back to disk
    let jpeg_bytes = {
        let guard = get_screenshot_jpeg().lock().map_err(|e| e.to_string())?;
        if let Some(ref bytes) = *guard {
            bytes.clone()
        } else {
            let mut path = app_data_dir();
            path.push("fullscreen_temp.jpg");
            if !path.exists() { return Err("原始截图文件不存在".to_string()); }
            fs::read(&path).map_err(|e| format!("读取全屏图失败：{}", e))?
        }
    };

    let img = screenshots::image::load_from_memory_with_format(&jpeg_bytes, screenshots::image::ImageFormat::Jpeg)
        .map_err(|e| format!("加载全屏图失败：{}", e))?;
    let iw = img.width() as i32;
    let ih = img.height() as i32;
    let sx = x.clamp(0, iw.saturating_sub(1));
    let sy = y.clamp(0, ih.saturating_sub(1));
    let sw = w.clamp(1, iw - sx);
    let sh = h.clamp(1, ih - sy);
    let cropped = img.crop_imm(sx as u32, sy as u32, sw as u32, sh as u32);
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped.write_to(&mut buffer, screenshots::image::ImageFormat::Png).map_err(|e| format!("图片编码 PNG 失败：{}", e))?;
    let bytes = buffer.into_inner();
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    let _ = fs::write(&cropped_path, &bytes);
    Ok(BASE64_STANDARD.encode(&bytes))
}


#[tauri::command]
fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let bytes = BASE64_STANDARD.decode(&image_base64).map_err(|e| format!("Base64解码失败：{}", e))?;
    let img = screenshots::image::load_from_memory_with_format(&bytes, screenshots::image::ImageFormat::Png).map_err(|e| format!("解析裁剪图像数据失败：{}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData { width: width as usize, height: height as usize, bytes: Cow::Owned(rgba.into_raw()) };
    clipboard.set_image(img_data).map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}

#[tauri::command]
async fn save_image_to_file(image_base64: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD.decode(&image_base64).map_err(|e| format!("Base64解码失败：{}", e))?;
    let file_path = rfd::AsyncFileDialog::new()
        .add_filter("PNG 图像", &["png"])
        .set_file_name("screenshot.png")
        .save_file()
        .await;
    if let Some(file_handle) = file_path {
        let path = file_handle.path();
        fs::write(path, &bytes).map_err(|e| format!("写入文件失败：{}", e))?;
        if !path.exists() { return Err("文件未成功写入磁盘".to_string()); }
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("用户取消了保存".to_string())
    }
}


use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Arc;
use std::time::Instant;

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
    OCR_MANAGER.get_or_init(|| {
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
    }).clone()
}

fn start_ocr_process(exe_path: &std::path::Path, config_key: &str) -> Result<LocalOcrProcess, String> {
    let exe_dir = exe_path.parent().ok_or_else(|| "无法获取可执行文件所在目录".to_string())?;
    
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

    let mut child = cmd.spawn()
        .map_err(|e| format!("启动 PaddleOCR 子进程失败: {}", e))?;
        
    let stdin = child.stdin.take().ok_or("无法打开 stdin 管道".to_string())?;
    let stdout = child.stdout.take().ok_or("无法打开 stdout 管道".to_string())?;
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
    
    Ok(LocalOcrProcess { child, stdin, reader, config_key: config_key.to_string() })
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
        return Err(format!("\u{5199}\u{5165} PaddleOCR-json \u{7ba1}\u{9053}\u{5931}\u{8d25}: {}", e));
    }
    if let Err(e) = proc.stdin.flush() {
        guard.process = None;
        return Err(format!("\u{5237}\u{65b0} PaddleOCR-json \u{7ba1}\u{9053}\u{5931}\u{8d25}: {}", e));
    }

    let mut resp_line = String::new();
    match proc.reader.read_line(&mut resp_line) {
        Ok(0) => {
            guard.process = None;
            Err("PaddleOCR \u{8fdb}\u{7a0b}\u{5f02}\u{5e38}\u{4e2d}\u{65ad}\u{9000}\u{51fa}".to_string())
        }
        Ok(_) => Ok(resp_line),
        Err(e) => {
            guard.process = None;
            Err(format!("\u{4ece} PaddleOCR \u{7ba1}\u{9053}\u{8bfb}\u{53d6}\u{6570}\u{636e}\u{53d1}\u{751f}\u{9519}\u{8bef}: {}", e))
        }
    }
}

fn parse_ocr_response(resp_line: &str, language_label: &str) -> Result<Vec<OcrBlock>, String> {
    let parsed: PaddleOcrOutput = serde_json::from_str(resp_line)
        .map_err(|e| format!("\u{89e3}\u{6790} PaddleOCR \u{8fd4}\u{56de}\u{7684} JSON \u{5931}\u{8d25}: {} (Raw: {})", e, resp_line))?;

    if parsed.code != 100 {
        let detail = parsed
            .msg
            .or_else(|| parsed.data.as_ref().and_then(|value| value.as_str().map(|s| s.to_string())))
            .unwrap_or_else(|| "\u{65e0}\u{8be6}\u{7ec6}\u{9519}\u{8bef}".to_string());
        return Err(format!("OCR \u{8bc6}\u{522b}\u{5931}\u{8d25}: PaddleOCR-json \u{8fd4}\u{56de} code={}, msg={}, \u{6a21}\u{578b}={}\u{3002}\u{5982}\u{679c}\u{6b63}\u{5728}\u{8bc6}\u{522b}\u{97e9}\u{6587}\u{ff0c}\u{7a0b}\u{5e8f}\u{4f1a}\u{81ea}\u{52a8}\u{5c1d}\u{8bd5}\u{97e9}\u{6587}\u{6a21}\u{578b}; \u{5426}\u{5219}\u{8bf7}\u{5728} OCR \u{914d}\u{7f6e}\u{9875}\u{66f4}\u{65b0}\u{8fd0}\u{884c}\u{5305}\u{6216}\u{66f4}\u{6362}\u{5bf9}\u{5e94}\u{8bed}\u{8a00}\u{6a21}\u{578b}\u{3002}", parsed.code, detail, language_label));
    }

    let mut ocr_blocks = Vec::new();
    if let Some(data) = parsed.data {
        if let Some(arr) = data.as_array() {
            for item in arr {
                let text = item.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string();
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
                ocr_blocks.push(OcrBlock { text, confidence, box_coords });
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
    timeout_ms: Option<u64>
) -> Result<Vec<OcrBlock>, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15000).clamp(500, 60000));
    let task = tokio::task::spawn_blocking(move || run_local_ocr_sync(app, image_base64, executable_path));
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
    executable_path: Option<String>
) -> Result<Vec<OcrBlock>, String> {
    let resolved_exe = resolve_local_ocr_executable(&app, executable_path)?;
    
    if !resolved_exe.exists() {
        return Err(format!("本地 OCR 执行文件不存在于 {:?}", resolved_exe));
    }
    
    // 2. 解码并使用高精度微秒级时间戳作为唯一标识保存临时识别图片，防并发冲突
    let bytes = BASE64_STANDARD.decode(&image_base64)
        .map_err(|e| format!("图片解码失败: {}", e))?;
    
    let rand_suffix: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    
    let mut ocr_temp_path = std::env::temp_dir();
    ocr_temp_path.push(format!("ocr-{}.png", rand_suffix));
    fs::write(&ocr_temp_path, &bytes)
        .map_err(|e| format!("保存临时识别图像失败: {}", e))?;
        
    let abs_image_path = ocr_temp_path.to_string_lossy().to_string();
    
    // 3. 通信与常驻进程管理
    let manager = get_ocr_manager();
    let mut guard = manager.lock().unwrap();
    let default_resp = request_ocr_with_config(&mut guard, &resolved_exe, &abs_image_path, "")?;
    let first_error = match parse_ocr_response(&default_resp, "default") {
        Ok(blocks) if !blocks.is_empty() => {
            let _ = fs::remove_file(&ocr_temp_path);
            return Ok(blocks);
        }
        Ok(_) => "OCR \u{8bc6}\u{522b}\u{5931}\u{8d25}: \u{9ed8}\u{8ba4}\u{6a21}\u{578b}\u{672a}\u{8bc6}\u{522b}\u{5230}\u{6587}\u{5b57}".to_string(),
        Err(error) => error,
    };

    let korean_config = resolved_exe
        .parent()
        .map(|dir| dir.join("models").join("config_korean.txt"))
        .filter(|path| path.exists());
    if korean_config.is_some() {
        match request_ocr_with_config(&mut guard, &resolved_exe, &abs_image_path, "korean")
            .and_then(|resp| parse_ocr_response(&resp, "korean"))
        {
            Ok(blocks) if !blocks.is_empty() => {
                let _ = fs::remove_file(&ocr_temp_path);
                return Ok(blocks);
            }
            Ok(_) => {
                let _ = fs::remove_file(&ocr_temp_path);
                return Err(first_error);
            }
            Err(korean_error) => {
                let _ = fs::remove_file(&ocr_temp_path);
                return Err(format!("{}\u{ff1b}\u{97e9}\u{6587}\u{6a21}\u{578b}\u{91cd}\u{8bd5}\u{5931}\u{8d25}: {}", first_error, korean_error));
            }
        }
    }

    let _ = fs::remove_file(&ocr_temp_path);
    Err(first_error)
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
    let max_records = cfg.get("historyMaxRecords").and_then(|v| v.as_u64()).unwrap_or(100).clamp(10, 5000) as usize;
    let max_bytes = cfg.get("historyMaxBytes").and_then(|v| v.as_u64()).unwrap_or(2 * 1024 * 1024).clamp(64 * 1024, 100 * 1024 * 1024);
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
    let bytes = if path.exists() { fs::metadata(&path).map_err(|e| e.to_string())?.len() } else { 0 };
    let count = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str::<Vec<serde_json::Value>>(&content).map(|items| items.len()).unwrap_or(0)
    } else { 0 };
    let dir = path.parent().map(|parent| parent.to_string_lossy().to_string()).unwrap_or_default();
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
    let mut dialog = rfd::FileDialog::new().set_title("\u{9009}\u{62e9}\u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{76ee}\u{5f55}");
    if let Some(dir) = current_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            dialog = dialog.set_directory(trimmed);
        }
    }
    Ok(dialog.pick_folder().map(|path| path.to_string_lossy().to_string()))
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_shortcut_status,
            get_config, get_history, get_history_info, choose_history_dir, add_history, clear_history,
            save_config,
            download_paddleocr_release,
            choose_ocr_install_dir,
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
            cancel_screenshot,
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
            let shortcut_status = register_global_shortcuts(app.handle(), &configured_hotkey, &configured_translate_hotkey);
            app.manage(AppShortcutStatus(std::sync::Mutex::new(shortcut_status)));

            let screenshot_item = tauri::menu::MenuItemBuilder::new("立即截图").id("screenshot").build(app)?;
            let show_item = tauri::menu::MenuItemBuilder::new("显示主窗口").id("show").build(app)?;
            let exit_item = tauri::menu::MenuItemBuilder::new("退出").id("exit").build(app)?;
            let tray_menu = tauri::menu::MenuBuilder::new(app).item(&screenshot_item).item(&show_item).separator().item(&exit_item).build()?;
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).unwrap())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
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
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } => {
                            let app = tray.app_handle();
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        tauri::tray::TrayIconEvent::DoubleClick { button: tauri::tray::MouseButton::Left, .. } => {
                            let app = tray.app_handle().clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = start_screenshot(app, None).await {
                                    eprintln!("Failed to start screenshot: {}", e);
                                }
                            });
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            if label == "screenshot" {
                match event {
                    tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                        CAPTURING.store(false, Ordering::SeqCst);
                    }
                    _ => {}
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
        let raw_json = r#"{"text": "Test OCR", "score": 0.975, "box_coords": [[0,0],[10,0],[10,5],[0,5]]}"#;
        let raw: RawOcrBlock = serde_json::from_str(raw_json).unwrap();
        let mapped = OcrBlock {
            text: raw.text,
            confidence: raw.score,
            box_coords: raw.box_coords,
        };
        assert_eq!(mapped.confidence, 0.975);
        assert_eq!(mapped.text, "Test OCR");
    }
}


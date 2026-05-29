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

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
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

fn read_configured_hotkey() -> String {
    let mut path = app_data_dir();
    path.push("config.json");
    let Ok(config_str) = fs::read_to_string(path) else {
        return DEFAULT_SCREENSHOT_HOTKEY.to_string();
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) else {
        return DEFAULT_SCREENSHOT_HOTKEY.to_string();
    };
    config
        .get("hotkey")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_SCREENSHOT_HOTKEY)
        .to_string()
}

fn register_global_shortcuts(app: &tauri::AppHandle, screenshot_hotkey: &str) -> Result<(), String> {
    let screenshot_shortcut = parse_hotkey(screenshot_hotkey)?;
    let translate_shortcut = parse_hotkey(TRANSLATE_HOTKEY_LABEL)?;

    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;

    let reg_res = app.global_shortcut().on_shortcut(screenshot_shortcut, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let app_h = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_screenshot(app_h, None).await {
                    eprintln!("Failed to start screenshot: {}", e);
                }
            });
        }
    });

    let reg_res_t = app.global_shortcut().on_shortcut(translate_shortcut, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let app_h = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_screenshot(app_h, Some("translate".to_string())).await {
                    eprintln!("Failed to start translate screenshot: {}", e);
                }
            });
        }
    });

    match (reg_res, reg_res_t) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(e1), Err(e2)) => Err(format!("{}: {}; {}: {}", screenshot_hotkey, e1, TRANSLATE_HOTKEY_LABEL, e2)),
        (Err(e), Ok(_)) => Err(format!("{}: {}", screenshot_hotkey, e)),
        (Ok(_), Err(e)) => Err(format!("{}: {}", TRANSLATE_HOTKEY_LABEL, e)),
    }
}


#[tauri::command]
fn re_register_shortcut(app: tauri::AppHandle, state: tauri::State<'_, AppShortcutStatus>, hotkey: String) -> Result<(), String> {
    let status = register_global_shortcuts(&app, hotkey.trim());
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
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT { pub x: i32, pub y: i32 }
    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct RECT { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }
    extern "system" {
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn WindowFromPoint(point: POINT) -> isize;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn GetForegroundWindow() -> isize;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        pub fn DwmSetWindowAttribute(hwnd: isize, dwAttribute: u32, pvAttribute: *const std::ffi::c_void, cbAttribute: u32) -> i32;
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
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

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
    let screen = &screens[0];
    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let (width, height) = image.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData { width: width as usize, height: height as usize, bytes: Cow::Owned(image.into_raw()) };
    clipboard.set_image(img_data).map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}


#[tauri::command]
fn get_window_rects() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut rects: Vec<serde_json::Value> = Vec::new();
        if let Some((cx, cy)) = get_cursor_position() {
            // SAFETY: Calling Win32 WindowFromPoint and GetWindowRect with valid parameters.
            unsafe {
                let hwnd = win32::WindowFromPoint(win32::POINT { x: cx, y: cy });
                if hwnd != 0 {
                    let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if win32::GetWindowRect(hwnd, &mut rect) != 0 {
                        let w = rect.right - rect.left;
                        let h = rect.bottom - rect.top;
                        if w > 50 && h > 50 {
                            rects.push(serde_json::json!({ "x": rect.left, "y": rect.top, "w": w, "h": h }));
                        }
                    }
                }
            }
        }
        // SAFETY: Calling Win32 GetForegroundWindow and GetWindowRect with valid parameters.
        unsafe {
            let fg = win32::GetForegroundWindow();
            if fg != 0 {
                let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
                if win32::GetWindowRect(fg, &mut rect) != 0 {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 50 && h > 50 {
                        let json_rect = serde_json::json!({ "x": rect.left, "y": rect.top, "w": w, "h": h });
                        if !rects.contains(&json_rect) {
                            rects.push(json_rect);
                        }
                    }
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


#[tauri::command]
async fn api_ocr(base64_image: String, server_url: String, client_token: String) -> Result<serde_json::Value, String> {
    let bytes = BASE64_STANDARD.decode(&base64_image).map_err(|e| format!("Base64解码失败：{}", e))?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("region.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new().part("image", part);
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败：{}", e))?;
    let resp = client
        .post(format!("{}/api/ocr", server_url.trim_end_matches('/')))
        .header("x-api-key", &client_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("网络请求失败：{} (请检查服务器地址是否正确、网络是否连通)", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("服务器返回错误 {}：{}", status, body));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| format!("解析OCR响应失败：{}", e))?;
    Ok(data)
}

#[tauri::command]
async fn api_translate(base64_image: String, server_url: String, client_token: String, target_lang: Option<String>) -> Result<serde_json::Value, String> {
    let bytes = BASE64_STANDARD.decode(&base64_image).map_err(|e| format!("Base64解码失败：{}", e))?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("region.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .part("image", part)
        .text("target_lang", target_lang.unwrap_or_else(|| "zh".to_string()));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败：{}", e))?;
    let resp = client
        .post(format!("{}/api/translate", server_url.trim_end_matches('/')))
        .header("x-api-key", &client_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("网络请求失败：{} (请检查服务器地址是否正确、网络是否连通)", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("服务器返回错误 {}：{}", status, body));
    }
    
    let texts_json = resp.headers().get("X-Translate-Texts")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
        
    let result_bytes = resp.bytes().await.map_err(|e| format!("读取翻译结果失败：{}", e))?;
    let image_base64 = BASE64_STANDARD.encode(&result_bytes);
    
    let result = serde_json::json!({
        "image": image_base64,
        "texts": texts_json
    });
    Ok(result)
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

fn start_ocr_process(exe_path: &std::path::Path) -> Result<LocalOcrProcess, String> {
    let exe_dir = exe_path.parent().ok_or_else(|| "无法获取可执行文件所在目录".to_string())?;
    
    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new(exe_path);
    cmd.current_dir(exe_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

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
    
    Ok(LocalOcrProcess { child, stdin, reader })
}

#[tauri::command]
async fn run_local_ocr(
    app: tauri::AppHandle,
    image_base64: String,
    executable_path: Option<String>
) -> Result<Vec<OcrBlock>, String> {
    use tauri::path::BaseDirectory;
    use tauri::Manager;
    
    // 1. 解析可执行文件路径（支持自定义或内置资源包）
    let resolved_exe = if let Some(path) = executable_path {
        std::path::PathBuf::from(path)
    } else {
        app.path()
            .resolve("resources/ocr/PaddleOCR-json.exe", BaseDirectory::Resource)
            .map_err(|e| format!("解析内置资源路径失败: {}", e))?
    };
    
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
    
    if guard.process.is_none() {
        let new_proc = start_ocr_process(&resolved_exe)?;
        guard.process = Some(new_proc);
    }
    
    guard.last_used = Instant::now();
    let proc = guard.process.as_mut().unwrap();
    
    // 写入 stdin 管道
    let req_payload = serde_json::json!({ "image_path": abs_image_path });
    let req_line = format!("{}\n", req_payload.to_string());
    
    if let Err(e) = proc.stdin.write_all(req_line.as_bytes()) {
        guard.process = None; // 管道断开，重置进程状态
        let _ = fs::remove_file(&ocr_temp_path);
        return Err(format!("写入 PaddleOCR-json 管道失败: {}", e));
    }
    if let Err(e) = proc.stdin.flush() {
        let _ = fs::remove_file(&ocr_temp_path);
        return Err(format!("刷新 PaddleOCR-json 管道失败: {}", e));
    }
    
    // 读取 stdout 响应
    let mut resp_line = String::new();
    match proc.reader.read_line(&mut resp_line) {
        Ok(0) => {
            guard.process = None; // 进程已关闭或异常崩溃
            let _ = fs::remove_file(&ocr_temp_path);
            return Err("PaddleOCR 进程异常中断退出".to_string());
        }
        Ok(_) => {
            let _ = fs::remove_file(&ocr_temp_path); // 立即清理临时图像文件
            
            let parsed: PaddleOcrOutput = serde_json::from_str(&resp_line)
                .map_err(|e| format!("解析 PaddleOCR 返回的 JSON 失败: {} (Raw: {})", e, resp_line))?;
                
            if parsed.code != 100 {
                return Err(parsed.msg.unwrap_or_else(|| "OCR 执行出错，返回非100状态码".to_string()));
            }
            
            let mut ocr_blocks = Vec::new();
            if let Some(data) = parsed.data {
                if let Some(arr) = data.as_array() {
                    for item in arr {
                        let text = item.get("text").and_then(|t| t.as_str()).unwrap_or_default().to_string();
                        // 显式将 raw_score 映射为 confidence
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
        Err(e) => {
            let _ = fs::remove_file(&ocr_temp_path);
            Err(format!("从 PaddleOCR 管道读取数据发生错误: {}", e))
        }
    }
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

#[tauri::command]
fn get_history() -> Result<String, String> {
    let mut path = app_data_dir();
    path.push("history.json");
    if !path.exists() {
        return Ok("[]".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_history(record: String) -> Result<(), String> {
    let mut path = app_data_dir();
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    path.push("history.json");
    let mut history: Vec<serde_json::Value> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };
    
    if let Ok(new_record) = serde_json::from_str::<serde_json::Value>(&record) {
        history.insert(0, new_record); // Add to beginning
        if history.len() > 100 { // Keep last 100 records max
            history.truncate(100);
        }
        let json_str = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
        fs::write(path, json_str).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn clear_history() -> Result<(), String> {
    let mut path = app_data_dir();
    path.push("history.json");
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
            get_config, get_history, add_history, clear_history,
            save_config,
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
            api_ocr,
            api_translate,
            overlay_ready_to_show,
            run_local_ocr,
            re_register_shortcut
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            if let Some(screenshot_win) = app.get_webview_window("screenshot") {
                disable_windows_transition(&screenshot_win);
            }

            let configured_hotkey = read_configured_hotkey();
            let shortcut_status = register_global_shortcuts(app.handle(), &configured_hotkey);
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


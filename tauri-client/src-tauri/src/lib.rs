use std::fs;
use std::path::PathBuf;
use std::process::Command;
use screenshots::Screen;
use base64::{prelude::BASE64_STANDARD, Engine};
use tauri::Manager;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code, ShortcutState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::time::{sleep as tokio_sleep, Duration};

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
static CAPTURING: AtomicBool = AtomicBool::new(false);

static SCREENSHOT_JPEG: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
fn get_screenshot_jpeg() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_JPEG.get_or_init(|| Mutex::new(None))
}

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);



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

    // 1. 每次创建新窗口前，先检测并销毁/关闭旧的 screenshot 窗口
    if let Some(old_win) = app.get_webview_window("screenshot") {
        let _ = old_win.destroy();
        tokio_sleep(Duration::from_millis(100)).await;
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

    // 2. 动态创建临时的主图遮罩窗口，透明、无边框、跳过任务栏、置顶且对焦
    let screenshot_win = tauri::WebviewWindowBuilder::new(
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
    .focused(true)
    .build()
    .map_err(|e| format!("创建截图窗口失败：{}", e))?;

    // Disable transition animation to avoid windows rendering delay/flicker
    disable_windows_transition(&screenshot_win);

    let (x, y, width, height) = screen_info;

    // Position and configure the window while still hidden
    let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));

    use tauri::Emitter;
    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let _ = screenshot_win.emit("screenshot-updated", base64_data);

    // Show immediately — no delay needed since the event is already dispatched
    let _ = screenshot_win.show();
    let _ = screenshot_win.set_focus();

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
        let _ = screenshot_win.destroy();
        tokio_sleep(Duration::from_millis(100)).await;
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
async fn api_translate(base64_image: String, server_url: String, client_token: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD.decode(&base64_image).map_err(|e| format!("Base64解码失败：{}", e))?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("region.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new().part("image", part);
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
    let result_bytes = resp.bytes().await.map_err(|e| format!("读取翻译结果失败：{}", e))?;
    Ok(BASE64_STANDARD.encode(&result_bytes))
}



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_shortcut_status,
            get_config,
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
            api_translate
        ])
        .setup(|app| {
            let shortcut_a = Shortcut::new(Some(Modifiers::ALT), Code::KeyA);
            let reg_res = app.global_shortcut().on_shortcut(shortcut_a, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = start_screenshot(app_h, None).await {
                            eprintln!("Failed to start screenshot: {}", e);
                        }
                    });
                }
            });

            let shortcut_t = Shortcut::new(Some(Modifiers::ALT), Code::KeyT);
            let reg_res_t = app.global_shortcut().on_shortcut(shortcut_t, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = start_screenshot(app_h, Some("translate".to_string())).await {
                            eprintln!("Failed to start translate screenshot: {}", e);
                        }
                    });
                }
            });

            let shortcut_status = match (reg_res, reg_res_t) {
                (Ok(_), Ok(_)) => Ok(()),
                (Err(e1), Err(e2)) => Err(format!("Alt+A: {}; Alt+T: {}", e1, e2)),
                (Err(e), Ok(_)) => Err(format!("Alt+A: {}", e)),
                (Ok(_), Err(e)) => Err(format!("Alt+T: {}", e)),
            };
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

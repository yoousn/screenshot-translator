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
use tokio::time::{sleep as tokio_sleep, Duration};

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
static IS_SCREENSHOTTING: AtomicBool = AtomicBool::new(false);

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static PIN_IMAGES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();


fn get_pin_images() -> &'static Mutex<HashMap<String, String>> {
    PIN_IMAGES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
fn get_pin_image(label: String) -> Result<String, String> {
    let map = get_pin_images().lock().map_err(|e| e.to_string())?;
    map.get(&label).cloned().ok_or_else(|| "未找到钉图数据".to_string())
}

#[tauri::command]
fn delete_pin_image(label: String) {
    if let Ok(mut map) = get_pin_images().lock() {
        map.remove(&label);
    }
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
    path.push("fullscreen_temp.png");
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


fn hide_pin_windows(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("pin_") {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
        }
    }
}

fn show_pin_windows(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("pin_") {
            let _ = window.show();
            let _ = window.set_always_on_top(true);
        }
    }
}

#[tauri::command]
async fn start_screenshot(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    if IS_SCREENSHOTTING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("正在截屏中，请勿重复操作".to_string());
    }
    struct ScreenshotGuard;
    impl Drop for ScreenshotGuard {
        fn drop(&mut self) {
            IS_SCREENSHOTTING.store(false, Ordering::SeqCst);
        }
    }
    let _guard = ScreenshotGuard;

    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }
    hide_pin_windows(&app);
    tokio_sleep(Duration::from_millis(250)).await;

    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() { return Err("未检测到显示器".to_string()); }
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
    } else {
        screens[0]
    };

    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, screenshots::image::ImageFormat::Png).map_err(|e| format!("生成PNG字节流失败：{}", e))?;
    let png_bytes = buffer.into_inner();

    let mut path = app_data_dir();
    if !path.exists() { let _ = fs::create_dir_all(&path); }
    path.push("fullscreen_temp.png");
    fs::write(&path, &png_bytes).map_err(|e| format!("保存临时图片失败：{}", e))?;

    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let width = screen.display_info.width;
        let height = screen.display_info.height;
        let x = screen.display_info.x;
        let y = screen.display_info.y;
        let _ = screenshot_win.hide();
        disable_windows_transition(&screenshot_win);
        let _ = screenshot_win.set_fullscreen(false);
        let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
        let _ = screenshot_win.set_always_on_top(true);
        let _ = screenshot_win.show();
        let _ = screenshot_win.set_focus();
        use tauri::Emitter;
        let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
        let _ = screenshot_win.emit("screenshot-updated", ());
    } else {
        show_pin_windows(&app);
        return Err("未获取到名为 screenshot 的窗口句柄".to_string());
    }
    Ok(())
}


#[tauri::command]
fn create_pin_window(app: tauri::AppHandle, image_base64: String, x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    let label = format!("pin_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    if let Ok(mut map) = get_pin_images().lock() {
        map.insert(label.clone(), image_base64.clone());
    }

    let webview = tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("index.html".into()))
        .title("YSN 钉图")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .transparent(true)
        .shadow(false)
        .visible(false)
        .build()
        .map_err(|e| format!("创建钉图窗口失败: {}", e))?;

    disable_windows_transition(&webview);
    let _ = webview.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = webview.set_size(tauri::PhysicalSize::new(w.max(1), h.max(1)));
    let _ = webview.show();
    let _ = webview.set_always_on_top(true);
    let _ = webview.set_focus();

    use tauri::Emitter;
    let _ = webview.emit("pin-image-data", &image_base64);
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let _ = screenshot_win.set_always_on_top(false);
        let _ = screenshot_win.set_fullscreen(false);
        let _ = screenshot_win.hide();
    }
    Ok(label)
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
    tokio_sleep(Duration::from_millis(250)).await;
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let _ = screenshot_win.set_always_on_top(false);
        let _ = screenshot_win.set_fullscreen(false);
        let _ = screenshot_win.hide();
    }
    Ok(())
}


#[tauri::command]
fn get_fullscreen_image() -> Result<String, String> {
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() { return Err("没有可用的全屏截图".to_string()); }
    let bytes = fs::read(&path).map_err(|e| format!("读取全屏图失败：{}", e))?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 { return Err("选区范围无效".to_string()); }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() { return Err("原始截图文件不存在".to_string()); }
    let img = screenshots::image::open(&path).map_err(|e| format!("加载全屏图失败：{}", e))?;
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
    let mut cropped_path = path.clone();
    cropped_path.set_file_name("cropped_temp.png");
    fs::write(&cropped_path, &bytes).map_err(|e| format!("保存裁剪临时图片失败：{}", e))?;
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
            create_pin_window,
            get_pin_image,
            delete_pin_image,
            quick_fullscreen_capture,
            cancel_screenshot,
            get_window_rects
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            if let Some(screenshot_win) = app.get_webview_window("screenshot") {
                disable_windows_transition(&screenshot_win);
            }

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
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == "main" || label == "screenshot" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

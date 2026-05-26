use std::fs;
use std::path::PathBuf;
use std::process::Command;
use screenshots::Screen;
use base64::{prelude::BASE64_STANDARD, Engine};
use tauri::Manager;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code, ShortcutState};

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);

#[tauri::command]
fn get_shortcut_status(state: tauri::State<'_, AppShortcutStatus>) -> Result<(), String> {
    match &*state.0.lock().unwrap() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

#[tauri::command]
fn get_config() -> Result<String, String> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\ysn\\AppData\\Local".to_string());
    let mut path = PathBuf::from(local_app_data);
    path.push("ScreenshotTranslator");
    path.push("config.json");

    if !path.exists() {
        return Ok("{}".to_string());
    }

    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config_str: String) -> Result<(), String> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\ysn\\AppData\\Local".to_string());
    let mut path = PathBuf::from(local_app_data);
    path.push("ScreenshotTranslator");

    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }

    path.push("config.json");
    fs::write(path, config_str).map_err(|e| e.to_string())
}

#[tauri::command]
fn is_autostart_enabled() -> bool {
    let output = Command::new("reg")
        .args(&[
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
            .args(&[
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
        let status = Command::new("reg")
            .args(&[
                "delete",
                "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "ScreenshotTranslator",
                "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to execute reg command: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
mod win32 {
    #[repr(C)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }
    #[repr(C)]
    pub struct RECT {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    
    extern "system" {
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn WindowFromPoint(point: POINT) -> isize;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn GetForegroundWindow() -> isize;
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(i32, i32)> {
    let mut point = win32::POINT { x: 0, y: 0 };
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

#[tauri::command]
fn start_screenshot(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());
    
    // 1. Hide main window first if it is visible
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }

    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() {
        return Err("未检测到显示器".to_string());
    }

    // Capture the screen that has the cursor
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| {
            screens[0]
        })
    } else {
        screens[0]
    };

    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("生成PNG字节流失败：{}", e))?;
    let png_bytes = buffer.into_inner();
    
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\ysn\\AppData\\Local".to_string());
    let mut path = PathBuf::from(local_app_data);
    path.push("ScreenshotTranslator");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("fullscreen_temp.png");
    fs::write(&path, &png_bytes).map_err(|e| format!("保存临时图片失败：{}", e))?;

    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let width = screen.display_info.width;
        let height = screen.display_info.height;
        let x = screen.display_info.x;
        let y = screen.display_info.y;
        
        // Ensure hidden → reposition → show (prevents geometry flash/jitter)
        let _ = screenshot_win.hide();
        let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
        let _ = screenshot_win.set_always_on_top(true);
        let _ = screenshot_win.show();
        let _ = screenshot_win.set_focus();
        
        // Emit events AFTER show so frontend loads the screenshot instantly
        use tauri::Emitter;
        let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
        let _ = screenshot_win.emit("screenshot-updated", ());
    } else {
        return Err("未获取到名为 screenshot 的窗口句柄".to_string());
    }
    Ok(())
}

#[tauri::command]
fn create_pin_window(app: tauri::AppHandle, image_base64: String) -> Result<String, String> {
    let label = format!("pin_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    let webview = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("YSN 贴图")
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .transparent(true)
    .inner_size(400.0, 300.0)
    .build()
    .map_err(|e| format!("创建贴图窗口失败: {}", e))?;

    // Send image data to the new pin window
    use tauri::Emitter;
    let _ = webview.emit("pin-image-data", &image_base64);

    Ok(label)
}

#[tauri::command]
fn quick_fullscreen_capture() -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() {
        return Err("未检测到显示器".to_string());
    }
    let screen = &screens[0];
    let image = screen.capture().map_err(|e| format!("截屏失败：{}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("生成PNG字节流失败：{}", e))?;
    let png_bytes = buffer.into_inner();

    let img = screenshots::image::load_from_memory_with_format(&png_bytes, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("解析截屏图像失败：{}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };
    clipboard.set_image(img_data).map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}

#[tauri::command]
fn get_window_rects() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut rects: Vec<serde_json::Value> = Vec::new();
        // Get window under cursor
        if let Some((cx, cy)) = get_cursor_position() {
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
        // Also get foreground window
        unsafe {
            let fg = win32::GetForegroundWindow();
            if fg != 0 {
                let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
                if win32::GetWindowRect(fg, &mut rect) != 0 {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 50 && h > 50 {
                        let json_rect = serde_json::json!({ "x": rect.left, "y": rect.top, "w": w, "h": h });
                        // Avoid duplicates
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
    {
        Ok("[]".to_string())
    }
}

#[tauri::command]
fn cancel_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    // Just hide screenshot window — main window stays closed (tray only)
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let _ = screenshot_win.hide();
    }
    Ok(())
}

#[tauri::command]
fn get_fullscreen_image() -> Result<String, String> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\ysn\\AppData\\Local".to_string());
    let mut path = PathBuf::from(local_app_data);
    path.push("ScreenshotTranslator");
    path.push("fullscreen_temp.png");
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
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\ysn\\AppData\\Local".to_string());
    let mut path = PathBuf::from(local_app_data);
    path.push("ScreenshotTranslator");
    path.push("fullscreen_temp.png");
    if !path.exists() {
        return Err("原始截图文件不存在".to_string());
    }
    
    let img = screenshots::image::open(&path).map_err(|e| format!("加载全屏图失败：{}", e))?;
    let cropped = img.crop_imm(x as u32, y as u32, w as u32, h as u32);
    
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped.write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("图片编码 PNG 失败：{}", e))?;
    
    let bytes = buffer.into_inner();
    
    let mut cropped_path = path.clone();
    cropped_path.set_file_name("cropped_temp.png");
    fs::write(&cropped_path, &bytes).map_err(|e| format!("保存裁剪临时图片失败：{}", e))?;

    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let bytes = BASE64_STANDARD.decode(&image_base64)
        .map_err(|e| format!("Base64解码失败：{}", e))?;
    let img = screenshots::image::load_from_memory_with_format(&bytes, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("解析裁剪图像数据失败：{}", e))?;
    
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    let mut clipboard = Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败：{}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };
    clipboard.set_image(img_data).map_err(|e| format!("复制图像到剪贴板失败：{}", e))?;
    Ok(())
}

#[tauri::command]
fn save_image_to_file(image_base64: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD.decode(&image_base64)
        .map_err(|e| format!("Base64解码失败：{}", e))?;
    
    let file_path = rfd::FileDialog::new()
        .add_filter("PNG 图像", &["png"])
        .set_file_name("screenshot.png")
        .save_file();
    
    if let Some(path) = file_path {
        fs::write(&path, &bytes).map_err(|e| format!("写入文件失败：{}", e))?;
        if !path.exists() {
            return Err("文件未成功写入磁盘".to_string());
        }
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
            quick_fullscreen_capture,
            cancel_screenshot,
            get_window_rects
        ])
        .setup(|app| {
            // Register Alt+A shortcut (normal screenshot)
            let shortcut_a = Shortcut::new(Some(Modifiers::ALT), Code::KeyA);
            let reg_res = app.global_shortcut().on_shortcut(shortcut_a, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = start_screenshot(app_h, None) {
                            eprintln!("Failed to start screenshot: {}", e);
                        }
                    });
                }
            });

            // Register Alt+T shortcut (translate screenshot)
            let shortcut_t = Shortcut::new(Some(Modifiers::ALT), Code::KeyT);
            let _ = app.global_shortcut().on_shortcut(shortcut_t, move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = start_screenshot(app_h, Some("translate".to_string())) {
                            eprintln!("Failed to start translate screenshot: {}", e);
                        }
                    });
                }
            });

            let shortcut_status = match reg_res {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            };

            app.manage(AppShortcutStatus(std::sync::Mutex::new(shortcut_status)));

            // Build Tray Icon
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
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).unwrap())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "screenshot" => {
                            let app_h = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = start_screenshot(app_h, None) {
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
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Main window starts hidden (visible:false in config). Only shown via tray menu.
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Any window close → just hide, don't exit the app
                // Only way to exit is tray right-click → 退出, or kill process
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

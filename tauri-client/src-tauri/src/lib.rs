use std::fs;
use std::path::PathBuf;
use std::process::Command;
use screenshots::Screen;
use base64::{prelude::BASE64_STANDARD, Engine};
use tauri::Manager;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

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

#[tauri::command]
fn start_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("无法获取显示设备：{}", e))?;
    if screens.is_empty() {
        return Err("未检测到显示器".to_string());
    }
    // 暂取第一个显示器作为主显示器
    let screen = screens[0];
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
        
        let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
        let _ = screenshot_win.set_always_on_top(true);
        let _ = screenshot_win.show();
        let _ = screenshot_win.set_focus();
    } else {
        return Err("未获取到名为 screenshot 的窗口句柄".to_string());
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
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("用户取消了保存".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            is_autostart_enabled,
            set_autostart_enabled,
            start_screenshot,
            get_fullscreen_image,
            capture_region,
            copy_image_to_clipboard,
            save_image_to_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

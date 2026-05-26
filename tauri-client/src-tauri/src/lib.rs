use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
fn start_screenshot() -> Result<(), String> {
    // 启动编译好的 release\ScreenshotTranslator.exe
    let abs_path = std::path::PathBuf::from("c:\\Users\\ysn\\Desktop\\zzjt\\release\\ScreenshotTranslator.exe");
    
    let exe_path = if abs_path.exists() {
        abs_path
    } else {
        let mut path = std::env::current_dir().unwrap_or_default();
        path.push("release");
        path.push("ScreenshotTranslator.exe");
        if path.exists() {
            path
        } else {
            let mut parent = std::env::current_dir().unwrap_or_default();
            parent.pop();
            parent.push("release");
            parent.push("ScreenshotTranslator.exe");
            parent
        }
    };

    if !exe_path.exists() {
        return Err(format!("找不到截图程序：{}", exe_path.to_string_lossy()));
    }

    let working_dir = exe_path.parent().unwrap_or(&exe_path);
    std::process::Command::new(&exe_path)
        .current_dir(working_dir)
        .arg("--screenshot")
        .spawn()
        .map_err(|e| format!("启动截图程序失败：{}", e))?;

    Ok(())
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
            start_screenshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}



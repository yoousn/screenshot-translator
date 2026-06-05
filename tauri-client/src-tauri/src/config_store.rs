use crate::*;
use std::fs;

#[tauri::command]
pub fn get_config() -> Result<String, String> {
    let mut path = app_data_dir();
    path.push("config.json");
    if !path.exists() {
        return Ok("{}".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(config_str: String) -> Result<(), String> {
    let mut path = app_data_dir();
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    path.push("config.json");
    fs::write(path, config_str).map_err(|e| e.to_string())
}

pub fn config_value_string(key: &str) -> Option<String> {
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

pub fn config_value_bool(key: &str) -> Option<bool> {
    let mut path = app_data_dir();
    path.push("config.json");
    let content = fs::read_to_string(path).ok()?;
    let config = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    match config.get(key)? {
        serde_json::Value::Bool(value) => Some(*value),
        serde_json::Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

#[tauri::command]
pub fn is_autostart_enabled() -> bool {
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
pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
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


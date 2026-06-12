use crate::*;
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

static CONFIG_CACHE: OnceLock<RwLock<Option<serde_json::Value>>> = OnceLock::new();

fn config_cache() -> &'static RwLock<Option<serde_json::Value>> {
    CONFIG_CACHE.get_or_init(|| RwLock::new(None))
}

fn config_path() -> PathBuf {
    let mut path = app_data_dir();
    path.push("config.json");
    path
}

fn load_config_value() -> Option<serde_json::Value> {
    if let Ok(guard) = config_cache().read() {
        if let Some(value) = guard.as_ref() {
            return Some(value.clone());
        }
    }

    let content = fs::read_to_string(config_path()).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    if let Ok(mut guard) = config_cache().write() {
        *guard = Some(value.clone());
    }
    Some(value)
}

fn update_config_cache_from_str(config_str: &str) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(config_str) {
        if let Ok(mut guard) = config_cache().write() {
            *guard = Some(value);
        }
    } else if let Ok(mut guard) = config_cache().write() {
        *guard = None;
    }
}

#[tauri::command]
pub fn get_config() -> Result<String, String> {
    let path = config_path();
    if !path.exists() {
        return Ok("{}".to_string());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    update_config_cache_from_str(&content);
    Ok(content)
}

#[tauri::command]
pub fn save_config(config_str: String) -> Result<(), String> {
    let mut path = app_data_dir();
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    path.push("config.json");
    fs::write(path, &config_str).map_err(|e| e.to_string())?;
    update_config_cache_from_str(&config_str);
    Ok(())
}

pub fn config_value_string(key: &str) -> Option<String> {
    let config = load_config_value()?;
    config
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn config_value_bool(key: &str) -> Option<bool> {
    let config = load_config_value()?;
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

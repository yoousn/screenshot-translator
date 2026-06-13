use crate::*;
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use tauri::Emitter;

static CONFIG_CACHE: OnceLock<RwLock<Option<serde_json::Value>>> = OnceLock::new();

/// 事件名：配置写入成功后广播给所有 webview 窗口。
/// 监听方收到后应重新 `get_config` 刷新内存中的 configRef/状态，
/// 使功能开关、模型版本等无需重启即可生效。
pub const CONFIG_CHANGED_EVENT: &str = "config-changed";

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

/// 写入配置到磁盘并刷新内存缓存。纯逻辑，不触发事件广播。
/// 供需要写配置但不便（或不应）广播的内部路径调用。
fn write_config_to_disk(config_str: &str) -> Result<(), String> {
    let mut path = app_data_dir();
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    path.push("config.json");
    fs::write(path, config_str).map_err(|e| e.to_string())?;
    update_config_cache_from_str(config_str);
    Ok(())
}

/// 广播配置变更事件给所有 webview 窗口。
/// 监听方据此重新 `get_config` 刷新内存中的 configRef，
/// 使功能开关、模型版本、翻译目标语等无需重启即可生效（工单②）。
fn emit_config_changed(app: &tauri::AppHandle, config_str: &str) {
    let payload = serde_json::json!({
        // 顺带把新配置内容带上，监听方可直接使用而无需再发起一次 get_config 往返。
        "config": config_str,
    });
    let _ = app.emit(CONFIG_CHANGED_EVENT, payload);
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config_str: String) -> Result<(), String> {
    write_config_to_disk(&config_str)?;
    // 工单②：写入成功后广播，让截图后台窗口 / 主设置窗口等重载配置，
    // 实现"功能开关保存即生效"，无需退出重进。
    emit_config_changed(&app, &config_str);
    Ok(())
}

/// 更新单个配置字段并在发生变化时写盘+广播。
/// 由无法直接持有 AppHandle 的内部路径使用（例如截图流程里记住上次保存目录）。
pub fn set_config_value_if_changed(
    app: &tauri::AppHandle,
    key: &str,
    value: serde_json::Value,
) -> Result<bool, String> {
    let mut config = load_config_value()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    if config.get(key) == Some(&value) {
        return Ok(false);
    }
    config.insert(key.to_string(), value);
    let config_str = serde_json::to_string_pretty(&serde_json::Value::Object(config))
        .map_err(|e| e.to_string())?;
    write_config_to_disk(&config_str)?;
    emit_config_changed(app, &config_str);
    Ok(true)
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

use crate::*;
use std::path::{Path, PathBuf};
use std::fs;

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

pub fn history_path_from_config() -> PathBuf {
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

pub fn history_limits_from_config() -> (usize, u64) {
    let mut config_path = app_data_dir();
    config_path.push("config.json");
    let cfg: serde_json::Value = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let max_records = cfg
        .get("historyMaxRecords")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .clamp(10, 5000) as usize;
    let max_bytes = cfg
        .get("historyMaxBytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(2 * 1024 * 1024)
        .clamp(64 * 1024, 100 * 1024 * 1024);
    (max_records, max_bytes)
}

#[tauri::command]
pub fn get_history() -> Result<String, String> {
    let path = history_path_from_config();
    if !path.exists() {
        return Ok("[]".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_history(record: String) -> Result<(), String> {
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
pub fn get_history_info() -> Result<serde_json::Value, String> {
    let path = history_path_from_config();
    let (max_records, max_bytes) = history_limits_from_config();
    let bytes = if path.exists() {
        fs::metadata(&path).map_err(|e| e.to_string())?.len()
    } else {
        0
    };
    let count = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str::<Vec<serde_json::Value>>(&content)
            .map(|items| items.len())
            .unwrap_or(0)
    } else {
        0
    };
    let dir = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_default();
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
pub fn choose_history_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("\u{9009}\u{62e9}\u{5386}\u{53f2}\u{8bb0}\u{5f55}\u{76ee}\u{5f55}");
    if let Some(dir) = current_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            dialog = dialog.set_directory(trimmed);
        }
    }
    Ok(dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    let path = history_path_from_config();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}


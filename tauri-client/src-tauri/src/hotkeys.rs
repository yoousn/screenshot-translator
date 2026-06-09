use crate::*;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

pub struct AppShortcutStatus(pub std::sync::Mutex<Result<(), String>>);

const DEFAULT_SCREENSHOT_HOTKEY: &str = "Alt+A";
const TRANSLATE_HOTKEY_LABEL: &str = "Alt+T";
const RECORDING_HOTKEY_LABEL: &str = "Alt+R";
static CAPTURE_ESCAPE_SHORTCUT_REGISTERED: AtomicBool = AtomicBool::new(false);

fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn accept_capture_shortcut_press() -> bool {
    let now = now_epoch_millis();
    let previous = LAST_CAPTURE_SHORTCUT_MS.load(Ordering::SeqCst);
    if now.saturating_sub(previous) < 450 {
        return false;
    }
    LAST_CAPTURE_SHORTCUT_MS.store(now, Ordering::SeqCst);
    true
}

pub fn normalize_key_code(key: &str) -> Option<String> {
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

pub fn parse_hotkey(hotkey: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = hotkey
        .split(|ch| ch == '+' || ch == '-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return Err("Hotkey requires at least one modifier, for example Alt+A".to_string());
    }

    let mut modifiers = Modifiers::empty();
    for part in &parts[..parts.len() - 1] {
        match part.to_ascii_lowercase().as_str() {
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "cmd" | "command" | "meta" | "win" | "windows" | "super" => {
                modifiers |= Modifiers::META
            }
            other => return Err(format!("Unsupported modifier key: {}", other)),
        }
    }
    if modifiers.is_empty() {
        return Err("Hotkey requires one of Alt/Ctrl/Shift/Win".to_string());
    }

    let key_part = parts.last().copied().unwrap_or_default();
    let code_name =
        normalize_key_code(key_part).ok_or_else(|| format!("Unsupported key: {}", key_part))?;
    let code = Code::from_str(&code_name).map_err(|_| format!("Unsupported key: {}", key_part))?;
    Ok(Shortcut::new(Some(modifiers), code))
}

pub fn read_configured_hotkeys() -> (String, String) {
    let mut path = app_data_dir();
    path.push("config.json");
    let Ok(config_str) = fs::read_to_string(path) else {
        return (
            DEFAULT_SCREENSHOT_HOTKEY.to_string(),
            TRANSLATE_HOTKEY_LABEL.to_string(),
        );
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) else {
        return (
            DEFAULT_SCREENSHOT_HOTKEY.to_string(),
            TRANSLATE_HOTKEY_LABEL.to_string(),
        );
    };
    let screenshot = config
        .get("hotkey")
        .and_then(|value| value.as_str())
        .unwrap_or(DEFAULT_SCREENSHOT_HOTKEY)
        .trim()
        .to_string();
    let translate = config
        .get("translateHotkey")
        .and_then(|value| value.as_str())
        .unwrap_or(TRANSLATE_HOTKEY_LABEL)
        .trim()
        .to_string();
    (screenshot, translate)
}

pub fn register_global_shortcuts(
    app: &tauri::AppHandle,
    screenshot_hotkey: &str,
    translate_hotkey: &str,
) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    CAPTURE_ESCAPE_SHORTCUT_REGISTERED.store(false, AtomicOrdering::SeqCst);
    let mut errors = Vec::new();

    if !screenshot_hotkey.trim().is_empty() {
        match parse_hotkey(screenshot_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed
                                && accept_capture_shortcut_press()
                            {
                                let app_h = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = start_screenshot(app_h, None).await {
                                        eprintln!("Failed to start screenshot: {}", e);
                                    }
                                });
                            }
                        })
                {
                    errors.push(format!("{}: {}", screenshot_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", screenshot_hotkey, e)),
        }
    }

    if !translate_hotkey.trim().is_empty() {
        match parse_hotkey(translate_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed
                                && accept_capture_shortcut_press()
                            {
                                let app_h = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) =
                                        start_screenshot(app_h, Some("translate".to_string())).await
                                    {
                                        eprintln!("Failed to start translate screenshot: {}", e);
                                    }
                                });
                            }
                        })
                {
                    errors.push(format!("{}: {}", translate_hotkey, e));
                }
            }
            Err(e) => errors.push(format!("{}: {}", translate_hotkey, e)),
        }
    }

    match parse_hotkey(RECORDING_HOTKEY_LABEL) {
        Ok(shortcut) => {
            if let Err(e) =
                app.global_shortcut()
                    .on_shortcut(shortcut, move |app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed
                            && accept_capture_shortcut_press()
                        {
                            let app_h = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) =
                                    start_screenshot(app_h, Some("record".to_string())).await
                                {
                                    eprintln!("Failed to start recording selection: {}", e);
                                }
                            });
                        }
                    })
            {
                errors.push(format!("{}: {}", RECORDING_HOTKEY_LABEL, e));
            }
        }
        Err(e) => errors.push(format!("{}: {}", RECORDING_HOTKEY_LABEL, e)),
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn capture_escape_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

pub fn register_capture_escape_shortcut(app: &tauri::AppHandle) {
    if CAPTURE_ESCAPE_SHORTCUT_REGISTERED.swap(true, AtomicOrdering::SeqCst) {
        return;
    }
    if let Err(error) = app.global_shortcut().on_shortcut(
        capture_escape_shortcut(),
        move |app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed || !CAPTURING.load(Ordering::SeqCst) {
                return;
            }
            let app_h = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) =
                    cancel_screenshot(app_h, Some("screenshot".to_string()), Some(true)).await
                {
                    eprintln!("Failed to cancel screenshot from Escape: {error}");
                }
            });
        },
    ) {
        CAPTURE_ESCAPE_SHORTCUT_REGISTERED.store(false, AtomicOrdering::SeqCst);
        eprintln!("[shortcut] Failed to register capture Escape shortcut: {error}");
    } else {
        println!("[shortcut] registered capture Escape shortcut");
    }
}

pub fn unregister_capture_escape_shortcut(app: &tauri::AppHandle) {
    if !CAPTURE_ESCAPE_SHORTCUT_REGISTERED.swap(false, AtomicOrdering::SeqCst) {
        return;
    }
    if let Err(error) = app.global_shortcut().unregister(capture_escape_shortcut()) {
        eprintln!("[shortcut] Failed to unregister capture Escape shortcut: {error}");
    } else {
        println!("[shortcut] unregistered capture Escape shortcut");
    }
}

#[tauri::command]
pub fn re_register_shortcut(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppShortcutStatus>,
    hotkey: String,
    translate_hotkey: Option<String>,
) -> Result<(), String> {
    let translate = translate_hotkey.unwrap_or_else(|| TRANSLATE_HOTKEY_LABEL.to_string());
    let status = register_global_shortcuts(&app, hotkey.trim(), translate.trim());
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = status.clone();
    status
}

#[tauri::command]
pub fn get_shortcut_status(state: tauri::State<'_, AppShortcutStatus>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match &*guard {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

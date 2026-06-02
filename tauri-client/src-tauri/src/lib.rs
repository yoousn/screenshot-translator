#[cfg(windows)]
use std::os::windows::process::CommandExt;

use arboard::{Clipboard, ImageData};
use base64::{prelude::BASE64_STANDARD, Engine};
use futures_util::StreamExt;
use screenshots::Screen;
use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tokio::time::Duration;

const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
static CAPTURING: AtomicBool = AtomicBool::new(false);
static RECORDING_OVERLAY: OnceLock<Mutex<Option<NativeRecordingOverlay>>> = OnceLock::new();
static RECORDING_OVERLAY_COLOR: AtomicU32 = AtomicU32::new(RECORDING_BORDER_BLUE);
static STARTUP_READINESS: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();

static SCREENSHOT_IMAGE: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();
fn get_screenshot_image() -> &'static Mutex<Option<Vec<u8>>> {
    SCREENSHOT_IMAGE.get_or_init(|| Mutex::new(None))
}

#[derive(Clone, Copy)]
struct NativeRecordingOverlay {
    hwnd: isize,
}

unsafe impl Send for NativeRecordingOverlay {}

fn get_recording_overlay() -> &'static Mutex<Option<NativeRecordingOverlay>> {
    RECORDING_OVERLAY.get_or_init(|| Mutex::new(None))
}

fn get_startup_readiness_cache() -> &'static Mutex<Option<serde_json::Value>> {
    STARTUP_READINESS.get_or_init(|| Mutex::new(None))
}

struct AppShortcutStatus(std::sync::Mutex<Result<(), String>>);

const DEFAULT_SCREENSHOT_HOTKEY: &str = "Alt+A";
const TRANSLATE_HOTKEY_LABEL: &str = "Alt+T";
const RECORDING_BORDER_BLUE: u32 = 0xeb6325;
const RECORDING_BORDER_RED: u32 = 0x4444ef;
const RECORDING_BORDER_YELLOW: u32 = 0x0b9ef5;
const RECORDING_HOTKEY_LABEL: &str = "Alt+R";

fn normalize_key_code(key: &str) -> Option<String> {
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

fn parse_hotkey(hotkey: &str) -> Result<Shortcut, String> {
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

fn read_configured_hotkeys() -> (String, String) {
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

fn register_global_shortcuts(
    app: &tauri::AppHandle,
    screenshot_hotkey: &str,
    translate_hotkey: &str,
) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    let mut errors = Vec::new();

    if !screenshot_hotkey.trim().is_empty() {
        match parse_hotkey(screenshot_hotkey.trim()) {
            Ok(shortcut) => {
                if let Err(e) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
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
                            if event.state() == ShortcutState::Pressed {
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
                        if event.state() == ShortcutState::Pressed {
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

#[tauri::command]
fn re_register_shortcut(
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
    let _ = stop_recording_internal(1500);
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    let mut legacy_path = app_data_dir();
    legacy_path.push("fullscreen_temp.jpg");
    if legacy_path.exists() {
        let _ = fs::remove_file(&legacy_path);
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

fn config_value_string(key: &str) -> Option<String> {
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

fn sanitize_tag(tag: &str) -> String {
    let safe: String = tag
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "latest".to_string()
    } else {
        safe
    }
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

#[cfg(target_os = "windows")]
mod win32 {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct RECT {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct PAINTSTRUCT {
        pub hdc: isize,
        pub f_erase: i32,
        pub rc_paint: RECT,
        pub f_restore: i32,
        pub f_inc_update: i32,
        pub rgb_reserved: [u8; 32],
    }
    #[repr(C)]
    pub struct WNDCLASSW {
        pub style: u32,
        pub lpfn_wnd_proc: Option<unsafe extern "system" fn(isize, u32, usize, isize) -> isize>,
        pub cb_cls_extra: i32,
        pub cb_wnd_extra: i32,
        pub h_instance: isize,
        pub h_icon: isize,
        pub h_cursor: isize,
        pub hbr_background: isize,
        pub lpsz_menu_name: *const u16,
        pub lpsz_class_name: *const u16,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct MSG {
        pub hwnd: isize,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt: POINT,
    }
    pub type EnumWindowsProc = Option<unsafe extern "system" fn(isize, isize) -> i32>;
    extern "system" {
        pub fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
        pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
        pub fn CreateWindowExW(
            dwExStyle: u32,
            lpClassName: *const u16,
            lpWindowName: *const u16,
            dwStyle: u32,
            X: i32,
            Y: i32,
            nWidth: i32,
            nHeight: i32,
            hWndParent: isize,
            hMenu: isize,
            hInstance: isize,
            lpParam: *mut c_void,
        ) -> isize;
        pub fn DefWindowProcW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> isize;
        pub fn DestroyWindow(hWnd: isize) -> i32;
        pub fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
        pub fn UpdateWindow(hWnd: isize) -> i32;
        pub fn PostMessageW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> i32;
        pub fn PostQuitMessage(nExitCode: i32);
        pub fn GetMessageW(
            lpMsg: *mut MSG,
            hWnd: isize,
            wMsgFilterMin: u32,
            wMsgFilterMax: u32,
        ) -> i32;
        pub fn TranslateMessage(lpMsg: *const MSG) -> i32;
        pub fn DispatchMessageW(lpMsg: *const MSG) -> isize;
        pub fn BeginPaint(hWnd: isize, lpPaint: *mut PAINTSTRUCT) -> isize;
        pub fn EndPaint(hWnd: isize, lpPaint: *const PAINTSTRUCT) -> i32;
        pub fn FillRect(hDC: isize, lprc: *const RECT, hbr: isize) -> i32;
        pub fn CreateSolidBrush(color: u32) -> isize;
        pub fn DeleteObject(ho: isize) -> i32;
        pub fn SetLayeredWindowAttributes(hwnd: isize, crKey: u32, bAlpha: u8, dwFlags: u32)
            -> i32;
        pub fn SetWindowDisplayAffinity(hWnd: isize, dwAffinity: u32) -> i32;
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn GetWindowTextLengthW(hWnd: isize) -> i32;
        pub fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
        pub fn GetWindowThreadProcessId(hWnd: isize, lpdwProcessId: *mut u32) -> u32;
        pub fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> isize;
        pub fn QueryFullProcessImageNameW(
            hProcess: isize,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
        pub fn CloseHandle(hObject: isize) -> i32;
        pub fn EnumWindows(lpEnumFunc: EnumWindowsProc, lParam: isize) -> i32;
        pub fn EnumChildWindows(
            hWndParent: isize,
            lpEnumFunc: EnumWindowsProc,
            lParam: isize,
        ) -> i32;
        pub fn IsWindowVisible(hWnd: isize) -> i32;
        pub fn SetCursorPos(X: i32, Y: i32) -> i32;
        pub fn mouse_event(dwFlags: u32, dx: u32, dy: u32, dwData: u32, dwExtraInfo: usize);
        pub fn InvalidateRect(hWnd: isize, lpRect: *const RECT, bErase: i32) -> i32;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        pub fn DwmSetWindowAttribute(
            hwnd: isize,
            dwAttribute: u32,
            pvAttribute: *const std::ffi::c_void,
            cbAttribute: u32,
        ) -> i32;
        pub fn DwmGetWindowAttribute(
            hwnd: isize,
            dwAttribute: u32,
            pvAttribute: *mut std::ffi::c_void,
            cbAttribute: u32,
        ) -> i32;
    }
}

#[cfg(target_os = "windows")]
fn set_hwnd_capture_excluded(hwnd: isize, excluded: bool) -> Result<(), String> {
    const WDA_NONE: u32 = 0x00000000;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;
    let affinity = if excluded {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    let ok = unsafe { win32::SetWindowDisplayAffinity(hwnd, affinity) };
    if ok == 0 {
        return Err("SetWindowDisplayAffinity failed".to_string());
    }
    Ok(())
}

fn set_webview_capture_excluded(
    app: &tauri::AppHandle,
    label: &str,
    excluded: bool,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let window = app
            .get_webview_window(label)
            .ok_or_else(|| format!("window not found: {}", label))?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        set_hwnd_capture_excluded(hwnd, excluded)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, label, excluded);
        Ok(())
    }
}

#[tauri::command]
fn set_window_capture_excluded(
    app: tauri::AppHandle,
    label: String,
    excluded: bool,
) -> Result<(), String> {
    set_webview_capture_excluded(&app, &label, excluded)
}

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(i32, i32)> {
    let mut point = win32::POINT { x: 0, y: 0 };
    // SAFETY: Calling Win32 API GetCursorPos with a valid mutable pointer to a POINT struct.
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

#[cfg(target_os = "windows")]
const RECORDING_OVERLAY_CLASS: &str = "YSNRecordingOverlayNative";
#[cfg(target_os = "windows")]
const WM_PAINT: u32 = 0x000F;
#[cfg(target_os = "windows")]
const WM_DESTROY: u32 = 0x0002;
#[cfg(target_os = "windows")]
const WM_CLOSE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const WM_NCHITTEST: u32 = 0x0084;
#[cfg(target_os = "windows")]
const HTTRANSPARENT: isize = -1;
#[cfg(target_os = "windows")]
const WS_POPUP: u32 = 0x80000000;
#[cfg(target_os = "windows")]
const WS_EX_TOPMOST: u32 = 0x00000008;
#[cfg(target_os = "windows")]
const WS_EX_TRANSPARENT: u32 = 0x00000020;
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: u32 = 0x00000080;
#[cfg(target_os = "windows")]
const WS_EX_LAYERED: u32 = 0x00080000;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: u32 = 0x08000000;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const LWA_COLORKEY: u32 = 0x00000001;
#[cfg(target_os = "windows")]
const TRANSPARENT_COLOR_KEY: u32 = 0x000000;
#[cfg(target_os = "windows")]
const RECORDING_BORDER_THICKNESS: i32 = 2;

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn recording_overlay_wnd_proc(
    hwnd: isize,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    match msg {
        WM_NCHITTEST => HTTRANSPARENT,
        WM_PAINT => {
            let mut ps = win32::PAINTSTRUCT {
                hdc: 0,
                f_erase: 0,
                rc_paint: win32::RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                f_restore: 0,
                f_inc_update: 0,
                rgb_reserved: [0; 32],
            };
            let hdc = win32::BeginPaint(hwnd, &mut ps);
            let width = ps.rc_paint.right.max(1);
            let height = ps.rc_paint.bottom.max(1);
            let transparent_brush = win32::CreateSolidBrush(TRANSPARENT_COLOR_KEY);
            let border_color = RECORDING_OVERLAY_COLOR.load(Ordering::Relaxed);
            let red_brush = win32::CreateSolidBrush(border_color);
            let full = win32::RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            let top = win32::RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: RECORDING_BORDER_THICKNESS.min(height),
            };
            let bottom = win32::RECT {
                left: 0,
                top: (height - RECORDING_BORDER_THICKNESS).max(0),
                right: width,
                bottom: height,
            };
            let left = win32::RECT {
                left: 0,
                top: 0,
                right: RECORDING_BORDER_THICKNESS.min(width),
                bottom: height,
            };
            let right = win32::RECT {
                left: (width - RECORDING_BORDER_THICKNESS).max(0),
                top: 0,
                right: width,
                bottom: height,
            };
            win32::FillRect(hdc, &full, transparent_brush);
            win32::FillRect(hdc, &top, red_brush);
            win32::FillRect(hdc, &bottom, red_brush);
            win32::FillRect(hdc, &left, red_brush);
            win32::FillRect(hdc, &right, red_brush);
            let _ = win32::DeleteObject(transparent_brush);
            let _ = win32::DeleteObject(red_brush);
            win32::EndPaint(hwnd, &ps);
            0
        }
        WM_CLOSE => {
            win32::DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            win32::PostQuitMessage(0);
            0
        }
        _ => win32::DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

#[cfg(target_os = "windows")]
fn hide_recording_overlay_internal() {
    let overlay = get_recording_overlay()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(overlay) = overlay {
        unsafe {
            let _ = win32::PostMessageW(overlay.hwnd, WM_CLOSE, 0, 0);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_recording_overlay_internal() {}

#[tauri::command]
fn hide_recording_overlay() -> Result<(), String> {
    hide_recording_overlay_internal();
    Ok(())
}
fn recording_color_ref(status: &str) -> u32 {
    match status {
        "recording" => RECORDING_BORDER_RED,
        "paused" => RECORDING_BORDER_YELLOW,
        _ => RECORDING_BORDER_BLUE,
    }
}

#[tauri::command]
fn set_recording_overlay_status(status: String) -> Result<(), String> {
    RECORDING_OVERLAY_COLOR.store(recording_color_ref(status.trim()), Ordering::Relaxed);
    #[cfg(target_os = "windows")]
    {
        if let Some(overlay) = get_recording_overlay().lock().ok().and_then(|guard| *guard) {
            unsafe {
                let _ = win32::InvalidateRect(overlay.hwnd, std::ptr::null(), 1);
                let _ = win32::UpdateWindow(overlay.hwnd);
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn show_recording_overlay(x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid recording region size".to_string());
    }
    hide_recording_overlay_internal();
    RECORDING_OVERLAY_COLOR.store(RECORDING_BORDER_BLUE, Ordering::Relaxed);
    #[cfg(target_os = "windows")]
    {
        let (tx, rx) = mpsc::channel::<Result<isize, String>>();
        std::thread::spawn(move || {
            let result = unsafe {
                let class_name = wide_null(RECORDING_OVERLAY_CLASS);
                let title = wide_null("YSN Recording Border");
                let h_instance = win32::GetModuleHandleW(std::ptr::null());
                let wnd_class = win32::WNDCLASSW {
                    style: 0,
                    lpfn_wnd_proc: Some(recording_overlay_wnd_proc),
                    cb_cls_extra: 0,
                    cb_wnd_extra: 0,
                    h_instance,
                    h_icon: 0,
                    h_cursor: 0,
                    hbr_background: 0,
                    lpsz_menu_name: std::ptr::null(),
                    lpsz_class_name: class_name.as_ptr(),
                };
                let _ = win32::RegisterClassW(&wnd_class);
                let hwnd = win32::CreateWindowExW(
                    WS_EX_TOPMOST
                        | WS_EX_TRANSPARENT
                        | WS_EX_TOOLWINDOW
                        | WS_EX_LAYERED
                        | WS_EX_NOACTIVATE,
                    class_name.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP,
                    x,
                    y,
                    w,
                    h,
                    0,
                    0,
                    h_instance,
                    std::ptr::null_mut(),
                );
                if hwnd == 0 {
                    Err("Failed to create native recording border".to_string())
                } else {
                    let _ = win32::SetLayeredWindowAttributes(
                        hwnd,
                        TRANSPARENT_COLOR_KEY,
                        255,
                        LWA_COLORKEY,
                    );
                    let _ = set_hwnd_capture_excluded(hwnd, true);
                    let _ = win32::ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let _ = win32::UpdateWindow(hwnd);
                    Ok(hwnd)
                }
            };
            let hwnd = match result {
                Ok(hwnd) => {
                    let _ = tx.send(Ok(hwnd));
                    hwnd
                }
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            };
            let mut msg = win32::MSG {
                hwnd: 0,
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt: win32::POINT { x: 0, y: 0 },
            };
            unsafe {
                while win32::GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    let _ = win32::TranslateMessage(&msg);
                    let _ = win32::DispatchMessageW(&msg);
                }
            }
            if let Ok(mut guard) = get_recording_overlay().lock() {
                if guard.map(|value| value.hwnd) == Some(hwnd) {
                    *guard = None;
                }
            }
        });
        let hwnd = rx
            .recv_timeout(std::time::Duration::from_millis(1000))
            .map_err(|_| "Timed out creating native recording border".to_string())??;
        *get_recording_overlay().lock().map_err(|e| e.to_string())? =
            Some(NativeRecordingOverlay { hwnd });
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

fn close_screenshot_windows(app: &tauri::AppHandle, include_primary: bool) {
    for (label, window) in app.webview_windows() {
        if label == "screenshot" && include_primary {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
        } else if label.starts_with("screenshot_") {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
            let _ = window.close();
        } else if label == "recording_border" || label.starts_with("recording_border_") {
            let _ = window.set_always_on_top(false);
            let _ = window.hide();
            let _ = window.close();
        }
    }
}

async fn start_screenshot_impl(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    let screenshot_mode = mode.unwrap_or_else(|| "normal".to_string());
    let mut main_hidden_for_capture = false;
    let mut main_excluded_for_capture = false;

    // Keep the settings panel visually independent from capture. On Windows, prefer
    // capture exclusion so the panel does not flash; hide only as a fallback.
    if let Some(main_win) = app.get_webview_window("main") {
        if main_win.is_visible().unwrap_or(false) {
            if set_webview_capture_excluded(&app, "main", true).is_ok() {
                main_excluded_for_capture = true;
            } else {
                let _ = main_win.hide();
                main_hidden_for_capture = true;
            }
        }
    }
    if let Some(screenshot_win) = app.get_webview_window("screenshot") {
        let _ = screenshot_win.set_always_on_top(false);
        let _ = screenshot_win.hide();
    }
    close_screenshot_windows(&app, false);
    if main_excluded_for_capture || main_hidden_for_capture {
        tokio::time::sleep(Duration::from_millis(70)).await;
    }

    // Capture and encode on a blocking thread to avoid blocking the async runtime
    let (png_bytes, base64_data, screen_info) = tokio::task::spawn_blocking(
        move || -> Result<(Vec<u8>, String, (i32, i32, u32, u32)), String> {
            let screens =
                Screen::all().map_err(|e| format!("Failed to enumerate displays: {}", e))?;
            if screens.is_empty() {
                return Err("No display detected".to_string());
            }
            let screen = if let Some((cx, cy)) = get_cursor_position() {
                Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
            } else {
                screens[0]
            };
            let info = screen.display_info;
            let screen_info = (info.x, info.y, info.width, info.height);

            let image = screen
                .capture()
                .map_err(|e| format!("Screenshot failed: {}", e))?;
            let mut buffer = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
                .map_err(|e| format!("Encode PNG failed: {}", e))?;
            let png_bytes = buffer.into_inner();
            let base64_data = BASE64_STANDARD.encode(&png_bytes);
            Ok((png_bytes, base64_data, screen_info))
        },
    )
    .await
    .map_err(|e| format!("Screenshot task failed: {}", e))??;

    if main_excluded_for_capture {
        let _ = set_webview_capture_excluded(&app, "main", false);
    }
    if main_hidden_for_capture {
        if let Some(main_win) = app.get_webview_window("main") {
            let _ = main_win.show();
        }
    }

    // Store lossless screenshot bytes in memory for OCR/cropping quality and speed.
    if let Ok(mut guard) = get_screenshot_image().lock() {
        *guard = Some(png_bytes.clone());
    }

    // Write to disk asynchronously (non-blocking) 鈥?only needed as a backup
    let write_dir = app_data_dir();
    let write_path = write_dir.join("fullscreen_temp.png");
    let legacy_write_path = write_dir.join("fullscreen_temp.jpg");
    let png_for_write = png_bytes.clone();
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = write_path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        let _ = fs::write(&write_path, &png_for_write);
        let _ = fs::remove_file(&legacy_write_path);
    });

    let screenshot_win = if let Some(win) = app.get_webview_window("screenshot") {
        win
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "screenshot",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("YSN Screenshot Helper")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .visible(false)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .build()
        .map_err(|e| format!("Create screenshot window failed: {}", e))?
    };

    // Disable transition animation to avoid windows rendering delay/flicker
    disable_windows_transition(&screenshot_win);

    let (x, y, width, height) = screen_info;

    // Position and configure the window while still hidden
    let _ = screenshot_win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = screenshot_win.set_size(tauri::PhysicalSize::new(width, height));
    let _ = screenshot_win.set_always_on_top(true);

    let _ = screenshot_win.emit("screenshot-mode", screenshot_mode.clone());
    let _ = screenshot_win.emit("screenshot-updated", base64_data);

    Ok(())
}

#[tauri::command]
async fn overlay_ready_to_show(app: tauri::AppHandle, label: Option<String>) -> Result<(), String> {
    let target_label = label.unwrap_or_else(|| "screenshot".to_string());
    if target_label != "screenshot" && !target_label.starts_with("screenshot_") {
        return Ok(());
    }
    if let Some(screenshot_win) = app.get_webview_window(&target_label) {
        let _ = screenshot_win.show();
        let _ = screenshot_win.set_focus();
        let _ = screenshot_win.set_always_on_top(true);
    }
    Ok(())
}

#[tauri::command]
async fn start_screenshot(app: tauri::AppHandle, mode: Option<String>) -> Result<(), String> {
    // Restart cleanly on repeated hotkey presses instead of racing two overlay sessions.
    if CAPTURING.swap(true, Ordering::SeqCst) {
        hide_recording_overlay_internal();
        close_screenshot_windows(&app, true);
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
async fn force_close_screenshots(app: tauri::AppHandle) -> Result<(), String> {
    hide_recording_overlay_internal();
    close_screenshot_windows(&app, true);
    CAPTURING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn quick_fullscreen_capture() -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("Failed to enumerate displays: {}", e))?;
    if screens.is_empty() {
        return Err("No display detected".to_string());
    }
    let screen = if let Some((cx, cy)) = get_cursor_position() {
        Screen::from_point(cx, cy).unwrap_or_else(|_| screens[0])
    } else {
        screens[0]
    };
    let image = screen
        .capture()
        .map_err(|e| format!("Screenshot failed: {}", e))?;
    let (width, height) = image.dimensions();
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Initialize clipboard failed: {}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(image.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Copy image to clipboard failed: {}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn current_screen_origin() -> (i32, i32, i32, i32) {
    if let Some((cx, cy)) = get_cursor_position() {
        if let Ok(screen) = Screen::from_point(cx, cy) {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
    }
    if let Ok(screens) = Screen::all() {
        if let Some(screen) = screens.first() {
            let info = screen.display_info;
            return (info.x, info.y, info.width as i32, info.height as i32);
        }
    }
    (0, 0, i32::MAX, i32::MAX)
}

#[cfg(target_os = "windows")]
fn hwnd_rect(hwnd: isize, prefer_dwm_bounds: bool) -> Option<win32::RECT> {
    if hwnd == 0 {
        return None;
    }
    if prefer_dwm_bounds {
        let mut rect = win32::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: DwmGetWindowAttribute is called with a valid HWND and RECT buffer.
        let hr = unsafe {
            win32::DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut win32::RECT as *mut std::ffi::c_void,
                std::mem::size_of::<win32::RECT>() as u32,
            )
        };
        if hr == 0 && rect.right > rect.left && rect.bottom > rect.top {
            return Some(rect);
        }
    }
    let mut rect = win32::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: GetWindowRect is called with a valid HWND and RECT buffer.
    let ok = unsafe { win32::GetWindowRect(hwnd, &mut rect) };
    if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
        Some(rect)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn push_rect_candidate(
    rects: &mut Vec<serde_json::Value>,
    rect: win32::RECT,
    kind: &str,
    screen: (i32, i32, i32, i32),
    min_size: i32,
) {
    let (screen_x, screen_y, screen_w, screen_h) = screen;
    let left = rect.left.max(screen_x);
    let top = rect.top.max(screen_y);
    let right = rect.right.min(screen_x + screen_w);
    let bottom = rect.bottom.min(screen_y + screen_h);
    let w = right - left;
    let h = bottom - top;
    if w < min_size || h < min_size {
        return;
    }
    let json_rect = serde_json::json!({
        "x": left - screen_x,
        "y": top - screen_y,
        "w": w,
        "h": h,
        "kind": kind,
    });
    let duplicate = rects.iter().any(|item| {
        item.get("x") == json_rect.get("x")
            && item.get("y") == json_rect.get("y")
            && item.get("w") == json_rect.get("w")
            && item.get("h") == json_rect.get("h")
    });
    if !duplicate {
        rects.push(json_rect);
    }
}

#[cfg(target_os = "windows")]
struct WindowSearchContext {
    cursor_x: i32,
    cursor_y: i32,
    excluded_hwnds: Vec<isize>,
    matches: Vec<isize>,
    min_size: i32,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || ctx.excluded_hwnds.contains(&hwnd) || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, true) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let contains_cursor = ctx.cursor_x >= rect.left
            && ctx.cursor_x <= rect.right
            && ctx.cursor_y >= rect.top
            && ctx.cursor_y <= rect.bottom;
        if contains_cursor && w >= ctx.min_size && h >= ctx.min_size {
            ctx.matches.push(hwnd);
        }
    }
    1
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_child_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, false) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let contains_cursor = ctx.cursor_x >= rect.left
            && ctx.cursor_x <= rect.right
            && ctx.cursor_y >= rect.top
            && ctx.cursor_y <= rect.bottom;
        if contains_cursor && w >= ctx.min_size && h >= ctx.min_size {
            ctx.matches.push(hwnd);
        }
    }
    1
}

#[cfg(target_os = "windows")]
fn excluded_app_hwnds(app: &tauri::AppHandle) -> Vec<isize> {
    let mut excluded = Vec::new();
    for label in ["screenshot", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            if let Ok(hwnd) = window.hwnd() {
                excluded.push(hwnd.0 as isize);
            }
        }
    }
    excluded
}

#[cfg(target_os = "windows")]
fn top_level_windows_at_cursor(
    cursor_x: i32,
    cursor_y: i32,
    excluded_hwnds: Vec<isize>,
) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds,
        matches: Vec::new(),
        min_size: 50,
    };
    // SAFETY: EnumWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumWindows(
            Some(enum_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[cfg(target_os = "windows")]
fn child_windows_at_cursor(root: isize, cursor_x: i32, cursor_y: i32) -> Vec<isize> {
    let mut ctx = WindowSearchContext {
        cursor_x,
        cursor_y,
        excluded_hwnds: Vec::new(),
        matches: Vec::new(),
        min_size: 12,
    };
    // SAFETY: EnumChildWindows calls the callback synchronously while ctx remains valid.
    unsafe {
        win32::EnumChildWindows(
            root,
            Some(enum_child_windows_for_cursor),
            &mut ctx as *mut WindowSearchContext as isize,
        );
    }
    ctx.matches
}

#[tauri::command]
fn get_window_rects(
    app: tauri::AppHandle,
    include_controls: Option<bool>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let mut rects: Vec<serde_json::Value> = Vec::new();
        let screen = current_screen_origin();
        let include_controls = include_controls.unwrap_or(false);
        if let Some((cx, cy)) = get_cursor_position() {
            let excluded_hwnds = excluded_app_hwnds(&app);
            let windows = top_level_windows_at_cursor(cx, cy, excluded_hwnds);
            if let Some(hwnd) = windows.first().copied() {
                if include_controls {
                    for child in child_windows_at_cursor(hwnd, cx, cy)
                        .into_iter()
                        .rev()
                        .take(1)
                    {
                        if let Some(rect) = hwnd_rect(child, false) {
                            push_rect_candidate(&mut rects, rect, "control", screen, 12);
                        }
                    }
                }
                if let Some(rect) = hwnd_rect(hwnd, true) {
                    push_rect_candidate(&mut rects, rect, "window", screen, 50);
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
async fn cancel_screenshot(app: tauri::AppHandle, label: Option<String>) -> Result<(), String> {
    if let Some(target_label) = label {
        if target_label == "screenshot" || target_label.starts_with("screenshot_") {
            if let Some(screenshot_win) = app.get_webview_window(&target_label) {
                let _ = screenshot_win.set_always_on_top(false);
                let _ = screenshot_win.hide();
            }
            close_screenshot_windows(&app, false);
        }
    } else {
        close_screenshot_windows(&app, true);
    }
    CAPTURING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_fullscreen_image() -> Result<String, String> {
    // Try memory first (fast), fall back to disk
    if let Ok(guard) = get_screenshot_image().lock() {
        if let Some(ref bytes) = *guard {
            return Ok(BASE64_STANDARD.encode(bytes));
        }
    }
    let mut path = app_data_dir();
    path.push("fullscreen_temp.png");
    if !path.exists() {
        return Err("No display detected".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))?;
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection region".to_string());
    }

    // Try memory first (fast), fall back to disk
    let screenshot_bytes = {
        let guard = get_screenshot_image().lock().map_err(|e| e.to_string())?;
        if let Some(ref bytes) = *guard {
            bytes.clone()
        } else {
            let mut path = app_data_dir();
            path.push("fullscreen_temp.png");
            if !path.exists() {
                path = app_data_dir();
                path.push("fullscreen_temp.jpg");
            }
            if !path.exists() {
                return Err("No display detected".to_string());
            }
            fs::read(&path).map_err(|e| format!("Read fullscreen image failed: {}", e))?
        }
    };

    let img = screenshots::image::load_from_memory(&screenshot_bytes)
        .map_err(|e| format!("Load fullscreen image failed: {}", e))?;
    let iw = img.width() as i32;
    let ih = img.height() as i32;
    let sx = x.clamp(0, iw.saturating_sub(1));
    let sy = y.clamp(0, ih.saturating_sub(1));
    let sw = w.clamp(1, iw - sx);
    let sh = h.clamp(1, ih - sy);
    let cropped = img.crop_imm(sx as u32, sy as u32, sw as u32, sh as u32);
    let mut buffer = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("Encode PNG failed: {}", e))?;
    let bytes = buffer.into_inner();
    let mut cropped_path = app_data_dir();
    cropped_path.push("cropped_temp.png");
    let _ = fs::write(&cropped_path, &bytes);
    Ok(BASE64_STANDARD.encode(&bytes))
}

#[tauri::command]
fn capture_live_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    if w <= 0 || h <= 0 {
        return Err("Invalid selection area".to_string());
    }
    let (origin_x, origin_y, _, _) = current_screen_origin();
    let global_x = origin_x + x;
    let global_y = origin_y + y;
    let center_x = global_x + w / 2;
    let center_y = global_y + h / 2;
    let screen = Screen::from_point(center_x, center_y)
        .map_err(|e| format!("Failed to locate screen for scroll capture: {}", e))?;
    let rel_x = global_x - screen.display_info.x;
    let rel_y = global_y - screen.display_info.y;
    let image = screen
        .capture_area(rel_x, rel_y, w as u32, h as u32)
        .map_err(|e| format!("Failed to capture live region: {}", e))?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    screenshots::image::DynamicImage::ImageRgba8(image)
        .write_to(&mut buffer, screenshots::image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    Ok(BASE64_STANDARD.encode(buffer.into_inner()))
}

#[tauri::command]
fn scroll_mouse_at(x: i32, y: i32, delta: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const MOUSEEVENTF_WHEEL: u32 = 0x0800;
        let (origin_x, origin_y, _, _) = current_screen_origin();
        let global_x = origin_x + x;
        let global_y = origin_y + y;
        unsafe {
            let _ = win32::SetCursorPos(global_x, global_y);
            win32::mouse_event(MOUSEEVENTF_WHEEL, 0, 0, delta as u32, 0);
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (x, y, delta);
        Err("Automatic scrolling is not supported on this platform".to_string())
    }
}

#[tauri::command]
fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Decode base64 failed: {}", e))?;
    let img = screenshots::image::load_from_memory_with_format(
        &bytes,
        screenshots::image::ImageFormat::Png,
    )
    .map_err(|e| format!("Parse cropped image data failed: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Initialize clipboard failed: {}", e))?;
    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Copy image to clipboard failed: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn save_image_to_file(image_base64: String) -> Result<String, String> {
    let bytes = BASE64_STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Decode base64 failed: {}", e))?;
    let file_path = rfd::AsyncFileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_file_name("screenshot.png")
        .save_file()
        .await;
    if let Some(file_handle) = file_path {
        let path = file_handle.path();
        fs::write(path, &bytes).map_err(|e| format!("Write file failed: {}", e))?;
        if !path.exists() {
            return Err("No display detected".to_string());
        }
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Save cancelled by user".to_string())
    }
}

#[derive(Debug, Deserialize)]
struct RecordingOptions {
    fps: Option<u32>,
    resolution: Option<String>,
    audio_mode: Option<String>,
    mic_device: Option<String>,
    system_audio_device: Option<String>,
    output_dir: Option<String>,
    region_x: Option<i32>,
    region_y: Option<i32>,
    region_w: Option<i32>,
    region_h: Option<i32>,
}

fn ffmpeg_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    use tauri::path::BaseDirectory;
    let mut candidates = Vec::new();

    if let Some(path) = config_value_string("recordingFfmpegPath") {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(path) = std::env::var("FFMPEG_PATH") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path.trim()));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("tools").join("ffmpeg").join("ffmpeg.exe"));
            candidates.push(dir.join("plugins").join("ffmpeg").join("ffmpeg.exe"));
        }
    }

    if let Ok(path) = app
        .path()
        .resolve("resources/ffmpeg/ffmpeg.exe", BaseDirectory::Resource)
    {
        candidates.push(path);
    }

    let mut app_ffmpeg = app_data_dir();
    app_ffmpeg.push("ffmpeg");
    app_ffmpeg.push("ffmpeg.exe");
    candidates.push(app_ffmpeg);
    candidates.push(PathBuf::from("ffmpeg"));
    candidates
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseInfo {
    tag_name: String,
    html_url: Option<String>,
    assets: Vec<GithubReleaseAsset>,
}

fn emit_ffmpeg_progress(
    app: &tauri::AppHandle,
    phase: &str,
    downloaded: u64,
    total: Option<u64>,
    percent: u8,
) {
    let _ = app.emit(
        "ffmpeg-download-progress",
        serde_json::json!({
            "phase": phase,
            "downloaded": downloaded,
            "total": total,
            "percent": percent,
        }),
    );
}

fn default_ffmpeg_install_dir() -> PathBuf {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            return dir.join("ffmpeg");
        }
    }
    let mut dir = app_data_dir();
    dir.push("ffmpeg");
    dir
}

fn ensure_writable_dir(preferred: PathBuf) -> PathBuf {
    if fs::create_dir_all(&preferred).is_ok() {
        let probe = preferred.join(".write-test");
        if fs::write(&probe, b"ok").is_ok() {
            let _ = fs::remove_file(probe);
            return preferred;
        }
    }
    let mut fallback = app_data_dir();
    fallback.push("ffmpeg");
    fallback
}

fn extract_ffmpeg_exe_from_zip(
    bytes: &[u8],
    install_dir: &std::path::Path,
) -> Result<PathBuf, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Read ffmpeg archive failed: {}", e))?;
    fs::create_dir_all(install_dir)
        .map_err(|e| format!("Create ffmpeg directory failed: {}", e))?;
    let target = install_dir.join("ffmpeg.exe");
    let mut found = false;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Read ffmpeg archive entry failed: {}", e))?;
        if !file
            .name()
            .replace('\\', "/")
            .to_ascii_lowercase()
            .ends_with("/bin/ffmpeg.exe")
            && !file.name().eq_ignore_ascii_case("ffmpeg.exe")
        {
            continue;
        }
        let mut out =
            fs::File::create(&target).map_err(|e| format!("Create ffmpeg.exe failed: {}", e))?;
        std::io::copy(&mut file, &mut out)
            .map_err(|e| format!("Extract ffmpeg.exe failed: {}", e))?;
        found = true;
        break;
    }
    if !found {
        return Err("ffmpeg.exe was not found in the archive".to_string());
    }
    Ok(target)
}

fn cleanup_finished_recording_process() -> Result<bool, String> {
    let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
    let finished = if let Some(child) = guard.as_mut() {
        child
            .try_wait()
            .map_err(|e| format!("Read recording process status failed: {}", e))?
            .is_some()
    } else {
        false
    };
    if finished {
        *guard = None;
    }
    Ok(finished)
}

fn find_ffmpeg_executable(app: &tauri::AppHandle) -> Option<PathBuf> {
    for candidate in ffmpeg_candidates(app) {
        if candidate.to_string_lossy().eq_ignore_ascii_case("ffmpeg") {
            if hidden_ffmpeg_command(Path::new("ffmpeg"))
                .arg("-version")
                .output()
                .is_ok()
            {
                return Some(candidate);
            }
        } else if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn parse_quoted_audio_devices(
    output: &str,
    audio_marker_required: bool,
    prefix: Option<&str>,
) -> Vec<String> {
    let mut devices = Vec::new();
    for line in output.lines() {
        if audio_marker_required && !line.contains("(audio)") {
            continue;
        }
        if let Some(first_quote) = line.find('"') {
            if let Some(second_quote) = line[first_quote + 1..].find('"') {
                let name = line[first_quote + 1..first_quote + 1 + second_quote].trim();
                if !name.is_empty() {
                    let value = match prefix {
                        Some(prefix) => format!("{}{}", prefix, name),
                        None => name.to_string(),
                    };
                    if !devices.contains(&value) {
                        devices.push(value);
                    }
                }
            }
        }
    }
    devices
}

fn ffmpeg_supports_input_format(formats_output: &str, format_name: &str) -> bool {
    formats_output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("D") && trimmed.split_whitespace().nth(1) == Some(format_name)
    })
}

fn hidden_ffmpeg_command(ffmpeg_path: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg_path);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn ffmpeg_input_formats(ffmpeg_path: &Path) -> String {
    hidden_ffmpeg_command(ffmpeg_path)
        .args(["-hide_banner", "-formats"])
        .output()
        .map(|out| {
            format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
        .unwrap_or_default()
}

fn collect_ffmpeg_audio_devices(ffmpeg_path: &Path) -> Vec<String> {
    let mut devices = Vec::new();
    let input_formats = ffmpeg_input_formats(ffmpeg_path);
    if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .output()
    {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        devices.extend(parse_quoted_audio_devices(&combined, true, None));
    }
    if ffmpeg_supports_input_format(&input_formats, "wasapi") {
        if let Ok(out) = hidden_ffmpeg_command(ffmpeg_path)
            .args([
                "-hide_banner",
                "-list_devices",
                "true",
                "-f",
                "wasapi",
                "-i",
                "dummy",
            ])
            .output()
        {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            devices.extend(parse_quoted_audio_devices(
                &combined,
                false,
                Some("wasapi:"),
            ));
        }
        if !devices.contains(&"wasapi:default".to_string()) {
            devices.push("wasapi:default".to_string());
        }
    }
    devices
}

#[cfg(target_os = "windows")]
struct RecordingWindowListContext {
    excluded_hwnds: Vec<isize>,
    windows: Vec<serde_json::Value>,
}

#[cfg(target_os = "windows")]
fn window_title(hwnd: isize) -> String {
    let len = unsafe { win32::GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; (len + 1) as usize];
    let copied = unsafe { win32::GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

#[cfg(target_os = "windows")]
fn process_path_for_hwnd(hwnd: isize) -> Option<PathBuf> {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    let mut pid: u32 = 0;
    unsafe {
        win32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
    }
    if pid == 0 {
        return None;
    }
    let handle = unsafe { win32::OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    let ok = unsafe {
        win32::QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size as *mut u32)
    };
    unsafe {
        let _ = win32::CloseHandle(handle);
    }
    if ok == 0 || size == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize])))
}

#[cfg(target_os = "windows")]
fn exe_name_from_path(path: Option<&PathBuf>) -> String {
    path.and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("app.exe")
        .to_string()
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_recording_windows(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut RecordingWindowListContext);
    if hwnd == 0 || ctx.excluded_hwnds.contains(&hwnd) || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let title = window_title(hwnd);
    if title.is_empty() {
        return 1;
    }
    if let Some(rect) = hwnd_rect(hwnd, true) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w >= 120 && h >= 80 {
            let process_path = process_path_for_hwnd(hwnd);
            let exe_name = exe_name_from_path(process_path.as_ref());
            ctx.windows.push(serde_json::json!({
                "id": hwnd.to_string(),
                "title": title,
                "exeName": exe_name,
                "processPath": process_path.map(|path| path.to_string_lossy().to_string()),
                "iconDataUrl": null,
                "x": rect.left,
                "y": rect.top,
                "w": w,
                "h": h,
            }));
        }
    }
    1
}

#[tauri::command]
fn get_recording_targets(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let displays = Screen::all()
        .map_err(|e| format!("Failed to enumerate displays: {}", e))?
        .into_iter()
        .enumerate()
        .map(|(index, screen)| {
            let info = screen.display_info;
            serde_json::json!({
                "id": index.to_string(),
                "title": format!("Display {} ({}x{})", index + 1, info.width, info.height),
                "x": info.x,
                "y": info.y,
                "w": info.width,
                "h": info.height,
            })
        })
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    let windows = {
        let mut ctx = RecordingWindowListContext {
            excluded_hwnds: excluded_app_hwnds(&app),
            windows: Vec::new(),
        };
        unsafe {
            win32::EnumWindows(
                Some(enum_recording_windows),
                &mut ctx as *mut RecordingWindowListContext as isize,
            );
        }
        ctx.windows
    };
    #[cfg(not(target_os = "windows"))]
    let windows: Vec<serde_json::Value> = Vec::new();

    Ok(serde_json::json!({
        "windows": windows,
        "displays": displays,
    }))
}

fn build_diagnostic_readiness_by_module(
    ocr_runtime: &serde_json::Value,
    recording: &serde_json::Value,
) -> serde_json::Value {
    let ocr_steps = ocr_runtime["readinessSteps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let ocr_ready_steps = ocr_steps
        .iter()
        .filter(|step| step["ready"].as_bool().unwrap_or(false))
        .count();
    let first_blocked_ocr_step = ocr_steps
        .iter()
        .find(|step| !step["ready"].as_bool().unwrap_or(false))
        .cloned()
        .unwrap_or_else(|| serde_json::json!(null));
    let ffmpeg_ready = recording["ffmpegFound"].as_bool().unwrap_or(false);
    let audio_ready = recording["audioDevices"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let recording_steps = serde_json::json!([
        {
            "id": "ffmpeg",
            "ready": ffmpeg_ready,
            "label": "FFmpeg executable",
            "nextAction": if ffmpeg_ready { "detect-audio-devices" } else { "download-or-choose-ffmpeg" }
        },
        {
            "id": "audio-devices",
            "ready": audio_ready,
            "label": "Recording audio devices",
            "nextAction": if audio_ready { "ready" } else { "recheck-recording-audio-devices" }
        }
    ]);
    let recording_ready_steps = recording_steps
        .as_array()
        .map(|steps| {
            steps
                .iter()
                .filter(|step| step["ready"].as_bool().unwrap_or(false))
                .count()
        })
        .unwrap_or(0);
    let first_blocked_recording_step = recording_steps
        .as_array()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| !step["ready"].as_bool().unwrap_or(false))
                .cloned()
        })
        .unwrap_or_else(|| serde_json::json!(null));

    serde_json::json!({
        "ocrRuntime": {
            "ready": ocr_runtime["ready"].as_bool().unwrap_or(false),
            "readySteps": ocr_ready_steps,
            "totalSteps": ocr_steps.len(),
            "firstBlockedStep": first_blocked_ocr_step,
            "steps": ocr_steps,
        },
        "recording": {
            "ready": ffmpeg_ready && audio_ready,
            "readySteps": recording_ready_steps,
            "totalSteps": recording_steps.as_array().map(|steps| steps.len()).unwrap_or(0),
            "firstBlockedStep": first_blocked_recording_step,
            "steps": recording_steps,
        }
    })
}

#[tauri::command]
fn get_diagnostics_report(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let generated_at = chrono::Local::now().to_rfc3339();
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unavailable".to_string());
    let startup_probe_path = startup_diagnostics_probe_path()
        .to_string_lossy()
        .to_string();
    let recording = get_recording_info(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ok": false,
            "error": error,
        })
    });
    let ocr_runtime = get_rapid_ocr_status(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ready": false,
            "error": error,
        })
    });
    let shortcut_status = serde_json::json!({
        "registered": true,
        "note": "Shortcut registration errors are surfaced during app startup; detailed shortcut state is managed in AppShortcutStatus."
    });

    let mut issues = Vec::new();
    if !ocr_runtime["ready"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "ocrRuntime",
            "code": "rapidocr-not-ready",
            "message": "RapidOCR text recognition is not ready.",
            "nextAction": "Open the text recognition panel and run the RapidOCR check."
        }));
    }
    if !ocr_runtime["runnerReady"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "warning",
            "module": "ocrRuntime",
            "code": "rapidocr-runner-missing",
            "message": "RapidOCR runner is not available.",
            "nextAction": "Install the RapidOCR package for development or bundle rapidocr-runner.exe for release."
        }));
    }
    if ocr_runtime["lastError"].as_str().is_some() {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "ocrRuntime",
            "code": "rapidocr-probe-failed",
            "message": "RapidOCR probe failed.",
            "nextAction": "Run the RapidOCR self-test and reinstall the model/runtime package if needed."
        }));
    }
    if !recording["ffmpegFound"].as_bool().unwrap_or(false) {
        issues.push(serde_json::json!({
            "severity": "error",
            "module": "recording",
            "code": "ffmpeg-not-found",
            "message": "FFmpeg was not found, so video recording cannot be fully ready.",
            "nextAction": "Download FFmpeg from the video recording dependency panel or choose ffmpeg.exe manually."
        }));
    }
    if recording["audioDevices"]
        .as_array()
        .map(|items| items.is_empty())
        .unwrap_or(true)
    {
        issues.push(serde_json::json!({
            "severity": "warning",
            "module": "recording",
            "code": "audio-devices-empty",
            "message": "No FFmpeg audio devices were detected.",
            "nextAction": "Re-check recording dependency after FFmpeg is installed; verify Windows audio devices if needed."
        }));
    }

    let critical_count = issues
        .iter()
        .filter(|issue| issue["severity"].as_str() == Some("error"))
        .count();
    let mut issues_by_module = std::collections::BTreeMap::<String, usize>::new();
    for issue in &issues {
        if let Some(module) = issue["module"].as_str() {
            *issues_by_module.entry(module.to_string()).or_insert(0) += 1;
        }
    }
    let readiness_by_module = build_diagnostic_readiness_by_module(&ocr_runtime, &recording);

    Ok(serde_json::json!({
        "schemaVersion": 2,
        "generatedAt": generated_at,
        "app": {
            "name": "YSN Screenshot Translator",
            "version": env!("CARGO_PKG_VERSION"),
            "appDataDir": app_data_dir,
            "startupProbePath": startup_probe_path,
        },
        "health": {
            "ready": critical_count == 0,
            "criticalCount": critical_count,
            "issueCount": issues.len(),
            "issuesByModule": issues_by_module,
            "readinessByModule": readiness_by_module,
            "issues": issues,
        },
        "ocrRuntime": ocr_runtime,
        "recording": recording,
        "shortcuts": shortcut_status,
        "recovery": {
            "ocr": "Open the text recognition panel, choose Rapid OCR V5 or V4, then run self-test.",
            "recording": "Install or choose ffmpeg.exe, then re-check video recording dependency.",
            "shortcuts": "If global shortcuts fail, restart the app or change conflicting hotkeys in settings."
        }
    }))
}

fn startup_diagnostics_probe_path() -> PathBuf {
    std::env::temp_dir()
        .join("ysn_screenshot_translator")
        .join("startup_status.json")
}

fn write_startup_diagnostics_probe(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let path = startup_diagnostics_probe_path();
    let parent = path
        .parent()
        .ok_or_else(|| "failed to resolve startup diagnostics directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("create startup diagnostics directory failed: {}", e))?;
    let report = get_diagnostics_report(app.clone())?;
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "generatedAt": chrono::Local::now().to_rfc3339(),
        "processId": std::process::id(),
        "diagnostics": report,
    });
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize startup diagnostics failed: {}", e))?;
    fs::write(&path, body).map_err(|e| format!("write startup diagnostics failed: {}", e))?;
    Ok(path)
}

#[tauri::command]
fn get_startup_diagnostics_probe_path() -> Result<String, String> {
    Ok(startup_diagnostics_probe_path()
        .to_string_lossy()
        .to_string())
}

fn build_startup_readiness_snapshot(app: tauri::AppHandle) -> serde_json::Value {
    let checked_at = chrono::Local::now().to_rfc3339();
    let rapid_ocr = get_rapid_ocr_status(app.clone()).unwrap_or_else(|error| {
        serde_json::json!({
            "ready": false,
            "runtime": "rapidocr",
            "lastError": error,
        })
    });
    let recording = get_recording_info(app).unwrap_or_else(|error| {
        serde_json::json!({
            "ffmpegFound": false,
            "isRecording": false,
            "audioDevices": [],
            "lastError": error,
        })
    });
    serde_json::json!({
        "checkedAt": checked_at,
        "rapidOcr": rapid_ocr,
        "recording": recording,
    })
}

fn cache_startup_readiness_snapshot(snapshot: serde_json::Value) {
    if let Ok(mut guard) = get_startup_readiness_cache().lock() {
        *guard = Some(snapshot);
    }
}

#[tauri::command]
fn get_startup_readiness_snapshot() -> Result<serde_json::Value, String> {
    let snapshot = get_startup_readiness_cache()
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    Ok(snapshot.unwrap_or_else(|| serde_json::json!({
        "checkedAt": null,
        "rapidOcr": null,
        "recording": null,
        "pending": true,
    })))
}

#[tauri::command]
async fn run_startup_readiness_probe(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let snapshot = tokio::task::spawn_blocking(move || build_startup_readiness_snapshot(app))
        .await
        .map_err(|error| format!("startup readiness probe task failed: {error}"))?;
    cache_startup_readiness_snapshot(snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
fn get_recording_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let _ = cleanup_finished_recording_process()?;
    let ffmpeg = find_ffmpeg_executable(&app);
    let is_recording = get_recording_process()
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    let audio_devices = if let Some(ffmpeg_path) = &ffmpeg {
        collect_ffmpeg_audio_devices(ffmpeg_path)
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "ffmpegFound": ffmpeg.is_some(),
        "ffmpegPath": ffmpeg.map(|path| path.to_string_lossy().to_string()),
        "isRecording": is_recording,
        "audioDevices": audio_devices,
    }))
}

fn recording_temp_dir() -> PathBuf {
    let mut dir = app_data_dir();
    dir.push("recordings");
    dir
}

fn default_recording_output_dir() -> PathBuf {
    dirs::video_dir().unwrap_or_else(app_data_dir).join("YSN")
}

fn timestamped_recording_file_name() -> String {
    let now = chrono::Local::now();
    format!("YSN_{}.mp4", now.format("%Y%m%d_%H%M%S"))
}

fn unique_recording_output_path() -> Result<PathBuf, String> {
    let dir = default_recording_output_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create recording directory failed: {}", e))?;
    let base = timestamped_recording_file_name();
    let path = dir.join(&base);
    if !path.exists() {
        return Ok(path);
    }
    let stem = base.trim_end_matches(".mp4");
    for index in 2..1000 {
        let candidate = dir.join(format!("{}_{}.mp4", stem, index));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("failed to create unique recording filename".to_string())
}

fn recording_output_path(output_dir: Option<String>) -> Result<PathBuf, String> {
    let dir = output_dir
        .filter(|value| !value.trim().is_empty())
        .map(|value| PathBuf::from(value.trim()))
        .unwrap_or_else(recording_temp_dir);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(dir.join(format!("recording_{}.mp4", millis)))
}

#[tauri::command]
fn get_default_recording_output_dir() -> Result<String, String> {
    let dir = default_recording_output_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create recording directory failed: {}", e))?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
fn open_path_in_file_manager(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is empty".to_string());
    }
    let input_path = PathBuf::from(trimmed);
    let target_path = if input_path.exists() {
        input_path
    } else {
        fs::create_dir_all(&input_path)
            .map_err(|e| format!("create directory before opening failed: {}", e))?;
        input_path
    };

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer.exe");
        if target_path.is_file() {
            command.arg(format!("/select,{}", target_path.to_string_lossy()));
        } else {
            command.arg(target_path.to_string_lossy().to_string());
        }
        command
            .spawn()
            .map_err(|e| format!("open path with Explorer failed: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&target_path)
            .spawn()
            .map_err(|e| format!("open path failed: {}", e))?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&target_path)
            .spawn()
            .map_err(|e| format!("open path failed: {}", e))?;
        Ok(())
    }
}

fn resolution_scale_filter(resolution: &str) -> Option<&'static str> {
    match resolution {
        "480p" => Some("scale=-2:480"),
        "720p" => Some("scale=-2:720"),
        "1080p" => Some("scale=-2:1080"),
        "original" => None,
        _ => Some("scale=-2:1080"),
    }
}

fn push_recording_audio_input(
    device: Option<&str>,
    label: &str,
    args: &mut Vec<String>,
) -> Result<(), String> {
    let name = device
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Please choose {} audio device", label))?;
    let trimmed = name.trim();
    if let Some(wasapi_device) = trimmed.strip_prefix("wasapi:") {
        args.extend([
            "-f".to_string(),
            "wasapi".to_string(),
            "-i".to_string(),
            wasapi_device.trim().to_string(),
        ]);
    } else {
        let dshow_device = trimmed.strip_prefix("dshow:").unwrap_or(trimmed);
        args.extend([
            "-f".to_string(),
            "dshow".to_string(),
            "-i".to_string(),
            format!("audio={}", dshow_device.trim()),
        ]);
    }
    Ok(())
}

fn build_recording_args(
    options: &RecordingOptions,
    output_path: &Path,
) -> Result<Vec<String>, String> {
    let fps = options.fps.unwrap_or(30).clamp(1, 60).to_string();
    let resolution = options.resolution.as_deref().unwrap_or("1080p");
    let audio_mode = options.audio_mode.as_deref().unwrap_or("none");

    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-f".to_string(),
        "gdigrab".to_string(),
        "-framerate".to_string(),
        fps.clone(),
    ];
    if let (Some(x), Some(y), Some(w), Some(h)) = (
        options.region_x,
        options.region_y,
        options.region_w,
        options.region_h,
    ) {
        if w <= 0 || h <= 0 {
            return Err("Invalid recording region size".to_string());
        }
        args.extend([
            "-offset_x".to_string(),
            x.to_string(),
            "-offset_y".to_string(),
            y.to_string(),
            "-video_size".to_string(),
            format!("{}x{}", w, h),
        ]);
    }
    args.extend(["-i".to_string(), "desktop".to_string()]);

    let audio_inputs = match audio_mode {
        "none" => 0,
        "mic" => {
            push_recording_audio_input(options.mic_device.as_deref(), "microphone", &mut args)?;
            1
        }
        "system" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "绯荤粺澹伴煶",
                &mut args,
            )?;
            1
        }
        "system_mic" => {
            push_recording_audio_input(
                options.system_audio_device.as_deref(),
                "绯荤粺澹伴煶",
                &mut args,
            )?;
            push_recording_audio_input(options.mic_device.as_deref(), "microphone", &mut args)?;
            2
        }
        _ => return Err("Unknown recording audio mode".to_string()),
    };

    args.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-r".to_string(),
        fps,
    ]);
    if let Some(filter) = resolution_scale_filter(resolution) {
        args.extend(["-vf".to_string(), filter.to_string()]);
    }

    match audio_inputs {
        0 => args.push("-an".to_string()),
        1 => args.extend([
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "1:a".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "160k".to_string(),
        ]),
        2 => args.extend([
            "-filter_complex".to_string(),
            "[1:a][2:a]amix=inputs=2:duration=longest[aout]".to_string(),
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "[aout]".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "160k".to_string(),
        ]),
        _ => {}
    }
    args.push(output_path.to_string_lossy().to_string());
    Ok(args)
}

#[tauri::command]
async fn get_ffmpeg_release_info() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("Create request client failed: {}", e))?;
    let release = client
        .get("https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest")
        .send()
        .await
        .map_err(|e| format!("Check ffmpeg release failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Read ffmpeg release response failed: {}", e))?
        .json::<GithubReleaseInfo>()
        .await
        .map_err(|e| format!("Parse ffmpeg release failed: {}", e))?;

    let asset = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".zip")
                && name.contains("win64")
                && name.contains("gpl")
                && !name.contains("shared")
        })
        .or_else(|| {
            release.assets.iter().find(|asset| {
                let name = asset.name.to_ascii_lowercase();
                name.ends_with(".zip") && name.contains("win64") && !name.contains("shared")
            })
        })
        .ok_or_else(|| {
            "No Windows x64 ffmpeg zip asset found in the official release".to_string()
        })?;

    Ok(serde_json::json!({
        "tag": release.tag_name,
        "pageUrl": release.html_url,
        "assetName": asset.name,
        "downloadUrl": asset.browser_download_url,
        "size": asset.size,
        "installDir": default_ffmpeg_install_dir().to_string_lossy().to_string(),
    }))
}

#[tauri::command]
async fn download_ffmpeg_release(
    app: tauri::AppHandle,
    url: String,
    tag: String,
) -> Result<serde_json::Value, String> {
    let allowed = [
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/",
        "https://objects.githubusercontent.com/github-production-release-asset-",
    ];
    if !allowed.iter().any(|prefix| url.starts_with(prefix))
        || !url.to_ascii_lowercase().ends_with(".zip")
    {
        return Err(
            "Please choose an official Windows zip from BtbN/FFmpeg-Builds GitHub Releases"
                .to_string(),
        );
    }

    emit_ffmpeg_progress(&app, "Preparing", 0, None, 1);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("ScreenshotTranslator/1.0")
        .build()
        .map_err(|e| format!("Create download client failed: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download ffmpeg failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Download ffmpeg failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read ffmpeg download stream failed: {}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let percent = total
            .map(|value| ((downloaded as f64 / value.max(1) as f64) * 80.0).round() as u8)
            .unwrap_or(10)
            .clamp(1, 80);
        emit_ffmpeg_progress(&app, "Downloading", downloaded, total, percent);
    }

    let safe_tag = sanitize_tag(&tag);
    let mut download_dir = app_data_dir();
    download_dir.push("ffmpeg");
    download_dir.push("downloads");
    fs::create_dir_all(&download_dir)
        .map_err(|e| format!("Create ffmpeg download directory failed: {}", e))?;
    let archive_path = download_dir.join(format!("ffmpeg-{}.zip", safe_tag));
    fs::write(&archive_path, &bytes).map_err(|e| format!("Save ffmpeg archive failed: {}", e))?;

    emit_ffmpeg_progress(&app, "Installing", downloaded, total, 85);
    let install_dir = ensure_writable_dir(default_ffmpeg_install_dir());
    let exe_path = extract_ffmpeg_exe_from_zip(&bytes, &install_dir)?;
    let _ = fs::remove_file(&archive_path);
    emit_ffmpeg_progress(&app, "瀹屾垚", downloaded, total, 100);

    Ok(serde_json::json!({
        "path": exe_path.to_string_lossy().to_string(),
        "installDir": install_dir.to_string_lossy().to_string(),
        "bytes": bytes.len(),
    }))
}

#[tauri::command]
fn choose_ffmpeg_executable(current_path: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Choose ffmpeg.exe")
        .add_filter("ffmpeg", &["exe"]);
    if let Some(path) = current_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            let path_buf = PathBuf::from(trimmed);
            if let Some(parent) = path_buf.parent() {
                dialog = dialog.set_directory(parent);
            }
        }
    }
    Ok(dialog
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn choose_recording_output_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
    let mut dialog = rfd::FileDialog::new().set_title("Choose recording output directory");
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
fn start_recording(app: tauri::AppHandle, options: RecordingOptions) -> Result<String, String> {
    let _ = cleanup_finished_recording_process()?;
    {
        let guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("Recording is already running".to_string());
        }
    }

    let ffmpeg = find_ffmpeg_executable(&app).ok_or_else(|| {
        "ffmpeg.exe was not found. Put ffmpeg.exe next to the app or choose ffmpeg.exe in settings.".to_string()
    })?;
    let output_path = recording_output_path(options.output_dir.clone())?;
    let args = build_recording_args(&options, &output_path)?;

    let mut cmd = hidden_ffmpeg_command(&ffmpeg);
    cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start ffmpeg recording: {}", e))?;
    if let Some(status) = child
        .try_wait()
        .map_err(|e| format!("Failed to inspect ffmpeg recording process: {}", e))?
    {
        return Err(format!("ffmpeg recording exited immediately with status {}. Check recording options, audio device, or ffmpeg version.", status));
    }
    let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Recording is already running".to_string());
    }
    *guard = Some(child);
    Ok(output_path.to_string_lossy().to_string())
}

fn stop_recording_internal(grace_ms: u64) -> Result<(), String> {
    let child = {
        let mut guard = get_recording_process().lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(mut child) = child {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(b"q\n");
            let _ = stdin.flush();
        }
        let attempts = (grace_ms / 100).max(1);
        let mut exited = false;
        for attempt in 0..attempts {
            if child
                .try_wait()
                .map_err(|e| format!("Failed to stop recording process: {}", e))?
                .is_some()
            {
                exited = true;
                break;
            }
            if attempt + 1 < attempts {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        if !exited {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    Ok(())
}
#[tauri::command]
fn stop_recording() -> Result<(), String> {
    stop_recording_internal(800)
}

#[tauri::command]
fn cancel_recording_process() -> Result<(), String> {
    stop_recording_internal(250)
}

fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "'\\''")
}

#[tauri::command]
fn concat_recording_segments(
    app: tauri::AppHandle,
    segment_paths: Vec<String>,
) -> Result<String, String> {
    if segment_paths.is_empty() {
        return Err("no recording segments to merge".to_string());
    }
    let existing_segments: Vec<PathBuf> = segment_paths
        .iter()
        .map(|path| PathBuf::from(path.trim()))
        .filter(|path| path.exists())
        .collect();
    if existing_segments.is_empty() {
        return Err("video file does not exist".to_string());
    }

    let save_path = unique_recording_output_path()?;
    if existing_segments.len() == 1 {
        fs::copy(&existing_segments[0], &save_path)
            .map_err(|e| format!("save recording failed: {}", e))?;
        return Ok(save_path.to_string_lossy().to_string());
    }

    let ffmpeg = find_ffmpeg_executable(&app)
        .ok_or_else(|| "ffmpeg.exe not found, cannot merge recording segments".to_string())?;
    let mut list_path = recording_temp_dir();
    fs::create_dir_all(&list_path)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    list_path.push(format!("concat_{}.txt", millis));
    let list_body = existing_segments
        .iter()
        .map(|path| format!("file '{}'", escape_concat_path(path)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&list_path, list_body)
        .map_err(|e| format!("create recording temp directory failed: {}", e))?;

    let args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        list_path.to_string_lossy().to_string(),
        "-c".to_string(),
        "copy".to_string(),
        save_path.to_string_lossy().to_string(),
    ];
    let status = hidden_ffmpeg_command(&ffmpeg)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("failed to start ffmpeg merge: {}", e))?;
    let _ = fs::remove_file(&list_path);
    if !status.success() {
        return Err(format!(
            "ffmpeg failed to merge recording segments: {}",
            status
        ));
    }
    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
fn copy_file_to_clipboard(path: String) -> Result<(), String> {
    let file_path = PathBuf::from(path.trim());
    if !file_path.is_file() {
        return Err("video file does not exist".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Set-Clipboard -LiteralPath {}",
            shell_escape_powershell_single(&file_path.to_string_lossy())
        );
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .status()
            .map_err(|e| format!("failed to start clipboard command: {}", e))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!(
            "failed to copy video file to clipboard: {}",
            status
        ));
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("copying video files is not supported on this platform".to_string())
    }
}

fn shell_escape_powershell_single(value: &str) -> String {
    format!("'{}'", value.replace("'", "''"))
}

fn is_recording_temp_file(path: &Path, temp_dir: &Path) -> bool {
    let Ok(canonical_path) = fs::canonicalize(path) else {
        return false;
    };
    let Ok(canonical_temp_dir) = fs::canonicalize(temp_dir) else {
        return false;
    };
    canonical_path.starts_with(canonical_temp_dir)
        && canonical_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("mp4"))
            .unwrap_or(false)
}

#[tauri::command]
fn cleanup_recording_files(paths: Vec<String>) -> Result<(), String> {
    let temp_dir = recording_temp_dir();
    for path in paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path_buf = PathBuf::from(trimmed);
        if path_buf.exists() && is_recording_temp_file(&path_buf, &temp_dir) {
            let _ = fs::remove_file(path_buf);
        }
    }
    Ok(())
}

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Child, Stdio};
use std::time::Instant;

static RECORDING_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
fn get_recording_process() -> &'static Mutex<Option<Child>> {
    RECORDING_PROCESS.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f64,
    pub box_coords: Vec<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
struct RapidOcrRunnerOutput {
    status: String,
    engine: Option<String>,
    #[serde(rename = "modelVersion")]
    model_version: Option<String>,
    #[serde(rename = "selectedLang")]
    selected_lang: Option<String>,
    blocks: Option<Vec<OcrBlock>>,
    timings: Option<serde_json::Value>,
    candidates: Option<Vec<serde_json::Value>>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct RapidOcrCommandSpec {
    program: PathBuf,
    args_prefix: Vec<String>,
    kind: String,
}

#[tauri::command]
async fn prewarm_local_ocr_models(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let model_version = rapid_ocr_model_version();
        run_rapidocr_probe(&app, &model_version)?;
        Ok(vec![format!("rapidocr-{model_version}")])
    })
    .await
    .map_err(|error| format!("RapidOCR prewarm task failed: {error}"))?
}

#[tauri::command]
async fn run_local_ocr(
    app: tauri::AppHandle,
    image_base64: String,
    executable_path: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<Vec<OcrBlock>, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(15000).clamp(500, 60000));
    let task =
        tokio::task::spawn_blocking(move || run_local_ocr_sync(app, image_base64, executable_path));
    match tokio::time::timeout(timeout, task).await {
        Ok(joined) => joined.map_err(|e| format!("Local OCR task failed: {}", e))?,
        Err(_) => Err(format!("Local OCR timed out ({} ms)", timeout.as_millis())),
    }
}

fn run_local_ocr_sync(
    app: tauri::AppHandle,
    image_base64: String,
    _executable_path: Option<String>,
) -> Result<Vec<OcrBlock>, String> {
    match run_rapidocr_sync(&app, &image_base64) {
        Ok(blocks) if !blocks.is_empty() => return Ok(blocks),
        Ok(_) => {
            return Err(
                "\u{672c}\u{5730}\u{622a}\u{56fe}\u{7ffb}\u{8bd1}\u{672a}\u{8bc6}\u{522b}\u{5230}\u{6587}\u{5b57}\u{3002}\u{8bf7}\u{91cd}\u{65b0}\u{6846}\u{9009}\u{66f4}\u{6e05}\u{6670}\u{3001}\u{66f4}\u{5b8c}\u{6574}\u{7684}\u{6587}\u{5b57}\u{533a}\u{57df}\u{3002}".to_string(),
            );
        }
        Err(error) => return Err(error),
    }
}

fn run_rapidocr_sync(app: &tauri::AppHandle, image_base64: &str) -> Result<Vec<OcrBlock>, String> {
    let total_started = Instant::now();
    let image_bytes = BASE64_STANDARD
        .decode(image_base64)
        .map_err(|error| format!("Decode RapidOCR image failed: {error}"))?;
    let temp_path = write_rapidocr_temp_image(&image_bytes)?;
    let model_version = rapid_ocr_model_version();
    let mode = rapid_ocr_mode();
    let model_root = rapid_ocr_model_root();
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    if !missing_models.is_empty() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "RapidOCR model files are missing from {}: {}",
            model_root.display(),
            missing_models.join(", ")
        ));
    }
    let args = vec![
        "--image".to_string(),
        temp_path.to_string_lossy().to_string(),
        "--model-version".to_string(),
        model_version.clone(),
        "--mode".to_string(),
        mode,
        "--model-root".to_string(),
        model_root.to_string_lossy().to_string(),
    ];
    let result = run_rapidocr_json(app, args);
    let _ = fs::remove_file(&temp_path);
    let output = result?;
    if output.status != "success" {
        return Err(output
            .error
            .unwrap_or_else(|| "RapidOCR returned a failed status.".to_string()));
    }
    let blocks = output.blocks.unwrap_or_default();
    eprintln!(
        "[local-screenshot-translate] rapidocr total={}ms runner={} model={} lang={} blocks={} timings={}",
        total_started.elapsed().as_millis(),
        output.engine.as_deref().unwrap_or("rapidocr"),
        output.model_version.as_deref().unwrap_or(&model_version),
        output.selected_lang.as_deref().unwrap_or("auto"),
        blocks.len(),
        serde_json::to_string(&output.timings).unwrap_or_else(|_| "null".to_string())
    );
    if let Some(candidates) = output.candidates {
        eprintln!(
            "[local-screenshot-translate] rapidocr candidates {}",
            serde_json::to_string(&candidates).unwrap_or_else(|_| "[]".to_string())
        );
    }
    Ok(blocks)
}

fn rapid_ocr_model_version() -> String {
    match config_value_string("rapidOcrModelVersion")
        .unwrap_or_else(|| "v5".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "v4" => "v4".to_string(),
        _ => "v5".to_string(),
    }
}

fn rapid_ocr_mode() -> String {
    match config_value_string("rapidOcrMode")
        .unwrap_or_else(|| "auto".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "full" => "full".to_string(),
        "latin" => "latin".to_string(),
        _ => "auto".to_string(),
    }
}

fn repo_root_from_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|client_root| client_root.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn rapid_ocr_model_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = config_value_string("rapidOcrModelRoot")
        .or_else(|| std::env::var("YSN_RAPIDOCR_MODEL_ROOT").ok())
        .map(PathBuf::from)
    {
        candidates.push(path);
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("models").join("rapidocr"));
            if let Some(parent) = exe_dir.parent() {
                candidates.push(parent.join("models").join("rapidocr"));
            }
        }
    }
    candidates.push(repo_root_from_manifest().join("models").join("rapidocr"));
    candidates
}

fn rapid_ocr_model_root() -> PathBuf {
    rapid_ocr_model_root_candidates()
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| repo_root_from_manifest().join("models").join("rapidocr"))
}

fn rapid_ocr_required_model_files(model_version: &str) -> Vec<&'static str> {
    let mut files = vec![
        "ch_PP-LCNet_x0_25_textline_ori_cls_mobile.onnx",
        "ppocr_keys_v1.txt",
        "ppocrv5_dict.txt",
    ];
    if model_version == "v4" {
        files.extend([
            "ch_PP-OCRv4_det_mobile.onnx",
            "ch_PP-OCRv4_rec_mobile.onnx",
            "latin_PP-OCRv3_rec_mobile.onnx",
        ]);
    } else {
        files.extend([
            "ch_PP-OCRv5_det_mobile.onnx",
            "ch_PP-OCRv5_rec_mobile.onnx",
            "latin_PP-OCRv5_rec_mobile.onnx",
            "korean_PP-OCRv5_rec_mobile.onnx",
            "arabic_PP-OCRv5_rec_mobile.onnx",
            "cyrillic_PP-OCRv5_rec_mobile.onnx",
            "th_PP-OCRv5_rec_mobile.onnx",
        ]);
    }
    files
}

fn rapid_ocr_missing_model_files(model_root: &Path, model_version: &str) -> Vec<String> {
    rapid_ocr_required_model_files(model_version)
        .into_iter()
        .filter(|name| !model_root.join(name).is_file())
        .map(str::to_string)
        .collect()
}

fn write_rapidocr_temp_image(image_bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("ysn-screenshot-translator").join("rapidocr");
    fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "failed to create RapidOCR temp directory {}: {error}",
            dir.display()
        )
    })?;
    let path = dir.join(format!(
        "ocr-{}-{}.png",
        std::process::id(),
        chrono::Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
    ));
    fs::write(&path, image_bytes).map_err(|error| {
        format!(
            "failed to write RapidOCR temp image {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn resolve_rapidocr_command(app: &tauri::AppHandle) -> Result<RapidOcrCommandSpec, String> {
    if let Some(path) = config_value_string("rapidOcrRunnerPath")
        .or_else(|| std::env::var("YSN_RAPIDOCR_RUNNER").ok())
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return Ok(RapidOcrCommandSpec {
            program: path,
            args_prefix: Vec::new(),
            kind: "custom-runner".to_string(),
        });
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if let Some(path) = find_rapidocr_runner_exe(exe_dir) {
                return Ok(RapidOcrCommandSpec {
                    program: path,
                    args_prefix: Vec::new(),
                    kind: "bundled-runner".to_string(),
                });
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        for relative in [
            PathBuf::from("rapidocr")
                .join("rapidocr-runner")
                .join("rapidocr-runner.exe"),
            PathBuf::from("rapidocr").join("rapidocr-runner.exe"),
            PathBuf::from("resources")
                .join("rapidocr")
                .join("rapidocr-runner")
                .join("rapidocr-runner.exe"),
            PathBuf::from("resources")
                .join("rapidocr")
                .join("rapidocr-runner.exe"),
        ] {
            let path = resource_dir.join(relative);
            if path.exists() {
                return Ok(RapidOcrCommandSpec {
                    program: path,
                    args_prefix: Vec::new(),
                    kind: "resource-runner".to_string(),
                });
            }
        }
    }

    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("rapidocr")
        .join("rapidocr_runner.py");
    if script_path.exists() {
        return Ok(RapidOcrCommandSpec {
            program: PathBuf::from("python"),
            args_prefix: vec![script_path.to_string_lossy().to_string()],
            kind: "python-runner".to_string(),
        });
    }

    Err("RapidOCR runner was not found. Expected bundled rapidocr-runner.exe or src-tauri/rapidocr/rapidocr_runner.py.".to_string())
}

fn find_rapidocr_runner_exe(dir: &Path) -> Option<PathBuf> {
    let exact = dir.join("rapidocr-runner.exe");
    if exact.exists() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(|name| name.starts_with("rapidocr-runner") && name.ends_with(".exe"))
                .unwrap_or(false)
        })
}

fn run_rapidocr_json(
    app: &tauri::AppHandle,
    args: Vec<String>,
) -> Result<RapidOcrRunnerOutput, String> {
    let spec = resolve_rapidocr_command(app)?;
    let mut command = Command::new(&spec.program);
    command.args(&spec.args_prefix);
    command.args(&args);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command.output().map_err(|error| {
        format!(
            "failed to start RapidOCR runner ({}): {error}",
            spec.program.display()
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!(
            "RapidOCR runner failed with status {}. stdout: {} stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| {
            format!(
                "RapidOCR runner did not return JSON. stderr: {}",
                stderr.trim()
            )
        })?;
    let mut parsed: RapidOcrRunnerOutput = serde_json::from_str(json_line.trim())
        .map_err(|error| format!("failed to parse RapidOCR JSON: {error}; output: {json_line}"))?;
    if parsed.engine.is_none() {
        parsed.engine = Some(spec.kind);
    }
    Ok(parsed)
}

fn run_rapidocr_probe(
    app: &tauri::AppHandle,
    model_version: &str,
) -> Result<RapidOcrRunnerOutput, String> {
    let model_root = rapid_ocr_model_root();
    run_rapidocr_json(
        app,
        vec![
            "--probe".to_string(),
            "--model-version".to_string(),
            model_version.to_string(),
            "--model-root".to_string(),
            model_root.to_string_lossy().to_string(),
        ],
    )
}

#[tauri::command]
fn get_rapid_ocr_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let model_version = rapid_ocr_model_version();
    let model_root = rapid_ocr_model_root();
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    let runner = resolve_rapidocr_command(&app);
    let mut last_error: Option<String> = None;
    let mut probe_timings = serde_json::json!(null);
    let mut probe_ok = false;
    if !missing_models.is_empty() {
        last_error = Some(format!(
            "RapidOCR model files are missing from {}: {}",
            model_root.display(),
            missing_models.join(", ")
        ));
    } else if runner.is_ok() {
        match run_rapidocr_probe(&app, &model_version) {
            Ok(output) if output.status == "success" => {
                probe_ok = true;
                probe_timings = output.timings.unwrap_or_else(|| serde_json::json!(null));
            }
            Ok(output) => {
                last_error = Some(
                    output
                        .error
                        .unwrap_or_else(|| "RapidOCR probe failed.".to_string()),
                );
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    } else if let Err(error) = &runner {
        last_error = Some(error.clone());
    }
    let runner_kind = runner
        .as_ref()
        .map(|spec| spec.kind.clone())
        .unwrap_or_else(|_| "missing".to_string());
    let runner_path = runner
        .as_ref()
        .map(|spec| spec.program.to_string_lossy().to_string())
        .unwrap_or_default();
    let models_ready = missing_models.is_empty();
    let ready = runner.is_ok() && models_ready && probe_ok;
    Ok(serde_json::json!({
        "ready": ready,
        "runnerReady": runner.is_ok(),
        "runtimeInferenceReady": ready,
        "modelPacksReady": models_ready,
        "activeModelsReady": models_ready,
        "selfTestReady": probe_ok,
        "runtime": "rapidocr",
        "engine": "rapidocr",
        "runnerKind": runner_kind,
        "runnerPath": runner_path,
        "runtimeVersion": "rapidocr-python-3.x",
        "modelSetVersion": format!("rapidocr-{}", model_version),
        "rapidOcrModelVersion": model_version,
        "modelDir": model_root.to_string_lossy().to_string(),
        "modelRoot": model_root.to_string_lossy().to_string(),
        "missingModelFiles": missing_models,
        "defaultSourceLanguage": "auto",
        "defaultProfile": "balanced",
        "lastError": last_error,
        "probeTimings": probe_timings,
        "supportedModelVersions": ["v5", "v4"],
        "readinessSteps": [
            {
                "id": "rapidocr-runner",
                "ready": runner.is_ok(),
                "severity": if runner.is_ok() { "success" } else { "error" },
                "label": "RapidOCR runner",
                "description": "RapidOCR runner executable or development Python runner is available.",
                "nextAction": if runner.is_ok() { "run-ocr-self-test" } else { "install-rapidocr-runner" }
            },
            {
                "id": "rapidocr-probe",
                "ready": probe_ok,
                "severity": if probe_ok { "success" } else { "error" },
                "label": "RapidOCR probe",
                "description": "RapidOCR can initialize the configured PP-OCR model version.",
                "nextAction": if probe_ok { "ready" } else { "run-ocr-self-test" }
            },
            {
                "id": "rapidocr-root-models",
                "ready": models_ready,
                "severity": if models_ready { "success" } else { "error" },
                "label": "RapidOCR root models",
                "description": "RapidOCR model files are present under the repository or app root models/rapidocr directory.",
                "nextAction": if models_ready { "ready" } else { "restore-root-models" }
            }
        ]
    }))
}

#[tauri::command]
fn run_rapid_ocr_self_test(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let tested_at = chrono::Local::now().to_rfc3339();
    let model_version = rapid_ocr_model_version();
    let model_root = rapid_ocr_model_root();
    let missing_models = rapid_ocr_missing_model_files(&model_root, &model_version);
    if !missing_models.is_empty() {
        return Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "modelRoot": model_root.to_string_lossy().to_string(),
            "message": format!("RapidOCR model files are missing from {}: {}", model_root.display(), missing_models.join(", ")),
            "samples": []
        }));
    }
    match run_rapidocr_probe(&app, &model_version) {
        Ok(output) if output.status == "success" => Ok(serde_json::json!({
            "ok": true,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "modelRoot": model_root.to_string_lossy().to_string(),
            "message": "RapidOCR probe passed.",
            "timings": output.timings,
            "samples": [
                { "id": "engine-init", "ok": true, "confidence": 1.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
        Ok(output) => Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "message": output.error.unwrap_or_else(|| "RapidOCR probe failed.".to_string()),
            "samples": [
                { "id": "engine-init", "ok": false, "confidence": 0.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
        Err(error) => Ok(serde_json::json!({
            "ok": false,
            "testedAt": tested_at,
            "runtime": "rapidocr",
            "modelVersion": model_version,
            "message": error,
            "samples": [
                { "id": "engine-init", "ok": false, "confidence": 0.0, "modelId": format!("rapidocr-{}", model_version) }
            ]
        })),
    }
}
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

fn history_path_from_config() -> PathBuf {
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

fn history_limits_from_config() -> (usize, u64) {
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
fn get_history() -> Result<String, String> {
    let path = history_path_from_config();
    if !path.exists() {
        return Ok("[]".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_history(record: String) -> Result<(), String> {
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
fn get_history_info() -> Result<serde_json::Value, String> {
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
fn choose_history_dir(current_dir: Option<String>) -> Result<Option<String>, String> {
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
fn clear_history() -> Result<(), String> {
    let path = history_path_from_config();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_shortcut_status,
            get_config,
            get_history,
            get_history_info,
            choose_history_dir,
            add_history,
            clear_history,
            get_recording_info,
            get_default_recording_output_dir,
            open_path_in_file_manager,
            get_recording_targets,
            get_ffmpeg_release_info,
            download_ffmpeg_release,
            choose_ffmpeg_executable,
            choose_recording_output_dir,
            start_recording,
            stop_recording,
            cancel_recording_process,
            set_window_capture_excluded,
            show_recording_overlay,
            hide_recording_overlay,
            set_recording_overlay_status,
            concat_recording_segments,
            cleanup_recording_files,
            copy_file_to_clipboard,
            save_config,
            is_autostart_enabled,
            set_autostart_enabled,
            start_screenshot,
            get_fullscreen_image,
            capture_region,
            copy_image_to_clipboard,
            save_image_to_file,
            quick_fullscreen_capture,
            capture_live_region,
            scroll_mouse_at,
            cancel_screenshot,
            force_close_screenshots,
            get_window_rects,
            overlay_ready_to_show,
            run_local_ocr,
            prewarm_local_ocr_models,
            re_register_shortcut,
            get_diagnostics_report,
            get_startup_diagnostics_probe_path,
            get_startup_readiness_snapshot,
            run_startup_readiness_probe,
            get_rapid_ocr_status,
            run_rapid_ocr_self_test
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            if let Some(screenshot_win) = app.get_webview_window("screenshot") {
                disable_windows_transition(&screenshot_win);
            }

            let (configured_hotkey, configured_translate_hotkey) = read_configured_hotkeys();
            let shortcut_status = register_global_shortcuts(
                app.handle(),
                &configured_hotkey,
                &configured_translate_hotkey,
            );
            app.manage(AppShortcutStatus(std::sync::Mutex::new(shortcut_status)));
            if let Err(error) = write_startup_diagnostics_probe(app.handle()) {
                eprintln!("Failed to write startup diagnostics probe: {}", error);
            }
            let readiness_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let probe_app = readiness_app.clone();
                match tokio::task::spawn_blocking(move || build_startup_readiness_snapshot(probe_app))
                    .await
                {
                    Ok(snapshot) => cache_startup_readiness_snapshot(snapshot),
                    Err(error) => eprintln!("Failed to run startup readiness probe: {}", error),
                }
            });

            let screenshot_item = tauri::menu::MenuItemBuilder::new("Screenshot Now")
                .id("screenshot")
                .build(app)?;
            let show_item = tauri::menu::MenuItemBuilder::new("Show Main Window")
                .id("show")
                .build(app)?;
            let exit_item = tauri::menu::MenuItemBuilder::new("Exit")
                .id("exit")
                .build(app)?;
            let tray_menu = tauri::menu::MenuBuilder::new(app)
                .item(&screenshot_item)
                .item(&show_item)
                .separator()
                .item(&exit_item)
                .build()?;
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(
                    tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).unwrap(),
                )
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
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
                })
                .on_tray_icon_event(|tray, event| match event {
                    tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    tauri::tray::TrayIconEvent::DoubleClick {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = start_screenshot(app, None).await {
                                eprintln!("Failed to start screenshot: {}", e);
                            }
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            if label == "screenshot" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let _ = window.set_always_on_top(false);
                        let _ = window.hide();
                        CAPTURING.store(false, Ordering::SeqCst);
                        api.prevent_close();
                    }
                    tauri::WindowEvent::Destroyed => {
                        CAPTURING.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            } else if label.starts_with("screenshot_") {
                if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed =
                    event
                {
                    CAPTURING.store(false, Ordering::SeqCst);
                }
            } else if label == "recording_border" || label.starts_with("recording_border_") {
                if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed =
                    event
                {
                    let _ = window.set_always_on_top(false);
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

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct RawOcrBlock {
        text: String,
        score: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[derive(Debug, Serialize)]
    struct OcrBlock {
        text: String,
        confidence: f64,
        box_coords: Vec<Vec<i32>>,
    }

    #[test]
    fn test_raw_score_mapping() {
        let raw_json =
            r#"{"text": "Test OCR", "score": 0.975, "box_coords": [[0,0],[10,0],[10,5],[0,5]]}"#;
        let raw: RawOcrBlock = serde_json::from_str(raw_json).unwrap();
        let mapped = OcrBlock {
            text: raw.text,
            confidence: raw.score,
            box_coords: raw.box_coords,
        };
        assert_eq!(mapped.confidence, 0.975);
        assert_eq!(mapped.text, "Test OCR");
    }

    #[test]
    fn test_recording_resolution_filter_defaults_to_1080p() {
        assert_eq!(super::resolution_scale_filter("480p"), Some("scale=-2:480"));
        assert_eq!(super::resolution_scale_filter("720p"), Some("scale=-2:720"));
        assert_eq!(
            super::resolution_scale_filter("1080p"),
            Some("scale=-2:1080")
        );
        assert_eq!(super::resolution_scale_filter("original"), None);
        assert_eq!(
            super::resolution_scale_filter("unexpected"),
            Some("scale=-2:1080")
        );
    }

    fn recording_options(audio_mode: &str) -> super::RecordingOptions {
        super::RecordingOptions {
            fps: Some(60),
            resolution: Some("1080p".to_string()),
            audio_mode: Some(audio_mode.to_string()),
            mic_device: Some("dshow:Microphone Array".to_string()),
            system_audio_device: Some("wasapi:default".to_string()),
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        }
    }

    fn output_path() -> &'static std::path::Path {
        std::path::Path::new("recording_test.mp4")
    }

    #[test]
    fn test_recording_args_without_audio_use_default_1080p() {
        let options = super::RecordingOptions {
            fps: None,
            resolution: None,
            audio_mode: None,
            mic_device: None,
            system_audio_device: None,
            output_dir: None,
            region_x: None,
            region_y: None,
            region_w: None,
            region_h: None,
        };
        let args = super::build_recording_args(&options, output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-framerate", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-r", "30"]));
        assert!(args.windows(2).any(|pair| pair == ["-vf", "scale=-2:1080"]));
        assert!(args.contains(&"-an".to_string()));
        assert_eq!(args.last().unwrap(), "recording_test.mp4");
    }

    #[test]
    fn test_recording_args_original_resolution_omits_scale_filter() {
        let mut options = recording_options("none");
        options.resolution = Some("original".to_string());
        let args = super::build_recording_args(&options, output_path()).unwrap();
        assert!(!args.contains(&"-vf".to_string()));
    }

    #[test]
    fn test_recording_args_system_audio_uses_wasapi() {
        let args =
            super::build_recording_args(&recording_options("system"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "wasapi"]));
        assert!(args.windows(2).any(|pair| pair == ["-i", "default"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "1:a"]));
    }

    #[test]
    fn test_recording_args_microphone_uses_dshow() {
        let args = super::build_recording_args(&recording_options("mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-f", "dshow"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-i", "audio=Microphone Array"]));
    }

    #[test]
    fn test_recording_args_system_and_microphone_mix_audio() {
        let args =
            super::build_recording_args(&recording_options("system_mic"), output_path()).unwrap();
        assert!(args.windows(2).any(|pair| pair
            == [
                "-filter_complex",
                "[1:a][2:a]amix=inputs=2:duration=longest[aout]"
            ]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[aout]"]));
    }

    #[test]
    fn test_recording_args_reject_missing_or_unknown_audio() {
        let mut missing_mic = recording_options("mic");
        missing_mic.mic_device = Some("  ".to_string());
        assert!(super::build_recording_args(&missing_mic, output_path())
            .unwrap_err()
            .contains("microphone"));

        let unknown = recording_options("speaker_only");
        assert_eq!(
            super::build_recording_args(&unknown, output_path()).unwrap_err(),
            "Unknown recording audio mode"
        );
    }

    #[test]
    fn test_audio_device_parser_deduplicates_dshow_devices() {
        let output = r#"
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "Stereo Mix" (audio)
[dshow @ 000]  "Microphone Array" (audio)
[dshow @ 000]  "USB Camera" (video)
"#;
        let devices = super::parse_quoted_audio_devices(output, true, None);
        assert_eq!(
            devices,
            vec!["Microphone Array".to_string(), "Stereo Mix".to_string()]
        );
    }

    #[test]
    fn test_audio_device_parser_prefixes_wasapi_devices() {
        let output = r#"
[wasapi @ 000] "default"
[wasapi @ 000] "Speakers (Realtek Audio)"
"#;
        let devices = super::parse_quoted_audio_devices(output, false, Some("wasapi:"));
        assert_eq!(
            devices,
            vec![
                "wasapi:default".to_string(),
                "wasapi:Speakers (Realtek Audio)".to_string()
            ]
        );
    }

    #[test]
    fn test_ffmpeg_input_format_detection() {
        let output = r#"
File formats:
 D  dshow           DirectShow capture
 DE gdigrab         GDI API Windows frame grabber
  E mp4             MP4 muxer
"#;
        assert!(super::ffmpeg_supports_input_format(output, "dshow"));
        assert!(super::ffmpeg_supports_input_format(output, "gdigrab"));
        assert!(!super::ffmpeg_supports_input_format(output, "wasapi"));
        assert!(!super::ffmpeg_supports_input_format(output, "mp4"));
    }

    #[test]
    fn test_sanitize_tag_keeps_release_names_filesystem_safe() {
        assert_eq!(super::sanitize_tag("v1.2.3"), "v1.2.3");
        assert_eq!(
            super::sanitize_tag("release/2026:01 beta"),
            "release_2026_01_beta"
        );
        assert_eq!(super::sanitize_tag("***"), "___");
    }
    #[test]
    fn test_recording_overlay_status_color_mapping() {
        assert_eq!(
            super::recording_color_ref("ready"),
            super::RECORDING_BORDER_BLUE
        );
        assert_eq!(
            super::recording_color_ref("recording"),
            super::RECORDING_BORDER_RED
        );
        assert_eq!(
            super::recording_color_ref("paused"),
            super::RECORDING_BORDER_YELLOW
        );
        assert_eq!(
            super::recording_color_ref("saved"),
            super::RECORDING_BORDER_BLUE
        );
    }

    #[test]
    fn test_default_recording_output_dir_ends_with_ysn() {
        let dir = super::default_recording_output_dir();
        assert_eq!(
            dir.file_name().and_then(|value| value.to_str()),
            Some("YSN")
        );
    }

    #[test]
    fn test_cleanup_recording_files_only_deletes_temp_mp4() {
        let temp_dir = super::recording_temp_dir();
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_file = temp_dir.join("unit_test_cleanup_boundary.mp4");
        std::fs::write(&temp_file, b"temp").unwrap();

        let external_dir = std::env::temp_dir().join("ysn_recording_boundary_external");
        std::fs::create_dir_all(&external_dir).unwrap();
        let external_file = external_dir.join("unit_test_external.mp4");
        std::fs::write(&external_file, b"external").unwrap();

        super::cleanup_recording_files(vec![
            temp_file.to_string_lossy().to_string(),
            external_file.to_string_lossy().to_string(),
        ])
        .unwrap();

        assert!(!temp_file.exists());
        assert!(external_file.exists());

        let _ = std::fs::remove_file(external_file);
        let _ = std::fs::remove_dir(external_dir);
    }

    #[test]
    fn test_startup_diagnostics_probe_path_is_in_temp_dir() {
        let path = super::startup_diagnostics_probe_path();
        assert!(path.starts_with(std::env::temp_dir()));
        assert_eq!(
            path.file_name().and_then(|value| value.to_str()),
            Some("startup_status.json")
        );
    }

    #[test]
    fn test_diagnostic_readiness_by_module_keeps_ocr_not_ready() {
        let ocr_runtime = serde_json::json!({
            "ready": false,
            "readinessSteps": [
                { "id": "rapidocr-runner", "ready": true },
                { "id": "rapidocr-probe", "ready": false, "nextAction": "run-ocr-self-test" }
            ]
        });
        let recording = serde_json::json!({ "ffmpegFound": false, "audioDevices": [] });
        let readiness = super::build_diagnostic_readiness_by_module(&ocr_runtime, &recording);
        assert_eq!(readiness["ocrRuntime"]["ready"].as_bool(), Some(false));
        assert_eq!(readiness["ocrRuntime"]["readySteps"].as_u64(), Some(1));
        assert_eq!(readiness["ocrRuntime"]["totalSteps"].as_u64(), Some(2));
        assert_eq!(
            readiness["ocrRuntime"]["firstBlockedStep"]["id"].as_str(),
            Some("rapidocr-probe")
        );
        assert_eq!(readiness["recording"]["ready"].as_bool(), Some(false));
    }
}

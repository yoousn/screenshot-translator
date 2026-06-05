use std::time::Duration;
use tauri::Manager;
#[cfg(target_os = "windows")]
use crate::win32;
use crate::recording_overlay::*;

#[cfg(target_os = "windows")]
const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
#[cfg(target_os = "windows")]
const SW_SHOW: i32 = 5;
#[cfg(target_os = "windows")]
const HWND_TOPMOST: isize = -1;
#[cfg(target_os = "windows")]
const SWP_NOSIZE: u32 = 0x0001;
#[cfg(target_os = "windows")]
const SWP_NOMOVE: u32 = 0x0002;
#[cfg(target_os = "windows")]
const SWP_SHOWWINDOW: u32 = 0x0040;

#[cfg(target_os = "windows")]
pub fn set_hwnd_capture_excluded(hwnd: isize, excluded: bool) -> Result<(), String> {
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
#[tauri::command]
pub fn set_window_capture_excluded(
    app: tauri::AppHandle,
    label: String,
    excluded: bool,
) -> Result<(), String> {
    set_webview_capture_excluded(&app, &label, excluded)
}
pub fn disable_windows_transition<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
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
pub fn activate_webview_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as isize;
        unsafe {
            let foreground = win32::GetForegroundWindow();
            let current_thread = win32::GetCurrentThreadId();
            let foreground_thread = if foreground != 0 {
                win32::GetWindowThreadProcessId(foreground, std::ptr::null_mut())
            } else {
                0
            };
            let attached = foreground_thread != 0
                && foreground_thread != current_thread
                && win32::AttachThreadInput(current_thread, foreground_thread, 1) != 0;
            let _ = win32::ShowWindow(hwnd, SW_SHOW);
            let _ = win32::SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = win32::BringWindowToTop(hwnd);
            let _ = win32::SetForegroundWindow(hwnd);
            let _ = win32::SetActiveWindow(hwnd);
            let _ = win32::SetFocus(hwnd);
            if attached {
                let _ = win32::AttachThreadInput(current_thread, foreground_thread, 0);
            }
        }
    }
    let _ = window.set_focus();
}
pub fn close_screenshot_windows(app: &tauri::AppHandle, include_primary: bool) {
    println!("[screenshot-trace] close_screenshot_windows called, include_primary={}", include_primary);
    for (label, window) in app.webview_windows() {
        if label == "screenshot" && include_primary {
            let is_visible = window.is_visible().unwrap_or(false);
            println!("[screenshot-trace] close_screenshot_windows: hiding screenshot window (visible={})", is_visible);
            let _ = window.set_always_on_top(false);
            robust_hide_window(&window);
        } else if label.starts_with("screenshot_") {
            let is_visible = window.is_visible().unwrap_or(false);
            println!("[screenshot-trace] close_screenshot_windows: hiding and closing secondary screenshot window {}, visible={}", label, is_visible);
            let _ = window.set_always_on_top(false);
            robust_hide_window(&window);
            let win_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = win_clone.close();
            });
        }
    }
}
#[tauri::command]
pub async fn overlay_ready_to_show(app: tauri::AppHandle, label: Option<String>) -> Result<(), String> {
    let target_label = label.unwrap_or_else(|| "screenshot".to_string());
    if target_label != "screenshot" && !target_label.starts_with("screenshot_") {
        return Ok(());
    }
    if let Some(screenshot_win) = app.get_webview_window(&target_label) {
        activate_webview_window(&screenshot_win);
        tokio::time::sleep(Duration::from_millis(35)).await;
        activate_webview_window(&screenshot_win);
    }
    Ok(())
}
#[tauri::command]
pub async fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        println!("[window-trace] source=command-hide_main_window action=hide-main label=main");
        robust_hide_window(&main);
    }
    Ok(())
}
#[tauri::command]
pub async fn force_close_recording_controls(app: tauri::AppHandle, source: Option<String>) -> Result<(), String> {
    let source_str = source.unwrap_or_else(|| "unknown".to_string());
    println!("[window-trace] source=command-force_close_recording_controls caller={} action=start", source_str);
    
    // 1. force_close_recording_controls 执行前诊断
    dump_all_windows_state_internal(&app, format!("force_close_recording_controls-before({})", source_str));

    if let Some(main) = app.get_webview_window("main") {
        println!("[window-trace] source=command-force_close_recording_controls action=hide-main label=main");
        robust_hide_window(&main);
    }
    hide_recording_overlay_internal();
    let mut recording_windows: Vec<tauri::WebviewWindow> = Vec::new();
    let all_labels: Vec<String> = app.webview_windows().keys().cloned().collect();
    println!("[window-trace] app.webview_windows() labels: {:?}", all_labels);
    for (label, window) in app.webview_windows() {
        let is_visible = window.is_visible().unwrap_or(false);
        let title = window.title().unwrap_or_default();
        println!("[window-trace] window state label={}, title='{}', visible={}", label, title, is_visible);
        if label == "recording_notice" || label.starts_with("recording_control") {
            println!("[window-trace] match recording window label={}", label);
            recording_windows.push(window);
        }
    }
    for window in recording_windows {
        println!("[window-trace] action=close label={}", window.label());
        let _ = window.set_always_on_top(false);
        robust_hide_window(&window);
        // Delay closing to prevent the transparent window from flashing white on Windows
        let win_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let _ = win_clone.close();
        });
    }

    // 2. force_close_recording_controls 执行后 100ms 诊断
    // 录制控制条等被拥有的窗口在 close() 时，Windows 会把焦点/激活回交给 owner（main），
    // 可能导致 main 在关闭后被重新激活并显示出白色窗口。
    // 由于关闭是延迟 50ms 执行的，这里在窗口确实销毁之后再补一次隐藏 main，消除残留白窗。
    let app_clone = app.clone();
    let source_str_clone = source_str.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(main) = app_clone.get_webview_window("main") {
            println!("[window-trace] source=command-force_close_recording_controls action=hide-main-after-close label=main");
            robust_hide_window(&main);
        }
        dump_all_windows_state_internal(&app_clone, format!("force_close_recording_controls-after-100ms({})", source_str_clone));
    });

    Ok(())
}

#[cfg(target_os = "windows")]
struct NativeWindowDumpContext {
    current_pid: u32,
    windows: Vec<serde_json::Value>,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_dump_callback(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut NativeWindowDumpContext);
    let mut pid: u32 = 0;
    win32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
    if pid == ctx.current_pid {
        let is_visible = win32::IsWindowVisible(hwnd) != 0;
        let title = window_title(hwnd);
        let class_name = window_class_name(hwnd);
        let mut rect = win32::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let has_rect = win32::GetWindowRect(hwnd, &mut rect) != 0;
        
        ctx.windows.push(serde_json::json!({
            "hwnd": hwnd,
            "title": title,
            "class_name": class_name,
            "visible": is_visible,
            "rect": if has_rect {
                Some(serde_json::json!({
                    "left": rect.left,
                    "top": rect.top,
                    "right": rect.right,
                    "bottom": rect.bottom,
                }))
            } else {
                None
            }
        }));
    }
    1
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
fn window_class_name(hwnd: isize) -> String {
    let mut buffer = vec![0u16; 256];
    let copied = unsafe { win32::GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..copied as usize])
        .trim()
        .to_string()
}

pub fn dump_all_windows_state_internal(app: &tauri::AppHandle, source: String) -> serde_json::Value {
    let mut tauri_windows = Vec::new();
    
    println!("[window-trace-dump] source={} --- start ---", source);
    for (label, window) in app.webview_windows() {
        let is_visible = window.is_visible().unwrap_or(false);
        let is_focused = window.is_focused().unwrap_or(false);
        let is_minimized = window.is_minimized().unwrap_or(false);
        let title = window.title().unwrap_or_default();
        let pos = window.outer_position().ok();
        let size = window.outer_size().ok();
        let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        
        println!(
            "[window-trace-dump] TauriWindow label={} title='{}' visible={} focused={} minimized={} pos={:?} size={:?} hwnd={}",
            label, title, is_visible, is_focused, is_minimized, pos, size, hwnd
        );
        
        tauri_windows.push(serde_json::json!({
            "label": label,
            "title": title,
            "visible": is_visible,
            "focused": is_focused,
            "minimized": is_minimized,
            "outer_position": pos.map(|p| (p.x, p.y)),
            "outer_size": size.map(|s| (s.width, s.height)),
            "hwnd": hwnd,
        }));
    }

    #[cfg(target_os = "windows")]
    let native_windows = {
        let current_pid = std::process::id();
        let mut ctx = NativeWindowDumpContext {
            current_pid,
            windows: Vec::new(),
        };
        unsafe {
            win32::EnumWindows(
                Some(enum_windows_dump_callback),
                &mut ctx as *mut NativeWindowDumpContext as isize,
            );
        }
        for win in &ctx.windows {
            println!(
                "[window-trace-dump] NativeWindow hwnd={} title='{}' class_name='{}' visible={} rect={:?}",
                win["hwnd"], win["title"], win["class_name"], win["visible"], win["rect"]
            );
        }
        ctx.windows
    };
    #[cfg(not(target_os = "windows"))]
    let native_windows = Vec::<serde_json::Value>::new();
    println!("[window-trace-dump] source={} --- end ---", source);

    serde_json::json!({
        "source": source,
        "tauri_windows": tauri_windows,
        "native_windows": native_windows,
    })
}

#[tauri::command]
pub async fn dump_all_windows_state(app: tauri::AppHandle, source: String) -> Result<serde_json::Value, String> {
    Ok(dump_all_windows_state_internal(&app, source))
}



pub fn set_webview_capture_excluded(
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

/// Hide all non-overlay application windows (main, screenshot, screenshot_*) to prevent
/// Windows OS focus-fallback from activating any of them when a recording overlay closes.
pub fn hide_all_app_windows(app: &tauri::AppHandle, trigger: &str) {
    for (lbl, win) in app.webview_windows() {
        // 只 hide main 和 screenshot 系列窗口，不 hide 正在关闭中的 recording_control 自身
        if lbl == "main" || lbl == "screenshot" || lbl.starts_with("screenshot_") {
            let is_visible = win.is_visible().unwrap_or(false);
            if is_visible {
                println!("[window-trace] source=hide_all_app_windows action=hide label={} trigger={}", lbl, trigger);
                let _ = win.hide();
            }
        }
    }
}


pub fn robust_hide_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.hide();
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            win32::ShowWindow(hwnd.0 as isize, 0); // SW_HIDE
        }
    }
}


pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let label = window.label();
    if label == "screenshot" {
        match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.set_always_on_top(false);
                let _ = window.hide();
                #[cfg(target_os = "windows")]
                if let Ok(hwnd) = window.hwnd() {
                    unsafe { crate::win32::ShowWindow(hwnd.0 as isize, 0); }
                }
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                api.prevent_close();
            }
            tauri::WindowEvent::Destroyed => {
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
    } else if label.starts_with("screenshot_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    } else if label == "recording_border" || label.starts_with("recording_border_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            let _ = window.set_always_on_top(false);
        }
    } else if label == "recording_notice" || label.starts_with("recording_control") {
        match &event {
            tauri::WindowEvent::CloseRequested { .. } => {
                println!("[window-trace] source=window-event action=event label={} event=CloseRequested", label);
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
            }
            tauri::WindowEvent::Destroyed => {
                println!("[window-trace] source=window-event action=destroyed label={} event=Destroyed", label);
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
                let app_handle2 = app_handle.clone();
                let label_owned = label.to_string();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    hide_all_app_windows(&app_handle2, &label_owned);
                    dump_all_windows_state_internal(&app_handle2, format!("recording_control-Destroyed-after-100ms({})", label_owned));
                });
            }
            _ => {}
        }
    } else if label == "main" {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let _ = window.hide();
            #[cfg(target_os = "windows")]
            if let Ok(hwnd) = window.hwnd() {
                unsafe { crate::win32::ShowWindow(hwnd.0 as isize, 0); }
            }
            api.prevent_close();
        }
    }
}

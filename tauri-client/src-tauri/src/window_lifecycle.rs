use crate::recording_overlay::*;
#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use crate::window_targets::window_title;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::Manager;

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
const SWP_NOACTIVATE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const SWP_SHOWWINDOW: u32 = 0x0040;
#[cfg(target_os = "windows")]
const SWP_HIDEWINDOW: u32 = 0x0080;
#[cfg(target_os = "windows")]
const HWND_NOTOPMOST: isize = -2;
#[cfg(target_os = "windows")]
const HIDDEN_MAIN_PARK_X: i32 = -32000;
#[cfg(target_os = "windows")]
const HIDDEN_MAIN_PARK_Y: i32 = -32000;

#[derive(Debug, Clone, Copy)]
struct MainWindowScreenshotState {
    was_visible: bool,
    was_minimized: bool,
}

static MAIN_WINDOW_SCREENSHOT_STATE: OnceLock<Mutex<Option<MainWindowScreenshotState>>> =
    OnceLock::new();
static HIDDEN_MAIN_WINDOW_POSITION: OnceLock<Mutex<Option<tauri::PhysicalPosition<i32>>>> =
    OnceLock::new();

fn get_main_window_screenshot_state() -> &'static Mutex<Option<MainWindowScreenshotState>> {
    MAIN_WINDOW_SCREENSHOT_STATE.get_or_init(|| Mutex::new(None))
}

fn get_hidden_main_window_position() -> &'static Mutex<Option<tauri::PhysicalPosition<i32>>> {
    HIDDEN_MAIN_WINDOW_POSITION.get_or_init(|| Mutex::new(None))
}

fn peek_main_window_screenshot_state() -> Option<MainWindowScreenshotState> {
    get_main_window_screenshot_state()
        .lock()
        .ok()
        .and_then(|guard| *guard)
}

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

fn show_screenshot_overlay_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.set_skip_taskbar(true);
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as isize;
        unsafe {
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
        }
        return;
    }
    let _ = window.set_focus();
}
pub fn activate_webview_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    if window.label() == "main" {
        restore_parked_main_window_position(window, "activate-webview-window");
    }
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
    println!(
        "[screenshot-trace] close_screenshot_windows called, include_primary={}",
        include_primary
    );
    for (label, window) in app.webview_windows() {
        if label == "screenshot" && include_primary {
            let is_visible = window.is_visible().unwrap_or(false);
            println!("[screenshot-trace] close_screenshot_windows: hiding screenshot window (visible={})", is_visible);
            let _ = window.set_always_on_top(false);
            prepare_focus_for_screenshot_overlay_close(app, "close-primary-screenshot");
            hide_window_without_activation(&window);
        } else if label.starts_with("screenshot_") {
            let is_visible = window.is_visible().unwrap_or(false);
            println!("[screenshot-trace] close_screenshot_windows: hiding and closing secondary screenshot window {}, visible={}", label, is_visible);
            let _ = window.set_always_on_top(false);
            prepare_focus_for_screenshot_overlay_close(app, "close-secondary-screenshot");
            hide_window_without_activation(&window);
            let win_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = win_clone.close();
            });
        }
    }
}

pub fn prepare_main_window_for_screenshot(app: &tauri::AppHandle) -> bool {
    if get_main_window_screenshot_state()
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
    {
        return false;
    }

    let Some(main) = app.get_webview_window("main") else {
        return false;
    };
    let was_visible = main.is_visible().unwrap_or(false);
    let was_minimized = main.is_minimized().unwrap_or(false);
    if let Ok(mut guard) = get_main_window_screenshot_state().lock() {
        *guard = Some(MainWindowScreenshotState {
            was_visible,
            was_minimized,
        });
    }

    if was_visible && !was_minimized {
        println!("[window-trace] source=prepare-main-for-screenshot action=hide-main label=main");
        disable_windows_transition(&main);
        robust_hide_window(&main);
        true
    } else if !was_visible && !was_minimized {
        println!(
            "[window-trace] source=prepare-main-for-screenshot action=park-hidden-main label=main"
        );
        park_hidden_main_window_for_screenshot(&main, "prepare-main-for-screenshot");
        true
    } else {
        false
    }
}

pub async fn wait_for_hidden_main_capture_settle() {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let _ = win32::DwmFlush();
        }
        tokio::time::sleep(Duration::from_millis(16)).await;
        unsafe {
            let _ = win32::DwmFlush();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
}

pub fn restore_main_window_after_screenshot(app: &tauri::AppHandle, reason: &str) {
    let state = get_main_window_screenshot_state()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    let Some(state) = state else {
        return;
    };
    if !state.was_visible {
        println!(
            "[window-trace] source=restore-main-after-screenshot action=keep-hidden reason={} was_visible={} was_minimized={}",
            reason, state.was_visible, state.was_minimized
        );
        keep_main_window_hidden_after_screenshot(app, reason);
        return;
    }
    if state.was_minimized {
        println!(
            "[window-trace] source=restore-main-after-screenshot action=keep-minimized reason={} was_visible={} was_minimized={}",
            reason, state.was_visible, state.was_minimized
        );
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.minimize();
        }
        return;
    }
    if let Some(main) = app.get_webview_window("main") {
        println!(
            "[window-trace] source=restore-main-after-screenshot action=show-main label=main reason={}",
            reason
        );
        restore_parked_main_window_position(&main, "restore-main-after-screenshot");
        let _ = main.show();
        let _ = main.unminimize();
    }
}

pub fn restore_parked_main_window_position<W: tauri::Runtime>(
    window: &tauri::WebviewWindow<W>,
    reason: &str,
) {
    let parked_position = get_hidden_main_window_position()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    let Some(position) = parked_position else {
        return;
    };
    println!(
        "[window-trace] source=restore-parked-main-position action=restore label=main reason={} x={} y={}",
        reason, position.x, position.y
    );
    let _ = window.set_position(position);
}

fn keep_main_window_hidden_after_screenshot(app: &tauri::AppHandle, reason: &str) {
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    println!(
        "[window-trace] source=restore-main-after-screenshot action=hide-main label=main reason={}",
        reason
    );
    robust_hide_window(&main);

    let main_clone = main.clone();
    let reason_owned = reason.to_string();
    tauri::async_runtime::spawn(async move {
        for delay_ms in [120_u64, 280_u64] {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            println!(
                "[window-trace] source=restore-main-after-screenshot action=hide-main-delayed label=main reason={} delay_ms={}",
                reason_owned, delay_ms
            );
            robust_hide_window(&main_clone);
        }
    });
}

#[cfg(target_os = "windows")]
struct ForegroundHandoffContext {
    current_pid: u32,
    candidate: isize,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_foreground_handoff_candidate(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut ForegroundHandoffContext);
    if hwnd == 0 || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }

    let mut pid: u32 = 0;
    win32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
    if pid == ctx.current_pid {
        return 1;
    }

    let mut rect = win32::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if win32::GetWindowRect(hwnd, &mut rect) == 0 {
        return 1;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width < 120 || height < 80 {
        return 1;
    }

    if window_title(hwnd).trim().is_empty() {
        return 1;
    }

    ctx.candidate = hwnd;
    0
}

#[cfg(target_os = "windows")]
fn find_foreground_handoff_target() -> isize {
    let mut ctx = ForegroundHandoffContext {
        current_pid: std::process::id(),
        candidate: 0,
    };
    unsafe {
        win32::EnumWindows(
            Some(enum_foreground_handoff_candidate),
            &mut ctx as *mut ForegroundHandoffContext as isize,
        );
        if ctx.candidate != 0 {
            return ctx.candidate;
        }
        win32::GetShellWindow()
    }
}

#[cfg(target_os = "windows")]
fn hide_hwnd_without_activation(hwnd: isize) {
    unsafe {
        let _ = win32::SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW,
        );
        let _ = win32::InvalidateRect(0, std::ptr::null(), 1);
        let _ = win32::DwmFlush();
    }
}

#[cfg(target_os = "windows")]
fn park_hwnd_offscreen_without_activation(hwnd: isize) {
    unsafe {
        let _ = win32::SetWindowPos(
            hwnd,
            HWND_NOTOPMOST,
            HIDDEN_MAIN_PARK_X,
            HIDDEN_MAIN_PARK_Y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW,
        );
        let _ = win32::InvalidateRect(0, std::ptr::null(), 1);
        let _ = win32::DwmFlush();
    }
}

pub fn hide_window_without_activation<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.set_always_on_top(false);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        hide_hwnd_without_activation(hwnd.0 as isize);
        return;
    }
    let _ = window.hide();
}

fn hide_tauri_window_without_activation(window: &tauri::Window) {
    let _ = window.set_always_on_top(false);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        hide_hwnd_without_activation(hwnd.0 as isize);
        return;
    }
    let _ = window.hide();
}

fn park_hidden_main_window_for_screenshot<W: tauri::Runtime>(
    window: &tauri::WebviewWindow<W>,
    reason: &str,
) {
    if let Ok(position) = window.outer_position() {
        if position.x > -30000 || position.y > -30000 {
            if let Ok(mut guard) = get_hidden_main_window_position().lock() {
                if guard.is_none() {
                    println!(
                        "[window-trace] source=park-hidden-main action=save-position label=main reason={} x={} y={}",
                        reason, position.x, position.y
                    );
                    *guard = Some(position);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        println!(
            "[window-trace] source=park-hidden-main action=park-offscreen label=main reason={}",
            reason
        );
        park_hwnd_offscreen_without_activation(hwnd.0 as isize);
        return;
    }

    let _ = window.set_position(tauri::PhysicalPosition::new(-32000, -32000));
    let _ = window.hide();
}

pub fn prepare_focus_for_screenshot_overlay_close(app: &tauri::AppHandle, reason: &str) {
    let Some(state) = peek_main_window_screenshot_state() else {
        return;
    };
    if state.was_visible {
        return;
    }

    if let Some(main) = app.get_webview_window("main") {
        println!(
            "[window-trace] source=prepare-screenshot-close-focus action=park-main-before-overlay-close label=main reason={}",
            reason
        );
        park_hidden_main_window_for_screenshot(&main, reason);
    }

    #[cfg(target_os = "windows")]
    unsafe {
        let target = find_foreground_handoff_target();
        if target != 0 {
            let ok = win32::SetForegroundWindow(target);
            println!(
                "[window-trace] source=prepare-screenshot-close-focus action=set-foreground target={} ok={} reason={}",
                target, ok, reason
            );
        } else {
            println!(
                "[window-trace] source=prepare-screenshot-close-focus action=no-foreground-target reason={}",
                reason
            );
        }
        let _ = win32::SetActiveWindow(0);
        let _ = win32::SetFocus(0);
        let _ = win32::DwmFlush();
    }
}

#[tauri::command]
pub async fn overlay_ready_to_show(
    app: tauri::AppHandle,
    label: Option<String>,
) -> Result<(), String> {
    let target_label = label.unwrap_or_else(|| "screenshot".to_string());
    if target_label != "screenshot" && !target_label.starts_with("screenshot_") {
        return Ok(());
    }
    let Some(screenshot_win) = app.get_webview_window(&target_label) else {
        return Err(format!(
            "Screenshot overlay window not found: {}",
            target_label
        ));
    };
    show_screenshot_overlay_window(&screenshot_win);
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

fn should_hide_main_for_recording_cleanup(source: &str) -> bool {
    !matches!(
        source,
        "clearRecordingState-idle" | "screenshot-idle-cleanup"
    )
}

#[tauri::command]
pub async fn force_close_recording_controls(
    app: tauri::AppHandle,
    source: Option<String>,
    hide_main: Option<bool>,
) -> Result<(), String> {
    let source_str = source.unwrap_or_else(|| "unknown".to_string());
    let hide_main_for_cleanup =
        hide_main.unwrap_or_else(|| should_hide_main_for_recording_cleanup(&source_str));
    println!(
        "[window-trace] source=command-force_close_recording_controls caller={} action=start hide_main={}",
        source_str, hide_main_for_cleanup
    );

    // 1. force_close_recording_controls 鎵ц鍓嶈瘖鏂?
    dump_all_windows_state_internal(
        &app,
        format!("force_close_recording_controls-before({})", source_str),
    );

    if hide_main_for_cleanup {
        if let Some(main) = app.get_webview_window("main") {
            println!("[window-trace] source=command-force_close_recording_controls action=hide-main label=main");
            robust_hide_window(&main);
        }
    } else {
        println!(
            "[window-trace] source=command-force_close_recording_controls action=preserve-main caller={}",
            source_str
        );
    }
    hide_recording_overlay_internal();
    let mut recording_windows: Vec<tauri::WebviewWindow> = Vec::new();
    let all_labels: Vec<String> = app.webview_windows().keys().cloned().collect();
    println!(
        "[window-trace] app.webview_windows() labels: {:?}",
        all_labels
    );
    for (label, window) in app.webview_windows() {
        let is_visible = window.is_visible().unwrap_or(false);
        let title = window.title().unwrap_or_default();
        println!(
            "[window-trace] window state label={}, title='{}', visible={}",
            label, title, is_visible
        );
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

    // 2. force_close_recording_controls 鎵ц鍚?100ms 璇婃柇
    // 褰曞埗鎺у埗鏉＄瓑琚嫢鏈夌殑绐楀彛鍦?close() 鏃讹紝Windows 浼氭妸鐒︾偣/婵€娲诲洖浜ょ粰 owner锛坢ain锛夛紝
    // 鍙兘瀵艰嚧 main 鍦ㄥ叧闂悗琚噸鏂版縺娲诲苟鏄剧ず鍑虹櫧鑹茬獥鍙ｃ€?
    // 鐢变簬鍏抽棴鏄欢杩?50ms 鎵ц鐨勶紝杩欓噷鍦ㄧ獥鍙ｇ‘瀹為攢姣佷箣鍚庡啀琛ヤ竴娆￠殣钘?main锛屾秷闄ゆ畫鐣欑櫧绐椼€?
    let app_clone = app.clone();
    let source_str_clone = source_str.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if hide_main_for_cleanup {
            if let Some(main) = app_clone.get_webview_window("main") {
                println!("[window-trace] source=command-force_close_recording_controls action=hide-main-after-close label=main");
                robust_hide_window(&main);
            }
        } else {
            println!(
                "[window-trace] source=command-force_close_recording_controls action=preserve-main-after-close caller={}",
                source_str_clone
            );
        }
        dump_all_windows_state_internal(
            &app_clone,
            format!(
                "force_close_recording_controls-after-100ms({})",
                source_str_clone
            ),
        );
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
        let mut rect = win32::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
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

pub fn dump_all_windows_state_internal(
    app: &tauri::AppHandle,
    source: String,
) -> serde_json::Value {
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
pub async fn dump_all_windows_state(
    app: tauri::AppHandle,
    source: String,
) -> Result<serde_json::Value, String> {
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
        // 鍙?hide main 鍜?screenshot 绯诲垪绐楀彛锛屼笉 hide 姝ｅ湪鍏抽棴涓殑 recording_control 鑷韩
        if lbl == "main" || lbl == "screenshot" || lbl.starts_with("screenshot_") {
            let is_visible = win.is_visible().unwrap_or(false);
            if is_visible {
                println!(
                    "[window-trace] source=hide_all_app_windows action=hide label={} trigger={}",
                    lbl, trigger
                );
                robust_hide_window(&win);
            }
        }
    }
}

pub fn robust_hide_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.hide();
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let hwnd = hwnd.0 as isize;
            win32::ShowWindow(hwnd, 0); // SW_HIDE
            let _ = win32::SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW,
            );
            let _ = win32::InvalidateRect(0, std::ptr::null(), 1);
            let _ = win32::DwmFlush();
        }
    }
}

pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let label = window.label();
    if label == "screenshot" {
        match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                prepare_focus_for_screenshot_overlay_close(
                    window.app_handle(),
                    "screenshot-close-requested",
                );
                hide_tauri_window_without_activation(window);
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                restore_main_window_after_screenshot(
                    window.app_handle(),
                    "screenshot-close-requested",
                );
                api.prevent_close();
            }
            tauri::WindowEvent::Destroyed => {
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                restore_main_window_after_screenshot(window.app_handle(), "screenshot-destroyed");
            }
            _ => {}
        }
    } else if label.starts_with("screenshot_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
            restore_main_window_after_screenshot(
                window.app_handle(),
                "secondary-screenshot-closed",
            );
        }
    } else if label == "recording_border" || label.starts_with("recording_border_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            let _ = window.set_always_on_top(false);
        }
    } else if label == "recording_notice" || label.starts_with("recording_control") {
        match &event {
            tauri::WindowEvent::CloseRequested { .. } => {
                println!(
                    "[window-trace] source=window-event action=event label={} event=CloseRequested",
                    label
                );
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
            }
            tauri::WindowEvent::Destroyed => {
                println!(
                    "[window-trace] source=window-event action=destroyed label={} event=Destroyed",
                    label
                );
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                let app_handle = window.app_handle().clone();
                hide_all_app_windows(&app_handle, label);
                let app_handle2 = app_handle.clone();
                let label_owned = label.to_string();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    hide_all_app_windows(&app_handle2, &label_owned);
                    dump_all_windows_state_internal(
                        &app_handle2,
                        format!("recording_control-Destroyed-after-100ms({})", label_owned),
                    );
                });
            }
            _ => {}
        }
    } else if label == "main" {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let _ = window.hide();
            #[cfg(target_os = "windows")]
            if let Ok(hwnd) = window.hwnd() {
                unsafe {
                    crate::win32::ShowWindow(hwnd.0 as isize, 0);
                }
            }
            api.prevent_close();
        }
    }
}

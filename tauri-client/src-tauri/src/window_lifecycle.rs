use crate::recording_overlay::*;
#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use crate::window_targets::window_title;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::Manager;

#[cfg(target_os = "windows")]
const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
#[cfg(target_os = "windows")]
const SW_SHOW: i32 = 5;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const HWND_TOPMOST: isize = -1;
#[cfg(target_os = "windows")]
const SWP_NOSIZE: u32 = 0x0001;
#[cfg(target_os = "windows")]
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const SWP_FRAMECHANGED: u32 = 0x0020;
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
#[cfg(target_os = "windows")]
const GWL_EXSTYLE: i32 = -20;
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: isize = 0x00000080;
#[cfg(target_os = "windows")]
const WS_EX_APPWINDOW: isize = 0x00040000;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: isize = 0x08000000;

#[derive(Debug, Clone, Copy)]
struct MainWindowScreenshotState {
    was_visible: bool,
    was_minimized: bool,
}

static MAIN_WINDOW_SCREENSHOT_STATE: OnceLock<Mutex<Option<MainWindowScreenshotState>>> =
    OnceLock::new();
static HIDDEN_MAIN_WINDOW_POSITION: OnceLock<Mutex<Option<tauri::PhysicalPosition<i32>>>> =
    OnceLock::new();
#[cfg(target_os = "windows")]
static PRE_SCREENSHOT_FOREGROUND_HWND: OnceLock<Mutex<Option<isize>>> = OnceLock::new();
static SUPPRESS_NEXT_SCREENSHOT_RESTORE: AtomicBool = AtomicBool::new(false);

fn get_main_window_screenshot_state() -> &'static Mutex<Option<MainWindowScreenshotState>> {
    MAIN_WINDOW_SCREENSHOT_STATE.get_or_init(|| Mutex::new(None))
}

pub fn suppress_next_screenshot_restore() {
    SUPPRESS_NEXT_SCREENSHOT_RESTORE.store(true, Ordering::SeqCst);
}

fn should_suppress_screenshot_restore() -> bool {
    SUPPRESS_NEXT_SCREENSHOT_RESTORE.swap(false, Ordering::SeqCst)
}

fn get_hidden_main_window_position() -> &'static Mutex<Option<tauri::PhysicalPosition<i32>>> {
    HIDDEN_MAIN_WINDOW_POSITION.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn get_pre_screenshot_foreground_hwnd() -> &'static Mutex<Option<isize>> {
    PRE_SCREENSHOT_FOREGROUND_HWND.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn is_external_visible_window(hwnd: isize) -> bool {
    if hwnd == 0 || unsafe { win32::IsWindowVisible(hwnd) } == 0 {
        return false;
    }
    let mut pid: u32 = 0;
    unsafe {
        win32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
    }
    pid != 0 && pid != std::process::id()
}

#[cfg(target_os = "windows")]
pub fn remember_pre_screenshot_foreground(reason: &str) {
    let hwnd = unsafe { win32::GetForegroundWindow() };
    let target = if is_external_visible_window(hwnd) {
        Some(hwnd)
    } else {
        None
    };
    if let Ok(mut guard) = get_pre_screenshot_foreground_hwnd().lock() {
        *guard = target;
    }
    println!(
        "[window-trace] source=pre-screenshot-foreground action=remember target={} valid={} reason={}",
        hwnd,
        target.is_some(),
        reason
    );
}

#[cfg(not(target_os = "windows"))]
pub fn remember_pre_screenshot_foreground(_reason: &str) {}

#[cfg(target_os = "windows")]
fn set_pre_screenshot_foreground(reason: &str, consume: bool) -> bool {
    let target = get_pre_screenshot_foreground_hwnd()
        .lock()
        .ok()
        .and_then(|mut guard| if consume { guard.take() } else { *guard });
    let Some(target) = target else {
        return false;
    };
    if !is_external_visible_window(target) {
        println!(
            "[window-trace] source=pre-screenshot-foreground action=skip-invalid target={} reason={}",
            target, reason
        );
        return false;
    }
    let ok = unsafe { win32::SetForegroundWindow(target) };
    println!(
        "[window-trace] source=pre-screenshot-foreground action=set-foreground target={} ok={} consume={} reason={}",
        target, ok, consume, reason
    );
    ok != 0
}

#[cfg(target_os = "windows")]
fn clear_pre_screenshot_foreground() {
    if let Ok(mut guard) = get_pre_screenshot_foreground_hwnd().lock() {
        *guard = None;
    }
}

#[cfg(not(target_os = "windows"))]
fn clear_pre_screenshot_foreground() {}

fn peek_main_window_screenshot_state() -> Option<MainWindowScreenshotState> {
    get_main_window_screenshot_state()
        .lock()
        .ok()
        .and_then(|guard| *guard)
}

pub fn current_screenshot_capture_needs_settle() -> bool {
    peek_main_window_screenshot_state()
        .map(|state| state.was_visible && !state.was_minimized)
        .unwrap_or(false)
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

#[cfg(target_os = "windows")]
fn screenshot_overlay_ex_style(current_ex_style: isize, no_activate: bool) -> isize {
    let mut next = (current_ex_style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
    if no_activate {
        next |= WS_EX_NOACTIVATE;
    } else {
        next &= !WS_EX_NOACTIVATE;
    }
    next
}

pub fn apply_screenshot_overlay_window_styles<W: tauri::Runtime>(
    window: &tauri::WebviewWindow<W>,
    no_activate: bool,
) {
    let _ = window.set_skip_taskbar(true);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as isize;
        unsafe {
            let current = win32::GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if current == 0 {
                println!(
                    "[window-trace] source=screenshot-overlay-style action=skip-empty-style hwnd={}",
                    hwnd
                );
                return;
            }
            let next = screenshot_overlay_ex_style(current, no_activate);
            if next != current {
                let _ = win32::SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
            }
            let _ = win32::SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            println!(
                "[window-trace] source=screenshot-overlay-style action=apply hwnd={} no_activate={} appwindow_removed={} toolwindow={} noactivate_style={}",
                hwnd,
                no_activate,
                next & WS_EX_APPWINDOW == 0,
                next & WS_EX_TOOLWINDOW != 0,
                next & WS_EX_NOACTIVATE != 0
            );
        }
    }
}

pub fn show_screenshot_overlay_window<W: tauri::Runtime>(window: &tauri::WebviewWindow<W>) {
    let _ = window.set_skip_taskbar(true);
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as isize;
        if !screenshot_focus_on_ready_enabled() {
            apply_screenshot_overlay_window_styles(window, true);
            let _ = window.set_always_on_top(true);
            unsafe {
                let _ = win32::ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = win32::SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                );
                let _ = win32::DwmFlush();
            }
            println!(
                "[window-trace] source=show-screenshot-overlay action=show-noactivate hwnd={}",
                hwnd
            );
            return;
        }
        apply_screenshot_overlay_window_styles(window, false);
        let _ = window.show();
        let _ = window.set_always_on_top(true);
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
        println!(
            "[window-trace] source=show-screenshot-overlay action=show-activate hwnd={} reason=YSN_SCREENSHOT_FOCUS_ON_READY",
            hwnd
        );
        return;
    }
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
}

pub fn activate_screenshot_overlay_window_for_interaction<W: tauri::Runtime>(
    window: &tauri::WebviewWindow<W>,
) {
    apply_screenshot_overlay_window_styles(window, false);
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
        println!(
            "[window-trace] source=screenshot-overlay-interaction action=activate hwnd={}",
            hwnd
        );
    }
    let _ = window.set_focus();
}

#[tauri::command]
pub async fn activate_screenshot_overlay_for_interaction(
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
    activate_screenshot_overlay_window_for_interaction(&screenshot_win);
    Ok(())
}

fn screenshot_focus_on_ready_enabled() -> bool {
    std::env::var("YSN_SCREENSHOT_FOCUS_ON_READY")
        .ok()
        .as_deref()
        == Some("1")
}

#[cfg(test)]
mod screenshot_overlay_show_policy_tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn focus_on_ready_is_disabled_unless_explicitly_enabled() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("YSN_SCREENSHOT_FOCUS_ON_READY");
        assert!(!screenshot_focus_on_ready_enabled());

        std::env::set_var("YSN_SCREENSHOT_FOCUS_ON_READY", "0");
        assert!(!screenshot_focus_on_ready_enabled());

        std::env::set_var("YSN_SCREENSHOT_FOCUS_ON_READY", "1");
        assert!(screenshot_focus_on_ready_enabled());
        std::env::remove_var("YSN_SCREENSHOT_FOCUS_ON_READY");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn screenshot_overlay_style_hides_taskbar_and_avoids_activation() {
        let current = WS_EX_APPWINDOW;

        let next = screenshot_overlay_ex_style(current, true);

        assert_eq!(next & WS_EX_APPWINDOW, 0);
        assert_ne!(next & WS_EX_TOOLWINDOW, 0);
        assert_ne!(next & WS_EX_NOACTIVATE, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn screenshot_overlay_focus_rollback_still_hides_taskbar() {
        let current = WS_EX_APPWINDOW | WS_EX_NOACTIVATE;

        let next = screenshot_overlay_ex_style(current, false);

        assert_eq!(next & WS_EX_APPWINDOW, 0);
        assert_ne!(next & WS_EX_TOOLWINDOW, 0);
        assert_eq!(next & WS_EX_NOACTIVATE, 0);
    }
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
        false
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
        clear_pre_screenshot_foreground();
        return;
    };
    if !state.was_visible {
        println!(
            "[window-trace] source=restore-main-after-screenshot action=keep-hidden reason={} was_visible={} was_minimized={}",
            reason, state.was_visible, state.was_minimized
        );
        keep_main_window_hidden_after_screenshot(app, reason);
        #[cfg(target_os = "windows")]
        let _ = set_pre_screenshot_foreground(reason, true);
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
        clear_pre_screenshot_foreground();
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
    clear_pre_screenshot_foreground();
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

    if !state.was_visible {
        if let Some(main) = app.get_webview_window("main") {
            println!(
                "[window-trace] source=prepare-screenshot-close-focus action=park-main-before-overlay-close label=main reason={}",
                reason
            );
            park_hidden_main_window_for_screenshot(&main, reason);
        }
    }

    #[cfg(target_os = "windows")]
    unsafe {
        if set_pre_screenshot_foreground(reason, false) {
            println!(
                "[window-trace] source=prepare-screenshot-close-focus action=set-remembered-foreground reason={}",
                reason
            );
        } else {
            let target = if state.was_visible {
                0
            } else {
                find_foreground_handoff_target()
            };
            if target != 0 {
                let ok = win32::SetForegroundWindow(target);
                println!(
                    "[window-trace] source=prepare-screenshot-close-focus action=set-foreground target={} ok={} reason={}",
                    target, ok, reason
                );
            } else {
                println!(
                    "[window-trace] source=prepare-screenshot-close-focus action=no-foreground-target reason={} was_visible={}",
                    reason, state.was_visible
                );
            }
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
    session_id: Option<String>,
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
    if let Some(session_id) = session_id.as_deref() {
        if crate::screenshot_commands::is_screenshot_session_cancelled(session_id) {
            println!(
                "[screenshot-baseline] session={} phase=overlay_show_skipped_cancelled elapsed_ms=0 label={}",
                session_id, target_label
            );
            return Ok(());
        }
    }
    let started_at = std::time::Instant::now();
    show_screenshot_overlay_window(&screenshot_win);
    let session_raise =
        crate::screenshot_native::raise_cpu_native_overlay_session("webview-overlay-ready-topmost");
    if session_raise.active || session_raise.visible {
        println!(
            "[screenshot-trace] native_first_frame_session_raised session={} state={} active={} visible={} hwnd={}",
            session_id.as_deref().unwrap_or("unknown"),
            session_raise.state.as_str(),
            session_raise.active,
            session_raise.visible,
            session_raise.hwnd.unwrap_or(0)
        );
    }
    schedule_native_first_frame_session_dismiss(session_id.clone());
    println!(
        "[screenshot-baseline] session={} phase=overlay_show_result elapsed_ms={} label={}",
        session_id.unwrap_or_else(|| "unknown".to_string()),
        started_at.elapsed().as_millis(),
        target_label
    );
    Ok(())
}

fn native_first_frame_session_dismiss_delay_ms() -> u64 {
    std::env::var("YSN_NATIVE_FIRST_FRAME_SESSION_DISMISS_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(16, 500))
        .unwrap_or(64)
}

fn schedule_native_first_frame_session_dismiss(session_id: Option<String>) {
    let delay_ms = native_first_frame_session_dismiss_delay_ms();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        let diagnostics = crate::screenshot_native::cancel_cpu_native_overlay_session_if_matches(
            session_id.as_deref(),
            "webview-overlay-ready",
        );
        let Some(diagnostics) = diagnostics else {
            return;
        };
        if diagnostics.active
            || !matches!(
                diagnostics.state,
                crate::screenshot_native::NativeOverlaySessionState::Empty
            )
        {
            println!(
                "[screenshot-trace] native_first_frame_session_dismissed session={} delay_ms={} state={} active={} visible={}",
                session_id.as_deref().unwrap_or("unknown"),
                delay_ms,
                diagnostics.state.as_str(),
                diagnostics.active,
                diagnostics.visible
            );
        }
    });
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

    // 1. force_close_recording_controls 閹笛嗩攽閸撳秷鐦栭弬?
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

    // 2. force_close_recording_controls 閹笛嗩攽閸?100ms 鐠囧﹥鏌?
    // 瑜版洖鍩楅幒褍鍩楅弶锛勭搼鐞氼偅瀚㈤張澶屾畱缁愭褰涢崷?close() 閺冭绱漌indows 娴兼碍濡搁悞锔惧仯/濠碘偓濞茶娲栨禍銈囩舶 owner閿涘潰ain閿涘绱?
    // 閸欘垵鍏樼€佃壈鍤?main 閸︺劌鍙ч梻顓炴倵鐞氼偊鍣搁弬鐗堢负濞茶鑻熼弰鍓с仛閸戣櫣娅ч懝鑼崶閸欙絻鈧?
    // 閻㈠彉绨崗鎶芥４閺勵垰娆㈡潻?50ms 閹笛嗩攽閻ㄥ嫸绱濇潻娆撳櫡閸︺劎鐛ラ崣锝団€樼€圭偤鏀㈠В浣风閸氬骸鍟€鐞涖儰绔村▎锟犳閽?main閿涘本绉烽梽銈嗙暙閻ｆ瑧娅х粣妞尖偓?
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
        // 閸?hide main 閸?screenshot 缁鍨粣妤€褰涢敍灞肩瑝 hide 濮濓絽婀崗鎶芥４娑擃厾娈?recording_control 閼奉亣闊?
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
                if !should_suppress_screenshot_restore() {
                    restore_main_window_after_screenshot(
                        window.app_handle(),
                        "screenshot-close-requested",
                    );
                }
                api.prevent_close();
            }
            tauri::WindowEvent::Destroyed => {
                crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
                if !should_suppress_screenshot_restore() {
                    restore_main_window_after_screenshot(
                        window.app_handle(),
                        "screenshot-destroyed",
                    );
                }
            }
            _ => {}
        }
    } else if label.starts_with("screenshot_") {
        if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
            crate::CAPTURING.store(false, std::sync::atomic::Ordering::SeqCst);
            if !should_suppress_screenshot_restore() {
                restore_main_window_after_screenshot(
                    window.app_handle(),
                    "secondary-screenshot-closed",
                );
            }
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

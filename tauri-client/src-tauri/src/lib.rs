pub mod app_paths;
pub use app_paths::*;

pub mod config_store;
pub use config_store::*;

pub mod history_commands;
pub use history_commands::*;

pub mod file_commands;

pub mod hotkeys;
pub use hotkeys::*;

pub mod screenshot_commands;
pub use screenshot_commands::*;

pub(crate) mod screenshot_diagnostics_json;
pub(crate) mod screenshot_dxgi_diagnostics_json;
pub(crate) mod screenshot_shared_buffer;
pub(crate) mod screenshot_win32_diagnostics_json;

pub mod screenshot_diagnostics_requests;

pub mod screenshot_wgc_diagnostic_commands;
pub mod screenshot_wgc_selected_output_diagnostic_commands;

pub mod screenshot_native;
pub use screenshot_native::*;

pub mod window_targets;
pub use window_targets::*;

pub mod window_lifecycle;
pub use window_lifecycle::*;

pub mod ffmpeg_dependency;

pub mod diagnostics;
pub use diagnostics::*;

pub mod rapid_ocr;
pub use rapid_ocr::*;

pub mod text_source;
pub use text_source::*;
pub mod recording_overlay;
pub mod recording_process;
pub use recording_process::*;
pub mod recording_commands;
pub use recording_overlay::*;

use arboard::{Clipboard, ImageData};
use base64::{prelude::BASE64_STANDARD, Engine};
use screenshots::Screen;
use std::borrow::Cow;
use std::fs;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
static CAPTURING: AtomicBool = AtomicBool::new(false);
static LAST_CAPTURE_SHORTCUT_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "windows")]
pub(crate) mod win32 {
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
    #[allow(non_snake_case)]
    pub struct MONITORINFO {
        pub cbSize: u32,
        pub rcMonitor: RECT,
        pub rcWork: RECT,
        pub dwFlags: u32,
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
        pub fn SetWindowPos(
            hWnd: isize,
            hWndInsertAfter: isize,
            X: i32,
            Y: i32,
            cx: i32,
            cy: i32,
            uFlags: u32,
        ) -> i32;
        pub fn BringWindowToTop(hWnd: isize) -> i32;
        pub fn SetForegroundWindow(hWnd: isize) -> i32;
        pub fn SetActiveWindow(hWnd: isize) -> isize;
        pub fn SetFocus(hWnd: isize) -> isize;
        pub fn SetCapture(hWnd: isize) -> isize;
        pub fn ReleaseCapture() -> i32;
        pub fn GetForegroundWindow() -> isize;
        pub fn GetShellWindow() -> isize;
        pub fn GetCurrentThreadId() -> u32;
        pub fn AttachThreadInput(idAttach: u32, idAttachTo: u32, fAttach: i32) -> i32;
        pub fn PostMessageW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> i32;
        pub fn PostQuitMessage(nExitCode: i32);
        pub fn GetMessageW(
            lpMsg: *mut MSG,
            hWnd: isize,
            wMsgFilterMin: u32,
            wMsgFilterMax: u32,
        ) -> i32;
        pub fn PeekMessageW(
            lpMsg: *mut MSG,
            hWnd: isize,
            wMsgFilterMin: u32,
            wMsgFilterMax: u32,
            wRemoveMsg: u32,
        ) -> i32;
        pub fn MsgWaitForMultipleObjectsEx(
            nCount: u32,
            pHandles: *const isize,
            dwMilliseconds: u32,
            dwWakeMask: u32,
            dwFlags: u32,
        ) -> u32;
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
        pub fn GetWindowLongPtrW(hWnd: isize, nIndex: i32) -> isize;
        pub fn SetWindowLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
        pub fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        pub fn GetWindowRect(hWnd: isize, lpRect: *mut RECT) -> i32;
        pub fn MonitorFromPoint(pt: POINT, dwFlags: u32) -> isize;
        pub fn GetMonitorInfoW(hMonitor: isize, lpmi: *mut MONITORINFO) -> i32;
        pub fn GetWindowTextLengthW(hWnd: isize) -> i32;
        pub fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
        pub fn GetClassNameW(hWnd: isize, lpClassName: *mut u16, nMaxCount: i32) -> i32;
        pub fn GetWindowThreadProcessId(hWnd: isize, lpdwProcessId: *mut u32) -> u32;
        pub fn GetAsyncKeyState(vKey: i32) -> i16;
        pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
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
        pub fn IsWindow(hWnd: isize) -> i32;
        pub fn IsWindowVisible(hWnd: isize) -> i32;
        pub fn IsIconic(hWnd: isize) -> i32;
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
        pub fn DwmFlush() -> i32;
    }
}

#[cfg(target_os = "windows")]
#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || ctx.excluded_hwnds.contains(&hwnd) || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if is_system_capture_window(hwnd) {
        return 1;
    }
    if hwnd_contains_cursor(hwnd, ctx.cursor_x, ctx.cursor_y, ctx.min_size) {
        ctx.matches.push(hwnd);
    }
    1
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_child_windows_for_cursor(hwnd: isize, lparam: isize) -> i32 {
    let ctx = &mut *(lparam as *mut WindowSearchContext);
    if hwnd == 0 || win32::IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    if hwnd_contains_cursor(hwnd, ctx.cursor_x, ctx.cursor_y, ctx.min_size) {
        ctx.matches.push(hwnd);
    }
    1
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

#[cfg(target_os = "windows")]
struct RecordingWindowListContext {
    excluded_hwnds: Vec<isize>,
    windows: Vec<serde_json::Value>,
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
fn get_default_recording_output_dir() -> Result<String, String> {
    let dir = default_recording_output_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create recording directory failed: {}", e))?;
    Ok(dir.to_string_lossy().to_string())
}

use serde::{Deserialize, Serialize};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    configure_webview2_default_background();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                println!("[window-trace] source=single-instance action=show-main label=main reason=single_instance");
                crate::window_lifecycle::restore_parked_main_window_position(
                    &window,
                    "single_instance",
                );
                let _ = window.show();
                println!("[window-trace] source=single-instance action=unminimize-main label=main reason=single_instance");
                let _ = window.unminimize();
                println!("[window-trace] source=single-instance action=focus-main label=main reason=single_instance");
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
            get_latest_screenshot_payload,
            get_latest_screenshot_shell_payload,
            show_save_feedback_toast,
            get_fullscreen_image,
            get_fullscreen_image_bytes,
            get_fullscreen_rgba_bytes,
            post_fullscreen_rgba_shared_buffer,
            capture_region,
            copy_image_to_clipboard,
            save_image_to_file,
            choose_image_save_path,
            choose_image_save_directory,
            write_image_to_file,
            log_screenshot_perf,
            quick_fullscreen_capture,
            capture_live_region,
            scroll_mouse_at,
            cancel_screenshot,
            force_close_screenshots,
            hide_main_window,
            force_close_recording_controls,
            dump_all_windows_state,
            get_window_rects,
            get_text_source_snapshot,
            overlay_ready_to_show,
            activate_screenshot_overlay_for_interaction,
            get_screenshot_pointer_state,
            build_native_selected_image_bridge,
            copy_native_selected_output_to_clipboard,
            get_native_screenshot_diagnostics_status,
            screenshot_wgc_diagnostic_commands::run_native_wgc_one_frame_probe_smoke,
            screenshot_wgc_diagnostic_commands::resolve_native_wgc_monitor_target_diagnostic,
            screenshot_wgc_diagnostic_commands::run_native_wgc_monitor_session_smoke,
            screenshot_wgc_selected_output_diagnostic_commands::run_native_wgc_selected_output_clipboard_acceptance_smoke,
            screenshot_wgc_selected_output_diagnostic_commands::run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke,
            run_native_dxgi_texture_smoke,
            run_native_dxgi_desktop_update_pulse_diagnostic_smoke,
            run_native_dxgi_pulse_before_acquire_probe,
            run_native_dxgi_frame_info_probe,
            run_native_dxgi_default_vs_selected_acquire_comparison_smoke,
            run_native_dxgi_selected_readback_smoke,
            run_native_dxgi_selected_output_bridge_dry_run,
            run_native_dxgi_selected_output_clipboard_acceptance_smoke,
            run_native_dxgi_cursor_nudge_diagnostic_smoke,
            run_native_cursor_nudge_smoke,
            run_native_input_synthetic_smoke,
            run_native_overlay_planned_smoke,
            run_local_ocr,
            prewarm_local_ocr_models,
            re_register_shortcut,
            get_diagnostics_report,
            set_last_translation_diagnostics,
            get_startup_diagnostics_probe_path,
            get_startup_readiness_snapshot,
            run_startup_readiness_probe,
            get_rapid_ocr_status,
            run_rapid_ocr_self_test,
            start_rapid_ocr_worker,
            stop_rapid_ocr_worker,
            restart_rapid_ocr_worker,
            get_rapid_ocr_worker_status
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
            prewarm_screenshot_window(app.handle().clone());
            if std::env::var("YSN_SCREENSHOT_LIFECYCLE_SMOKE").ok().as_deref() == Some("1") {
                let smoke_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                    run_screenshot_lifecycle_smoke(smoke_app).await;
                });
            }
            if std::env::var("YSN_SCREENSHOT_AUTO_START_SMOKE").ok().as_deref() == Some("1") {
                let smoke_app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let delay_ms = std::env::var("YSN_SCREENSHOT_AUTO_START_SMOKE_DELAY_MS")
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(900);
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    if let Err(error) = start_screenshot(smoke_app, None).await {
                        eprintln!("[screenshot-smoke] auto start failed: {error}");
                    }
                });
            }
            let readiness_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let probe_app = readiness_app.clone();
                match tokio::task::spawn_blocking(move || {
                    build_startup_readiness_snapshot(probe_app)
                })
                .await
                {
                    Ok(snapshot) => cache_startup_readiness_snapshot(snapshot),
                    Err(error) => eprintln!("Failed to run startup readiness probe: {}", error),
                }
            });

            let screenshot_item = tauri::menu::MenuItemBuilder::new("立即截图")
                .id("screenshot")
                .build(app)?;
            let delayed_screenshot_item = tauri::menu::MenuItemBuilder::new("延迟 3 秒截图")
                .id("screenshot_delay_3s")
                .build(app)?;
            let show_item = tauri::menu::MenuItemBuilder::new("显示主窗口")
                .id("show")
                .build(app)?;
            let exit_item = tauri::menu::MenuItemBuilder::new("退出")
                .id("exit")
                .build(app)?;
            let tray_menu = tauri::menu::MenuBuilder::new(app)
                .item(&screenshot_item)
                .item(&delayed_screenshot_item)
                .item(&show_item)
                .separator()
                .item(&exit_item)
                .build()?;
            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(
                    tauri::image::Image::from_bytes(include_bytes!("../icons/taskbar-32x32.png"))
                        .unwrap(),
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
                    "screenshot_delay_3s" => {
                        let app_h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            if let Err(e) = start_screenshot(app_h, None).await {
                                eprintln!("Failed to start delayed screenshot: {}", e);
                            }
                        });
                    }
                    "show" => {
                        let _ = crate::window_lifecycle::set_webview_capture_excluded(app, "main", false);
                        if let Some(win) = app.get_webview_window("main") {
                            println!("[window-trace] source=tray-menu action=show-main label=main reason=tray_menu");
                            crate::window_lifecycle::restore_parked_main_window_position(
                                &win,
                                "tray_menu",
                            );
                            let _ = win.show();
                            println!("[window-trace] source=tray-menu action=focus-main label=main reason=tray_menu");
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
                        let has_overlay_window = app.webview_windows().keys().any(|label| {
                            label == "screenshot"
                                || label.starts_with("screenshot_")
                                || label == "recording_notice"
                                || label.starts_with("recording_control")
                        });

                        if has_overlay_window {
                            println!("[window-trace] source=tray-click action=skip-show-main reason=overlay-active");
                            return;
                        }

                        if let Some(win) = app.get_webview_window("main") {
                            println!("[window-trace] source=tray-click action=show-main label=main reason=tray_click");
                            crate::window_lifecycle::restore_parked_main_window_position(
                                &win,
                                "tray_click",
                            );
                            let _ = win.show();
                            println!("[window-trace] source=tray-click action=focus-main label=main reason=tray_click");
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
        .on_window_event(|window, event| { crate::window_lifecycle::handle_window_event(window, event); })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn configure_webview2_default_background() {
    #[cfg(target_os = "windows")]
    {
        let opaque_requested = std::env::var("YSN_SCREENSHOT_OPAQUE_WINDOW")
            .ok()
            .as_deref()
            == Some("1")
            || std::env::var("YSN_SCREENSHOT_TRANSPARENT_WINDOW")
                .ok()
                .as_deref()
                == Some("0");
        if !opaque_requested && std::env::var("WEBVIEW2_DEFAULT_BACKGROUND_COLOR").is_err() {
            std::env::set_var("WEBVIEW2_DEFAULT_BACKGROUND_COLOR", "00000000");
        }
    }
}

#[cfg(test)]
mod tests;

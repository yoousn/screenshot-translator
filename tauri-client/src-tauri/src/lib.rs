#[cfg(windows)]
use std::os::windows::process::CommandExt;
use crate::recording_process::*;


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
use futures_util::StreamExt;
use screenshots::Screen;
use std::borrow::Cow;
use std::fs;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::path::PathBuf;
use std::process::{ChildStdin, ChildStdout, Command};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tokio::time::Duration;

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
        pub fn GetForegroundWindow() -> isize;
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
use std::process::{Child, Stdio};
use std::time::Instant;













































#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                println!("[window-trace] source=single-instance action=show-main label=main reason=single_instance");
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
            get_fullscreen_image,
            capture_region,
            copy_image_to_clipboard,
            save_image_to_file,
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
            get_screenshot_pointer_state,
            run_local_ocr,
            prewarm_local_ocr_models,
            re_register_shortcut,
            get_diagnostics_report,
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
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            println!("[window-trace] source=tray-menu action=show-main label=main reason=tray_menu");
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
    fn test_escape_concat_path_uses_ffmpeg_file_list_syntax() {
        let path = std::path::Path::new(r"C:\Users\Alice\Videos\Bob's clip.mp4");
        assert_eq!(
            super::escape_concat_path(path),
            "C:/Users/Alice/Videos/Bob\\'s clip.mp4"
        );
    }

    #[test]
    fn test_ffmpeg_stderr_excerpt_keeps_tail_context() {
        let stderr = (0..20)
            .map(|index| format!("line {}", index))
            .collect::<Vec<_>>()
            .join("\n");
        let excerpt = super::ffmpeg_stderr_excerpt(stderr.as_bytes());
        assert!(!excerpt.contains("line 0"));
        assert!(excerpt.contains("line 19"));
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

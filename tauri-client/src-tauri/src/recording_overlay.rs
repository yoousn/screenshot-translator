use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

#[cfg(target_os = "windows")]
use crate::set_hwnd_capture_excluded;
#[cfg(target_os = "windows")]
use crate::win32;

static RECORDING_OVERLAY: OnceLock<Mutex<Option<NativeRecordingOverlay>>> = OnceLock::new();
static RECORDING_OVERLAY_COLOR: AtomicU32 = AtomicU32::new(RECORDING_BORDER_BLUE);
#[derive(Clone, Copy)]
struct NativeRecordingOverlay {
    hwnd: isize,
}

unsafe impl Send for NativeRecordingOverlay {}

fn get_recording_overlay() -> &'static Mutex<Option<NativeRecordingOverlay>> {
    RECORDING_OVERLAY.get_or_init(|| Mutex::new(None))
}

pub(crate) const RECORDING_BORDER_BLUE: u32 = 0xeb6325;
pub(crate) const RECORDING_BORDER_RED: u32 = 0x4444ef;
pub(crate) const RECORDING_BORDER_YELLOW: u32 = 0x0b9ef5;
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
pub(crate) fn hide_recording_overlay_internal() {
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
pub(crate) fn hide_recording_overlay_internal() {}

#[tauri::command]
pub fn hide_recording_overlay() -> Result<(), String> {
    hide_recording_overlay_internal();
    Ok(())
}
pub(crate) fn recording_color_ref(status: &str) -> u32 {
    match status {
        "recording" => RECORDING_BORDER_RED,
        "paused" => RECORDING_BORDER_YELLOW,
        _ => RECORDING_BORDER_BLUE,
    }
}

#[tauri::command]
pub fn set_recording_overlay_status(status: String) -> Result<(), String> {
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
pub fn show_recording_overlay(x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
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
    {}
}

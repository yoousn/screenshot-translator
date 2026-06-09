use super::MonitorCaptureBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopUpdatePulseRequest {
    pub bounds: MonitorCaptureBounds,
    pub pulse_size_px: u32,
    pub pulse_alpha: u8,
    pub dwell_ms: u64,
}

impl DesktopUpdatePulseRequest {
    pub const fn new(
        bounds: MonitorCaptureBounds,
        pulse_size_px: u32,
        pulse_alpha: u8,
        dwell_ms: u64,
    ) -> Self {
        Self {
            bounds,
            pulse_size_px,
            pulse_alpha,
            dwell_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopUpdatePulseReport {
    pub attempted: bool,
    pub ok: bool,
    pub requested_bounds: MonitorCaptureBounds,
    pub pulse_bounds: Option<MonitorCaptureBounds>,
    pub pulse_size_px: u32,
    pub pulse_alpha: u8,
    pub dwell_ms: u64,
    pub class_registered: bool,
    pub window_created: bool,
    pub layered_attributes_set: bool,
    pub shown_no_activate: bool,
    pub update_window_called: bool,
    pub invalidate_called: bool,
    pub dwm_flush_called: bool,
    pub destroy_attempted: bool,
    pub destroy_confirmed: bool,
    pub hidden_from_alt_tab: bool,
    pub no_activate: bool,
    pub appwindow_excluded: bool,
    pub error: Option<String>,
}

impl DesktopUpdatePulseReport {
    fn blocked(request: DesktopUpdatePulseRequest, error: impl ToString) -> Self {
        Self {
            attempted: false,
            ok: false,
            requested_bounds: request.bounds,
            pulse_bounds: None,
            pulse_size_px: request.pulse_size_px,
            pulse_alpha: request.pulse_alpha,
            dwell_ms: request.dwell_ms,
            class_registered: false,
            window_created: false,
            layered_attributes_set: false,
            shown_no_activate: false,
            update_window_called: false,
            invalidate_called: false,
            dwm_flush_called: false,
            destroy_attempted: false,
            destroy_confirmed: false,
            hidden_from_alt_tab: false,
            no_activate: false,
            appwindow_excluded: false,
            error: Some(error.to_string()),
        }
    }
}

pub fn run_desktop_update_pulse(request: DesktopUpdatePulseRequest) -> DesktopUpdatePulseReport {
    if request.bounds.is_empty() {
        return DesktopUpdatePulseReport::blocked(
            request,
            "desktop update pulse requires non-empty bounds",
        );
    }
    if request.pulse_size_px == 0 || request.pulse_size_px > 4 {
        return DesktopUpdatePulseReport::blocked(
            request,
            "desktop update pulse size must be 1..=4 px",
        );
    }
    if request.pulse_alpha == 0 || request.pulse_alpha > 16 {
        return DesktopUpdatePulseReport::blocked(
            request,
            "desktop update pulse alpha must be 1..=16",
        );
    }
    if request.dwell_ms > 250 {
        return DesktopUpdatePulseReport::blocked(
            request,
            "desktop update pulse dwell must be <= 250 ms",
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        DesktopUpdatePulseReport::blocked(request, "desktop update pulse requires Windows")
    }

    #[cfg(target_os = "windows")]
    {
        run_desktop_update_pulse_windows(request)
    }
}

#[cfg(target_os = "windows")]
fn run_desktop_update_pulse_windows(
    request: DesktopUpdatePulseRequest,
) -> DesktopUpdatePulseReport {
    use crate::win32;
    use std::ffi::c_void;
    use std::time::Duration;

    let pulse_bounds = pulse_bounds(request);
    let class_name = wide_null(WIN32_DESKTOP_UPDATE_PULSE_CLASS_NAME);
    let title = wide_null("YSN DXGI Desktop Update Pulse");
    let h_instance = unsafe { win32::GetModuleHandleW(std::ptr::null()) };
    let wnd_class = win32::WNDCLASSW {
        style: 0,
        lpfn_wnd_proc: Some(desktop_update_pulse_wnd_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance,
        h_icon: 0,
        h_cursor: 0,
        hbr_background: 0,
        lpsz_menu_name: std::ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
    };
    let class_registered = unsafe { win32::RegisterClassW(&wnd_class) } != 0;
    let ex_style = pulse_ex_style();
    let hwnd = unsafe {
        win32::CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            pulse_bounds.origin_x,
            pulse_bounds.origin_y,
            pulse_bounds.width as i32,
            pulse_bounds.height as i32,
            0,
            0,
            h_instance,
            std::ptr::null_mut::<c_void>(),
        )
    };
    if hwnd == 0 {
        return DesktopUpdatePulseReport {
            attempted: true,
            ok: false,
            requested_bounds: request.bounds,
            pulse_bounds: Some(pulse_bounds),
            pulse_size_px: request.pulse_size_px,
            pulse_alpha: request.pulse_alpha,
            dwell_ms: request.dwell_ms,
            class_registered,
            window_created: false,
            layered_attributes_set: false,
            shown_no_activate: false,
            update_window_called: false,
            invalidate_called: false,
            dwm_flush_called: false,
            destroy_attempted: false,
            destroy_confirmed: false,
            hidden_from_alt_tab: style_hides_from_alt_tab(ex_style),
            no_activate: style_no_activate(ex_style),
            appwindow_excluded: style_excludes_appwindow(ex_style),
            error: Some("CreateWindowExW failed for desktop update pulse".to_string()),
        };
    }

    let layered_attributes_set =
        unsafe { win32::SetLayeredWindowAttributes(hwnd, 0, request.pulse_alpha, LWA_ALPHA) } != 0;
    let shown_no_activate = unsafe {
        win32::SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            pulse_bounds.origin_x,
            pulse_bounds.origin_y,
            pulse_bounds.width as i32,
            pulse_bounds.height as i32,
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        )
    } != 0;
    let _ = unsafe { win32::ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    let update_window_called = unsafe { win32::UpdateWindow(hwnd) } != 0;
    let invalidate_called = unsafe { win32::InvalidateRect(hwnd, std::ptr::null(), 1) } != 0;
    let dwm_flush_called = unsafe { win32::DwmFlush() } == 0;
    if request.dwell_ms > 0 {
        std::thread::sleep(Duration::from_millis(request.dwell_ms));
    }
    let destroy_attempted = true;
    let destroy_confirmed = unsafe { win32::DestroyWindow(hwnd) } != 0;
    let error = if !destroy_confirmed {
        Some("DestroyWindow failed for desktop update pulse".to_string())
    } else if !layered_attributes_set || !shown_no_activate {
        Some(
            "desktop update pulse window was created but not fully shown as layered/no-activate"
                .to_string(),
        )
    } else {
        None
    };

    DesktopUpdatePulseReport {
        attempted: true,
        ok: error.is_none(),
        requested_bounds: request.bounds,
        pulse_bounds: Some(pulse_bounds),
        pulse_size_px: request.pulse_size_px,
        pulse_alpha: request.pulse_alpha,
        dwell_ms: request.dwell_ms,
        class_registered,
        window_created: true,
        layered_attributes_set,
        shown_no_activate,
        update_window_called,
        invalidate_called,
        dwm_flush_called,
        destroy_attempted,
        destroy_confirmed,
        hidden_from_alt_tab: style_hides_from_alt_tab(ex_style),
        no_activate: style_no_activate(ex_style),
        appwindow_excluded: style_excludes_appwindow(ex_style),
        error,
    }
}

fn pulse_bounds(request: DesktopUpdatePulseRequest) -> MonitorCaptureBounds {
    let size = request
        .pulse_size_px
        .min(request.bounds.width)
        .min(request.bounds.height)
        .max(1);
    MonitorCaptureBounds::new(request.bounds.origin_x, request.bounds.origin_y, size, size)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn desktop_update_pulse_wnd_proc(
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    crate::win32::DefWindowProcW(hwnd, message, w_param, l_param)
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn pulse_ex_style() -> u32 {
    (WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOPMOST) & !WS_EX_APPWINDOW
}

#[cfg(target_os = "windows")]
fn style_hides_from_alt_tab(style: u32) -> bool {
    style & WS_EX_TOOLWINDOW != 0 && style & WS_EX_APPWINDOW == 0
}

#[cfg(target_os = "windows")]
fn style_no_activate(style: u32) -> bool {
    style & WS_EX_NOACTIVATE != 0
}

#[cfg(target_os = "windows")]
fn style_excludes_appwindow(style: u32) -> bool {
    style & WS_EX_APPWINDOW == 0
}

#[cfg(target_os = "windows")]
const WIN32_DESKTOP_UPDATE_PULSE_CLASS_NAME: &str = "YSN_DXGI_DESKTOP_UPDATE_PULSE";
#[cfg(target_os = "windows")]
const HWND_TOPMOST: isize = -1;
#[cfg(target_os = "windows")]
const SWP_NOACTIVATE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const SWP_SHOWWINDOW: u32 = 0x0040;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const WS_POPUP: u32 = 0x80000000;
#[cfg(target_os = "windows")]
const WS_EX_TOPMOST: u32 = 0x00000008;
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: u32 = 0x00000080;
#[cfg(target_os = "windows")]
const WS_EX_APPWINDOW: u32 = 0x00040000;
#[cfg(target_os = "windows")]
const WS_EX_LAYERED: u32 = 0x00080000;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: u32 = 0x08000000;
#[cfg(target_os = "windows")]
const LWA_ALPHA: u32 = 0x00000002;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_rejects_empty_bounds() {
        let report = run_desktop_update_pulse(DesktopUpdatePulseRequest::new(
            MonitorCaptureBounds::new(0, 0, 0, 1),
            1,
            1,
            0,
        ));
        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("non-empty bounds"));
    }

    #[test]
    fn pulse_rejects_large_size() {
        let report = run_desktop_update_pulse(DesktopUpdatePulseRequest::new(
            MonitorCaptureBounds::new(0, 0, 10, 10),
            5,
            1,
            0,
        ));
        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("1..=4"));
    }

    #[test]
    fn pulse_rejects_large_dwell() {
        let report = run_desktop_update_pulse(DesktopUpdatePulseRequest::new(
            MonitorCaptureBounds::new(0, 0, 10, 10),
            1,
            1,
            251,
        ));
        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("<= 250"));
    }
}

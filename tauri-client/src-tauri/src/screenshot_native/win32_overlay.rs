#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use std::ffi::c_void;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32OverlayHandle {
    hwnd: isize,
}

impl Win32OverlayHandle {
    pub const fn new(hwnd: isize) -> Self {
        Self { hwnd }
    }

    pub const fn null() -> Self {
        Self { hwnd: 0 }
    }

    pub const fn hwnd(self) -> isize {
        self.hwnd
    }

    pub const fn is_valid(self) -> bool {
        self.hwnd != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32OverlayConfig {
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub topmost: bool,
    pub no_activate: bool,
    pub exclude_from_capture: bool,
}

impl Default for Win32OverlayConfig {
    fn default() -> Self {
        Self {
            title: "YSN Screenshot Overlay".to_string(),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            topmost: true,
            no_activate: true,
            exclude_from_capture: true,
        }
    }
}

impl Win32OverlayConfig {
    pub fn fullscreen_like(width: i32, height: i32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32OverlayLifecycleState {
    Created,
    Visible,
    Hidden,
    Destroyed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Win32OverlayError {
    InvalidHandle(&'static str),
    InvalidConfig(&'static str),
    RegisterClassFailed,
    CreateWindowFailed,
    SetCaptureExclusionFailed,
    ShowWindowFailed,
    HideWindowFailed,
    DestroyWindowFailed,
}

impl fmt::Display for Win32OverlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(action) => {
                write!(formatter, "invalid Win32 overlay handle: {action}")
            }
            Self::InvalidConfig(field) => {
                write!(formatter, "invalid Win32 overlay config: {field}")
            }
            Self::RegisterClassFailed => {
                formatter.write_str("failed to register Win32 overlay class")
            }
            Self::CreateWindowFailed => {
                formatter.write_str("failed to create Win32 overlay window")
            }
            Self::SetCaptureExclusionFailed => {
                formatter.write_str("failed to update Win32 overlay capture exclusion")
            }
            Self::ShowWindowFailed => formatter.write_str("failed to show Win32 overlay window"),
            Self::HideWindowFailed => formatter.write_str("failed to hide Win32 overlay window"),
            Self::DestroyWindowFailed => {
                formatter.write_str("failed to destroy Win32 overlay window")
            }
        }
    }
}

impl std::error::Error for Win32OverlayError {}

#[derive(Debug, PartialEq, Eq)]
pub struct Win32OverlayWindow {
    handle: Win32OverlayHandle,
    state: Win32OverlayLifecycleState,
}

impl Win32OverlayWindow {
    pub const fn from_handle(handle: Win32OverlayHandle) -> Self {
        Self {
            handle,
            state: Win32OverlayLifecycleState::Created,
        }
    }

    pub const fn handle(&self) -> Win32OverlayHandle {
        self.handle
    }

    pub const fn state(&self) -> Win32OverlayLifecycleState {
        self.state
    }

    fn mark_destroyed(&mut self) {
        self.handle = Win32OverlayHandle::null();
        self.state = Win32OverlayLifecycleState::Destroyed;
    }
}

impl Drop for Win32OverlayWindow {
    fn drop(&mut self) {
        if self.handle.is_valid() {
            let _ = destroy_platform_overlay(self.handle);
            self.mark_destroyed();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32OverlayLifecycleDiagnostic {
    pub hwnd: isize,
    pub state: Win32OverlayLifecycleState,
    pub hidden_from_taskbar_and_alt_tab: bool,
    pub no_activate: bool,
}

impl Win32OverlayLifecycleDiagnostic {
    pub const fn from_config(
        handle: Win32OverlayHandle,
        state: Win32OverlayLifecycleState,
        config: &Win32OverlayConfig,
    ) -> Self {
        Self {
            hwnd: handle.hwnd(),
            state,
            hidden_from_taskbar_and_alt_tab: true,
            no_activate: config.no_activate,
        }
    }

    pub const fn requires_recovery(self) -> bool {
        !self.hidden_from_taskbar_and_alt_tab || !self.no_activate
    }
}

pub fn create_win32_overlay(
    config: &Win32OverlayConfig,
) -> Result<Win32OverlayWindow, Win32OverlayError> {
    validate_config(config)?;
    create_platform_overlay(config)
}

pub fn destroy_win32_overlay(window: &mut Win32OverlayWindow) -> Result<(), Win32OverlayError> {
    ensure_handle(window.handle, "destroy")?;
    destroy_platform_overlay(window.handle)?;
    window.mark_destroyed();
    Ok(())
}

pub fn show_win32_overlay(
    window: &mut Win32OverlayWindow,
    config: &Win32OverlayConfig,
) -> Result<(), Win32OverlayError> {
    ensure_handle(window.handle, "show")?;
    show_platform_overlay(window.handle, config)?;
    window.state = Win32OverlayLifecycleState::Visible;
    Ok(())
}

pub fn hide_win32_overlay(window: &mut Win32OverlayWindow) -> Result<(), Win32OverlayError> {
    ensure_handle(window.handle, "hide")?;
    hide_platform_overlay(window.handle)?;
    window.state = Win32OverlayLifecycleState::Hidden;
    Ok(())
}

pub fn diagnose_win32_overlay_lifecycle(
    window: &Win32OverlayWindow,
    config: &Win32OverlayConfig,
) -> Win32OverlayLifecycleDiagnostic {
    Win32OverlayLifecycleDiagnostic::from_config(window.handle, window.state, config)
}

fn validate_config(config: &Win32OverlayConfig) -> Result<(), Win32OverlayError> {
    if config.width <= 0 {
        return Err(Win32OverlayError::InvalidConfig("width"));
    }
    if config.height <= 0 {
        return Err(Win32OverlayError::InvalidConfig("height"));
    }
    Ok(())
}

fn ensure_handle(
    handle: Win32OverlayHandle,
    action: &'static str,
) -> Result<(), Win32OverlayError> {
    if handle.is_valid() {
        Ok(())
    } else {
        Err(Win32OverlayError::InvalidHandle(action))
    }
}

#[cfg(target_os = "windows")]
fn create_platform_overlay(
    config: &Win32OverlayConfig,
) -> Result<Win32OverlayWindow, Win32OverlayError> {
    let class_name = wide_null(WIN32_OVERLAY_CLASS_NAME);
    let title = wide_null(&config.title);
    let h_instance = unsafe { win32::GetModuleHandleW(std::ptr::null()) };
    let wnd_class = win32::WNDCLASSW {
        style: 0,
        lpfn_wnd_proc: Some(win32_overlay_wnd_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance,
        h_icon: 0,
        h_cursor: 0,
        hbr_background: 0,
        lpsz_menu_name: std::ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
    };

    let _ = unsafe { win32::RegisterClassW(&wnd_class) };

    let hwnd = unsafe {
        win32::CreateWindowExW(
            overlay_ex_style(config),
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            config.x,
            config.y,
            config.width,
            config.height,
            0,
            0,
            h_instance,
            std::ptr::null_mut::<c_void>(),
        )
    };
    if hwnd == 0 {
        return Err(Win32OverlayError::CreateWindowFailed);
    }

    let window = Win32OverlayWindow::from_handle(Win32OverlayHandle::new(hwnd));
    if config.exclude_from_capture {
        set_capture_exclusion(window.handle(), true)?;
    }

    Ok(window)
}

#[cfg(not(target_os = "windows"))]
fn create_platform_overlay(
    _config: &Win32OverlayConfig,
) -> Result<Win32OverlayWindow, Win32OverlayError> {
    Err(Win32OverlayError::CreateWindowFailed)
}

#[cfg(target_os = "windows")]
fn destroy_platform_overlay(handle: Win32OverlayHandle) -> Result<(), Win32OverlayError> {
    let ok = unsafe { win32::DestroyWindow(handle.hwnd()) };
    if ok == 0 {
        Err(Win32OverlayError::DestroyWindowFailed)
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn destroy_platform_overlay(_handle: Win32OverlayHandle) -> Result<(), Win32OverlayError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn show_platform_overlay(
    handle: Win32OverlayHandle,
    config: &Win32OverlayConfig,
) -> Result<(), Win32OverlayError> {
    if config.exclude_from_capture {
        set_capture_exclusion(handle, true)?;
    }
    let insert_after = if config.topmost {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | no_activate_swp_flag(config);
    let positioned = unsafe { win32::SetWindowPos(handle.hwnd(), insert_after, 0, 0, 0, 0, flags) };
    if positioned == 0 {
        return Err(Win32OverlayError::ShowWindowFailed);
    }
    let show_command = if config.no_activate {
        SW_SHOWNOACTIVATE
    } else {
        SW_SHOW
    };
    let _ = unsafe { win32::ShowWindow(handle.hwnd(), show_command) };
    let _ = unsafe { win32::UpdateWindow(handle.hwnd()) };
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn show_platform_overlay(
    _handle: Win32OverlayHandle,
    _config: &Win32OverlayConfig,
) -> Result<(), Win32OverlayError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn hide_platform_overlay(handle: Win32OverlayHandle) -> Result<(), Win32OverlayError> {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW;
    let positioned =
        unsafe { win32::SetWindowPos(handle.hwnd(), HWND_NOTOPMOST, 0, 0, 0, 0, flags) };
    if positioned == 0 {
        return Err(Win32OverlayError::HideWindowFailed);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn hide_platform_overlay(_handle: Win32OverlayHandle) -> Result<(), Win32OverlayError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_capture_exclusion(
    handle: Win32OverlayHandle,
    excluded: bool,
) -> Result<(), Win32OverlayError> {
    let affinity = if excluded {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    let ok = unsafe { win32::SetWindowDisplayAffinity(handle.hwnd(), affinity) };
    if ok == 0 {
        Err(Win32OverlayError::SetCaptureExclusionFailed)
    } else {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn overlay_ex_style(config: &Win32OverlayConfig) -> u32 {
    let mut style = WS_EX_TOOLWINDOW | WS_EX_LAYERED;
    if config.topmost {
        style |= WS_EX_TOPMOST;
    }
    if config.no_activate {
        style |= WS_EX_NOACTIVATE;
    }
    style & !WS_EX_APPWINDOW
}

#[cfg(all(test, target_os = "windows"))]
fn hides_from_taskbar_and_alt_tab(style: u32) -> bool {
    style & WS_EX_TOOLWINDOW != 0 && style & WS_EX_APPWINDOW == 0
}

#[cfg(target_os = "windows")]
fn no_activate_swp_flag(config: &Win32OverlayConfig) -> u32 {
    if config.no_activate {
        SWP_NOACTIVATE
    } else {
        0
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn win32_overlay_wnd_proc(
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    win32::DefWindowProcW(hwnd, message, w_param, l_param)
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
const WIN32_OVERLAY_CLASS_NAME: &str = "YSN_SCREENSHOT_NATIVE_OVERLAY";
#[cfg(target_os = "windows")]
const HWND_TOPMOST: isize = -1;
#[cfg(target_os = "windows")]
const HWND_NOTOPMOST: isize = -2;
#[cfg(target_os = "windows")]
const SWP_NOSIZE: u32 = 0x0001;
#[cfg(target_os = "windows")]
const SWP_NOMOVE: u32 = 0x0002;
#[cfg(target_os = "windows")]
const SWP_NOACTIVATE: u32 = 0x0010;
#[cfg(target_os = "windows")]
const SWP_SHOWWINDOW: u32 = 0x0040;
#[cfg(target_os = "windows")]
const SWP_HIDEWINDOW: u32 = 0x0080;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const SW_SHOW: i32 = 5;
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
const WDA_NONE: u32 = 0x00000000;
#[cfg(target_os = "windows")]
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn overlay_ex_style_uses_toolwindow_without_appwindow() {
        let config = Win32OverlayConfig::default();
        let style = overlay_ex_style(&config);
        assert!(hides_from_taskbar_and_alt_tab(style));
        assert_ne!(style & WS_EX_NOACTIVATE, 0);
    }

    #[test]
    fn overlay_ex_style_stays_hidden_when_activation_is_allowed() {
        let config = Win32OverlayConfig {
            no_activate: false,
            ..Win32OverlayConfig::default()
        };
        let style = overlay_ex_style(&config);
        assert!(hides_from_taskbar_and_alt_tab(style));
        assert_eq!(style & WS_EX_NOACTIVATE, 0);
    }

    #[test]
    fn lifecycle_diagnostic_flags_activation_risk() {
        let config = Win32OverlayConfig {
            no_activate: false,
            ..Win32OverlayConfig::default()
        };
        let window = Win32OverlayWindow::from_handle(Win32OverlayHandle::null());
        let diagnostic = diagnose_win32_overlay_lifecycle(&window, &config);
        assert!(diagnostic.requires_recovery());
    }
}

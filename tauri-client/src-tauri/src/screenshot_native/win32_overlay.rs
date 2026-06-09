#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::c_void;
use std::fmt;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

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
    clear_win32_overlay_bitmap(window.handle());
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

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct Win32OverlayBitmap {
    bgra_bytes: Vec<u8>,
    width: u32,
    height: u32,
}

#[cfg(target_os = "windows")]
static WIN32_OVERLAY_BITMAPS: OnceLock<Mutex<HashMap<isize, Win32OverlayBitmap>>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn overlay_bitmap_store() -> &'static Mutex<HashMap<isize, Win32OverlayBitmap>> {
    WIN32_OVERLAY_BITMAPS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "windows")]
pub fn set_win32_overlay_bitmap(
    handle: Win32OverlayHandle,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(), Win32OverlayError> {
    ensure_handle(handle, "set-bitmap")?;
    if width == 0 || height == 0 {
        return Err(Win32OverlayError::InvalidConfig("bitmap dimensions"));
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|w| w.checked_mul(usize::try_from(height).ok()?))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(Win32OverlayError::InvalidConfig("bitmap size"))?;
    if bytes.len() != expected_len {
        return Err(Win32OverlayError::InvalidConfig("bitmap bytes"));
    }
    let bgra_bytes = rgba_to_bgra_dib_bytes(bytes);
    if let Ok(mut guard) = overlay_bitmap_store().lock() {
        guard.insert(
            handle.hwnd(),
            Win32OverlayBitmap {
                bgra_bytes,
                width,
                height,
            },
        );
    }
    unsafe {
        let _ = win32::InvalidateRect(handle.hwnd(), std::ptr::null(), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn rgba_to_bgra_dib_bytes(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        bgra.push(pixel[2]);
        bgra.push(pixel[1]);
        bgra.push(pixel[0]);
        bgra.push(pixel[3]);
    }
    bgra
}

#[cfg(not(target_os = "windows"))]
pub fn set_win32_overlay_bitmap(
    _handle: Win32OverlayHandle,
    _bytes: &[u8],
    _width: u32,
    _height: u32,
) -> Result<(), Win32OverlayError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn clear_win32_overlay_bitmap(handle: Win32OverlayHandle) {
    if let Ok(mut guard) = overlay_bitmap_store().lock() {
        guard.remove(&handle.hwnd());
    }
}

#[cfg(not(target_os = "windows"))]
fn clear_win32_overlay_bitmap(_handle: Win32OverlayHandle) {}

#[cfg(not(target_os = "windows"))]
fn create_platform_overlay(
    _config: &Win32OverlayConfig,
) -> Result<Win32OverlayWindow, Win32OverlayError> {
    Err(Win32OverlayError::CreateWindowFailed)
}

#[cfg(target_os = "windows")]
fn destroy_platform_overlay(handle: Win32OverlayHandle) -> Result<(), Win32OverlayError> {
    clear_win32_overlay_bitmap(handle);
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
    let _ = unsafe { win32::InvalidateRect(handle.hwnd(), std::ptr::null(), 0) };
    let _ = unsafe { win32::UpdateWindow(handle.hwnd()) };
    unsafe {
        let _ = win32::DwmFlush();
    }
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
    let mut style = WS_EX_TOOLWINDOW;
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
    match message {
        WM_NCHITTEST => return HTTRANSPARENT,
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            paint_overlay_bitmap(hwnd);
            return 0;
        }
        _ => {}
    }
    win32::DefWindowProcW(hwnd, message, w_param, l_param)
}

#[cfg(target_os = "windows")]
fn paint_overlay_bitmap(hwnd: isize) {
    let bitmap = overlay_bitmap_store()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&hwnd).cloned());
    let mut paint = win32::PAINTSTRUCT {
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
    let hdc = unsafe { win32::BeginPaint(hwnd, &mut paint as *mut win32::PAINTSTRUCT) };
    if hdc == 0 {
        return;
    }
    if let Some(bitmap) = bitmap {
        let info = BitmapInfo {
            header: BitmapInfoHeader {
                bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                bi_width: bitmap.width as i32,
                bi_height: -(bitmap.height as i32),
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            colors: [0; 3],
        };
        let dst_width = (paint.rc_paint.right - paint.rc_paint.left).max(bitmap.width as i32);
        let dst_height = (paint.rc_paint.bottom - paint.rc_paint.top).max(bitmap.height as i32);
        unsafe {
            let _ = StretchDIBits(
                hdc,
                0,
                0,
                dst_width,
                dst_height,
                0,
                0,
                bitmap.width as i32,
                bitmap.height as i32,
                bitmap.bgra_bytes.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }
    let _ = unsafe { win32::EndPaint(hwnd, &paint as *const win32::PAINTSTRUCT) };
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
const WM_NCHITTEST: u32 = 0x0084;
#[cfg(target_os = "windows")]
const WM_ERASEBKGND: u32 = 0x0014;
#[cfg(target_os = "windows")]
const WM_PAINT: u32 = 0x000F;
#[cfg(target_os = "windows")]
const HTTRANSPARENT: isize = -1;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: u32 = 0x08000000;
#[cfg(target_os = "windows")]
const WDA_NONE: u32 = 0x00000000;
#[cfg(target_os = "windows")]
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;

#[cfg(target_os = "windows")]
#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 3],
}

#[cfg(target_os = "windows")]
const BI_RGB: u32 = 0;
#[cfg(target_os = "windows")]
const DIB_RGB_COLORS: u32 = 0;
#[cfg(target_os = "windows")]
const SRCCOPY: u32 = 0x00CC0020;

#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(
        hdc: isize,
        x_dest: i32,
        y_dest: i32,
        dest_width: i32,
        dest_height: i32,
        x_src: i32,
        y_src: i32,
        src_width: i32,
        src_height: i32,
        bits: *const std::ffi::c_void,
        bitmap_info: *const BitmapInfo,
        usage: u32,
        rop: u32,
    ) -> i32;
}

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

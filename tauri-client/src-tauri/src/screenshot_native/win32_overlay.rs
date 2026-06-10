#[cfg(target_os = "windows")]
use super::win32_input::{
    WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN,
};
#[cfg(target_os = "windows")]
use super::win32_overlay_input::{
    apply_win32_overlay_input, clear_win32_overlay_input_state,
    initialize_win32_overlay_input_state,
};
#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::c_void;
use std::fmt;
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex, OnceLock};

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
    initialize_win32_overlay_input_state(window.handle());
    if config.exclude_from_capture {
        if let Err(error) = set_capture_exclusion(window.handle(), true) {
            clear_win32_overlay_input_state(window.handle());
            let _ = unsafe { win32::DestroyWindow(hwnd) };
            return Err(error);
        }
    }

    Ok(window)
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct Win32OverlayBitmap {
    bgra_bytes: Arc<[u8]>,
    dimmed_bgra_bytes: Arc<[u8]>,
    width: u32,
    height: u32,
    candidate: Option<Win32OverlaySelectionRect>,
    selection: Option<Win32OverlaySelectionRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32OverlaySelectionRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
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
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|a| a.checked_mul(4));
    let Some(expected_len) = expected_len else {
        return Err(Win32OverlayError::InvalidConfig(
            "bitmap dimensions overflow",
        ));
    };
    if bytes.len() != expected_len {
        return Err(Win32OverlayError::InvalidConfig("bitmap bytes"));
    }
    let bgra_bytes: Arc<[u8]> = Arc::from(rgba_to_bgra_dib_bytes(bytes));
    if let Ok(mut guard) = overlay_bitmap_store().lock() {
        guard.insert(
            handle.hwnd(),
            Win32OverlayBitmap {
                dimmed_bgra_bytes: Arc::from(
                    bgra_bytes
                        .chunks_exact(4)
                        .flat_map(|chunk| {
                            [
                                chunk[0].saturating_sub(chunk[0] / 2),
                                chunk[1].saturating_sub(chunk[1] / 2),
                                chunk[2].saturating_sub(chunk[2] / 2),
                                chunk[3],
                            ]
                        })
                        .collect::<Vec<u8>>(),
                ),
                bgra_bytes,
                width,
                height,
                candidate: None,
                selection: None,
            },
        );
    }
    unsafe {
        win32::InvalidateRect(handle.hwnd(), std::ptr::null(), 0);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_win32_overlay_selection(
    handle: Win32OverlayHandle,
    selection: Option<Win32OverlaySelectionRect>,
) {
    if !handle.is_valid() {
        return;
    }
    if let Ok(mut guard) = overlay_bitmap_store().lock() {
        if let Some(bitmap) = guard.get_mut(&handle.hwnd()) {
            bitmap.selection = selection;
        }
    }
    unsafe {
        win32::InvalidateRect(handle.hwnd(), std::ptr::null(), 0);
    }
}

#[cfg(target_os = "windows")]
pub fn set_win32_overlay_candidate(
    handle: Win32OverlayHandle,
    candidate: Option<Win32OverlaySelectionRect>,
) {
    if !handle.is_valid() {
        return;
    }
    if let Ok(mut guard) = overlay_bitmap_store().lock() {
        if let Some(bitmap) = guard.get_mut(&handle.hwnd()) {
            bitmap.candidate = candidate;
        }
    }
    unsafe {
        win32::InvalidateRect(handle.hwnd(), std::ptr::null(), 0);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_win32_overlay_candidate(
    _handle: Win32OverlayHandle,
    _candidate: Option<Win32OverlaySelectionRect>,
) {
}

#[cfg(not(target_os = "windows"))]
pub fn set_win32_overlay_selection(
    _handle: Win32OverlayHandle,
    _selection: Option<Win32OverlaySelectionRect>,
) {
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
    clear_win32_overlay_input_state(handle);
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
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            paint_overlay_bitmap(hwnd);
            return 0;
        }
        WM_NCHITTEST => return native_overlay_hit_test_result(),
        WM_LBUTTONDOWN | WM_MOUSEMOVE | WM_LBUTTONUP | WM_KEYDOWN | WM_SYSKEYDOWN
        | WM_KILLFOCUS => {
            if dispatch_win32_overlay_input(hwnd, message, w_param, l_param) {
                return 0;
            }
        }
        WM_NCDESTROY => {
            clear_win32_overlay_bitmap(Win32OverlayHandle::new(hwnd));
            clear_win32_overlay_input_state(Win32OverlayHandle::new(hwnd));
        }
        _ => {}
    }
    win32::DefWindowProcW(hwnd, message, w_param, l_param)
}

#[cfg(target_os = "windows")]
fn dispatch_win32_overlay_input(hwnd: isize, message: u32, w_param: usize, l_param: isize) -> bool {
    let dispatch = apply_win32_overlay_input(hwnd, message, w_param, l_param);
    if !dispatch.handled {
        return false;
    }

    if dispatch.set_capture {
        unsafe {
            let _ = win32::SetCapture(hwnd);
        }
    }
    if let Some(selection) = dispatch.selection {
        set_win32_overlay_selection(Win32OverlayHandle::new(hwnd), selection);
    }
    if dispatch.release_capture {
        unsafe {
            let _ = win32::ReleaseCapture();
        }
    }
    true
}

#[cfg(target_os = "windows")]
const fn native_overlay_hit_test_result() -> isize {
    HTCLIENT
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
        // 1. Paint the full dimmed image
        unsafe {
            let _ = StretchDIBits(
                hdc,
                0,
                0,
                bitmap.width as i32,
                bitmap.height as i32,
                0,
                0,
                bitmap.width as i32,
                bitmap.height as i32,
                bitmap.dimmed_bgra_bytes.as_ptr() as *const std::ffi::c_void,
                &info as *const BitmapInfo as *const _,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }

        if let Some(candidate) = bitmap.candidate {
            paint_overlay_rect_outline(
                hdc,
                candidate,
                bitmap.width as i32,
                bitmap.height as i32,
                CANDIDATE_BORDER_COLORREF,
                2,
            );
        }

        // 2. Paint the selected region from the original image over the dimmed background
        if let Some(sel) = bitmap.selection {
            let width = bitmap.width as i32;
            let height = bitmap.height as i32;
            let left = sel.left.min(sel.right).max(0).min(width);
            let right = sel.left.max(sel.right).max(0).min(width);
            let top = sel.top.min(sel.bottom).max(0).min(height);
            let bottom = sel.top.max(sel.bottom).max(0).min(height);
            let sel_w = right - left;
            let sel_h = bottom - top;

            if sel_w > 0 && sel_h > 0 {
                // For top-down DIBs (biHeight < 0), StretchDIBits source coords use top-left origin.
                unsafe {
                    let _ = StretchDIBits(
                        hdc,
                        left,
                        top,
                        sel_w,
                        sel_h,
                        left,
                        top,
                        sel_w,
                        sel_h,
                        bitmap.bgra_bytes.as_ptr() as *const std::ffi::c_void,
                        &info as *const BitmapInfo as *const _,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                }
                paint_overlay_rect_outline(
                    hdc,
                    Win32OverlaySelectionRect {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    width,
                    height,
                    SELECTION_BORDER_COLORREF,
                    2,
                );
            }
        }
    }
    let _ = unsafe { win32::EndPaint(hwnd, &paint as *const win32::PAINTSTRUCT) };
}

#[cfg(target_os = "windows")]
fn paint_overlay_rect_outline(
    hdc: isize,
    rect: Win32OverlaySelectionRect,
    width: i32,
    height: i32,
    color: u32,
    thickness: i32,
) {
    let Some((left, top, right, bottom)) = clamp_overlay_rect(rect, width, height) else {
        return;
    };
    let thickness = thickness
        .max(1)
        .min((right - left).max(1))
        .min((bottom - top).max(1));
    let brush = unsafe { win32::CreateSolidBrush(color) };
    if brush == 0 {
        return;
    }
    let rects = [
        win32::RECT {
            left,
            top,
            right,
            bottom: (top + thickness).min(bottom),
        },
        win32::RECT {
            left,
            top: (bottom - thickness).max(top),
            right,
            bottom,
        },
        win32::RECT {
            left,
            top,
            right: (left + thickness).min(right),
            bottom,
        },
        win32::RECT {
            left: (right - thickness).max(left),
            top,
            right,
            bottom,
        },
    ];
    for rect in rects {
        unsafe {
            let _ = win32::FillRect(hdc, &rect as *const win32::RECT, brush);
        }
    }
    unsafe {
        let _ = win32::DeleteObject(brush);
    }
}

#[cfg(target_os = "windows")]
fn clamp_overlay_rect(
    rect: Win32OverlaySelectionRect,
    width: i32,
    height: i32,
) -> Option<(i32, i32, i32, i32)> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let left = rect.left.min(rect.right).max(0).min(width);
    let right = rect.left.max(rect.right).max(0).min(width);
    let top = rect.top.min(rect.bottom).max(0).min(height);
    let bottom = rect.top.max(rect.bottom).max(0).min(height);
    if right <= left || bottom <= top {
        return None;
    }
    Some((left, top, right, bottom))
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
const WM_NCDESTROY: u32 = 0x0082;
#[cfg(target_os = "windows")]
const HTCLIENT: isize = 1;
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
const CANDIDATE_BORDER_COLORREF: u32 = 0x00FFB000;
#[cfg(target_os = "windows")]
const SELECTION_BORDER_COLORREF: u32 = 0x00FF7716;

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

    #[test]
    fn native_overlay_hit_test_claims_client_input() {
        assert_eq!(native_overlay_hit_test_result(), HTCLIENT);
    }

    #[test]
    fn clamp_overlay_rect_clips_to_bitmap_bounds() {
        assert_eq!(
            clamp_overlay_rect(
                Win32OverlaySelectionRect {
                    left: -10,
                    top: 4,
                    right: 40,
                    bottom: 60,
                },
                32,
                48,
            ),
            Some((0, 4, 32, 48))
        );
        assert_eq!(
            clamp_overlay_rect(
                Win32OverlaySelectionRect {
                    left: 20,
                    top: 20,
                    right: 20,
                    bottom: 40,
                },
                32,
                48,
            ),
            None
        );
    }
}

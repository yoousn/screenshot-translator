#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayState {
    Idle,
    Preparing,
    Visible,
    Closing,
}

impl NativeOverlayState {
    pub fn can_show(self) -> bool {
        matches!(self, Self::Idle | Self::Preparing)
    }

    pub fn can_close(self) -> bool {
        matches!(self, Self::Preparing | Self::Visible)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayOptions {
    pub no_activate: bool,
    pub exclude_from_capture: bool,
    pub topmost: bool,
}

impl Default for NativeOverlayOptions {
    fn default() -> Self {
        Self {
            no_activate: true,
            exclude_from_capture: true,
            topmost: true,
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayWin32Policy {
    pub create_style: u32,
    pub create_ex_style: isize,
    pub apply_swp_flags: u32,
    pub show_swp_flags: u32,
    pub hide_swp_flags: u32,
    pub insert_after: isize,
    pub show_command: i32,
    pub display_affinity: u32,
}

#[cfg(target_os = "windows")]
impl NativeOverlayWin32Policy {
    pub const fn hides_from_taskbar_and_alt_tab(self) -> bool {
        self.create_ex_style & WS_EX_TOOLWINDOW != 0 && self.create_ex_style & WS_EX_APPWINDOW == 0
    }

    pub const fn avoids_activation(self) -> bool {
        self.create_ex_style & WS_EX_NOACTIVATE != 0
            && self.apply_swp_flags & SWP_NOACTIVATE != 0
            && self.show_swp_flags & SWP_NOACTIVATE != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayLifecycleDiagnosticKind {
    TaskbarOrAltTabExposure,
    ActivationRisk,
    RepeatHotkeyCleanupPending,
    FocusRestorePending,
}

impl NativeOverlayLifecycleDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskbarOrAltTabExposure => "taskbar-or-alt-tab-exposure",
            Self::ActivationRisk => "activation-risk",
            Self::RepeatHotkeyCleanupPending => "repeat-hotkey-cleanup-pending",
            Self::FocusRestorePending => "focus-restore-pending",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayLifecycleDiagnostic {
    pub kind: NativeOverlayLifecycleDiagnosticKind,
    pub state: NativeOverlayState,
}

impl NativeOverlayLifecycleDiagnostic {
    pub const fn new(
        kind: NativeOverlayLifecycleDiagnosticKind,
        state: NativeOverlayState,
    ) -> Self {
        Self { kind, state }
    }
}

#[cfg(target_os = "windows")]
impl NativeOverlayOptions {
    pub fn to_win32_policy(self) -> NativeOverlayWin32Policy {
        NativeOverlayWin32Policy {
            create_style: win32_overlay_create_style(),
            create_ex_style: win32_overlay_create_ex_style(self),
            apply_swp_flags: win32_overlay_apply_swp_flags(),
            show_swp_flags: win32_overlay_show_swp_flags(self),
            hide_swp_flags: win32_overlay_hide_swp_flags(),
            insert_after: win32_overlay_insert_after(self),
            show_command: win32_overlay_show_command(self),
            display_affinity: win32_overlay_display_affinity(self.exclude_from_capture),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayWindowStrategy {
    pub options: NativeOverlayOptions,
    pub state: NativeOverlayState,
}

impl Default for NativeOverlayWindowStrategy {
    fn default() -> Self {
        Self::new(NativeOverlayOptions::default())
    }
}

impl NativeOverlayWindowStrategy {
    pub fn new(options: NativeOverlayOptions) -> Self {
        Self {
            options,
            state: NativeOverlayState::Idle,
        }
    }

    pub fn prepare(&mut self) {
        if matches!(self.state, NativeOverlayState::Idle) {
            self.state = NativeOverlayState::Preparing;
        }
    }

    pub fn mark_visible(&mut self) {
        if self.state.can_show() {
            self.state = NativeOverlayState::Visible;
        }
    }

    pub fn mark_closing(&mut self) {
        if self.state.can_close() {
            self.state = NativeOverlayState::Closing;
        }
    }

    pub fn reset(&mut self) {
        self.state = NativeOverlayState::Idle;
    }

    pub fn apply_to_hwnd(self, hwnd: isize) -> Result<(), String> {
        apply_native_overlay_options(hwnd, self.options)
    }

    pub fn show_hwnd(self, hwnd: isize) -> Result<(), String> {
        show_native_overlay_hwnd(hwnd, self.options)
    }

    pub fn hide_hwnd(self, hwnd: isize) -> Result<(), String> {
        hide_native_overlay_hwnd(hwnd, self.options)
    }
}

pub fn apply_native_overlay_options(
    hwnd: isize,
    options: NativeOverlayOptions,
) -> Result<(), String> {
    if hwnd == 0 {
        return Err("native overlay hwnd is empty".to_string());
    }
    apply_platform_overlay_options(hwnd, options)
}

pub fn show_native_overlay_hwnd(hwnd: isize, options: NativeOverlayOptions) -> Result<(), String> {
    if hwnd == 0 {
        return Err("native overlay hwnd is empty".to_string());
    }
    apply_native_overlay_options(hwnd, options)?;
    show_platform_overlay_hwnd(hwnd, options)
}

pub fn hide_native_overlay_hwnd(hwnd: isize, options: NativeOverlayOptions) -> Result<(), String> {
    if hwnd == 0 {
        return Err("native overlay hwnd is empty".to_string());
    }
    hide_platform_overlay_hwnd(hwnd, options)
}

#[cfg(target_os = "windows")]
fn apply_platform_overlay_options(
    hwnd: isize,
    options: NativeOverlayOptions,
) -> Result<(), String> {
    let policy = options.to_win32_policy();
    let mut ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    if ex_style == 0 {
        return Err("GetWindowLongPtrW(GWL_EXSTYLE) failed".to_string());
    }

    ex_style = win32_apply_overlay_ex_style(ex_style, options);

    let previous_style = unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style) };
    if previous_style == 0 {
        return Err("SetWindowLongPtrW(GWL_EXSTYLE) failed".to_string());
    }

    set_capture_exclusion(hwnd, options.exclude_from_capture)?;
    let ok = unsafe {
        SetWindowPos(
            hwnd,
            policy.insert_after,
            0,
            0,
            0,
            0,
            policy.apply_swp_flags,
        )
    };
    if ok == 0 {
        return Err("SetWindowPos(apply overlay strategy) failed".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn apply_platform_overlay_options(
    _hwnd: isize,
    _options: NativeOverlayOptions,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn show_platform_overlay_hwnd(hwnd: isize, options: NativeOverlayOptions) -> Result<(), String> {
    let policy = options.to_win32_policy();
    let ok = unsafe { SetWindowPos(hwnd, policy.insert_after, 0, 0, 0, 0, policy.show_swp_flags) };
    if ok == 0 {
        return Err("SetWindowPos(show overlay) failed".to_string());
    }
    if options.no_activate {
        let _ = unsafe { ShowWindow(hwnd, policy.show_command) };
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn show_platform_overlay_hwnd(_hwnd: isize, _options: NativeOverlayOptions) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn hide_platform_overlay_hwnd(hwnd: isize, options: NativeOverlayOptions) -> Result<(), String> {
    let policy = options.to_win32_policy();
    if options.exclude_from_capture {
        let _ = set_capture_exclusion(hwnd, false);
    }
    let ok = unsafe { SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, policy.hide_swp_flags) };
    if ok == 0 {
        return Err("SetWindowPos(hide overlay) failed".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn hide_platform_overlay_hwnd(_hwnd: isize, _options: NativeOverlayOptions) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_capture_exclusion(hwnd: isize, excluded: bool) -> Result<(), String> {
    let affinity = win32_overlay_display_affinity(excluded);
    let ok = unsafe { SetWindowDisplayAffinity(hwnd, affinity) };
    if ok == 0 {
        return Err("SetWindowDisplayAffinity failed".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn win32_overlay_create_style() -> u32 {
    WS_POPUP
}

#[cfg(target_os = "windows")]
fn win32_overlay_create_ex_style(options: NativeOverlayOptions) -> isize {
    if options.no_activate {
        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
    } else {
        0
    }
}

#[cfg(target_os = "windows")]
fn win32_apply_overlay_ex_style(current_ex_style: isize, options: NativeOverlayOptions) -> isize {
    if options.no_activate {
        (current_ex_style | win32_overlay_create_ex_style(options)) & !WS_EX_APPWINDOW
    } else {
        current_ex_style
    }
}

#[cfg(target_os = "windows")]
fn win32_overlay_insert_after(options: NativeOverlayOptions) -> isize {
    if options.topmost {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    }
}

#[cfg(target_os = "windows")]
fn win32_overlay_apply_swp_flags() -> u32 {
    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED
}

#[cfg(target_os = "windows")]
fn win32_overlay_show_swp_flags(options: NativeOverlayOptions) -> u32 {
    SWP_NOMOVE
        | SWP_NOSIZE
        | SWP_SHOWWINDOW
        | if options.no_activate {
            SWP_NOACTIVATE
        } else {
            0
        }
}

#[cfg(target_os = "windows")]
fn win32_overlay_hide_swp_flags() -> u32 {
    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW
}

#[cfg(target_os = "windows")]
fn win32_overlay_show_command(options: NativeOverlayOptions) -> i32 {
    if options.no_activate {
        SW_SHOWNOACTIVATE
    } else {
        SW_SHOW
    }
}

#[cfg(target_os = "windows")]
fn win32_overlay_display_affinity(excluded: bool) -> u32 {
    if excluded {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    }
}

#[cfg(target_os = "windows")]
const GWL_EXSTYLE: i32 = -20;
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
const SWP_FRAMECHANGED: u32 = 0x0020;
#[cfg(target_os = "windows")]
const SW_SHOWNOACTIVATE: i32 = 4;
#[cfg(target_os = "windows")]
const SW_SHOW: i32 = 5;
#[cfg(target_os = "windows")]
const WS_POPUP: u32 = 0x80000000;
#[cfg(target_os = "windows")]
const WS_EX_NOACTIVATE: isize = 0x08000000;
#[cfg(target_os = "windows")]
const WS_EX_TOOLWINDOW: isize = 0x00000080;
#[cfg(target_os = "windows")]
const WS_EX_APPWINDOW: isize = 0x00040000;
#[cfg(target_os = "windows")]
const WDA_NONE: u32 = 0x00000000;
#[cfg(target_os = "windows")]
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetWindowLongPtrW(hWnd: isize, nIndex: i32) -> isize;
    fn SetWindowLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
    fn SetWindowDisplayAffinity(hWnd: isize, dwAffinity: u32) -> i32;
    fn SetWindowPos(
        hWnd: isize,
        hWndInsertAfter: isize,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> i32;
    fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    const TEST_WS_VISIBLE: u32 = 0x10000000;

    #[test]
    fn default_policy_hides_overlay_from_taskbar_and_alt_tab() {
        let policy = NativeOverlayOptions::default().to_win32_policy();

        assert_eq!(policy.create_style & TEST_WS_VISIBLE, 0);
        assert!(policy.hides_from_taskbar_and_alt_tab());
        assert!(policy.avoids_activation());
    }

    #[test]
    fn applying_overlay_style_removes_appwindow() {
        let current = WS_EX_APPWINDOW | WS_EX_TOOLWINDOW;

        let applied = win32_apply_overlay_ex_style(current, NativeOverlayOptions::default());

        assert_eq!(applied & WS_EX_APPWINDOW, 0);
        assert_ne!(applied & WS_EX_TOOLWINDOW, 0);
    }
}

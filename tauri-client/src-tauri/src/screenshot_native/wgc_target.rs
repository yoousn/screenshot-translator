use std::fmt;

use super::wgc_session::WgcCaptureTarget;
use super::MonitorCaptureBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcTargetKind {
    Monitor,
    Window,
}

impl WgcTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Monitor => "monitor",
            Self::Window => "window",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgcTargetValidationError {
    UnsupportedPlatform,
    ZeroHandle { kind: WgcTargetKind },
    NotAWindow,
    NotVisible,
    Minimized,
    EmptyBounds,
    MonitorUnavailable,
    MonitorInfoUnavailable,
}

impl fmt::Display for WgcTargetValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("WGC target validation requires Windows")
            }
            Self::ZeroHandle { kind } => {
                write!(formatter, "{} handle must be non-zero", kind.as_str())
            }
            Self::NotAWindow => formatter.write_str("HWND is not a live window"),
            Self::NotVisible => formatter.write_str("HWND is not visible"),
            Self::Minimized => formatter.write_str("HWND is minimized"),
            Self::EmptyBounds => formatter.write_str("target bounds are empty"),
            Self::MonitorUnavailable => formatter.write_str("monitor handle is unavailable"),
            Self::MonitorInfoUnavailable => formatter.write_str("monitor info is unavailable"),
        }
    }
}

impl std::error::Error for WgcTargetValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcTargetValidationReport {
    pub target: WgcCaptureTarget,
    pub kind: WgcTargetKind,
    pub valid: bool,
    pub bounds: Option<MonitorCaptureBounds>,
    pub error: Option<WgcTargetValidationError>,
}

impl WgcTargetValidationReport {
    pub fn valid(
        target: WgcCaptureTarget,
        kind: WgcTargetKind,
        bounds: Option<MonitorCaptureBounds>,
    ) -> Self {
        Self {
            target,
            kind,
            valid: true,
            bounds,
            error: None,
        }
    }

    pub fn invalid(
        target: WgcCaptureTarget,
        kind: WgcTargetKind,
        error: WgcTargetValidationError,
    ) -> Self {
        Self {
            target,
            kind,
            valid: false,
            bounds: None,
            error: Some(error),
        }
    }

    pub fn error_message(&self) -> Option<String> {
        self.error.as_ref().map(ToString::to_string)
    }
}

pub fn validate_wgc_capture_target_basics(
    target: WgcCaptureTarget,
) -> Result<(), WgcTargetValidationError> {
    match target {
        WgcCaptureTarget::Monitor { hmonitor } if hmonitor == 0 => {
            Err(WgcTargetValidationError::ZeroHandle {
                kind: WgcTargetKind::Monitor,
            })
        }
        WgcCaptureTarget::Window { hwnd } if hwnd == 0 => {
            Err(WgcTargetValidationError::ZeroHandle {
                kind: WgcTargetKind::Window,
            })
        }
        _ => Ok(()),
    }
}

pub fn validate_wgc_capture_target(target: WgcCaptureTarget) -> WgcTargetValidationReport {
    if let Err(error) = validate_wgc_capture_target_basics(target) {
        return WgcTargetValidationReport::invalid(target, target_kind(target), error);
    }
    validate_wgc_capture_target_platform(target)
}

pub fn resolve_wgc_monitor_target_from_bounds(
    bounds: MonitorCaptureBounds,
) -> WgcTargetValidationReport {
    if bounds.is_empty() || bounds.right().is_none() || bounds.bottom().is_none() {
        return WgcTargetValidationReport::invalid(
            WgcCaptureTarget::Monitor { hmonitor: 0 },
            WgcTargetKind::Monitor,
            WgcTargetValidationError::EmptyBounds,
        );
    }
    resolve_wgc_monitor_target_from_bounds_platform(bounds)
}

fn target_kind(target: WgcCaptureTarget) -> WgcTargetKind {
    match target {
        WgcCaptureTarget::Monitor { .. } => WgcTargetKind::Monitor,
        WgcCaptureTarget::Window { .. } => WgcTargetKind::Window,
    }
}

pub(crate) fn monitor_bounds_from_rect_edges(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Option<MonitorCaptureBounds> {
    if right <= left || bottom <= top {
        return None;
    }
    Some(MonitorCaptureBounds::new(
        left,
        top,
        (right - left) as u32,
        (bottom - top) as u32,
    ))
}

#[cfg(not(windows))]
fn validate_wgc_capture_target_platform(target: WgcCaptureTarget) -> WgcTargetValidationReport {
    WgcTargetValidationReport::invalid(
        target,
        target_kind(target),
        WgcTargetValidationError::UnsupportedPlatform,
    )
}

#[cfg(windows)]
fn validate_wgc_capture_target_platform(target: WgcCaptureTarget) -> WgcTargetValidationReport {
    match target {
        WgcCaptureTarget::Monitor { hmonitor } => validate_monitor_handle(hmonitor),
        WgcCaptureTarget::Window { hwnd } => validate_window_handle(hwnd),
    }
}

#[cfg(not(windows))]
fn resolve_wgc_monitor_target_from_bounds_platform(
    _bounds: MonitorCaptureBounds,
) -> WgcTargetValidationReport {
    WgcTargetValidationReport::invalid(
        WgcCaptureTarget::Monitor { hmonitor: 0 },
        WgcTargetKind::Monitor,
        WgcTargetValidationError::UnsupportedPlatform,
    )
}

#[cfg(windows)]
fn resolve_wgc_monitor_target_from_bounds_platform(
    bounds: MonitorCaptureBounds,
) -> WgcTargetValidationReport {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let center_x = bounds.origin_x.saturating_add((bounds.width / 2) as i32);
    let center_y = bounds.origin_y.saturating_add((bounds.height / 2) as i32);
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: center_x,
                y: center_y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    if monitor.0.is_null() {
        return WgcTargetValidationReport::invalid(
            WgcCaptureTarget::Monitor { hmonitor: 0 },
            WgcTargetKind::Monitor,
            WgcTargetValidationError::MonitorUnavailable,
        );
    }
    let mut info = MONITORINFO::default();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info) };
    if ok.as_bool() {
        let target = WgcCaptureTarget::Monitor {
            hmonitor: monitor.0 as isize,
        };
        if let Some(monitor_bounds) = monitor_bounds_from_rect_edges(
            info.rcMonitor.left,
            info.rcMonitor.top,
            info.rcMonitor.right,
            info.rcMonitor.bottom,
        ) {
            return WgcTargetValidationReport::valid(
                target,
                WgcTargetKind::Monitor,
                Some(monitor_bounds),
            );
        }
    }
    WgcTargetValidationReport::invalid(
        WgcCaptureTarget::Monitor {
            hmonitor: monitor.0 as isize,
        },
        WgcTargetKind::Monitor,
        WgcTargetValidationError::MonitorInfoUnavailable,
    )
}

#[cfg(windows)]
fn validate_monitor_handle(hmonitor: isize) -> WgcTargetValidationReport {
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};

    let monitor = HMONITOR(hmonitor as *mut _);
    let mut info = MONITORINFO::default();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info) };
    if ok.as_bool() {
        if let Some(bounds) = monitor_bounds_from_rect_edges(
            info.rcMonitor.left,
            info.rcMonitor.top,
            info.rcMonitor.right,
            info.rcMonitor.bottom,
        ) {
            return WgcTargetValidationReport::valid(
                WgcCaptureTarget::Monitor { hmonitor },
                WgcTargetKind::Monitor,
                Some(bounds),
            );
        }
    }
    WgcTargetValidationReport::invalid(
        WgcCaptureTarget::Monitor { hmonitor },
        WgcTargetKind::Monitor,
        WgcTargetValidationError::MonitorInfoUnavailable,
    )
}

#[cfg(windows)]
fn validate_window_handle(hwnd: isize) -> WgcTargetValidationReport {
    use crate::win32;

    if unsafe { win32::IsWindow(hwnd) } == 0 {
        return WgcTargetValidationReport::invalid(
            WgcCaptureTarget::Window { hwnd },
            WgcTargetKind::Window,
            WgcTargetValidationError::NotAWindow,
        );
    }
    if unsafe { win32::IsWindowVisible(hwnd) } == 0 {
        return WgcTargetValidationReport::invalid(
            WgcCaptureTarget::Window { hwnd },
            WgcTargetKind::Window,
            WgcTargetValidationError::NotVisible,
        );
    }
    if unsafe { win32::IsIconic(hwnd) } != 0 {
        return WgcTargetValidationReport::invalid(
            WgcCaptureTarget::Window { hwnd },
            WgcTargetKind::Window,
            WgcTargetValidationError::Minimized,
        );
    }
    let mut rect = win32::RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { win32::GetWindowRect(hwnd, &mut rect) } != 0
        && rect.right > rect.left
        && rect.bottom > rect.top
    {
        let bounds = MonitorCaptureBounds::new(
            rect.left,
            rect.top,
            (rect.right - rect.left) as u32,
            (rect.bottom - rect.top) as u32,
        );
        return WgcTargetValidationReport::valid(
            WgcCaptureTarget::Window { hwnd },
            WgcTargetKind::Window,
            Some(bounds),
        );
    }
    WgcTargetValidationReport::invalid(
        WgcCaptureTarget::Window { hwnd },
        WgcTargetKind::Window,
        WgcTargetValidationError::EmptyBounds,
    )
}

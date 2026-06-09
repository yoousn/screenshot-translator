use std::fmt;

use super::d3d11_frame::D3d11TextureFrame;
use super::output::SelectedImageContract;
use super::wgc_contract::{WgcOneFrameProbeDiagnostics, WgcOneFrameProbeRequest};
use super::MonitorCaptureBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcCaptureTarget {
    Monitor { hmonitor: isize },
    Window { hwnd: isize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcOneFrameSessionOptions {
    pub request: WgcOneFrameProbeRequest,
    pub target: WgcCaptureTarget,
    pub width: u32,
    pub height: u32,
    pub requested_bounds: Option<MonitorCaptureBounds>,
    pub target_bounds: Option<MonitorCaptureBounds>,
    pub include_cursor: bool,
    pub require_border: bool,
    pub buffer_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcSelectedMonitorFrameEvidence {
    pub diagnostic_only: bool,
    pub requested_bounds_physical: Option<MonitorCaptureBounds>,
    pub target_monitor_bounds_physical: Option<MonitorCaptureBounds>,
    pub framepool_size_source: &'static str,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub frame_matches_target_monitor_bounds: bool,
    pub selected_crop_within_frame: bool,
    pub selected_png_produced: bool,
    pub readback_bytes_present: bool,
    pub persistent_handle_exposed: bool,
    pub readiness_changed: bool,
}

impl WgcSelectedMonitorFrameEvidence {
    pub fn from_session(
        requested_bounds: Option<MonitorCaptureBounds>,
        target_bounds: Option<MonitorCaptureBounds>,
        frame_width: Option<u32>,
        frame_height: Option<u32>,
        readback_bytes_present: bool,
        selected_png_produced: bool,
    ) -> Self {
        let frame_matches_target_monitor_bounds = target_bounds
            .zip(frame_width.zip(frame_height))
            .map(|(target, (width, height))| target.width == width && target.height == height)
            .unwrap_or(false);
        let selected_crop_within_frame = requested_bounds
            .zip(target_bounds)
            .map(|(requested, target)| selected_bounds_within_target(requested, target))
            .unwrap_or(false);
        Self {
            diagnostic_only: true,
            requested_bounds_physical: requested_bounds,
            target_monitor_bounds_physical: target_bounds,
            framepool_size_source: "target-monitor-bounds",
            frame_width,
            frame_height,
            frame_matches_target_monitor_bounds,
            selected_crop_within_frame,
            selected_png_produced,
            readback_bytes_present,
            persistent_handle_exposed: false,
            readiness_changed: false,
        }
    }
}

fn selected_bounds_within_target(
    requested: MonitorCaptureBounds,
    target: MonitorCaptureBounds,
) -> bool {
    let Some(requested_right) = requested.right() else {
        return false;
    };
    let Some(requested_bottom) = requested.bottom() else {
        return false;
    };
    let Some(target_right) = target.right() else {
        return false;
    };
    let Some(target_bottom) = target.bottom() else {
        return false;
    };
    requested.origin_x >= target.origin_x
        && requested.origin_y >= target.origin_y
        && requested_right <= target_right
        && requested_bottom <= target_bottom
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcSessionState {
    Disabled,
    ApiUnavailable,
    InvalidRequest,
    DeviceReady,
    CaptureItemReady,
    FramePoolReady,
    SessionReady,
    CaptureStarted,
    FrameAcquired,
    TimedOut,
    Failed,
}

impl WgcSessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::ApiUnavailable => "api-unavailable",
            Self::InvalidRequest => "invalid-request",
            Self::DeviceReady => "device-ready",
            Self::CaptureItemReady => "capture-item-ready",
            Self::FramePoolReady => "framepool-ready",
            Self::SessionReady => "session-ready",
            Self::CaptureStarted => "capture-started",
            Self::FrameAcquired => "frame-acquired",
            Self::TimedOut => "timed-out",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcOneFrameSessionReport {
    pub state: WgcSessionState,
    pub attempted_real_wgc_api: bool,
    pub created_device: bool,
    pub created_item: bool,
    pub created_frame_pool: bool,
    pub created_session: bool,
    pub started_capture: bool,
    pub acquired_frame: bool,
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub elapsed_ms: u64,
    pub diagnostics: WgcOneFrameProbeDiagnostics,
    pub selected_monitor_frame_evidence: WgcSelectedMonitorFrameEvidence,
    pub frame: Option<D3d11TextureFrame>,
    pub selected_image: Option<SelectedImageContract>,
    pub error: Option<WgcSessionError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgcSessionError {
    ExplicitOptInRequired,
    RealApiNotAllowed,
    NativeApiUnavailable { reason: String },
    InvalidFrameTimeoutMs { timeout_ms: u64 },
    InvalidDimensions { width: u32, height: u32 },
    InvalidBufferCount { buffer_count: i32 },
    InvalidTarget { reason: String },
    UnsupportedPlatform { reason: String },
    DeviceBridge { reason: String },
    CaptureItem { reason: String },
    FramePool { reason: String },
    CaptureSession { reason: String },
    StartCapture { reason: String },
    FrameTimeout { timeout_ms: u64 },
    FrameSurface { reason: String },
    TextureContract { reason: String },
}

impl fmt::Display for WgcSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitOptInRequired => {
                formatter.write_str("WGC one-frame session requires explicit opt-in")
            }
            Self::RealApiNotAllowed => {
                formatter.write_str("WGC one-frame session real API calls are not allowed")
            }
            Self::NativeApiUnavailable { reason } => {
                write!(formatter, "WGC native API is unavailable: {reason}")
            }
            Self::InvalidFrameTimeoutMs { timeout_ms } => {
                write!(
                    formatter,
                    "invalid WGC one-frame session timeout: {timeout_ms}ms"
                )
            }
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "invalid WGC session dimensions: {width}x{height}"
                )
            }
            Self::InvalidBufferCount { buffer_count } => {
                write!(
                    formatter,
                    "invalid WGC framepool buffer count: {buffer_count}"
                )
            }
            Self::InvalidTarget { reason } => write!(formatter, "invalid WGC target: {reason}"),
            Self::UnsupportedPlatform { reason } => formatter.write_str(reason),
            Self::DeviceBridge { reason } => {
                write!(formatter, "WGC device bridge failed: {reason}")
            }
            Self::CaptureItem { reason } => write!(formatter, "WGC capture item failed: {reason}"),
            Self::FramePool { reason } => write!(formatter, "WGC framepool failed: {reason}"),
            Self::CaptureSession { reason } => {
                write!(formatter, "WGC capture session failed: {reason}")
            }
            Self::StartCapture { reason } => write!(formatter, "WGC StartCapture failed: {reason}"),
            Self::FrameTimeout { timeout_ms } => {
                write!(
                    formatter,
                    "WGC one-frame session timed out after {timeout_ms}ms"
                )
            }
            Self::FrameSurface { reason } => {
                write!(formatter, "WGC frame surface failed: {reason}")
            }
            Self::TextureContract { reason } => {
                write!(formatter, "WGC texture contract failed: {reason}")
            }
        }
    }
}

impl std::error::Error for WgcSessionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_monitor_frame_evidence_matches_target_monitor() {
        let requested = MonitorCaptureBounds::new(100, 100, 320, 200);
        let target = MonitorCaptureBounds::new(0, 0, 1920, 1080);

        let evidence = WgcSelectedMonitorFrameEvidence::from_session(
            Some(requested),
            Some(target),
            Some(1920),
            Some(1080),
            false,
            false,
        );

        assert!(evidence.diagnostic_only);
        assert!(evidence.frame_matches_target_monitor_bounds);
        assert!(evidence.selected_crop_within_frame);
        assert!(!evidence.selected_png_produced);
        assert!(!evidence.persistent_handle_exposed);
        assert!(!evidence.readiness_changed);
    }

    #[test]
    fn selected_monitor_frame_evidence_rejects_selected_sized_frame() {
        let requested = MonitorCaptureBounds::new(100, 100, 320, 200);
        let target = MonitorCaptureBounds::new(0, 0, 1920, 1080);

        let evidence = WgcSelectedMonitorFrameEvidence::from_session(
            Some(requested),
            Some(target),
            Some(320),
            Some(200),
            false,
            false,
        );

        assert!(!evidence.frame_matches_target_monitor_bounds);
        assert!(evidence.selected_crop_within_frame);
    }

    #[test]
    fn selected_monitor_frame_evidence_detects_out_of_target_crop() {
        let requested = MonitorCaptureBounds::new(-10, 100, 320, 200);
        let target = MonitorCaptureBounds::new(0, 0, 1920, 1080);

        let evidence = WgcSelectedMonitorFrameEvidence::from_session(
            Some(requested),
            Some(target),
            Some(1920),
            Some(1080),
            false,
            false,
        );

        assert!(evidence.frame_matches_target_monitor_bounds);
        assert!(!evidence.selected_crop_within_frame);
    }
}

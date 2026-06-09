use super::wgc_contract::WgcOneFrameProbeRequest;
use super::wgc_session::*;

const LIVE_SMOKE_ENV: &str = "YSN_WGC_SESSION_LIVE_SMOKE";

fn live_smoke_enabled() -> bool {
    std::env::var(LIVE_SMOKE_ENV).ok().as_deref() == Some("1")
}

fn explicit_real_options() -> WgcOneFrameSessionOptions {
    WgcOneFrameSessionOptions {
        request: WgcOneFrameProbeRequest {
            explicit_opt_in: true,
            allow_real_wgc_api: true,
            frame_timeout_ms: 250,
        },
        target: WgcCaptureTarget::Monitor { hmonitor: 1 },
        width: 1920,
        height: 1080,
        requested_bounds: Some(super::MonitorCaptureBounds::new(100, 100, 320, 200)),
        target_bounds: Some(super::MonitorCaptureBounds::new(0, 0, 1920, 1080)),
        include_cursor: false,
        require_border: false,
        buffer_count: 1,
    }
}

#[test]
fn default_session_options_do_not_attempt_real_wgc_api() {
    let options = default_wgc_one_frame_session_options(
        WgcCaptureTarget::Monitor { hmonitor: 1 },
        1920,
        1080,
    );
    let report = guarded_wgc_one_frame_session(options);

    assert_eq!(report.state, WgcSessionState::Disabled);
    assert!(!report.attempted_real_wgc_api);
    assert!(!report.created_device);
    assert!(!report.created_frame_pool);
    assert!(!report.started_capture);
    assert!(!report.acquired_frame);
    assert!(
        !report
            .selected_monitor_frame_evidence
            .frame_matches_target_monitor_bounds
    );
    assert!(
        !report
            .selected_monitor_frame_evidence
            .selected_crop_within_frame
    );
    assert!(!report.selected_monitor_frame_evidence.selected_png_produced);
    assert!(matches!(
        report.error,
        Some(WgcSessionError::ExplicitOptInRequired)
    ));
}

#[test]
fn explicit_placeholder_still_blocks_real_api() {
    let mut options =
        default_wgc_one_frame_session_options(WgcCaptureTarget::Window { hwnd: 1 }, 1280, 720);
    options.request = WgcOneFrameProbeRequest::explicit_placeholder(250);
    let report = guarded_wgc_one_frame_session(options);

    assert_eq!(report.state, WgcSessionState::Disabled);
    assert!(!report.attempted_real_wgc_api);
    assert!(matches!(
        report.error,
        Some(WgcSessionError::RealApiNotAllowed)
    ));
}

#[test]
fn invalid_timeout_fails_before_real_api_attempt() {
    let mut options = explicit_real_options();
    options.request.frame_timeout_ms = 0;
    let report = guarded_wgc_one_frame_session(options);

    assert_eq!(report.state, WgcSessionState::InvalidRequest);
    assert!(!report.attempted_real_wgc_api);
    assert!(matches!(
        report.error,
        Some(WgcSessionError::InvalidFrameTimeoutMs { timeout_ms: 0 })
    ));
}

#[test]
fn invalid_dimensions_fail_before_real_api_attempt() {
    let mut options = explicit_real_options();
    options.width = 0;
    let report = guarded_wgc_one_frame_session(options);

    assert_eq!(report.state, WgcSessionState::InvalidRequest);
    assert!(!report.attempted_real_wgc_api);
    assert!(matches!(
        report.error,
        Some(WgcSessionError::InvalidDimensions {
            width: 0,
            height: 1080
        })
    ));
}

#[test]
fn invalid_target_handle_fails_before_real_api_attempt() {
    let mut options = explicit_real_options();
    options.target = WgcCaptureTarget::Window { hwnd: 0 };
    let report = guarded_wgc_one_frame_session(options);

    assert_eq!(report.state, WgcSessionState::InvalidRequest);
    assert!(!report.attempted_real_wgc_api);
    assert!(matches!(
        report.error,
        Some(WgcSessionError::InvalidTarget { .. })
    ));
}

#[test]
#[ignore = "requires real WGC session and YSN_WGC_SESSION_LIVE_SMOKE=1"]
fn unsupported_api_remains_fallback_without_frame_claim() {
    if !live_smoke_enabled() {
        eprintln!("skipping WGC session live smoke; set {LIVE_SMOKE_ENV}=1 to run");
        return;
    }

    let report = guarded_wgc_one_frame_session(explicit_real_options());
    if report.acquired_frame {
        assert_eq!(report.state, WgcSessionState::FrameAcquired);
        assert!(report.frame.is_some());
        assert_eq!(report.frame_id, 1);
        assert!(report.width > 0);
        assert!(report.height > 0);
        assert!(
            report
                .selected_monitor_frame_evidence
                .frame_matches_target_monitor_bounds
        );
        assert!(
            report
                .selected_monitor_frame_evidence
                .selected_crop_within_frame
        );
        assert!(!report.selected_monitor_frame_evidence.selected_png_produced);
    } else {
        assert!(matches!(
            report.state,
            WgcSessionState::ApiUnavailable
                | WgcSessionState::TimedOut
                | WgcSessionState::Failed
                | WgcSessionState::DeviceReady
                | WgcSessionState::CaptureItemReady
                | WgcSessionState::FramePoolReady
                | WgcSessionState::SessionReady
                | WgcSessionState::CaptureStarted
        ));
        assert!(!report.acquired_frame);
        assert!(report.frame.is_none());
    }
}

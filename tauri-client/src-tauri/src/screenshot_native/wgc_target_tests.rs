use super::wgc_session::WgcCaptureTarget;
use super::wgc_target::*;
use super::MonitorCaptureBounds;

#[test]
fn target_kind_labels_are_stable() {
    assert_eq!(WgcTargetKind::Monitor.as_str(), "monitor");
    assert_eq!(WgcTargetKind::Window.as_str(), "window");
}

#[test]
fn zero_monitor_handle_fails_basic_validation() {
    let error = validate_wgc_capture_target_basics(WgcCaptureTarget::Monitor { hmonitor: 0 })
        .expect_err("zero monitor rejected");

    assert_eq!(
        error,
        WgcTargetValidationError::ZeroHandle {
            kind: WgcTargetKind::Monitor
        }
    );
}

#[test]
fn zero_window_handle_fails_basic_validation() {
    let error = validate_wgc_capture_target_basics(WgcCaptureTarget::Window { hwnd: 0 })
        .expect_err("zero hwnd rejected");

    assert_eq!(
        error,
        WgcTargetValidationError::ZeroHandle {
            kind: WgcTargetKind::Window
        }
    );
}

#[test]
fn empty_monitor_bounds_do_not_attempt_platform_resolution() {
    let report = resolve_wgc_monitor_target_from_bounds(MonitorCaptureBounds::new(0, 0, 0, 1));

    assert!(!report.valid);
    assert_eq!(report.kind, WgcTargetKind::Monitor);
    assert_eq!(report.error, Some(WgcTargetValidationError::EmptyBounds));
}

#[test]
fn overflowing_monitor_bounds_are_rejected_before_platform_resolution() {
    let report =
        resolve_wgc_monitor_target_from_bounds(MonitorCaptureBounds::new(i32::MAX, 0, 2, 1));

    assert!(!report.valid);
    assert_eq!(report.error, Some(WgcTargetValidationError::EmptyBounds));
}

#[test]
fn monitor_rect_edges_convert_to_desktop_bounds() {
    assert_eq!(
        monitor_bounds_from_rect_edges(-1920, 0, 0, 1080),
        Some(MonitorCaptureBounds::new(-1920, 0, 1920, 1080))
    );
    assert_eq!(monitor_bounds_from_rect_edges(10, 10, 10, 20), None);
    assert_eq!(monitor_bounds_from_rect_edges(10, 10, 20, 10), None);
}

#[cfg(windows)]
#[test]
fn resolved_monitor_bounds_match_validated_monitor_handle_bounds() {
    let requested_selection = MonitorCaptureBounds::new(0, 0, 1, 1);
    let resolution = resolve_wgc_monitor_target_from_bounds(requested_selection);

    if resolution.valid {
        let validation = validate_wgc_capture_target(resolution.target);
        assert!(validation.valid);
        assert_eq!(resolution.bounds, validation.bounds);
        if let Some(bounds) = resolution.bounds {
            if bounds.width > 1 || bounds.height > 1 {
                assert_ne!(bounds, requested_selection);
            }
        }
    } else {
        assert_eq!(resolution.kind, WgcTargetKind::Monitor);
    }
}

#[test]
fn non_zero_basic_targets_pass_preflight() {
    validate_wgc_capture_target_basics(WgcCaptureTarget::Monitor { hmonitor: 1 })
        .expect("non-zero monitor preflight");
    validate_wgc_capture_target_basics(WgcCaptureTarget::Window { hwnd: 1 })
        .expect("non-zero hwnd preflight");
}

#[test]
fn platform_validation_never_reports_zero_handle_valid() {
    let report = validate_wgc_capture_target(WgcCaptureTarget::Window { hwnd: 0 });

    assert!(!report.valid);
    assert_eq!(report.target, WgcCaptureTarget::Window { hwnd: 0 });
    assert_eq!(
        report.error,
        Some(WgcTargetValidationError::ZeroHandle {
            kind: WgcTargetKind::Window
        })
    );
}

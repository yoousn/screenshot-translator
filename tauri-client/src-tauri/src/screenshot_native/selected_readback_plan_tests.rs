use super::capture::MonitorCaptureBounds;
use super::monitor_output_mapping::MonitorOutputSelectionMappingError;
use super::output::ImageBounds;
use super::selected_readback_plan::{
    plan_selected_readback_from_desktop_bounds, SelectedReadbackPlanBackend,
    SelectedReadbackPlanError,
};

#[test]
fn plans_negative_origin_wgc_monitor_local_crop() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::WgcMonitor,
        MonitorCaptureBounds::new(-1900, 25, 300, 200),
        MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect("plan");

    assert_eq!(plan.backend.as_str(), "wgc-monitor");
    assert_eq!(plan.mapping.monitor_local_selection.x, 20);
    assert_eq!(plan.mapping.monitor_local_selection.y, 25);
    assert_eq!(plan.mapping.crop.x, 20);
    assert_eq!(plan.mapping.crop.y, 25);
    assert_eq!(plan.mapping.crop.width, 300);
    assert_eq!(plan.mapping.crop.height, 200);
    assert_eq!(plan.requested_target_intersection_ratio, 1.0);
    assert!(plan.crop_within_target());
    assert!(plan.frame_pool_matches_capture_item);
    assert!(plan.selected_output_ready());
    assert!(plan.diagnostic_only);
    assert!(!plan.readiness_changed);
}

#[test]
fn reports_dxgi_cross_monitor_overflow_and_intersection_ratio() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        MonitorCaptureBounds::new(-100, 10, 250, 100),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect("plan");

    assert_eq!(plan.backend.as_str(), "dxgi-output");
    assert_eq!(plan.crop_overflow_physical.left, 100);
    assert_eq!(plan.crop_overflow_physical.top, 0);
    assert_eq!(plan.crop_overflow_physical.right, 0);
    assert_eq!(plan.crop_overflow_physical.bottom, 0);
    assert_eq!(plan.mapping.crop.x, 0);
    assert_eq!(plan.mapping.crop.width, 150);
    assert!((plan.requested_target_intersection_ratio - 0.6).abs() < f64::EPSILON);
    assert!(!plan.crop_within_target());
    assert!(!plan.selected_output_ready());
}

#[test]
fn maps_dxgi_negative_origin_output_desktop_coordinates() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        MonitorCaptureBounds::new(-1910, 20, 300, 200),
        MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect("dxgi negative-origin output plan");

    assert_eq!(plan.mapping.monitor_local_selection.x, 10);
    assert_eq!(plan.mapping.monitor_local_selection.y, 20);
    assert_eq!(plan.mapping.crop.x, 10);
    assert_eq!(plan.mapping.crop.y, 20);
    assert_eq!(plan.mapping.crop.width, 300);
    assert_eq!(plan.mapping.crop.height, 200);
    assert_eq!(plan.requested_target_intersection_ratio, 1.0);
    assert!(plan.selected_output_ready());
}

#[test]
fn maps_dxgi_cross_output_selection_into_left_output_crop() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        MonitorCaptureBounds::new(-50, 100, 200, 120),
        MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect("dxgi cross-output left plan");

    assert_eq!(plan.mapping.crop.x, 1870);
    assert_eq!(plan.mapping.crop.y, 100);
    assert_eq!(plan.mapping.crop.width, 50);
    assert_eq!(plan.mapping.crop.height, 120);
    assert_eq!(plan.crop_overflow_physical.right, 150);
    assert!((plan.requested_target_intersection_ratio - 0.25).abs() < f64::EPSILON);
    assert!(!plan.crop_within_target());
    assert!(!plan.selected_output_ready());
}

#[test]
fn ready_only_when_selected_bounds_match_target_frame() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect("plan");

    assert!(plan.crop_within_target());
    assert!(plan.frame_pool_matches_capture_item);
    assert!(plan.mapping.frame_matches_monitor_bounds);
    assert!(plan.selected_output_ready());
    assert_eq!(
        plan.session_frame_pool_requested_size,
        ImageBounds::new(1920, 1080)
    );
    assert_eq!(
        plan.capture_item_expected_size,
        ImageBounds::new(1920, 1080)
    );
}

#[test]
fn reports_frame_size_mismatch_without_runtime_claim() {
    let plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::WgcMonitor,
        MonitorCaptureBounds::new(1200, 650, 500, 300),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1280, 720),
    )
    .expect("plan");

    assert_eq!(plan.mapping.crop.x, 1200);
    assert_eq!(plan.mapping.crop.y, 650);
    assert_eq!(plan.mapping.crop.width, 80);
    assert_eq!(plan.mapping.crop.height, 70);
    assert!(!plan.mapping.frame_matches_monitor_bounds);
    assert!(plan.frame_pool_matches_capture_item);
    assert!(!plan.selected_output_ready());
    assert!(plan.diagnostic_only);
    assert!(!plan.readiness_changed);
}

#[test]
fn rejects_empty_requested_bounds_before_mapping() {
    let error = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::WgcMonitor,
        MonitorCaptureBounds::new(0, 0, 0, 100),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect_err("empty rejected");

    assert_eq!(error, SelectedReadbackPlanError::EmptyRequestedBounds);
}

#[test]
fn rejects_requested_bounds_overflow_before_mapping() {
    let error = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        MonitorCaptureBounds::new(i32::MAX, 0, 2, 100),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect_err("overflow rejected");

    assert_eq!(error, SelectedReadbackPlanError::RequestedBoundsOverflow);
}

#[test]
fn preserves_mapping_error_for_selection_outside_target() {
    let error = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::WgcMonitor,
        MonitorCaptureBounds::new(-300, 100, 100, 100),
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
    )
    .expect_err("outside rejected");

    assert_eq!(
        error,
        SelectedReadbackPlanError::Mapping(
            MonitorOutputSelectionMappingError::SelectionOutsideMonitor
        )
    );
}

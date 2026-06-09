use super::capture::MonitorCaptureBounds;
use super::monitor_output_mapping::{
    map_desktop_selection_to_output_frame, MonitorOutputSelectionMappingError,
};
use super::output::{ImageBounds, SelectionRect};

#[test]
fn maps_negative_origin_desktop_selection_to_monitor_local_crop() {
    let mapping = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
        SelectionRect::new(-1900, 25, 300, 200),
    )
    .expect("mapping");

    assert_eq!(
        mapping.monitor_local_selection,
        SelectionRect::new(20, 25, 300, 200)
    );
    assert_eq!(mapping.crop.x, 20);
    assert_eq!(mapping.crop.y, 25);
    assert_eq!(mapping.crop.width, 300);
    assert_eq!(mapping.crop.height, 200);
    assert!(mapping.fully_inside_monitor());
    assert!(mapping.frame_matches_monitor_bounds);
}

#[test]
fn clamps_cross_monitor_selection_to_target_monitor() {
    let mapping = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
        SelectionRect::new(-100, 10, 250, 100),
    )
    .expect("mapping");

    assert_eq!(
        mapping.monitor_local_selection,
        SelectionRect::new(0, 10, 150, 100)
    );
    assert_eq!(mapping.crop.x, 0);
    assert_eq!(mapping.crop.y, 10);
    assert_eq!(mapping.crop.width, 150);
    assert_eq!(mapping.crop.height, 100);
    assert!(!mapping.fully_inside_monitor());
}

#[test]
fn normalizes_drag_direction_before_mapping() {
    let mapping = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(100, 50, 800, 600),
        ImageBounds::new(800, 600),
        SelectionRect::new(500, 350, -200, -150),
    )
    .expect("mapping");

    assert_eq!(
        mapping.desktop_selection,
        SelectionRect::new(300, 200, 200, 150)
    );
    assert_eq!(
        mapping.monitor_local_selection,
        SelectionRect::new(200, 150, 200, 150)
    );
    assert_eq!(mapping.crop.width, 200);
    assert_eq!(mapping.crop.height, 150);
}

#[test]
fn rejects_selection_outside_monitor() {
    let error = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1920, 1080),
        SelectionRect::new(-300, 100, 100, 100),
    )
    .expect_err("outside rejected");

    assert_eq!(
        error,
        MonitorOutputSelectionMappingError::SelectionOutsideMonitor
    );
}

#[test]
fn rejects_empty_monitor_frame_or_selection() {
    assert_eq!(
        map_desktop_selection_to_output_frame(
            MonitorCaptureBounds::new(0, 0, 0, 1080),
            ImageBounds::new(1920, 1080),
            SelectionRect::new(0, 0, 10, 10),
        )
        .expect_err("empty monitor"),
        MonitorOutputSelectionMappingError::EmptyMonitorBounds
    );
    assert_eq!(
        map_desktop_selection_to_output_frame(
            MonitorCaptureBounds::new(0, 0, 1920, 1080),
            ImageBounds::new(0, 1080),
            SelectionRect::new(0, 0, 10, 10),
        )
        .expect_err("empty frame"),
        MonitorOutputSelectionMappingError::EmptyFrameBounds
    );
    assert_eq!(
        map_desktop_selection_to_output_frame(
            MonitorCaptureBounds::new(0, 0, 1920, 1080),
            ImageBounds::new(1920, 1080),
            SelectionRect::new(0, 0, 0, 10),
        )
        .expect_err("empty selection"),
        MonitorOutputSelectionMappingError::InvalidDesktopSelection
    );
}

#[test]
fn reports_frame_size_mismatch_and_clamps_to_actual_frame() {
    let mapping = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(0, 0, 1920, 1080),
        ImageBounds::new(1280, 720),
        SelectionRect::new(1200, 650, 500, 300),
    )
    .expect("mapping");

    assert_eq!(
        mapping.monitor_local_selection,
        SelectionRect::new(1200, 650, 500, 300)
    );
    assert_eq!(mapping.crop.x, 1200);
    assert_eq!(mapping.crop.y, 650);
    assert_eq!(mapping.crop.width, 80);
    assert_eq!(mapping.crop.height, 70);
    assert!(!mapping.frame_matches_monitor_bounds);
    assert!(!mapping.fully_inside_monitor());
}

#[test]
fn rejects_overflowing_monitor_bounds_before_mapping() {
    let error = map_desktop_selection_to_output_frame(
        MonitorCaptureBounds::new(i32::MAX, 0, 2, 1080),
        ImageBounds::new(2, 1080),
        SelectionRect::new(i32::MAX, 0, 1, 1),
    )
    .expect_err("overflow rejected");

    assert_eq!(
        error,
        MonitorOutputSelectionMappingError::MonitorBoundsOverflow
    );
}

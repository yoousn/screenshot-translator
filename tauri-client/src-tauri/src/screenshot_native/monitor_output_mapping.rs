use std::fmt;

use super::capture::MonitorCaptureBounds;
use super::output::{CropRect, ImageBounds, SelectionRect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorOutputSelectionMappingError {
    EmptyMonitorBounds,
    MonitorBoundsOverflow,
    EmptyFrameBounds,
    InvalidDesktopSelection,
    SelectionOutsideMonitor,
    LocalSelectionOverflow,
}

impl fmt::Display for MonitorOutputSelectionMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMonitorBounds => formatter.write_str("monitor bounds are empty"),
            Self::MonitorBoundsOverflow => {
                formatter.write_str("monitor bounds overflow desktop coordinate space")
            }
            Self::EmptyFrameBounds => formatter.write_str("monitor frame bounds are empty"),
            Self::InvalidDesktopSelection => {
                formatter.write_str("desktop selection must have non-zero area")
            }
            Self::SelectionOutsideMonitor => {
                formatter.write_str("desktop selection does not intersect monitor bounds")
            }
            Self::LocalSelectionOverflow => {
                formatter.write_str("monitor-local selection exceeds supported coordinate range")
            }
        }
    }
}

impl std::error::Error for MonitorOutputSelectionMappingError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorOutputSelectionMapping {
    pub monitor_bounds: MonitorCaptureBounds,
    pub frame_bounds: ImageBounds,
    pub desktop_selection: SelectionRect,
    pub monitor_local_selection: SelectionRect,
    pub crop: CropRect,
    pub was_clamped_to_monitor: bool,
    pub frame_matches_monitor_bounds: bool,
}

impl MonitorOutputSelectionMapping {
    pub fn fully_inside_monitor(self) -> bool {
        !self.was_clamped_to_monitor
    }
}

pub fn map_desktop_selection_to_output_frame(
    monitor_bounds: MonitorCaptureBounds,
    frame_bounds: ImageBounds,
    desktop_selection: SelectionRect,
) -> Result<MonitorOutputSelectionMapping, MonitorOutputSelectionMappingError> {
    if monitor_bounds.is_empty() {
        return Err(MonitorOutputSelectionMappingError::EmptyMonitorBounds);
    }
    let monitor_right = monitor_bounds
        .right()
        .ok_or(MonitorOutputSelectionMappingError::MonitorBoundsOverflow)?;
    let monitor_bottom = monitor_bounds
        .bottom()
        .ok_or(MonitorOutputSelectionMappingError::MonitorBoundsOverflow)?;
    if frame_bounds.is_empty() {
        return Err(MonitorOutputSelectionMappingError::EmptyFrameBounds);
    }

    let desktop_selection = desktop_selection.normalized();
    if !desktop_selection.is_valid() {
        return Err(MonitorOutputSelectionMappingError::InvalidDesktopSelection);
    }

    let monitor_left = i64::from(monitor_bounds.origin_x);
    let monitor_top = i64::from(monitor_bounds.origin_y);
    let monitor_right = i64::from(monitor_right);
    let monitor_bottom = i64::from(monitor_bottom);
    let selection_left = i64::from(desktop_selection.x);
    let selection_top = i64::from(desktop_selection.y);
    let selection_right = desktop_selection.right();
    let selection_bottom = desktop_selection.bottom();

    let intersection_left = selection_left.max(monitor_left);
    let intersection_top = selection_top.max(monitor_top);
    let intersection_right = selection_right.min(monitor_right);
    let intersection_bottom = selection_bottom.min(monitor_bottom);

    if intersection_right <= intersection_left || intersection_bottom <= intersection_top {
        return Err(MonitorOutputSelectionMappingError::SelectionOutsideMonitor);
    }

    let local_x = checked_i32(intersection_left - monitor_left)?;
    let local_y = checked_i32(intersection_top - monitor_top)?;
    let local_width = checked_i32(intersection_right - intersection_left)?;
    let local_height = checked_i32(intersection_bottom - intersection_top)?;
    let monitor_local_selection = SelectionRect::new(local_x, local_y, local_width, local_height);

    let crop = monitor_local_selection
        .clamp_to(frame_bounds)
        .ok_or(MonitorOutputSelectionMappingError::SelectionOutsideMonitor)?
        .crop;
    let was_clamped_to_monitor = intersection_left != selection_left
        || intersection_top != selection_top
        || intersection_right != selection_right
        || intersection_bottom != selection_bottom
        || crop.as_selection_rect() != monitor_local_selection;

    Ok(MonitorOutputSelectionMapping {
        monitor_bounds,
        frame_bounds,
        desktop_selection,
        monitor_local_selection,
        crop,
        was_clamped_to_monitor,
        frame_matches_monitor_bounds: monitor_bounds.width == frame_bounds.width
            && monitor_bounds.height == frame_bounds.height,
    })
}

fn checked_i32(value: i64) -> Result<i32, MonitorOutputSelectionMappingError> {
    i32::try_from(value).map_err(|_| MonitorOutputSelectionMappingError::LocalSelectionOverflow)
}

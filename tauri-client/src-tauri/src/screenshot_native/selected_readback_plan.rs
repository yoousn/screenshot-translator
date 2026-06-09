use std::fmt;

use super::capture::MonitorCaptureBounds;
use super::monitor_output_mapping::{
    map_desktop_selection_to_output_frame, MonitorOutputSelectionMapping,
    MonitorOutputSelectionMappingError,
};
use super::output::{ImageBounds, SelectionRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedReadbackPlanBackend {
    WgcMonitor,
    DxgiOutput,
}

impl SelectedReadbackPlanBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WgcMonitor => "wgc-monitor",
            Self::DxgiOutput => "dxgi-output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectedReadbackPlanError {
    EmptyRequestedBounds,
    RequestedBoundsOverflow,
    Mapping(MonitorOutputSelectionMappingError),
}

impl fmt::Display for SelectedReadbackPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRequestedBounds => {
                formatter.write_str("requested selected bounds are empty")
            }
            Self::RequestedBoundsOverflow => formatter
                .write_str("requested selected bounds exceed supported selection coordinates"),
            Self::Mapping(error) => write!(formatter, "selected readback mapping failed: {error}"),
        }
    }
}

impl std::error::Error for SelectedReadbackPlanError {}

impl From<MonitorOutputSelectionMappingError> for SelectedReadbackPlanError {
    fn from(error: MonitorOutputSelectionMappingError) -> Self {
        Self::Mapping(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalOverflowPixels {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl PhysicalOverflowPixels {
    pub const fn none() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    pub const fn has_overflow(self) -> bool {
        self.left > 0 || self.top > 0 || self.right > 0 || self.bottom > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectedReadbackPlan {
    pub backend: SelectedReadbackPlanBackend,
    pub requested_bounds_physical: MonitorCaptureBounds,
    pub target_bounds_physical: MonitorCaptureBounds,
    pub output_frame_bounds: ImageBounds,
    pub mapping: MonitorOutputSelectionMapping,
    pub crop_overflow_physical: PhysicalOverflowPixels,
    pub requested_target_intersection_ratio: f64,
    pub session_frame_pool_requested_size: ImageBounds,
    pub capture_item_expected_size: ImageBounds,
    pub frame_pool_matches_capture_item: bool,
    pub diagnostic_only: bool,
    pub readiness_changed: bool,
}

impl SelectedReadbackPlan {
    pub fn crop_within_target(self) -> bool {
        !self.crop_overflow_physical.has_overflow()
    }

    pub fn selected_output_ready(self) -> bool {
        self.crop_within_target()
            && self.frame_pool_matches_capture_item
            && self.mapping.frame_matches_monitor_bounds
    }
}

pub fn plan_selected_readback_from_desktop_bounds(
    backend: SelectedReadbackPlanBackend,
    requested_bounds_physical: MonitorCaptureBounds,
    target_bounds_physical: MonitorCaptureBounds,
    output_frame_bounds: ImageBounds,
) -> Result<SelectedReadbackPlan, SelectedReadbackPlanError> {
    if requested_bounds_physical.is_empty() {
        return Err(SelectedReadbackPlanError::EmptyRequestedBounds);
    }
    if requested_bounds_physical.right().is_none() || requested_bounds_physical.bottom().is_none() {
        return Err(SelectedReadbackPlanError::RequestedBoundsOverflow);
    }
    let width = i32::try_from(requested_bounds_physical.width)
        .map_err(|_| SelectedReadbackPlanError::RequestedBoundsOverflow)?;
    let height = i32::try_from(requested_bounds_physical.height)
        .map_err(|_| SelectedReadbackPlanError::RequestedBoundsOverflow)?;
    let desktop_selection = SelectionRect::new(
        requested_bounds_physical.origin_x,
        requested_bounds_physical.origin_y,
        width,
        height,
    );
    let mapping = map_desktop_selection_to_output_frame(
        target_bounds_physical,
        output_frame_bounds,
        desktop_selection,
    )?;
    let crop_overflow_physical = crop_overflow(requested_bounds_physical, target_bounds_physical)?;
    let requested_target_intersection_ratio = intersection_ratio(
        requested_bounds_physical,
        target_bounds_physical,
        crop_overflow_physical,
    )?;
    let capture_item_expected_size =
        ImageBounds::new(target_bounds_physical.width, target_bounds_physical.height);
    let session_frame_pool_requested_size = capture_item_expected_size;

    Ok(SelectedReadbackPlan {
        backend,
        requested_bounds_physical,
        target_bounds_physical,
        output_frame_bounds,
        mapping,
        crop_overflow_physical,
        requested_target_intersection_ratio,
        session_frame_pool_requested_size,
        capture_item_expected_size,
        frame_pool_matches_capture_item: session_frame_pool_requested_size
            == capture_item_expected_size,
        diagnostic_only: true,
        readiness_changed: false,
    })
}

fn crop_overflow(
    requested: MonitorCaptureBounds,
    target: MonitorCaptureBounds,
) -> Result<PhysicalOverflowPixels, SelectedReadbackPlanError> {
    let requested_left = i64::from(requested.origin_x);
    let requested_top = i64::from(requested.origin_y);
    let requested_right = i64::from(
        requested
            .right()
            .ok_or(SelectedReadbackPlanError::RequestedBoundsOverflow)?,
    );
    let requested_bottom = i64::from(
        requested
            .bottom()
            .ok_or(SelectedReadbackPlanError::RequestedBoundsOverflow)?,
    );
    let target_left = i64::from(target.origin_x);
    let target_top = i64::from(target.origin_y);
    let target_right = i64::from(target.right().ok_or(SelectedReadbackPlanError::Mapping(
        MonitorOutputSelectionMappingError::MonitorBoundsOverflow,
    ))?);
    let target_bottom = i64::from(target.bottom().ok_or(SelectedReadbackPlanError::Mapping(
        MonitorOutputSelectionMappingError::MonitorBoundsOverflow,
    ))?);

    Ok(PhysicalOverflowPixels {
        left: saturating_u32(target_left - requested_left),
        top: saturating_u32(target_top - requested_top),
        right: saturating_u32(requested_right - target_right),
        bottom: saturating_u32(requested_bottom - target_bottom),
    })
}

fn intersection_ratio(
    requested: MonitorCaptureBounds,
    target: MonitorCaptureBounds,
    overflow: PhysicalOverflowPixels,
) -> Result<f64, SelectedReadbackPlanError> {
    let requested_area = u64::from(requested.width) * u64::from(requested.height);
    if requested_area == 0 {
        return Err(SelectedReadbackPlanError::EmptyRequestedBounds);
    }
    let intersection_width = requested
        .width
        .saturating_sub(overflow.left)
        .saturating_sub(overflow.right);
    let intersection_height = requested
        .height
        .saturating_sub(overflow.top)
        .saturating_sub(overflow.bottom);
    let target_area = u64::from(target.width) * u64::from(target.height);
    let intersection_area = (u64::from(intersection_width) * u64::from(intersection_height))
        .min(requested_area)
        .min(target_area);
    Ok(intersection_area as f64 / requested_area as f64)
}

fn saturating_u32(value: i64) -> u32 {
    if value <= 0 {
        0
    } else {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}

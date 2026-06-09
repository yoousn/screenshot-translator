use super::capture::MonitorCaptureBounds;
use super::output::{CropRect, ImageBounds, SelectionRect};
use super::selected_readback_plan::{
    plan_selected_readback_from_desktop_bounds, SelectedReadbackPlan, SelectedReadbackPlanBackend,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DxgiSelectedOutputBridgePlan {
    pub selection: SelectionRect,
    pub crop: CropRect,
    pub selected_readback_plan: SelectedReadbackPlan,
}

impl DxgiSelectedOutputBridgePlan {
    pub fn selected_output_ready_planning_only(self) -> bool {
        self.selected_readback_plan.selected_output_ready()
    }
}

pub fn plan_dxgi_selected_output_bridge(
    bounds: MonitorCaptureBounds,
    output_bounds: MonitorCaptureBounds,
    frame_bounds: ImageBounds,
) -> Result<DxgiSelectedOutputBridgePlan, String> {
    let selected_readback_plan = plan_selected_readback_from_desktop_bounds(
        SelectedReadbackPlanBackend::DxgiOutput,
        bounds,
        output_bounds,
        frame_bounds,
    )
    .map_err(|error| error.to_string())?;
    let crop = selected_readback_plan.mapping.crop;
    Ok(DxgiSelectedOutputBridgePlan {
        selection: crop.as_selection_rect(),
        crop,
        selected_readback_plan,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranked_output_plan_maps_negative_origin_to_output_local_crop() {
        let plan = plan_dxgi_selected_output_bridge(
            MonitorCaptureBounds::new(-1700, 120, 320, 200),
            MonitorCaptureBounds::new(-1920, 0, 1920, 1080),
            ImageBounds::new(1920, 1080),
        )
        .expect("ranked output crop");

        assert_eq!(plan.crop.x, 220);
        assert_eq!(plan.crop.y, 120);
        assert_eq!(plan.crop.width, 320);
        assert_eq!(plan.crop.height, 200);
        assert_eq!(plan.selection.x, 220);
        assert_eq!(plan.selection.y, 120);
        assert_eq!(plan.selection.width, 320);
        assert_eq!(plan.selection.height, 200);
        assert!(plan.selected_output_ready_planning_only());
    }
}

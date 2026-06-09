use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::capture::MonitorCaptureBounds;
use super::dxgi_capture::DxgiDesktopDuplicationBackend;
use super::dxgi_output::DxgiOutputRankingEvidence;
use super::dxgi_output_bridge_plan::plan_dxgi_selected_output_bridge;
use super::output::{CropRect, ImageBounds, SelectedImageContract};
use super::selected_image_bridge::{
    dry_run_selected_output_bridge_contracts, SelectedOutputBridgeDryRunDiagnostic,
    SelectedOutputBridgeDryRunReport,
};
use super::selected_readback_plan::SelectedReadbackPlan;
use super::win32_desktop_update_pulse::{
    run_desktop_update_pulse, DesktopUpdatePulseReport, DesktopUpdatePulseRequest,
};

#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::ID3D11Device;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DxgiSelectedOutputBridgeDryRunStage {
    NotAttempted,
    Started,
    TextureAcquired,
    SelectedReadback,
    BridgeValidated,
    FrameReleased,
    Stopped,
    Failed,
}

impl DxgiSelectedOutputBridgeDryRunStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotAttempted => "not-attempted",
            Self::Started => "started",
            Self::TextureAcquired => "texture-acquired",
            Self::SelectedReadback => "selected-readback",
            Self::BridgeValidated => "bridge-validated",
            Self::FrameReleased => "frame-released",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DxgiSelectedOutputBridgeDryRunReport {
    pub attempted: bool,
    pub ok: bool,
    pub stage: DxgiSelectedOutputBridgeDryRunStage,
    pub elapsed_ms: u128,
    pub frame_id: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub requested_bounds: MonitorCaptureBounds,
    pub output_bounds: Option<MonitorCaptureBounds>,
    pub adapter_index: Option<u32>,
    pub output_index: Option<u32>,
    pub output_ranking: Option<DxgiOutputRankingEvidence>,
    pub desktop_pulse: Option<DesktopUpdatePulseReport>,
    pub crop: Option<CropRect>,
    pub selected_readback_plan: Option<SelectedReadbackPlan>,
    pub selected_output_ready_planning_only: bool,
    pub selected_image: Option<SelectedImageContract>,
    pub bridge: Option<SelectedOutputBridgeDryRunReport>,
    pub action_diagnostics: Vec<SelectedOutputBridgeDryRunDiagnostic>,
    pub bridge_validated: bool,
    pub selected_only: bool,
    pub png_signature_valid: bool,
    pub released_frame: bool,
    pub stopped: bool,
    pub error: Option<String>,
}

impl DxgiSelectedOutputBridgeDryRunReport {
    fn failed(
        stage: DxgiSelectedOutputBridgeDryRunStage,
        elapsed_ms: u128,
        error: impl ToString,
    ) -> Self {
        Self {
            attempted: true,
            ok: false,
            stage: if matches!(stage, DxgiSelectedOutputBridgeDryRunStage::NotAttempted) {
                DxgiSelectedOutputBridgeDryRunStage::Failed
            } else {
                stage
            },
            elapsed_ms,
            frame_id: None,
            width: None,
            height: None,
            format: None,
            requested_bounds: MonitorCaptureBounds::new(0, 0, 0, 0),
            output_bounds: None,
            adapter_index: None,
            output_index: None,
            output_ranking: None,
            desktop_pulse: None,
            crop: None,
            selected_readback_plan: None,
            selected_output_ready_planning_only: false,
            selected_image: None,
            bridge: None,
            action_diagnostics: Vec::new(),
            bridge_validated: false,
            selected_only: false,
            png_signature_valid: false,
            released_frame: false,
            stopped: false,
            error: Some(error.to_string()),
        }
    }
}

pub fn run_dxgi_selected_output_bridge_dry_run(
    bounds: MonitorCaptureBounds,
) -> DxgiSelectedOutputBridgeDryRunReport {
    let started_at = Instant::now();
    if bounds.is_empty() {
        let mut report = DxgiSelectedOutputBridgeDryRunReport::failed(
            DxgiSelectedOutputBridgeDryRunStage::NotAttempted,
            started_at.elapsed().as_millis(),
            format!("empty selected output bridge bounds: {bounds:?}"),
        );
        report.requested_bounds = bounds;
        return report;
    }

    #[cfg(not(windows))]
    {
        let mut report = DxgiSelectedOutputBridgeDryRunReport::failed(
            DxgiSelectedOutputBridgeDryRunStage::NotAttempted,
            started_at.elapsed().as_millis(),
            "DXGI selected output bridge dry-run requires Windows",
        );
        report.requested_bounds = bounds;
        return report;
    }

    #[cfg(windows)]
    {
        run_dxgi_selected_output_bridge_dry_run_windows(bounds, started_at)
    }
}

#[cfg(windows)]
fn run_dxgi_selected_output_bridge_dry_run_windows(
    bounds: MonitorCaptureBounds,
    started_at: Instant,
) -> DxgiSelectedOutputBridgeDryRunReport {
    let mut stage = DxgiSelectedOutputBridgeDryRunStage::NotAttempted;
    let mut backend = DxgiDesktopDuplicationBackend::new();
    if let Err(error) = backend.start_for_bounds(bounds) {
        let mut report = DxgiSelectedOutputBridgeDryRunReport::failed(
            stage,
            started_at.elapsed().as_millis(),
            error,
        );
        report.requested_bounds = bounds;
        return report;
    }
    stage = DxgiSelectedOutputBridgeDryRunStage::Started;
    let output_bounds = backend.output_bounds();
    let output_identity = backend.output_identity();
    let output_ranking = backend.output_ranking().cloned();
    let desktop_pulse = output_bounds.map(|output_bounds| {
        run_desktop_update_pulse(DesktopUpdatePulseRequest::new(output_bounds, 2, 1, 16))
    });

    let frame = match backend.capture_texture_frame(bounds) {
        Ok(frame) => frame,
        Err(error) => {
            let stop_error = backend.stop().err().map(|error| error.to_string());
            let mut report = DxgiSelectedOutputBridgeDryRunReport::failed(
                stage,
                started_at.elapsed().as_millis(),
                error,
            );
            report.requested_bounds = bounds;
            report.output_bounds = output_bounds;
            if let Some(identity) = output_identity {
                report.adapter_index = Some(identity.0);
                report.output_index = Some(identity.1);
            }
            report.output_ranking = output_ranking;
            report.desktop_pulse = desktop_pulse;
            report.stopped = stop_error.is_none();
            if let Some(stop_error) = stop_error {
                report.error = report
                    .error
                    .map(|error| format!("{error}; stop failed: {stop_error}"));
            }
            return report;
        }
    };
    stage = DxgiSelectedOutputBridgeDryRunStage::TextureAcquired;
    let metadata = frame.metadata();
    let plan = output_bounds
        .ok_or_else(|| "DXGI output DesktopCoordinates are unavailable".to_string())
        .and_then(|output_bounds| {
            plan_dxgi_selected_output_bridge(
                bounds,
                output_bounds,
                ImageBounds::new(metadata.width, metadata.height),
            )
        });
    let (selection, crop, selected_readback_plan, selected_output_ready_planning_only) = match plan
    {
        Ok(plan) => {
            stage = DxgiSelectedOutputBridgeDryRunStage::SelectedReadback;
            (
                plan.selection,
                Some(plan.crop),
                Some(plan.selected_readback_plan),
                plan.selected_output_ready_planning_only(),
            )
        }
        Err(error) => {
            let release_error = backend
                .release_acquired_frame()
                .err()
                .map(|error| error.to_string());
            let stop_error = backend.stop().err().map(|error| error.to_string());
            let released_frame = release_error.is_none();
            let stopped = stop_error.is_none();
            let mut report = DxgiSelectedOutputBridgeDryRunReport::failed(
                stage,
                started_at.elapsed().as_millis(),
                combine_errors(Some(error), release_error, stop_error)
                    .unwrap_or_else(|| "selection build failed".to_string()),
            );
            report.requested_bounds = bounds;
            report.output_bounds = output_bounds;
            if let Some(identity) = output_identity {
                report.adapter_index = Some(identity.0);
                report.output_index = Some(identity.1);
            }
            report.output_ranking = output_ranking;
            report.desktop_pulse = desktop_pulse;
            report.released_frame = released_frame;
            report.stopped = stopped;
            return report;
        }
    };

    let bridge = match frame.texture() {
        Ok(texture) => {
            let device: Result<ID3D11Device, String> =
                unsafe { texture.GetDevice() }.map_err(|error| error.to_string());
            device.and_then(|device| {
                let context =
                    unsafe { device.GetImmediateContext() }.map_err(|error| error.to_string())?;
                let image = super::dxgi_readback::build_selected_png_contract_from_dxgi_texture(
                    &device,
                    &context,
                    texture,
                    ImageBounds::new(metadata.width, metadata.height),
                    selection,
                )
                .map_err(|error| error.to_string())?;
                let bridge = dry_run_selected_output_bridge_contracts(&image)
                    .map_err(|error| error.to_string())?;
                Ok((image, bridge))
            })
        }
        Err(error) => Err(error.to_string()),
    };
    let bridge_error = bridge.as_ref().err().cloned();
    if bridge.is_ok() {
        stage = DxgiSelectedOutputBridgeDryRunStage::BridgeValidated;
    }

    let release_error = backend
        .release_acquired_frame()
        .err()
        .map(|error| error.to_string());
    let released_frame = release_error.is_none();
    if released_frame && bridge_error.is_none() {
        stage = DxgiSelectedOutputBridgeDryRunStage::FrameReleased;
    }

    let stop_error = backend.stop().err().map(|error| error.to_string());
    let stopped = stop_error.is_none();
    if stopped && bridge_error.is_none() {
        stage = DxgiSelectedOutputBridgeDryRunStage::Stopped;
    }

    let error = combine_errors(bridge_error, release_error, stop_error);
    let bridge = bridge.ok();
    let selected_image = bridge.as_ref().map(|(image, _)| image.clone());
    let bridge = bridge.map(|(_, bridge)| bridge);
    let bridge_valid = bridge
        .as_ref()
        .map(|bridge| bridge.is_valid())
        .unwrap_or(false);
    let selected_only = bridge
        .as_ref()
        .map(|bridge| bridge.image.selected_only)
        .unwrap_or(false);
    let png_signature_valid = bridge
        .as_ref()
        .map(|bridge| {
            bridge
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.png_signature_valid)
        })
        .unwrap_or(false);
    let action_diagnostics = bridge
        .as_ref()
        .map(|bridge| bridge.diagnostics.clone())
        .unwrap_or_default();
    let ok = error.is_none() && bridge_valid && released_frame && stopped;

    DxgiSelectedOutputBridgeDryRunReport {
        attempted: true,
        ok,
        stage,
        elapsed_ms: started_at.elapsed().as_millis(),
        frame_id: Some(metadata.frame_id),
        width: Some(bounds.width),
        height: Some(bounds.height),
        format: Some(format!("{:?}", metadata.format)),
        requested_bounds: bounds,
        output_bounds,
        adapter_index: output_identity.map(|identity| identity.0),
        output_index: output_identity.map(|identity| identity.1),
        output_ranking,
        desktop_pulse,
        crop,
        selected_readback_plan,
        selected_output_ready_planning_only,
        selected_image,
        bridge,
        action_diagnostics,
        bridge_validated: bridge_valid,
        selected_only,
        png_signature_valid,
        released_frame,
        stopped,
        error,
    }
}

fn combine_errors(
    primary: Option<String>,
    release_error: Option<String>,
    stop_error: Option<String>,
) -> Option<String> {
    let mut errors = Vec::new();
    if let Some(error) = primary {
        errors.push(error);
    }
    if let Some(error) = release_error {
        errors.push(format!("release failed: {error}"));
    }
    if let Some(error) = stop_error {
        errors.push(format!("stop failed: {error}"));
    }
    if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_output_bridge_dry_run_rejects_empty_bounds() {
        let report = run_dxgi_selected_output_bridge_dry_run(MonitorCaptureBounds::new(0, 0, 0, 1));
        assert!(report.attempted);
        assert!(!report.ok);
        assert!(report.error.is_some());
    }

    #[test]
    fn selected_output_bridge_stage_labels_are_stable() {
        assert_eq!(
            DxgiSelectedOutputBridgeDryRunStage::BridgeValidated.as_str(),
            "bridge-validated"
        );
        assert_eq!(
            DxgiSelectedOutputBridgeDryRunStage::Stopped.as_str(),
            "stopped"
        );
    }
}

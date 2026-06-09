use super::super::capture::MonitorCaptureBounds;
use super::super::dxgi_capture::DxgiDesktopDuplicationBackend;
#[cfg(windows)]
use super::super::dxgi_readback::build_selected_png_contract_from_dxgi_texture;
use super::super::output::ImageBounds;
use super::super::selected_readback_plan::{
    plan_selected_readback_from_desktop_bounds, SelectedReadbackPlanBackend,
};
use super::types::{DxgiSelectedReadbackSmokeReport, DxgiSelectedReadbackSmokeStage};
use std::time::Instant;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
#[cfg(windows)]
pub fn run_dxgi_selected_readback_smoke(
    bounds: MonitorCaptureBounds,
) -> DxgiSelectedReadbackSmokeReport {
    let started_at = Instant::now();
    let mut backend = DxgiDesktopDuplicationBackend::new();
    let mut stage = DxgiSelectedReadbackSmokeStage::NotStarted;

    if bounds.is_empty() {
        let mut report = DxgiSelectedReadbackSmokeReport::failed(
            stage,
            started_at.elapsed().as_millis(),
            format!("invalid selected readback bounds: {bounds:?}"),
        );
        report.requested_bounds = bounds;
        return report;
    }

    if let Err(error) = backend.start_for_bounds(bounds) {
        return DxgiSelectedReadbackSmokeReport::failed(
            stage,
            started_at.elapsed().as_millis(),
            error,
        );
    }
    stage = DxgiSelectedReadbackSmokeStage::Started;

    let frame = match backend.capture_texture_frame(bounds) {
        Ok(frame) => frame,
        Err(error) => {
            let stop_error = backend.stop().err().map(|error| error.to_string());
            let mut report = DxgiSelectedReadbackSmokeReport::failed(
                stage,
                started_at.elapsed().as_millis(),
                error,
            );
            report.stopped = stop_error.is_none();
            if let Some(stop_error) = stop_error {
                report.error = report
                    .error
                    .map(|error| format!("{error}; stop failed: {stop_error}"));
            }
            return report;
        }
    };
    stage = DxgiSelectedReadbackSmokeStage::TextureAcquired;
    let metadata = frame.metadata();
    let output_bounds = backend.output_bounds();
    let output_identity = backend.output_identity();
    let plan = output_bounds
        .ok_or_else(|| "DXGI output DesktopCoordinates are unavailable".to_string())
        .and_then(|output_bounds| {
            plan_selected_readback_from_desktop_bounds(
                SelectedReadbackPlanBackend::DxgiOutput,
                bounds,
                output_bounds,
                ImageBounds::new(metadata.width, metadata.height),
            )
            .map_err(|error| error.to_string())
        });
    let (selection, crop, selected_readback_plan, selected_output_ready_planning_only) = match plan
    {
        Ok(plan) => (
            plan.mapping.crop.as_selection_rect(),
            Some(plan.mapping.crop),
            Some(plan.clone()),
            plan.selected_output_ready(),
        ),
        Err(error) => {
            let _ = backend.release_acquired_frame();
            let _ = backend.stop();
            let mut report = DxgiSelectedReadbackSmokeReport::failed(
                stage,
                started_at.elapsed().as_millis(),
                error,
            );
            report.requested_bounds = bounds;
            report.output_bounds = output_bounds;
            report.adapter_index = output_identity.map(|identity| identity.0);
            report.output_index = output_identity.map(|identity| identity.1);
            return report;
        }
    };
    let selected = match frame.texture() {
        Ok(texture) => {
            let device: Result<ID3D11Device, String> =
                unsafe { texture.GetDevice() }.map_err(|error| error.to_string());
            device.and_then(|device| {
                let context =
                    unsafe { device.GetImmediateContext() }.map_err(|error| error.to_string())?;
                build_selected_png_contract_from_dxgi_texture(
                    &device,
                    &context,
                    texture,
                    ImageBounds::new(metadata.width, metadata.height),
                    selection,
                )
                .map_err(|error| error.to_string())
            })
        }
        Err(error) => Err(error.to_string()),
    };
    if selected.is_ok() {
        stage = DxgiSelectedReadbackSmokeStage::SelectedReadback;
    }

    let release_error = backend.release_acquired_frame().err();
    let released_frame = release_error.is_none();
    if released_frame {
        stage = DxgiSelectedReadbackSmokeStage::FrameReleased;
    }

    let stop_error = backend.stop().err();
    let stopped = stop_error.is_none();
    if stopped {
        stage = DxgiSelectedReadbackSmokeStage::Stopped;
    }

    let error = selected
        .as_ref()
        .err()
        .cloned()
        .or_else(|| release_error.map(|error| format!("release failed: {error}")))
        .or_else(|| stop_error.map(|error| format!("stop failed: {error}")));

    let selected_image = selected.ok();
    let selected_only = selected_image
        .as_ref()
        .map(|image| image.is_selected_only_png())
        .unwrap_or(false);
    let png_signature_valid = selected_image
        .as_ref()
        .map(|image| {
            image
                .png_bytes
                .starts_with(&[137, 80, 78, 71, 13, 10, 26, 10])
        })
        .unwrap_or(false);

    DxgiSelectedReadbackSmokeReport {
        attempted: true,
        ok: error.is_none() && selected_only && png_signature_valid,
        stage,
        elapsed_ms: started_at.elapsed().as_millis(),
        frame_id: Some(metadata.frame_id),
        width: Some(bounds.width),
        height: Some(bounds.height),
        requested_bounds: bounds,
        output_bounds,
        adapter_index: output_identity.map(|identity| identity.0),
        output_index: output_identity.map(|identity| identity.1),
        crop,
        selected_readback_plan,
        selected_output_ready_planning_only,
        format: Some(metadata.format),
        selected_only,
        bounded_crop_valid: true,
        copy_subresource_region: selected_image.is_some(),
        bgra_to_rgba: true,
        png_signature_valid,
        released_frame,
        stopped,
        error,
    }
}

#[cfg(not(windows))]
pub fn run_dxgi_selected_readback_smoke(
    _bounds: MonitorCaptureBounds,
) -> DxgiSelectedReadbackSmokeReport {
    DxgiSelectedReadbackSmokeReport {
        attempted: false,
        ok: false,
        stage: DxgiSelectedReadbackSmokeStage::Failed,
        elapsed_ms: 0,
        frame_id: None,
        width: None,
        height: None,
        requested_bounds: MonitorCaptureBounds::new(0, 0, 0, 0),
        output_bounds: None,
        adapter_index: None,
        output_index: None,
        crop: None,
        selected_readback_plan: None,
        selected_output_ready_planning_only: false,
        format: None,
        selected_only: false,
        bounded_crop_valid: false,
        copy_subresource_region: false,
        bgra_to_rgba: false,
        png_signature_valid: false,
        released_frame: false,
        stopped: false,
        error: Some("DXGI selected readback smoke requires Windows".to_string()),
    }
}

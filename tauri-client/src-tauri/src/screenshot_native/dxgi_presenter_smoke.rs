use super::capture::CaptureBackendKind;
use super::d3d11_frame::{D3d11TextureFrame, D3d11TextureFrameFormat};
use super::dxgi_texture::{build_dxgi_texture_frame_contract, DxgiTextureDescriptor};
use super::presenter::{
    plan_presenter_with_fallback, GpuTextureDescriptor, PresentationFallbackReason,
    PresentationFrame, PresentationMode, PresentationReadiness, PresenterContract, PresenterKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiPresenterSmokeReport {
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub format: D3d11TextureFrameFormat,
    pub requires_gpu_texture: bool,
    pub has_cpu_readback: bool,
    pub presentation_mode: PresentationMode,
    pub active_presenter: PresenterKind,
    pub readiness: PresentationReadiness,
    pub fallback_reason: PresentationFallbackReason,
    pub can_present_cpu: bool,
    pub texture_presentable: bool,
    pub requires_cpu_copy: bool,
}

pub fn smoke_dxgi_presenter_contract(
    descriptor: DxgiTextureDescriptor,
) -> Result<DxgiPresenterSmokeReport, String> {
    let frame = build_dxgi_texture_frame_contract(descriptor).map_err(|error| error.to_string())?;
    Ok(smoke_dxgi_texture_frame_presenter_contract(frame))
}

pub fn smoke_dxgi_texture_frame_presenter_contract(
    frame: D3d11TextureFrame,
) -> DxgiPresenterSmokeReport {
    let metadata = frame.metadata;
    let texture = gpu_texture_descriptor_from_dxgi_frame(&frame);
    let requires_gpu_texture = frame.clone().requires_gpu_texture();
    let has_cpu_readback = frame.has_cpu_readback();
    let presentation_frame =
        PresentationFrame::gpu_pending_readback(CaptureBackendKind::DesktopDuplication);
    let plan = plan_presenter_with_fallback(
        presentation_frame,
        PresenterContract::d3d11_texture(),
        Some(texture),
    );
    let diagnostics = plan.diagnostics();

    DxgiPresenterSmokeReport {
        frame_id: metadata.frame_id,
        width: metadata.width,
        height: metadata.height,
        format: metadata.format,
        requires_gpu_texture,
        has_cpu_readback,
        presentation_mode: plan.contract.mode,
        active_presenter: diagnostics.active,
        readiness: diagnostics.readiness,
        fallback_reason: diagnostics.fallback_reason,
        can_present_cpu: diagnostics.can_present_cpu,
        texture_presentable: texture.is_presentable(),
        requires_cpu_copy: plan.contract.requires_cpu_copy,
    }
}

fn gpu_texture_descriptor_from_dxgi_frame(frame: &D3d11TextureFrame) -> GpuTextureDescriptor {
    GpuTextureDescriptor {
        width: frame.metadata.width,
        height: frame.metadata.height,
        format_rgba8: matches!(
            frame.metadata.format,
            D3d11TextureFrameFormat::Bgra8Unorm | D3d11TextureFrameFormat::Rgba8Unorm
        ),
        shared_handle_available: frame.metadata.texture.is_some()
            || frame.metadata.shared_handle.is_shared(),
        keyed_mutex_required: frame.metadata.shared_handle.keyed_mutex_required,
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::super::d3d11_frame::D3d11RawHandle;
    use super::*;

    fn descriptor(width: u32, height: u32) -> DxgiTextureDescriptor {
        DxgiTextureDescriptor::new(
            width,
            height,
            D3d11TextureFrameFormat::Bgra8Unorm,
            D3d11RawHandle::new(NonZeroU64::new(0xD11).expect("non-zero handle")),
            42,
        )
    }

    #[test]
    fn dxgi_texture_contract_reaches_d3d11_presenter_plan_without_cpu_readback() {
        let report = smoke_dxgi_presenter_contract(descriptor(8, 4)).expect("valid smoke report");

        assert_eq!(report.frame_id, 42);
        assert_eq!(report.width, 8);
        assert_eq!(report.height, 4);
        assert_eq!(report.format, D3d11TextureFrameFormat::Bgra8Unorm);
        assert!(report.requires_gpu_texture);
        assert!(!report.has_cpu_readback);
        assert_eq!(
            report.presentation_mode,
            PresentationMode::SharedD3d11Texture
        );
        assert_eq!(report.active_presenter, PresenterKind::D3d11Texture);
        assert_eq!(report.readiness, PresentationReadiness::NeedsReadback);
        assert_eq!(
            report.fallback_reason,
            PresentationFallbackReason::RecoverableFailure
        );
        assert!(!report.can_present_cpu);
        assert!(report.texture_presentable);
        assert!(!report.requires_cpu_copy);
    }

    #[test]
    fn empty_dxgi_texture_dimensions_fail_before_presenter_planning() {
        let error = smoke_dxgi_presenter_contract(descriptor(0, 4)).expect_err("empty width fails");
        assert!(error.contains("empty DXGI texture dimensions"));
    }
}

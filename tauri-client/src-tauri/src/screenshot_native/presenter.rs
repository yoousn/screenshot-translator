use super::capture::{CaptureBackendKind, RgbaFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenterKind {
    CpuRgba,
    D3d11Texture,
}

impl PresenterKind {
    pub const fn requires_gpu(self) -> bool {
        matches!(self, Self::D3d11Texture)
    }

    pub const fn fallback(self) -> Self {
        match self {
            Self::CpuRgba => Self::CpuRgba,
            Self::D3d11Texture => Self::CpuRgba,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationMode {
    ImmediateCpuUpload,
    SharedD3d11Texture,
    GpuWithCpuReadback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationReadiness {
    Ready,
    NeedsReadback,
    NeedsGpuInterop,
    FallbackRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationFailureKind {
    BackendUnavailable,
    TextureInteropUnavailable,
    ReadbackUnavailable,
    InvalidFrame,
    ProtectedContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationFallbackReason {
    None,
    CpuFrameReady,
    GpuInteropUnavailable,
    GpuReadbackPending,
    InvalidCpuFrame,
    InvalidTexture,
    RecoverableFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationFailure {
    pub kind: PresentationFailureKind,
    pub recoverable: bool,
    pub message: String,
}

impl PresentationFailure {
    pub fn recoverable(kind: PresentationFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            recoverable: true,
            message: message.into(),
        }
    }

    pub fn fatal(kind: PresentationFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            recoverable: false,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuTextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format_rgba8: bool,
    pub shared_handle_available: bool,
    pub keyed_mutex_required: bool,
}

impl GpuTextureDescriptor {
    pub const fn rgba8_shared(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format_rgba8: true,
            shared_handle_available: true,
            keyed_mutex_required: false,
        }
    }

    pub const fn is_presentable(self) -> bool {
        self.width > 0 && self.height > 0 && self.format_rgba8 && self.shared_handle_available
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenterContract {
    pub kind: PresenterKind,
    pub mode: PresentationMode,
    pub supports_dirty_regions: bool,
    pub supports_negative_origin: bool,
    pub requires_cpu_copy: bool,
}

impl PresenterContract {
    pub const fn cpu_rgba() -> Self {
        Self {
            kind: PresenterKind::CpuRgba,
            mode: PresentationMode::ImmediateCpuUpload,
            supports_dirty_regions: false,
            supports_negative_origin: true,
            requires_cpu_copy: true,
        }
    }

    pub const fn d3d11_texture() -> Self {
        Self {
            kind: PresenterKind::D3d11Texture,
            mode: PresentationMode::SharedD3d11Texture,
            supports_dirty_regions: true,
            supports_negative_origin: true,
            requires_cpu_copy: false,
        }
    }

    pub const fn gpu_readback() -> Self {
        Self {
            kind: PresenterKind::D3d11Texture,
            mode: PresentationMode::GpuWithCpuReadback,
            supports_dirty_regions: true,
            supports_negative_origin: true,
            requires_cpu_copy: true,
        }
    }

    pub const fn fallback(self) -> Self {
        match self.kind {
            PresenterKind::CpuRgba => Self::cpu_rgba(),
            PresenterKind::D3d11Texture => Self::cpu_rgba(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PresentationFrame {
    pub presenter: PresenterKind,
    pub backend: CaptureBackendKind,
    pub rgba: Option<RgbaFrame>,
}

impl PresentationFrame {
    pub fn cpu(backend: CaptureBackendKind, rgba: RgbaFrame) -> Self {
        Self {
            presenter: PresenterKind::CpuRgba,
            backend,
            rgba: Some(rgba),
        }
    }

    pub const fn gpu_pending_readback(backend: CaptureBackendKind) -> Self {
        Self {
            presenter: PresenterKind::D3d11Texture,
            backend,
            rgba: None,
        }
    }

    pub fn readiness(&self) -> PresentationReadiness {
        match (self.presenter, self.backend.is_gpu(), self.rgba.as_ref()) {
            (PresenterKind::CpuRgba, _, Some(frame)) if frame.is_tightly_packed_rgba() => {
                PresentationReadiness::Ready
            }
            (PresenterKind::CpuRgba, _, _) => PresentationReadiness::FallbackRequired,
            (PresenterKind::D3d11Texture, true, Some(frame)) if frame.is_tightly_packed_rgba() => {
                PresentationReadiness::NeedsGpuInterop
            }
            (PresenterKind::D3d11Texture, true, None) => PresentationReadiness::NeedsReadback,
            (PresenterKind::D3d11Texture, false, Some(frame)) if frame.is_tightly_packed_rgba() => {
                PresentationReadiness::Ready
            }
            (PresenterKind::D3d11Texture, _, _) => PresentationReadiness::FallbackRequired,
        }
    }

    pub fn can_present_cpu(&self) -> bool {
        self.rgba
            .as_ref()
            .is_some_and(|frame| frame.is_tightly_packed_rgba())
    }

    pub fn fallback_to_cpu(mut self) -> Self {
        self.presenter = self.presenter.fallback();
        self
    }
}

#[derive(Debug, Clone)]
pub struct PresentationPlan {
    pub contract: PresenterContract,
    pub frame: PresentationFrame,
    pub texture: Option<GpuTextureDescriptor>,
    pub failure: Option<PresentationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationDiagnostics {
    pub preferred: PresenterKind,
    pub active: PresenterKind,
    pub backend: CaptureBackendKind,
    pub readiness: PresentationReadiness,
    pub fallback_reason: PresentationFallbackReason,
    pub failure: Option<PresentationFailure>,
    pub can_present_cpu: bool,
}

impl PresentationPlan {
    pub fn ready_cpu(frame: PresentationFrame) -> Self {
        Self {
            contract: PresenterContract::cpu_rgba(),
            frame,
            texture: None,
            failure: None,
        }
    }

    pub fn ready_gpu(frame: PresentationFrame, texture: GpuTextureDescriptor) -> Self {
        Self {
            contract: PresenterContract::d3d11_texture(),
            frame,
            texture: Some(texture),
            failure: None,
        }
    }

    pub fn with_failure(
        frame: PresentationFrame,
        contract: PresenterContract,
        failure: PresentationFailure,
    ) -> Self {
        Self {
            contract,
            frame,
            texture: None,
            failure: Some(failure),
        }
    }

    pub fn needs_fallback(&self) -> bool {
        self.failure
            .as_ref()
            .is_some_and(|failure| failure.recoverable)
            || self.frame.readiness() == PresentationReadiness::FallbackRequired
            || self
                .texture
                .as_ref()
                .is_some_and(|texture| !texture.is_presentable())
    }

    pub fn fallback_reason(&self) -> PresentationFallbackReason {
        if self
            .failure
            .as_ref()
            .is_some_and(|failure| failure.recoverable)
        {
            return PresentationFallbackReason::RecoverableFailure;
        }

        if self
            .texture
            .as_ref()
            .is_some_and(|texture| !texture.is_presentable())
        {
            return PresentationFallbackReason::InvalidTexture;
        }

        match self.frame.readiness() {
            PresentationReadiness::Ready => PresentationFallbackReason::None,
            PresentationReadiness::NeedsGpuInterop => {
                PresentationFallbackReason::GpuInteropUnavailable
            }
            PresentationReadiness::NeedsReadback => PresentationFallbackReason::GpuReadbackPending,
            PresentationReadiness::FallbackRequired if self.frame.can_present_cpu() => {
                PresentationFallbackReason::CpuFrameReady
            }
            PresentationReadiness::FallbackRequired => PresentationFallbackReason::InvalidCpuFrame,
        }
    }

    pub fn with_cpu_fallback(mut self) -> Self {
        if self.needs_fallback() && self.frame.can_present_cpu() {
            self.contract = self.contract.fallback();
            self.frame = self.frame.fallback_to_cpu();
            self.texture = None;
        }
        self
    }

    pub fn diagnostics(&self) -> PresentationDiagnostics {
        PresentationDiagnostics {
            preferred: self.contract.kind,
            active: self.frame.presenter,
            backend: self.frame.backend,
            readiness: self.frame.readiness(),
            fallback_reason: self.fallback_reason(),
            failure: self.failure.clone(),
            can_present_cpu: self.frame.can_present_cpu(),
        }
    }
}

pub fn plan_presenter_with_fallback(
    frame: PresentationFrame,
    preferred: PresenterContract,
    texture: Option<GpuTextureDescriptor>,
) -> PresentationPlan {
    let failure = match (preferred.kind, frame.readiness(), texture) {
        (PresenterKind::D3d11Texture, PresentationReadiness::NeedsReadback, _) => {
            Some(PresentationFailure::recoverable(
                PresentationFailureKind::ReadbackUnavailable,
                "native presenter needs GPU readback before D3D11 presentation",
            ))
        }
        (PresenterKind::D3d11Texture, PresentationReadiness::NeedsGpuInterop, None) => {
            Some(PresentationFailure::recoverable(
                PresentationFailureKind::TextureInteropUnavailable,
                "native presenter has no shareable D3D11 texture descriptor",
            ))
        }
        (PresenterKind::D3d11Texture, _, Some(texture)) if !texture.is_presentable() => {
            Some(PresentationFailure::recoverable(
                PresentationFailureKind::TextureInteropUnavailable,
                "native presenter texture descriptor is not presentable",
            ))
        }
        (_, PresentationReadiness::FallbackRequired, _) => Some(PresentationFailure::recoverable(
            PresentationFailureKind::InvalidFrame,
            "native presenter frame is not tightly packed RGBA",
        )),
        _ => None,
    };

    PresentationPlan {
        contract: preferred,
        frame,
        texture,
        failure,
    }
    .with_cpu_fallback()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_cpu_when_gpu_texture_missing_but_rgba_is_ready() {
        let rgba = RgbaFrame::new(1, 1, vec![1, 2, 3, 4]).expect("valid frame");
        let frame = PresentationFrame {
            presenter: PresenterKind::D3d11Texture,
            backend: CaptureBackendKind::WindowsGraphicsCapture,
            rgba: Some(rgba),
        };

        let plan = plan_presenter_with_fallback(frame, PresenterContract::d3d11_texture(), None);

        assert_eq!(plan.contract.kind, PresenterKind::CpuRgba);
        assert_eq!(plan.frame.presenter, PresenterKind::CpuRgba);
        assert_eq!(plan.diagnostics().active, PresenterKind::CpuRgba);
    }

    #[test]
    fn keeps_gpu_plan_when_texture_is_presentable() {
        let frame = PresentationFrame::gpu_pending_readback(CaptureBackendKind::DesktopDuplication);
        let texture = GpuTextureDescriptor::rgba8_shared(8, 4);

        let plan =
            plan_presenter_with_fallback(frame, PresenterContract::d3d11_texture(), Some(texture));

        assert_eq!(plan.contract.kind, PresenterKind::D3d11Texture);
        assert_eq!(plan.texture, Some(texture));
    }
}

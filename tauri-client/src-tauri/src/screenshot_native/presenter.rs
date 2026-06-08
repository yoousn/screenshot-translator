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
}

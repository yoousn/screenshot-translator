#[path = "wgc_capture.rs"]
pub mod wgc_capture;

pub use wgc_capture::{
    DxgiDesktopDuplicationBackend, GpuCaptureBackendError, GpuCaptureBackendResult,
    GpuCaptureFrameSource, WindowsGraphicsCaptureBackend,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCaptureBackend {
    WindowsGraphicsCapture,
    DxgiDesktopDuplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTextureInterop {
    D3d11Texture,
    CpuReadableBitmap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCaptureStatus {
    Unknown,
    Unsupported,
    Initializing,
    Ready,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCapabilityRequirement {
    WindowsGraphicsCaptureApi,
    DxgiOutputDuplicationApi,
    D3d11Device,
    D3d11SharedTexture,
    UserCaptureConsent,
    CompatibleAdapter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuCaptureFallback {
    None,
    RetryBackend(GpuCaptureBackend),
    RetryTextureInterop(GpuTextureInterop),
    CpuScreenshot,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCaptureCapability {
    pub backend: GpuCaptureBackend,
    pub texture_interop: GpuTextureInterop,
    pub status: GpuCaptureStatus,
    pub missing_requirements: Vec<GpuCapabilityRequirement>,
    pub fallback: GpuCaptureFallback,
    pub reason: Option<String>,
}

impl GpuCaptureCapability {
    pub fn ready(backend: GpuCaptureBackend, texture_interop: GpuTextureInterop) -> Self {
        Self {
            backend,
            texture_interop,
            status: GpuCaptureStatus::Ready,
            missing_requirements: Vec::new(),
            fallback: GpuCaptureFallback::None,
            reason: None,
        }
    }

    pub fn blocked(
        backend: GpuCaptureBackend,
        texture_interop: GpuTextureInterop,
        missing_requirements: Vec<GpuCapabilityRequirement>,
        fallback: GpuCaptureFallback,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            texture_interop,
            status: GpuCaptureStatus::Unsupported,
            missing_requirements,
            fallback,
            reason: Some(reason.into()),
        }
    }

    pub fn degraded(
        backend: GpuCaptureBackend,
        texture_interop: GpuTextureInterop,
        fallback: GpuCaptureFallback,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            texture_interop,
            status: GpuCaptureStatus::Degraded,
            missing_requirements: Vec::new(),
            fallback,
            reason: Some(reason.into()),
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            GpuCaptureStatus::Ready | GpuCaptureStatus::Degraded
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCapturePlan {
    pub primary: GpuCaptureCapability,
    pub fallbacks: Vec<GpuCaptureCapability>,
}

impl GpuCapturePlan {
    pub fn selected(&self) -> Option<&GpuCaptureCapability> {
        std::iter::once(&self.primary)
            .chain(self.fallbacks.iter())
            .find(|capability| capability.is_usable())
    }
}

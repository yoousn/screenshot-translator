use std::fmt;

use super::{
    GpuCapabilityRequirement, GpuCaptureBackend, GpuCaptureCapability, GpuCaptureFallback,
    GpuTextureInterop,
};
use crate::screenshot_native::{CaptureBackendKind, MonitorCaptureBounds, RgbaFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuCaptureBackendError {
    NotImplemented {
        backend: GpuCaptureBackend,
    },
    Unsupported {
        backend: GpuCaptureBackend,
        reason: String,
    },
    InvalidBounds(MonitorCaptureBounds),
    CaptureUnavailable {
        backend: GpuCaptureBackend,
        reason: String,
    },
}

impl GpuCaptureBackendError {
    pub fn not_implemented(backend: GpuCaptureBackend) -> Self {
        Self::NotImplemented { backend }
    }

    pub fn unsupported(backend: GpuCaptureBackend, reason: impl Into<String>) -> Self {
        Self::Unsupported {
            backend,
            reason: reason.into(),
        }
    }

    pub fn capture_unavailable(backend: GpuCaptureBackend, reason: impl Into<String>) -> Self {
        Self::CaptureUnavailable {
            backend,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for GpuCaptureBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented { backend } => {
                write!(
                    formatter,
                    "GPU capture backend {backend:?} is not implemented yet"
                )
            }
            Self::Unsupported { backend, reason } => {
                write!(
                    formatter,
                    "GPU capture backend {backend:?} is unsupported: {reason}"
                )
            }
            Self::InvalidBounds(bounds) => write!(
                formatter,
                "invalid GPU capture bounds: x={}, y={}, width={}, height={}",
                bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
            ),
            Self::CaptureUnavailable { backend, reason } => write!(
                formatter,
                "GPU capture backend {backend:?} is unavailable: {reason}"
            ),
        }
    }
}

impl std::error::Error for GpuCaptureBackendError {}

pub type GpuCaptureBackendResult<T> = Result<T, GpuCaptureBackendError>;

pub trait GpuCaptureFrameSource {
    fn gpu_backend(&self) -> GpuCaptureBackend;

    fn capture_backend_kind(&self) -> CaptureBackendKind;

    fn capability(&self) -> GpuCaptureCapability;

    fn start(&mut self) -> GpuCaptureBackendResult<()> {
        Err(GpuCaptureBackendError::not_implemented(self.gpu_backend()))
    }

    fn stop(&mut self) -> GpuCaptureBackendResult<()> {
        Ok(())
    }

    fn capture_frame(&mut self, bounds: MonitorCaptureBounds)
        -> GpuCaptureBackendResult<RgbaFrame>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsGraphicsCaptureBackend;

impl WindowsGraphicsCaptureBackend {
    pub const fn new() -> Self {
        Self
    }
}

impl GpuCaptureFrameSource for WindowsGraphicsCaptureBackend {
    fn gpu_backend(&self) -> GpuCaptureBackend {
        GpuCaptureBackend::WindowsGraphicsCapture
    }

    fn capture_backend_kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::WindowsGraphicsCapture
    }

    fn capability(&self) -> GpuCaptureCapability {
        GpuCaptureCapability::blocked(
            self.gpu_backend(),
            GpuTextureInterop::D3d11Texture,
            vec![
                GpuCapabilityRequirement::WindowsGraphicsCaptureApi,
                GpuCapabilityRequirement::D3d11Device,
                GpuCapabilityRequirement::D3d11SharedTexture,
                GpuCapabilityRequirement::UserCaptureConsent,
            ],
            GpuCaptureFallback::RetryBackend(GpuCaptureBackend::DxgiDesktopDuplication),
            "Windows Graphics Capture backend is a Phase E placeholder; native API wiring is pending.",
        )
    }

    fn capture_frame(
        &mut self,
        bounds: MonitorCaptureBounds,
    ) -> GpuCaptureBackendResult<RgbaFrame> {
        if bounds.is_empty() {
            return Err(GpuCaptureBackendError::InvalidBounds(bounds));
        }
        Err(GpuCaptureBackendError::not_implemented(self.gpu_backend()))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DxgiDesktopDuplicationBackend;

impl DxgiDesktopDuplicationBackend {
    pub const fn new() -> Self {
        Self
    }
}

impl GpuCaptureFrameSource for DxgiDesktopDuplicationBackend {
    fn gpu_backend(&self) -> GpuCaptureBackend {
        GpuCaptureBackend::DxgiDesktopDuplication
    }

    fn capture_backend_kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::DesktopDuplication
    }

    fn capability(&self) -> GpuCaptureCapability {
        GpuCaptureCapability::blocked(
            self.gpu_backend(),
            GpuTextureInterop::D3d11Texture,
            vec![
                GpuCapabilityRequirement::DxgiOutputDuplicationApi,
                GpuCapabilityRequirement::D3d11Device,
                GpuCapabilityRequirement::D3d11SharedTexture,
                GpuCapabilityRequirement::CompatibleAdapter,
            ],
            GpuCaptureFallback::CpuScreenshot,
            "DXGI Desktop Duplication backend is a Phase E placeholder; native API wiring is pending.",
        )
    }

    fn capture_frame(
        &mut self,
        bounds: MonitorCaptureBounds,
    ) -> GpuCaptureBackendResult<RgbaFrame> {
        if bounds.is_empty() {
            return Err(GpuCaptureBackendError::InvalidBounds(bounds));
        }
        Err(GpuCaptureBackendError::not_implemented(self.gpu_backend()))
    }
}

use std::fmt;

use super::{
    GpuCapabilityRequirement, GpuCaptureBackend, GpuCaptureCapability, GpuCaptureFallback,
    GpuCaptureStatus, GpuTextureInterop,
};
use crate::screenshot_native::dxgi_capture::{
    DxgiCaptureError, DxgiDesktopDuplicationBackend as NativeDxgiDesktopDuplicationBackend,
};
use crate::screenshot_native::wgc_probe::{
    default_wgc_one_frame_probe_contract, probe_wgc_native_api_support, WgcOneFrameProbeDiagnostics,
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

impl From<DxgiCaptureError> for GpuCaptureBackendError {
    fn from(error: DxgiCaptureError) -> Self {
        match error {
            DxgiCaptureError::InvalidBounds(bounds) => Self::InvalidBounds(bounds),
            DxgiCaptureError::Unsupported { reason }
            | DxgiCaptureError::AdapterUnavailable { reason } => {
                Self::unsupported(GpuCaptureBackend::DxgiDesktopDuplication, reason)
            }
            DxgiCaptureError::PlaceholderOnly
            | DxgiCaptureError::AccessLost { .. }
            | DxgiCaptureError::FrameTimeout { .. }
            | DxgiCaptureError::FrameUnavailable { .. }
            | DxgiCaptureError::ProtectedContent => Self::capture_unavailable(
                GpuCaptureBackend::DxgiDesktopDuplication,
                error.to_string(),
            ),
        }
    }
}

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
        let api_probe = probe_wgc_native_api_support();
        let diagnostics = WgcOneFrameProbeDiagnostics::from_contract(
            default_wgc_one_frame_probe_contract(),
            api_probe.clone(),
        );
        let mut missing_requirements = vec![
            GpuCapabilityRequirement::D3d11Device,
            GpuCapabilityRequirement::D3d11SharedTexture,
            GpuCapabilityRequirement::UserCaptureConsent,
        ];
        if !api_probe.is_supported {
            missing_requirements.insert(0, GpuCapabilityRequirement::WindowsGraphicsCaptureApi);
        }

        GpuCaptureCapability {
            backend: self.gpu_backend(),
            texture_interop: GpuTextureInterop::D3d11Texture,
            status: GpuCaptureStatus::Unsupported,
            missing_requirements,
            fallback: GpuCaptureFallback::RetryBackend(GpuCaptureBackend::DxgiDesktopDuplication),
            reason: Some(api_probe.reason.unwrap_or_else(|| {
                format!(
                    "Windows Graphics Capture API is present; next guarded WGC contract stage is {}.",
                    diagnostics
                        .next_stage
                        .map(|stage| stage.as_str())
                        .unwrap_or("one-frame-readback")
                )
            })),
        }
    }

    fn start(&mut self) -> GpuCaptureBackendResult<()> {
        let api_probe = probe_wgc_native_api_support();
        if !api_probe.is_supported {
            return Err(GpuCaptureBackendError::unsupported(
                self.gpu_backend(),
                api_probe.reason.unwrap_or_else(|| {
                    "Windows Graphics Capture API support probe returned false".to_string()
                }),
            ));
        }

        Err(GpuCaptureBackendError::capture_unavailable(
            self.gpu_backend(),
            "WGC API is available, but guarded device/framepool/session contracts are diagnostics-only until one-frame capture is explicitly enabled.",
        ))
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

#[derive(Debug, Default, Clone)]
pub struct DxgiDesktopDuplicationBackend {
    inner: NativeDxgiDesktopDuplicationBackend,
}

impl DxgiDesktopDuplicationBackend {
    pub fn new() -> Self {
        Self {
            inner: NativeDxgiDesktopDuplicationBackend::new(),
        }
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
        self.inner.capability()
    }

    fn start(&mut self) -> GpuCaptureBackendResult<()> {
        self.inner.start().map_err(Into::into)
    }

    fn stop(&mut self) -> GpuCaptureBackendResult<()> {
        self.inner.stop().map_err(Into::into)
    }

    fn capture_frame(
        &mut self,
        bounds: MonitorCaptureBounds,
    ) -> GpuCaptureBackendResult<RgbaFrame> {
        if bounds.is_empty() {
            return Err(GpuCaptureBackendError::InvalidBounds(bounds));
        }
        self.inner.capture_frame(bounds).map_err(Into::into)
    }
}

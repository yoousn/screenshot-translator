use std::fmt;

use super::{
    CaptureBackendContract, CaptureBackendKind, GpuCapabilityRequirement, GpuCaptureBackend,
    GpuCaptureCapability, GpuCaptureFallback, GpuCaptureStatus, GpuTextureInterop,
    MonitorCaptureBounds, RgbaFrame,
};

pub const DXGI_PLACEHOLDER_REASON: &str =
    "DXGI Desktop Duplication backend is a placeholder; native API wiring is intentionally pending.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiCaptureFallbackTarget {
    ExistingCpuScreenshot,
    Unavailable,
}

impl DxgiCaptureFallbackTarget {
    pub const fn as_gpu_fallback(self) -> GpuCaptureFallback {
        match self {
            Self::ExistingCpuScreenshot => GpuCaptureFallback::CpuScreenshot,
            Self::Unavailable => GpuCaptureFallback::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiCaptureReadiness {
    PlaceholderOnly,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiDesktopDuplicationContract {
    pub capture_contract: CaptureBackendContract,
    pub backend: GpuCaptureBackend,
    pub texture_interop: GpuTextureInterop,
    pub required_capabilities: Vec<GpuCapabilityRequirement>,
    pub readiness: DxgiCaptureReadiness,
    pub fallback_target: DxgiCaptureFallbackTarget,
    pub reason: String,
}

impl DxgiDesktopDuplicationContract {
    pub fn placeholder() -> Self {
        Self {
            capture_contract: CaptureBackendContract::dxgi(),
            backend: GpuCaptureBackend::DxgiDesktopDuplication,
            texture_interop: GpuTextureInterop::D3d11Texture,
            required_capabilities: vec![
                GpuCapabilityRequirement::DxgiOutputDuplicationApi,
                GpuCapabilityRequirement::D3d11Device,
                GpuCapabilityRequirement::D3d11SharedTexture,
                GpuCapabilityRequirement::CompatibleAdapter,
            ],
            readiness: DxgiCaptureReadiness::PlaceholderOnly,
            fallback_target: DxgiCaptureFallbackTarget::ExistingCpuScreenshot,
            reason: DXGI_PLACEHOLDER_REASON.to_string(),
        }
    }

    pub fn capability(&self) -> GpuCaptureCapability {
        match self.readiness {
            DxgiCaptureReadiness::Ready => {
                GpuCaptureCapability::ready(self.backend, self.texture_interop)
            }
            DxgiCaptureReadiness::PlaceholderOnly | DxgiCaptureReadiness::Blocked => {
                GpuCaptureCapability {
                    backend: self.backend,
                    texture_interop: self.texture_interop,
                    status: GpuCaptureStatus::Unsupported,
                    missing_requirements: self.required_capabilities.clone(),
                    fallback: self.fallback_target.as_gpu_fallback(),
                    reason: Some(self.reason.clone()),
                }
            }
        }
    }

    pub fn fallback(&self) -> GpuCaptureFallback {
        self.fallback_target.as_gpu_fallback()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxgiCaptureError {
    PlaceholderOnly,
    Unsupported { reason: String },
    InvalidBounds(MonitorCaptureBounds),
    AdapterUnavailable { reason: String },
    FrameUnavailable { reason: String },
    ProtectedContent,
}

impl DxgiCaptureError {
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported {
            reason: reason.into(),
        }
    }

    pub fn adapter_unavailable(reason: impl Into<String>) -> Self {
        Self::AdapterUnavailable {
            reason: reason.into(),
        }
    }

    pub fn frame_unavailable(reason: impl Into<String>) -> Self {
        Self::FrameUnavailable {
            reason: reason.into(),
        }
    }

    pub fn fallback(&self) -> DxgiCaptureFallbackTarget {
        match self {
            Self::InvalidBounds(_) => DxgiCaptureFallbackTarget::Unavailable,
            Self::ProtectedContent => DxgiCaptureFallbackTarget::Unavailable,
            Self::PlaceholderOnly
            | Self::Unsupported { .. }
            | Self::AdapterUnavailable { .. }
            | Self::FrameUnavailable { .. } => DxgiCaptureFallbackTarget::ExistingCpuScreenshot,
        }
    }
}

impl fmt::Display for DxgiCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaceholderOnly => formatter.write_str(DXGI_PLACEHOLDER_REASON),
            Self::Unsupported { reason } => {
                write!(formatter, "DXGI capture is unsupported: {reason}")
            }
            Self::InvalidBounds(bounds) => write!(
                formatter,
                "invalid DXGI capture bounds: x={}, y={}, width={}, height={}",
                bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
            ),
            Self::AdapterUnavailable { reason } => {
                write!(formatter, "DXGI adapter is unavailable: {reason}")
            }
            Self::FrameUnavailable { reason } => {
                write!(formatter, "DXGI frame is unavailable: {reason}")
            }
            Self::ProtectedContent => {
                formatter.write_str("DXGI capture returned protected content")
            }
        }
    }
}

impl std::error::Error for DxgiCaptureError {}

pub type DxgiCaptureResult<T> = Result<T, DxgiCaptureError>;

#[derive(Debug, Default, Clone, Copy)]
pub struct DxgiDesktopDuplicationBackend;

impl DxgiDesktopDuplicationBackend {
    pub const fn new() -> Self {
        Self
    }

    pub fn contract(&self) -> DxgiDesktopDuplicationContract {
        DxgiDesktopDuplicationContract::placeholder()
    }

    pub fn capture_backend_kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::DesktopDuplication
    }

    pub fn capability(&self) -> GpuCaptureCapability {
        self.contract().capability()
    }

    pub fn start(&mut self) -> DxgiCaptureResult<()> {
        Err(DxgiCaptureError::PlaceholderOnly)
    }

    pub fn stop(&mut self) -> DxgiCaptureResult<()> {
        Ok(())
    }

    pub fn capture_frame(&mut self, bounds: MonitorCaptureBounds) -> DxgiCaptureResult<RgbaFrame> {
        if bounds.is_empty() {
            return Err(DxgiCaptureError::InvalidBounds(bounds));
        }
        Err(DxgiCaptureError::PlaceholderOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_contract_falls_back_to_cpu() {
        let contract = DxgiDesktopDuplicationContract::placeholder();

        assert_eq!(
            contract.capture_contract.backend,
            CaptureBackendKind::DesktopDuplication
        );
        assert_eq!(contract.fallback(), GpuCaptureFallback::CpuScreenshot);
        assert_eq!(contract.capability().status, GpuCaptureStatus::Unsupported);
    }

    #[test]
    fn empty_bounds_are_not_fallback_safe() {
        let error = DxgiCaptureError::InvalidBounds(MonitorCaptureBounds::new(0, 0, 0, 100));

        assert_eq!(error.fallback(), DxgiCaptureFallbackTarget::Unavailable);
    }
}

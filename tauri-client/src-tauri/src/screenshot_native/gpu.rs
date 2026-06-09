#[path = "wgc_capture.rs"]
pub mod wgc_capture;

pub use wgc_capture::{
    DxgiDesktopDuplicationBackend, GpuCaptureBackendError, GpuCaptureBackendResult,
    GpuCaptureFrameSource, WindowsGraphicsCaptureBackend,
};

use super::gpu_device::{
    create_d3d11_capture_device, D3d11AdapterPreference, D3d11DeviceCreateOptions,
    D3d11DeviceDiagnostics,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3d11GpuProbeReport {
    pub capability: GpuCaptureCapability,
    pub diagnostics: D3d11DeviceDiagnostics,
}

impl D3d11GpuProbeReport {
    const CONTRACT_ONLY_REASON: &'static str = "D3D11 device creation succeeded, but WGC/DXGI frame acquisition, texture readback, and selected-image handoff are still contract-only; keep existing CPU screenshot path as the safe runtime fallback.";

    pub fn safe_fallback_reason(&self) -> String {
        self.capability
            .reason
            .clone()
            .unwrap_or_else(|| "D3D11 probe did not provide a fallback reason".to_string())
    }
}

pub fn probe_d3d11_gpu_capability() -> D3d11GpuProbeReport {
    probe_d3d11_gpu_capability_with_options(D3d11DeviceCreateOptions {
        adapter_preference: D3d11AdapterPreference::Default,
        ..D3d11DeviceCreateOptions::default()
    })
}

#[cfg(windows)]
pub fn probe_d3d11_gpu_capability_with_options(
    options: D3d11DeviceCreateOptions,
) -> D3d11GpuProbeReport {
    match create_d3d11_capture_device(options) {
        Ok(device) if device.supports_capture() => D3d11GpuProbeReport {
            capability: GpuCaptureCapability::degraded(
                GpuCaptureBackend::WindowsGraphicsCapture,
                GpuTextureInterop::D3d11Texture,
                GpuCaptureFallback::CpuScreenshot,
                D3d11GpuProbeReport::CONTRACT_ONLY_REASON,
            ),
            diagnostics: D3d11DeviceDiagnostics {
                fallback_reason: Some(D3d11GpuProbeReport::CONTRACT_ONLY_REASON.to_string()),
                ..device.diagnostics
            },
        },
        Ok(device) => {
            let reason = format!(
                "D3D11 device initialized, but feature level {:?} is not accepted for capture; use existing CPU screenshot path.",
                device.feature_level
            );
            D3d11GpuProbeReport {
                capability: GpuCaptureCapability::blocked(
                    GpuCaptureBackend::WindowsGraphicsCapture,
                    GpuTextureInterop::D3d11Texture,
                    vec![GpuCapabilityRequirement::D3d11Device],
                    GpuCaptureFallback::CpuScreenshot,
                    reason.clone(),
                ),
                diagnostics: D3d11DeviceDiagnostics {
                    fallback_reason: Some(reason),
                    ..device.diagnostics
                },
            }
        }
        Err(error) => {
            let reason = error.safe_fallback_reason();
            D3d11GpuProbeReport {
                capability: GpuCaptureCapability::blocked(
                    GpuCaptureBackend::WindowsGraphicsCapture,
                    GpuTextureInterop::D3d11Texture,
                    vec![GpuCapabilityRequirement::D3d11Device],
                    GpuCaptureFallback::CpuScreenshot,
                    reason.clone(),
                ),
                diagnostics: D3d11DeviceDiagnostics::fallback(options, reason),
            }
        }
    }
}

#[cfg(not(windows))]
pub fn probe_d3d11_gpu_capability_with_options(
    options: D3d11DeviceCreateOptions,
) -> D3d11GpuProbeReport {
    let reason = match create_d3d11_capture_device(options) {
        Ok(_) => "D3D11 unexpectedly reported success on a non-Windows build".to_string(),
        Err(error) => error.safe_fallback_reason(),
    };
    D3d11GpuProbeReport {
        capability: GpuCaptureCapability::blocked(
            GpuCaptureBackend::WindowsGraphicsCapture,
            GpuTextureInterop::D3d11Texture,
            vec![GpuCapabilityRequirement::D3d11Device],
            GpuCaptureFallback::CpuScreenshot,
            reason.clone(),
        ),
        diagnostics: D3d11DeviceDiagnostics::fallback(options, reason),
    }
}

pub fn d3d11_first_gpu_capture_plan() -> GpuCapturePlan {
    let d3d11_probe = probe_d3d11_gpu_capability();
    GpuCapturePlan {
        primary: d3d11_probe.capability,
        fallbacks: vec![GpuCaptureCapability::degraded(
            GpuCaptureBackend::WindowsGraphicsCapture,
            GpuTextureInterop::CpuReadableBitmap,
            GpuCaptureFallback::CpuScreenshot,
            "D3D11 texture path is diagnostic-only until WGC/DXGI frame acquisition is integrated; keep existing CPU screenshot path as the safe runtime fallback.",
        )],
    }
}

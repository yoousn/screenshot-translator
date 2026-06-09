use super::capture::{CaptureBackendContract, CaptureBackendKind};
use super::dxgi_capture::{DxgiCaptureReadiness, DxgiDesktopDuplicationContract};
use super::gpu::{D3d11GpuProbeReport, GpuCaptureBackend, GpuCaptureStatus};
use super::wgc_contract::{WgcOneFrameProbePlan, WgcOneFrameProbeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRoutePreference {
    ExistingCpu,
    WindowsGraphicsCapture,
    DesktopDuplication,
}

impl CaptureRoutePreference {
    pub const fn backend_kind(self) -> CaptureBackendKind {
        match self {
            Self::ExistingCpu => CaptureBackendKind::ExistingCpu,
            Self::WindowsGraphicsCapture => CaptureBackendKind::WindowsGraphicsCapture,
            Self::DesktopDuplication => CaptureBackendKind::DesktopDuplication,
        }
    }

    pub const fn gpu_backend(self) -> Option<GpuCaptureBackend> {
        match self {
            Self::ExistingCpu => None,
            Self::WindowsGraphicsCapture => Some(GpuCaptureBackend::WindowsGraphicsCapture),
            Self::DesktopDuplication => Some(GpuCaptureBackend::DxgiDesktopDuplication),
        }
    }
}

impl Default for CaptureRoutePreference {
    fn default() -> Self {
        Self::ExistingCpu
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRouteFallbackReason {
    None,
    GpuBackendPlaceholder,
    GpuBackendUnavailable,
    D3d11ProbeUnavailable,
    WgcProbeDisabled,
    WgcProbePendingApiWiring,
    WgcProbeRejectedRealApi,
    WgcNativeApiUnavailable,
    WgcFrameAcquisitionContractOnly,
    DxgiCapturePlaceholder,
    DxgiCaptureUnavailable,
    DxgiContractBlocked,
    DxgiFrameAcquisitionContractOnly,
    SelectedBridgeUnavailable,
    SelectedBridgeInvalidImage,
    SelectedBridgeUnsupportedAction,
}

impl CaptureRouteFallbackReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::GpuBackendPlaceholder => "gpu-backend-placeholder",
            Self::GpuBackendUnavailable => "gpu-backend-unavailable",
            Self::D3d11ProbeUnavailable => "d3d11-probe-unavailable",
            Self::WgcProbeDisabled => "wgc-probe-disabled",
            Self::WgcProbePendingApiWiring => "wgc-probe-pending-api-wiring",
            Self::WgcProbeRejectedRealApi => "wgc-probe-rejected-real-api",
            Self::WgcNativeApiUnavailable => "wgc-native-api-unavailable",
            Self::WgcFrameAcquisitionContractOnly => "wgc-frame-acquisition-contract-only",
            Self::DxgiCapturePlaceholder => "dxgi-capture-placeholder",
            Self::DxgiCaptureUnavailable => "dxgi-capture-unavailable",
            Self::DxgiContractBlocked => "dxgi-contract-blocked",
            Self::DxgiFrameAcquisitionContractOnly => "dxgi-frame-acquisition-contract-only",
            Self::SelectedBridgeUnavailable => "selected-bridge-unavailable",
            Self::SelectedBridgeInvalidImage => "selected-bridge-invalid-image",
            Self::SelectedBridgeUnsupportedAction => "selected-bridge-unsupported-action",
        }
    }

    pub const fn is_probe_or_bridge_reason(self) -> bool {
        matches!(
            self,
            Self::D3d11ProbeUnavailable
                | Self::WgcProbeDisabled
                | Self::WgcProbePendingApiWiring
                | Self::WgcProbeRejectedRealApi
                | Self::WgcNativeApiUnavailable
                | Self::WgcFrameAcquisitionContractOnly
                | Self::DxgiCapturePlaceholder
                | Self::DxgiCaptureUnavailable
                | Self::DxgiContractBlocked
                | Self::DxgiFrameAcquisitionContractOnly
                | Self::SelectedBridgeUnavailable
                | Self::SelectedBridgeInvalidImage
                | Self::SelectedBridgeUnsupportedAction
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureRouteStatus {
    Ready,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRouteDecision {
    pub requested: CaptureBackendKind,
    pub selected: CaptureBackendKind,
    pub status: CaptureRouteStatus,
    pub fallback_reason: CaptureRouteFallbackReason,
    pub contract: CaptureBackendContract,
}

impl CaptureRouteDecision {
    pub const fn ready(backend: CaptureBackendKind) -> Self {
        Self {
            requested: backend,
            selected: backend,
            status: CaptureRouteStatus::Ready,
            fallback_reason: CaptureRouteFallbackReason::None,
            contract: backend.contract(),
        }
    }

    pub const fn fallback(
        requested: CaptureBackendKind,
        reason: CaptureRouteFallbackReason,
    ) -> Self {
        Self {
            requested,
            selected: CaptureBackendKind::ExistingCpu,
            status: CaptureRouteStatus::Fallback,
            fallback_reason: reason,
            contract: CaptureBackendKind::ExistingCpu.contract(),
        }
    }

    pub const fn uses_fallback(self) -> bool {
        matches!(self.status, CaptureRouteStatus::Fallback)
    }

    pub const fn wgc_probe_fallback(reason: CaptureRouteFallbackReason) -> Self {
        Self::fallback(CaptureBackendKind::WindowsGraphicsCapture, reason)
    }

    pub const fn dxgi_capture_fallback(reason: CaptureRouteFallbackReason) -> Self {
        Self::fallback(CaptureBackendKind::DesktopDuplication, reason)
    }

    pub const fn selected_bridge_fallback(
        requested: CaptureBackendKind,
        reason: CaptureRouteFallbackReason,
    ) -> Self {
        Self::fallback(requested, reason)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRouteOptions {
    pub preference: CaptureRoutePreference,
    pub wgc_available: bool,
    pub dxgi_available: bool,
}

impl CaptureRouteOptions {
    pub const fn existing_cpu() -> Self {
        Self {
            preference: CaptureRoutePreference::ExistingCpu,
            wgc_available: false,
            dxgi_available: false,
        }
    }

    pub const fn prefer_wgc(wgc_available: bool, dxgi_available: bool) -> Self {
        Self {
            preference: CaptureRoutePreference::WindowsGraphicsCapture,
            wgc_available,
            dxgi_available,
        }
    }

    pub const fn prefer_dxgi(dxgi_available: bool) -> Self {
        Self {
            preference: CaptureRoutePreference::DesktopDuplication,
            wgc_available: false,
            dxgi_available,
        }
    }
}

impl Default for CaptureRouteOptions {
    fn default() -> Self {
        Self::existing_cpu()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureDiagnosticsRouteOptions<'a> {
    pub preference: CaptureRoutePreference,
    pub d3d11_probe: &'a D3d11GpuProbeReport,
    pub wgc_probe: Option<&'a WgcOneFrameProbePlan>,
    pub dxgi_contract: Option<&'a DxgiDesktopDuplicationContract>,
}

impl<'a> CaptureDiagnosticsRouteOptions<'a> {
    pub const fn prefer_wgc(
        d3d11_probe: &'a D3d11GpuProbeReport,
        wgc_probe: &'a WgcOneFrameProbePlan,
        dxgi_contract: Option<&'a DxgiDesktopDuplicationContract>,
    ) -> Self {
        Self {
            preference: CaptureRoutePreference::WindowsGraphicsCapture,
            d3d11_probe,
            wgc_probe: Some(wgc_probe),
            dxgi_contract,
        }
    }

    pub const fn prefer_dxgi(
        d3d11_probe: &'a D3d11GpuProbeReport,
        dxgi_contract: &'a DxgiDesktopDuplicationContract,
    ) -> Self {
        Self {
            preference: CaptureRoutePreference::DesktopDuplication,
            d3d11_probe,
            wgc_probe: None,
            dxgi_contract: Some(dxgi_contract),
        }
    }
}

pub const fn default_capture_route_decision() -> CaptureRouteDecision {
    resolve_capture_route(CaptureRouteOptions::existing_cpu())
}

pub const fn resolve_capture_route(options: CaptureRouteOptions) -> CaptureRouteDecision {
    match options.preference {
        CaptureRoutePreference::ExistingCpu => {
            CaptureRouteDecision::ready(CaptureBackendKind::ExistingCpu)
        }
        CaptureRoutePreference::WindowsGraphicsCapture => {
            if options.wgc_available {
                CaptureRouteDecision::fallback(
                    CaptureBackendKind::WindowsGraphicsCapture,
                    CaptureRouteFallbackReason::GpuBackendPlaceholder,
                )
            } else if options.dxgi_available {
                CaptureRouteDecision::fallback(
                    CaptureBackendKind::DesktopDuplication,
                    CaptureRouteFallbackReason::GpuBackendPlaceholder,
                )
            } else {
                CaptureRouteDecision::fallback(
                    CaptureBackendKind::WindowsGraphicsCapture,
                    CaptureRouteFallbackReason::GpuBackendUnavailable,
                )
            }
        }
        CaptureRoutePreference::DesktopDuplication => {
            if options.dxgi_available {
                CaptureRouteDecision::fallback(
                    CaptureBackendKind::DesktopDuplication,
                    CaptureRouteFallbackReason::GpuBackendPlaceholder,
                )
            } else {
                CaptureRouteDecision::fallback(
                    CaptureBackendKind::DesktopDuplication,
                    CaptureRouteFallbackReason::GpuBackendUnavailable,
                )
            }
        }
    }
}

pub fn resolve_diagnostics_capture_route(
    options: CaptureDiagnosticsRouteOptions<'_>,
) -> CaptureRouteDecision {
    match options.preference {
        CaptureRoutePreference::ExistingCpu => {
            CaptureRouteDecision::ready(CaptureBackendKind::ExistingCpu)
        }
        CaptureRoutePreference::WindowsGraphicsCapture => resolve_wgc_diagnostics_route(
            options.d3d11_probe,
            options.wgc_probe,
            options.dxgi_contract,
        ),
        CaptureRoutePreference::DesktopDuplication => {
            resolve_dxgi_diagnostics_route(options.d3d11_probe, options.dxgi_contract)
        }
    }
}

fn d3d11_is_capture_ready(report: &D3d11GpuProbeReport) -> bool {
    matches!(report.capability.status, GpuCaptureStatus::Ready)
}

fn resolve_wgc_diagnostics_route(
    d3d11_probe: &D3d11GpuProbeReport,
    wgc_probe: Option<&WgcOneFrameProbePlan>,
    dxgi_contract: Option<&DxgiDesktopDuplicationContract>,
) -> CaptureRouteDecision {
    let Some(wgc_probe) = wgc_probe else {
        return CaptureRouteDecision::wgc_probe_fallback(
            CaptureRouteFallbackReason::WgcProbePendingApiWiring,
        );
    };

    if matches!(wgc_probe.status, WgcOneFrameProbeStatus::ProbeReady) {
        return CaptureRouteDecision::wgc_probe_fallback(if d3d11_is_capture_ready(d3d11_probe) {
            CaptureRouteFallbackReason::WgcFrameAcquisitionContractOnly
        } else {
            CaptureRouteFallbackReason::D3d11ProbeUnavailable
        });
    }

    if let Some(dxgi_contract) = dxgi_contract {
        let dxgi_decision = resolve_dxgi_diagnostics_route(d3d11_probe, Some(dxgi_contract));
        if !dxgi_decision.uses_fallback() {
            return dxgi_decision;
        }
    }

    CaptureRouteDecision::wgc_probe_fallback(wgc_fallback_reason(wgc_probe))
}

fn resolve_dxgi_diagnostics_route(
    d3d11_probe: &D3d11GpuProbeReport,
    dxgi_contract: Option<&DxgiDesktopDuplicationContract>,
) -> CaptureRouteDecision {
    let Some(dxgi_contract) = dxgi_contract else {
        return CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::DxgiCaptureUnavailable,
        );
    };

    if !d3d11_is_capture_ready(d3d11_probe) {
        return CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::D3d11ProbeUnavailable,
        );
    }

    match dxgi_contract.readiness {
        DxgiCaptureReadiness::Ready => CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::DxgiFrameAcquisitionContractOnly,
        ),
        DxgiCaptureReadiness::PlaceholderOnly => CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::DxgiCapturePlaceholder,
        ),
        DxgiCaptureReadiness::Blocked => CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::DxgiContractBlocked,
        ),
    }
}

fn wgc_fallback_reason(plan: &WgcOneFrameProbePlan) -> CaptureRouteFallbackReason {
    match plan.status {
        WgcOneFrameProbeStatus::Disabled => CaptureRouteFallbackReason::WgcProbeDisabled,
        WgcOneFrameProbeStatus::GuardedDiagnosticsReady => {
            CaptureRouteFallbackReason::WgcProbePendingApiWiring
        }
        WgcOneFrameProbeStatus::FallbackPlanned => {
            CaptureRouteFallbackReason::WgcNativeApiUnavailable
        }
        WgcOneFrameProbeStatus::InvalidRequest => {
            CaptureRouteFallbackReason::WgcProbeRejectedRealApi
        }
        WgcOneFrameProbeStatus::ProbeReady => {
            CaptureRouteFallbackReason::WgcFrameAcquisitionContractOnly
        }
    }
}

#[cfg(test)]
#[path = "capture_router_tests.rs"]
mod capture_router_tests;

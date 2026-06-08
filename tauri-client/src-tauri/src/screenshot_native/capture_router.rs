use super::capture::{CaptureBackendContract, CaptureBackendKind};
use super::gpu::GpuCaptureBackend;

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
    WgcProbeDisabled,
    WgcProbePendingApiWiring,
    WgcProbeRejectedRealApi,
    DxgiCapturePlaceholder,
    DxgiCaptureUnavailable,
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
            Self::WgcProbeDisabled => "wgc-probe-disabled",
            Self::WgcProbePendingApiWiring => "wgc-probe-pending-api-wiring",
            Self::WgcProbeRejectedRealApi => "wgc-probe-rejected-real-api",
            Self::DxgiCapturePlaceholder => "dxgi-capture-placeholder",
            Self::DxgiCaptureUnavailable => "dxgi-capture-unavailable",
            Self::SelectedBridgeUnavailable => "selected-bridge-unavailable",
            Self::SelectedBridgeInvalidImage => "selected-bridge-invalid-image",
            Self::SelectedBridgeUnsupportedAction => "selected-bridge-unsupported-action",
        }
    }

    pub const fn is_probe_or_bridge_reason(self) -> bool {
        matches!(
            self,
            Self::WgcProbeDisabled
                | Self::WgcProbePendingApiWiring
                | Self::WgcProbeRejectedRealApi
                | Self::DxgiCapturePlaceholder
                | Self::DxgiCaptureUnavailable
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_existing_cpu() {
        let decision = default_capture_route_decision();

        assert_eq!(decision.requested, CaptureBackendKind::ExistingCpu);
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(decision.status, CaptureRouteStatus::Ready);
        assert!(!decision.uses_fallback());
    }

    #[test]
    fn wgc_placeholder_falls_back_to_existing_cpu() {
        let decision = resolve_capture_route(CaptureRouteOptions::prefer_wgc(true, false));

        assert_eq!(
            decision.requested,
            CaptureBackendKind::WindowsGraphicsCapture
        );
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(
            decision.fallback_reason,
            CaptureRouteFallbackReason::GpuBackendPlaceholder
        );
        assert!(decision.uses_fallback());
    }

    #[test]
    fn dxgi_placeholder_falls_back_to_existing_cpu() {
        let decision = resolve_capture_route(CaptureRouteOptions::prefer_dxgi(true));

        assert_eq!(decision.requested, CaptureBackendKind::DesktopDuplication);
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(
            decision.fallback_reason,
            CaptureRouteFallbackReason::GpuBackendPlaceholder
        );
        assert!(decision.uses_fallback());
    }

    #[test]
    fn wgc_probe_reason_can_drive_route_decision() {
        let decision = CaptureRouteDecision::wgc_probe_fallback(
            CaptureRouteFallbackReason::WgcProbePendingApiWiring,
        );

        assert_eq!(
            decision.requested,
            CaptureBackendKind::WindowsGraphicsCapture
        );
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(
            decision.fallback_reason.as_str(),
            "wgc-probe-pending-api-wiring"
        );
        assert!(decision.fallback_reason.is_probe_or_bridge_reason());
    }

    #[test]
    fn dxgi_capture_reason_can_drive_route_decision() {
        let decision = CaptureRouteDecision::dxgi_capture_fallback(
            CaptureRouteFallbackReason::DxgiCapturePlaceholder,
        );

        assert_eq!(decision.requested, CaptureBackendKind::DesktopDuplication);
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(
            decision.fallback_reason.as_str(),
            "dxgi-capture-placeholder"
        );
        assert!(decision.fallback_reason.is_probe_or_bridge_reason());
    }

    #[test]
    fn selected_bridge_reason_can_drive_route_decision() {
        let decision = CaptureRouteDecision::selected_bridge_fallback(
            CaptureBackendKind::ExistingCpu,
            CaptureRouteFallbackReason::SelectedBridgeInvalidImage,
        );

        assert_eq!(decision.requested, CaptureBackendKind::ExistingCpu);
        assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
        assert_eq!(decision.status, CaptureRouteStatus::Fallback);
        assert_eq!(
            decision.fallback_reason.as_str(),
            "selected-bridge-invalid-image"
        );
    }
}

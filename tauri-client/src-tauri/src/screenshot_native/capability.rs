use super::capture::CaptureBackendKind;
use super::capture_router::{default_capture_route_decision, CaptureRouteDecision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayCapabilityFlag {
    Disabled,
    EnabledMvp,
}

impl NativeOverlayCapabilityFlag {
    pub const fn from_enabled(enabled: bool) -> Self {
        if enabled {
            Self::EnabledMvp
        } else {
            Self::Disabled
        }
    }

    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::EnabledMvp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlayFallbackReason {
    CapabilityDisabled,
    MvpNotWired,
    NativeGpuCaptureFallback,
}

impl NativeOverlayFallbackReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityDisabled => "capability-disabled",
            Self::MvpNotWired => "mvp-not-wired",
            Self::NativeGpuCaptureFallback => "native-gpu-capture-fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotOverlayRuntime {
    WebviewRgba,
    NativeOverlayMvp,
}

impl ScreenshotOverlayRuntime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WebviewRgba => "webview-rgba",
            Self::NativeOverlayMvp => "native-overlay-mvp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlayLaunchPlan {
    pub capability: NativeOverlayCapabilityFlag,
    pub runtime: ScreenshotOverlayRuntime,
    pub fallback_reason: Option<NativeOverlayFallbackReason>,
    pub capture_route: CaptureRouteDecision,
}

impl NativeOverlayLaunchPlan {
    pub const fn fallback(reason: NativeOverlayFallbackReason) -> Self {
        Self {
            capability: NativeOverlayCapabilityFlag::Disabled,
            runtime: ScreenshotOverlayRuntime::WebviewRgba,
            fallback_reason: Some(reason),
            capture_route: default_capture_route_decision(),
        }
    }

    pub const fn native_mvp(
        capability: NativeOverlayCapabilityFlag,
        capture_route: CaptureRouteDecision,
    ) -> Self {
        Self {
            capability,
            runtime: ScreenshotOverlayRuntime::NativeOverlayMvp,
            fallback_reason: None,
            capture_route,
        }
    }

    pub const fn uses_native_overlay(self) -> bool {
        matches!(self.runtime, ScreenshotOverlayRuntime::NativeOverlayMvp)
    }

    pub const fn uses_native_gpu_capture(self) -> bool {
        self.uses_native_overlay()
            && matches!(
                self.capture_route.selected,
                CaptureBackendKind::WindowsGraphicsCapture | CaptureBackendKind::DesktopDuplication
            )
    }
}

pub const fn resolve_native_overlay_launch_plan(
    capability: NativeOverlayCapabilityFlag,
) -> NativeOverlayLaunchPlan {
    resolve_native_overlay_launch_plan_with_route(capability, default_capture_route_decision())
}

pub const fn resolve_native_overlay_launch_plan_with_route(
    capability: NativeOverlayCapabilityFlag,
    capture_route: CaptureRouteDecision,
) -> NativeOverlayLaunchPlan {
    if capability.is_enabled() {
        if capture_route.uses_fallback()
            || matches!(
                capture_route.selected,
                CaptureBackendKind::ExistingCpu
                    | CaptureBackendKind::WindowsGraphicsCapture
                    | CaptureBackendKind::DesktopDuplication
            )
        {
            NativeOverlayLaunchPlan {
                capability,
                runtime: ScreenshotOverlayRuntime::WebviewRgba,
                fallback_reason: Some(NativeOverlayFallbackReason::NativeGpuCaptureFallback),
                capture_route,
            }
        } else {
            NativeOverlayLaunchPlan::native_mvp(capability, capture_route)
        }
    } else {
        NativeOverlayLaunchPlan::fallback(NativeOverlayFallbackReason::CapabilityDisabled)
    }
}

pub const fn default_native_overlay_capability() -> NativeOverlayCapabilityFlag {
    NativeOverlayCapabilityFlag::Disabled
}

pub const fn default_native_overlay_launch_plan() -> NativeOverlayLaunchPlan {
    resolve_native_overlay_launch_plan(default_native_overlay_capability())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenshot_native::{CaptureRouteFallbackReason, CaptureRouteStatus};

    #[test]
    fn enabled_capability_rejects_contract_only_wgc_route() {
        let plan = resolve_native_overlay_launch_plan_with_route(
            NativeOverlayCapabilityFlag::EnabledMvp,
            CaptureRouteDecision::ready(CaptureBackendKind::WindowsGraphicsCapture),
        );

        assert!(!plan.uses_native_overlay());
        assert!(!plan.uses_native_gpu_capture());
        assert_eq!(
            plan.fallback_reason,
            Some(NativeOverlayFallbackReason::NativeGpuCaptureFallback)
        );
    }

    #[test]
    fn enabled_capability_keeps_route_fallback_reason() {
        let plan = resolve_native_overlay_launch_plan_with_route(
            NativeOverlayCapabilityFlag::EnabledMvp,
            CaptureRouteDecision::fallback(
                CaptureBackendKind::WindowsGraphicsCapture,
                CaptureRouteFallbackReason::D3d11ProbeUnavailable,
            ),
        );

        assert!(!plan.uses_native_overlay());
        assert_eq!(
            plan.fallback_reason,
            Some(NativeOverlayFallbackReason::NativeGpuCaptureFallback)
        );
        assert_eq!(plan.capture_route.status, CaptureRouteStatus::Fallback);
        assert_eq!(
            plan.capture_route.fallback_reason,
            CaptureRouteFallbackReason::D3d11ProbeUnavailable
        );
    }
}

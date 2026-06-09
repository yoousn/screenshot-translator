use super::capture::{CaptureBackendKind, CaptureReadbackMode};
use super::{NativeOverlayLaunchPlan, Win32OverlayPumpContract};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScreenshotMainRoute {
    WebviewRgba,
    NativeDxgiSelectedOutput,
}

impl NativeScreenshotMainRoute {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WebviewRgba => "webview-rgba",
            Self::NativeDxgiSelectedOutput => "native-dxgi-selected-output",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeRouteReadinessBlocker {
    NativeOverlayLaunchPlanFallback,
    NativeOverlayRuntimeNotNative,
    PumpDoesNotOwnThread,
    PumpDoesNotDispatchInput,
    PumpDoesNotRestoreFocus,
    MessageLoopDispatchNotReady,
    WndProcInputNotReady,
    FocusRestoreNotIntegrated,
    AltAOwnedByGlobalShortcut,
    CaptureRouteFallback,
    CaptureRouteNotDxgi,
    CaptureRouteNotGpuReadback,
    SelectedOutputEffectsNotReady,
}

impl NativeRouteReadinessBlocker {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeOverlayLaunchPlanFallback => "native-overlay-launch-plan-fallback",
            Self::NativeOverlayRuntimeNotNative => "native-overlay-runtime-not-native",
            Self::PumpDoesNotOwnThread => "pump-does-not-own-thread",
            Self::PumpDoesNotDispatchInput => "pump-does-not-dispatch-input",
            Self::PumpDoesNotRestoreFocus => "pump-does-not-restore-focus",
            Self::MessageLoopDispatchNotReady => "message-loop-dispatch-not-ready",
            Self::WndProcInputNotReady => "wndproc-input-not-ready",
            Self::FocusRestoreNotIntegrated => "focus-restore-not-integrated",
            Self::AltAOwnedByGlobalShortcut => "alt-a-owned-by-global-shortcut",
            Self::CaptureRouteFallback => "capture-route-fallback",
            Self::CaptureRouteNotDxgi => "capture-route-not-dxgi",
            Self::CaptureRouteNotGpuReadback => "capture-route-not-gpu-readback",
            Self::SelectedOutputEffectsNotReady => "selected-output-effects-not-ready",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRouteReadinessInputs {
    pub launch_plan: NativeOverlayLaunchPlan,
    pub pump_contract: Win32OverlayPumpContract,
    pub message_loop_dispatch_ready: bool,
    pub wndproc_input_dispatch_ready: bool,
    pub focus_restore_integrated: bool,
    pub alt_a_owned_by_native_route: bool,
    pub selected_output_effects_ready: bool,
}

impl NativeRouteReadinessInputs {
    pub const fn current_diagnostics(
        launch_plan: NativeOverlayLaunchPlan,
        pump_contract: Win32OverlayPumpContract,
    ) -> Self {
        Self {
            launch_plan,
            pump_contract,
            message_loop_dispatch_ready: false,
            wndproc_input_dispatch_ready: false,
            focus_restore_integrated: false,
            alt_a_owned_by_native_route: false,
            selected_output_effects_ready: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRouteReadinessDecision {
    pub recommended_route: NativeScreenshotMainRoute,
    pub ready_for_native_dxgi_selected_output: bool,
    pub blockers: Vec<NativeRouteReadinessBlocker>,
}

impl NativeRouteReadinessDecision {
    pub fn blocker_labels(&self) -> Vec<&'static str> {
        self.blockers
            .iter()
            .map(|blocker| blocker.as_str())
            .collect()
    }
}

pub fn default_native_route_readiness(
    launch_plan: NativeOverlayLaunchPlan,
    pump_contract: Win32OverlayPumpContract,
) -> NativeRouteReadinessDecision {
    resolve_native_route_readiness(NativeRouteReadinessInputs::current_diagnostics(
        launch_plan,
        pump_contract,
    ))
}

pub fn resolve_native_route_readiness(
    inputs: NativeRouteReadinessInputs,
) -> NativeRouteReadinessDecision {
    let mut blockers = Vec::new();

    if inputs.launch_plan.fallback_reason.is_some() {
        blockers.push(NativeRouteReadinessBlocker::NativeOverlayLaunchPlanFallback);
    }
    if !inputs.launch_plan.uses_native_overlay() {
        blockers.push(NativeRouteReadinessBlocker::NativeOverlayRuntimeNotNative);
    }
    if !inputs.pump_contract.owns_thread {
        blockers.push(NativeRouteReadinessBlocker::PumpDoesNotOwnThread);
    }
    if !inputs.pump_contract.dispatches_input {
        blockers.push(NativeRouteReadinessBlocker::PumpDoesNotDispatchInput);
    }
    if !inputs.pump_contract.restores_focus_on_exit {
        blockers.push(NativeRouteReadinessBlocker::PumpDoesNotRestoreFocus);
    }
    if !inputs.message_loop_dispatch_ready {
        blockers.push(NativeRouteReadinessBlocker::MessageLoopDispatchNotReady);
    }
    if !inputs.wndproc_input_dispatch_ready {
        blockers.push(NativeRouteReadinessBlocker::WndProcInputNotReady);
    }
    if !inputs.focus_restore_integrated {
        blockers.push(NativeRouteReadinessBlocker::FocusRestoreNotIntegrated);
    }
    if !inputs.alt_a_owned_by_native_route {
        blockers.push(NativeRouteReadinessBlocker::AltAOwnedByGlobalShortcut);
    }
    if inputs.launch_plan.capture_route.uses_fallback() {
        blockers.push(NativeRouteReadinessBlocker::CaptureRouteFallback);
    }
    if !matches!(
        inputs.launch_plan.capture_route.selected,
        CaptureBackendKind::DesktopDuplication
    ) {
        blockers.push(NativeRouteReadinessBlocker::CaptureRouteNotDxgi);
    }
    if !matches!(
        inputs.launch_plan.capture_route.contract.readback_mode,
        CaptureReadbackMode::GpuTextureReadback
    ) {
        blockers.push(NativeRouteReadinessBlocker::CaptureRouteNotGpuReadback);
    }
    if !inputs.selected_output_effects_ready {
        blockers.push(NativeRouteReadinessBlocker::SelectedOutputEffectsNotReady);
    }

    let ready_for_native_dxgi_selected_output = blockers.is_empty();
    let recommended_route = if ready_for_native_dxgi_selected_output {
        NativeScreenshotMainRoute::NativeDxgiSelectedOutput
    } else {
        NativeScreenshotMainRoute::WebviewRgba
    };

    NativeRouteReadinessDecision {
        recommended_route,
        ready_for_native_dxgi_selected_output,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screenshot_native::{
        CaptureBackendKind, CaptureRouteDecision, NativeOverlayCapabilityFlag,
    };

    fn production_ready_pump_contract() -> Win32OverlayPumpContract {
        Win32OverlayPumpContract {
            owns_thread: true,
            dispatches_input: true,
            blocks_until_terminal: true,
            supports_timeout: true,
            restores_focus_on_exit: true,
        }
    }

    #[test]
    fn current_diagnostics_keep_webview_route_with_explicit_blockers() {
        let decision = default_native_route_readiness(
            crate::screenshot_native::default_native_overlay_launch_plan(),
            crate::screenshot_native::win32_overlay_pump_contract(),
        );
        let blockers = decision.blocker_labels();

        assert_eq!(
            decision.recommended_route,
            NativeScreenshotMainRoute::WebviewRgba
        );
        assert!(!decision.ready_for_native_dxgi_selected_output);
        assert!(blockers.contains(&"native-overlay-launch-plan-fallback"));
        assert!(blockers.contains(&"native-overlay-runtime-not-native"));
        assert!(blockers.contains(&"pump-does-not-own-thread"));
        assert!(blockers.contains(&"pump-does-not-restore-focus"));
        assert!(blockers.contains(&"wndproc-input-not-ready"));
        assert!(blockers.contains(&"alt-a-owned-by-global-shortcut"));
        assert!(blockers.contains(&"capture-route-not-dxgi"));
        assert!(blockers.contains(&"selected-output-effects-not-ready"));
    }

    #[test]
    fn ready_contract_selects_native_dxgi_selected_output() {
        let decision = resolve_native_route_readiness(NativeRouteReadinessInputs {
            launch_plan: NativeOverlayLaunchPlan::native_mvp(
                NativeOverlayCapabilityFlag::EnabledMvp,
                CaptureRouteDecision::ready(CaptureBackendKind::DesktopDuplication),
            ),
            pump_contract: production_ready_pump_contract(),
            message_loop_dispatch_ready: true,
            wndproc_input_dispatch_ready: true,
            focus_restore_integrated: true,
            alt_a_owned_by_native_route: true,
            selected_output_effects_ready: true,
        });

        assert_eq!(
            decision.recommended_route,
            NativeScreenshotMainRoute::NativeDxgiSelectedOutput
        );
        assert!(decision.ready_for_native_dxgi_selected_output);
        assert!(decision.blockers.is_empty());
    }
}

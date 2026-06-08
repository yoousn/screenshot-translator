#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySmokeStep {
    ResolveCapability,
    PlanLaunch,
    CreateOverlayWindow,
    AttachInputBridge,
    RenderBackdrop,
    RenderSelectionChrome,
    CaptureSelection,
    ReportOutcome,
}

impl NativeOverlaySmokeStep {
    pub const ALL: [Self; 8] = [
        Self::ResolveCapability,
        Self::PlanLaunch,
        Self::CreateOverlayWindow,
        Self::AttachInputBridge,
        Self::RenderBackdrop,
        Self::RenderSelectionChrome,
        Self::CaptureSelection,
        Self::ReportOutcome,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ResolveCapability => "resolve-capability",
            Self::PlanLaunch => "plan-launch",
            Self::CreateOverlayWindow => "create-overlay-window",
            Self::AttachInputBridge => "attach-input-bridge",
            Self::RenderBackdrop => "render-backdrop",
            Self::RenderSelectionChrome => "render-selection-chrome",
            Self::CaptureSelection => "capture-selection",
            Self::ReportOutcome => "report-outcome",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySmokeStatus {
    Planned,
    Passed,
    Failed,
    Skipped,
}

impl NativeOverlaySmokeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Passed | Self::Failed | Self::Skipped)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySmokeFailureKind {
    CapabilityUnavailable,
    LaunchPlanRejected,
    OverlayWindowUnavailable,
    InputBridgeUnavailable,
    RenderUnavailable,
    CaptureUnavailable,
    UnexpectedRuntimeError,
}

impl NativeOverlaySmokeFailureKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityUnavailable => "capability-unavailable",
            Self::LaunchPlanRejected => "launch-plan-rejected",
            Self::OverlayWindowUnavailable => "overlay-window-unavailable",
            Self::InputBridgeUnavailable => "input-bridge-unavailable",
            Self::RenderUnavailable => "render-unavailable",
            Self::CaptureUnavailable => "capture-unavailable",
            Self::UnexpectedRuntimeError => "unexpected-runtime-error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySmokeRecoveryAction {
    KeepWebviewFallback,
    RetryNativeOverlay,
    DisableNativeOverlayForSession,
    CollectDiagnostics,
}

impl NativeOverlaySmokeRecoveryAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepWebviewFallback => "keep-webview-fallback",
            Self::RetryNativeOverlay => "retry-native-overlay",
            Self::DisableNativeOverlayForSession => "disable-native-overlay-for-session",
            Self::CollectDiagnostics => "collect-diagnostics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlaySmokeFailure {
    pub step: NativeOverlaySmokeStep,
    pub kind: NativeOverlaySmokeFailureKind,
    pub recovery_action: NativeOverlaySmokeRecoveryAction,
    pub message: &'static str,
}

impl NativeOverlaySmokeFailure {
    pub const fn new(
        step: NativeOverlaySmokeStep,
        kind: NativeOverlaySmokeFailureKind,
        recovery_action: NativeOverlaySmokeRecoveryAction,
        message: &'static str,
    ) -> Self {
        Self {
            step,
            kind,
            recovery_action,
            message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlaySmokeStepReport {
    pub step: NativeOverlaySmokeStep,
    pub status: NativeOverlaySmokeStatus,
    pub failure: Option<NativeOverlaySmokeFailure>,
}

impl NativeOverlaySmokeStepReport {
    pub const fn planned(step: NativeOverlaySmokeStep) -> Self {
        Self {
            step,
            status: NativeOverlaySmokeStatus::Planned,
            failure: None,
        }
    }

    pub const fn passed(step: NativeOverlaySmokeStep) -> Self {
        Self {
            step,
            status: NativeOverlaySmokeStatus::Passed,
            failure: None,
        }
    }

    pub const fn skipped(step: NativeOverlaySmokeStep) -> Self {
        Self {
            step,
            status: NativeOverlaySmokeStatus::Skipped,
            failure: None,
        }
    }

    pub const fn failed(step: NativeOverlaySmokeStep, failure: NativeOverlaySmokeFailure) -> Self {
        Self {
            step,
            status: NativeOverlaySmokeStatus::Failed,
            failure: Some(failure),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlaySmokePlan {
    pub name: &'static str,
    pub steps: &'static [NativeOverlaySmokeStep],
    pub requires_native_window: bool,
    pub requires_input_bridge: bool,
    pub requires_capture_backend: bool,
    pub has_runtime_side_effects: bool,
}

impl NativeOverlaySmokePlan {
    pub const fn mvp() -> Self {
        Self {
            name: "native-overlay-mvp-smoke",
            steps: &NativeOverlaySmokeStep::ALL,
            requires_native_window: true,
            requires_input_bridge: true,
            requires_capture_backend: true,
            has_runtime_side_effects: false,
        }
    }

    pub const fn planned_report(self) -> NativeOverlaySmokeReport {
        NativeOverlaySmokeReport {
            plan: self,
            status: NativeOverlaySmokeStatus::Planned,
            completed_steps: 0,
            failed_step: None,
            recovery_action: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlaySmokeReport {
    pub plan: NativeOverlaySmokePlan,
    pub status: NativeOverlaySmokeStatus,
    pub completed_steps: usize,
    pub failed_step: Option<NativeOverlaySmokeStep>,
    pub recovery_action: Option<NativeOverlaySmokeRecoveryAction>,
}

impl NativeOverlaySmokeReport {
    pub const fn passed(plan: NativeOverlaySmokePlan) -> Self {
        Self {
            plan,
            status: NativeOverlaySmokeStatus::Passed,
            completed_steps: plan.steps.len(),
            failed_step: None,
            recovery_action: None,
        }
    }

    pub const fn skipped(
        plan: NativeOverlaySmokePlan,
        recovery_action: NativeOverlaySmokeRecoveryAction,
    ) -> Self {
        Self {
            plan,
            status: NativeOverlaySmokeStatus::Skipped,
            completed_steps: 0,
            failed_step: None,
            recovery_action: Some(recovery_action),
        }
    }

    pub const fn failed(
        plan: NativeOverlaySmokePlan,
        completed_steps: usize,
        failure: NativeOverlaySmokeFailure,
    ) -> Self {
        Self {
            plan,
            status: NativeOverlaySmokeStatus::Failed,
            completed_steps,
            failed_step: Some(failure.step),
            recovery_action: Some(failure.recovery_action),
        }
    }

    pub const fn is_success(self) -> bool {
        matches!(self.status, NativeOverlaySmokeStatus::Passed)
    }
}

pub const fn default_native_overlay_smoke_plan() -> NativeOverlaySmokePlan {
    NativeOverlaySmokePlan::mvp()
}

pub const fn planned_native_overlay_smoke_report() -> NativeOverlaySmokeReport {
    default_native_overlay_smoke_plan().planned_report()
}

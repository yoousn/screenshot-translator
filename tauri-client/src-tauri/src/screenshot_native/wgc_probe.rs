use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcOneFrameProbeDefault {
    Disabled,
}

impl WgcOneFrameProbeDefault {
    pub const fn is_enabled(self) -> bool {
        match self {
            Self::Disabled => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcOneFrameProbeStatus {
    Disabled,
    FallbackPlanned,
    ProbePendingApiWiring,
    InvalidRequest,
}

impl WgcOneFrameProbeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::FallbackPlanned => "fallback-planned",
            Self::ProbePendingApiWiring => "probe-pending-api-wiring",
            Self::InvalidRequest => "invalid-request",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcOneFrameProbeFallback {
    ExistingScreenshotPath,
    DesktopDuplicationPlaceholder,
    Unavailable,
}

impl WgcOneFrameProbeFallback {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExistingScreenshotPath => "existing-screenshot-path",
            Self::DesktopDuplicationPlaceholder => "desktop-duplication-placeholder",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WgcOneFrameProbeError {
    DefaultEnableRejected,
    RealApiCallRejected,
    InvalidFrameTimeoutMs { timeout_ms: u64 },
    NativeApiNotWired,
}

impl WgcOneFrameProbeError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DefaultEnableRejected => "wgc-probe-default-enable-rejected",
            Self::RealApiCallRejected => "wgc-real-api-call-rejected",
            Self::InvalidFrameTimeoutMs { .. } => "wgc-probe-invalid-timeout",
            Self::NativeApiNotWired => "wgc-native-api-not-wired",
        }
    }
}

impl fmt::Display for WgcOneFrameProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultEnableRejected => write!(
                formatter,
                "WGC one-frame probe must remain disabled by default"
            ),
            Self::RealApiCallRejected => write!(
                formatter,
                "WGC one-frame probe placeholder is not allowed to call real WGC APIs"
            ),
            Self::InvalidFrameTimeoutMs { timeout_ms } => write!(
                formatter,
                "invalid WGC one-frame probe timeout: {timeout_ms}ms"
            ),
            Self::NativeApiNotWired => write!(
                formatter,
                "WGC one-frame probe native API wiring is not implemented yet"
            ),
        }
    }
}

impl std::error::Error for WgcOneFrameProbeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WgcOneFrameProbeContract {
    pub default: WgcOneFrameProbeDefault,
    pub requires_explicit_opt_in: bool,
    pub may_call_real_wgc_api: bool,
}

impl WgcOneFrameProbeContract {
    pub const fn disabled_placeholder() -> Self {
        Self {
            default: WgcOneFrameProbeDefault::Disabled,
            requires_explicit_opt_in: true,
            may_call_real_wgc_api: false,
        }
    }

    pub const fn validates_no_default_enable(self) -> bool {
        !self.default.is_enabled() && self.requires_explicit_opt_in && !self.may_call_real_wgc_api
    }
}

impl Default for WgcOneFrameProbeContract {
    fn default() -> Self {
        Self::disabled_placeholder()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WgcOneFrameProbeRequest {
    pub explicit_opt_in: bool,
    pub allow_real_wgc_api: bool,
    pub frame_timeout_ms: u64,
}

impl WgcOneFrameProbeRequest {
    pub const fn disabled() -> Self {
        Self {
            explicit_opt_in: false,
            allow_real_wgc_api: false,
            frame_timeout_ms: 0,
        }
    }

    pub const fn explicit_placeholder(frame_timeout_ms: u64) -> Self {
        Self {
            explicit_opt_in: true,
            allow_real_wgc_api: false,
            frame_timeout_ms,
        }
    }
}

impl Default for WgcOneFrameProbeRequest {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcOneFrameProbePlan {
    pub contract: WgcOneFrameProbeContract,
    pub status: WgcOneFrameProbeStatus,
    pub should_attempt_probe: bool,
    pub fallback: WgcOneFrameProbeFallback,
    pub error: Option<WgcOneFrameProbeError>,
    pub reason: String,
}

impl WgcOneFrameProbePlan {
    pub fn fallback(
        status: WgcOneFrameProbeStatus,
        fallback: WgcOneFrameProbeFallback,
        error: Option<WgcOneFrameProbeError>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            contract: WgcOneFrameProbeContract::disabled_placeholder(),
            status,
            should_attempt_probe: false,
            fallback,
            error,
            reason: reason.into(),
        }
    }

    pub fn uses_fallback(&self) -> bool {
        !self.should_attempt_probe
            || !matches!(self.fallback, WgcOneFrameProbeFallback::Unavailable)
    }
}

pub const fn default_wgc_one_frame_probe_contract() -> WgcOneFrameProbeContract {
    WgcOneFrameProbeContract::disabled_placeholder()
}

pub fn default_wgc_one_frame_probe_plan() -> WgcOneFrameProbePlan {
    resolve_wgc_one_frame_probe_plan(WgcOneFrameProbeRequest::disabled())
}

pub fn resolve_wgc_one_frame_probe_plan(request: WgcOneFrameProbeRequest) -> WgcOneFrameProbePlan {
    let contract = default_wgc_one_frame_probe_contract();
    if !contract.validates_no_default_enable() {
        return WgcOneFrameProbePlan::fallback(
            WgcOneFrameProbeStatus::InvalidRequest,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            Some(WgcOneFrameProbeError::DefaultEnableRejected),
            "WGC one-frame probe contract rejected a default-enabled configuration.",
        );
    }

    if !request.explicit_opt_in {
        return WgcOneFrameProbePlan::fallback(
            WgcOneFrameProbeStatus::Disabled,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            None,
            "WGC one-frame probe is disabled by default; keep using the existing screenshot path.",
        );
    }

    if request.frame_timeout_ms == 0 {
        return WgcOneFrameProbePlan::fallback(
            WgcOneFrameProbeStatus::InvalidRequest,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            Some(WgcOneFrameProbeError::InvalidFrameTimeoutMs {
                timeout_ms: request.frame_timeout_ms,
            }),
            "WGC one-frame probe needs a positive frame timeout before it can be scheduled.",
        );
    }

    if request.allow_real_wgc_api {
        return WgcOneFrameProbePlan::fallback(
            WgcOneFrameProbeStatus::FallbackPlanned,
            WgcOneFrameProbeFallback::DesktopDuplicationPlaceholder,
            Some(WgcOneFrameProbeError::RealApiCallRejected),
            "Real WGC API calls are intentionally blocked in this phase; use fallback capture.",
        );
    }

    WgcOneFrameProbePlan::fallback(
        WgcOneFrameProbeStatus::ProbePendingApiWiring,
        WgcOneFrameProbeFallback::ExistingScreenshotPath,
        Some(WgcOneFrameProbeError::NativeApiNotWired),
        "WGC one-frame probe request was accepted as a placeholder, but native API wiring is pending.",
    )
}

pub fn run_wgc_one_frame_probe_placeholder(
    request: WgcOneFrameProbeRequest,
) -> Result<WgcOneFrameProbePlan, WgcOneFrameProbeError> {
    let plan = resolve_wgc_one_frame_probe_plan(request);
    match &plan.error {
        Some(error) => Err(error.clone()),
        None => Ok(plan),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_plan_is_disabled_and_falls_back() {
        let plan = default_wgc_one_frame_probe_plan();
        assert_eq!(plan.status, WgcOneFrameProbeStatus::Disabled);
        assert!(!plan.should_attempt_probe);
        assert_eq!(
            plan.fallback,
            WgcOneFrameProbeFallback::ExistingScreenshotPath
        );
        assert!(plan.error.is_none());
    }

    #[test]
    fn explicit_probe_never_calls_real_api_in_placeholder_phase() {
        let plan = resolve_wgc_one_frame_probe_plan(WgcOneFrameProbeRequest {
            explicit_opt_in: true,
            allow_real_wgc_api: true,
            frame_timeout_ms: 500,
        });
        assert_eq!(plan.status, WgcOneFrameProbeStatus::FallbackPlanned);
        assert_eq!(plan.error, Some(WgcOneFrameProbeError::RealApiCallRejected));
        assert!(!plan.should_attempt_probe);
    }
}

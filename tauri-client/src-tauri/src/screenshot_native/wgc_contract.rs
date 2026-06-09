use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum WgcOneFrameProbeDefault {
    Disabled,
}

impl WgcOneFrameProbeDefault {
    pub const fn is_enabled(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum WgcOneFrameProbeStatus {
    Disabled,

    FallbackPlanned,

    ProbeReady,

    GuardedDiagnosticsReady,

    InvalidRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgcOneFrameSmokeStatus {
    NotRun,
    ReadinessOnly,
    ReadyToAttempt,
    FallbackRequired,
    InvalidRequest,
}

impl WgcOneFrameSmokeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotRun => "not-run",
            Self::ReadinessOnly => "readiness-only",
            Self::ReadyToAttempt => "ready-to-attempt",
            Self::FallbackRequired => "fallback-required",
            Self::InvalidRequest => "invalid-request",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum WgcContractStage {
    Device,

    FramePool,

    Session,
}

impl WgcContractStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Device => "device",

            Self::FramePool => "framepool",

            Self::Session => "session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum WgcContractRequirement {
    WindowsGraphicsCaptureApi,

    D3d11Device,

    Bgra8TextureFormat,

    CpuReadbackTexture,

    CaptureItemConsent,

    FramePoolCreated,

    SessionCreated,
}

impl WgcContractRequirement {
    pub const fn stage(self) -> WgcContractStage {
        match self {
            Self::WindowsGraphicsCaptureApi
            | Self::D3d11Device
            | Self::Bgra8TextureFormat
            | Self::CpuReadbackTexture => WgcContractStage::Device,

            Self::FramePoolCreated => WgcContractStage::FramePool,

            Self::CaptureItemConsent | Self::SessionCreated => WgcContractStage::Session,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum WgcOneFrameProbeFallback {
    ExistingScreenshotPath,

    DesktopDuplicationPlaceholder,

    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub enum WgcOneFrameProbeError {
    DefaultEnableRejected,

    RealApiCallRejected,

    InvalidFrameTimeoutMs { timeout_ms: u64 },

    ContractNotReady { requirement: WgcContractRequirement },

    NativeApiUnavailable { reason: String },
}

impl WgcOneFrameProbeError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DefaultEnableRejected => "wgc-probe-default-enable-rejected",

            Self::RealApiCallRejected => "wgc-real-api-call-rejected",

            Self::InvalidFrameTimeoutMs { .. } => "wgc-probe-invalid-timeout",

            Self::ContractNotReady { .. } => "wgc-contract-not-ready",

            Self::NativeApiUnavailable { .. } => "wgc-native-api-unavailable",
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

            Self::ContractNotReady { requirement } => write!(
                formatter,
                "WGC one-frame probe contract is not ready: {requirement:?} ({})",
                requirement.stage().as_str()
            ),

            Self::NativeApiUnavailable { reason } => write!(
                formatter,
                "WGC one-frame probe native API is unavailable: {reason}"
            ),
        }
    }
}

impl std::error::Error for WgcOneFrameProbeError {}

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct WgcNativeApiProbe {
    pub is_windows: bool,

    pub is_supported: bool,

    pub reason: Option<String>,
}

impl WgcNativeApiProbe {
    pub fn supported() -> Self {
        Self {
            is_windows: true,
            is_supported: true,
            reason: None,
        }
    }

    pub fn unavailable(is_windows: bool, reason: impl Into<String>) -> Self {
        Self {
            is_windows,
            is_supported: false,
            reason: Some(reason.into()),
        }
    }

    pub fn fallback_error(&self) -> WgcOneFrameProbeError {
        WgcOneFrameProbeError::NativeApiUnavailable {
            reason: self.reason.clone().unwrap_or_else(|| {
                "Windows Graphics Capture support probe returned false".to_string()
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub struct WgcDeviceContract {
    pub requires_wgc_api: bool,

    pub requires_d3d11_device: bool,

    pub requires_bgra8_format: bool,

    pub requires_cpu_readback: bool,
}

impl WgcDeviceContract {
    pub const fn one_frame_readback() -> Self {
        Self {
            requires_wgc_api: true,
            requires_d3d11_device: true,
            requires_bgra8_format: true,
            requires_cpu_readback: true,
        }
    }

    pub fn missing_requirements(self) -> Vec<WgcContractRequirement> {
        [
            (
                self.requires_wgc_api,
                WgcContractRequirement::WindowsGraphicsCaptureApi,
            ),
            (
                self.requires_d3d11_device,
                WgcContractRequirement::D3d11Device,
            ),
            (
                self.requires_bgra8_format,
                WgcContractRequirement::Bgra8TextureFormat,
            ),
            (
                self.requires_cpu_readback,
                WgcContractRequirement::CpuReadbackTexture,
            ),
        ]
        .into_iter()
        .filter_map(|(required, requirement)| required.then_some(requirement))
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub struct WgcFramePoolContract {
    pub buffer_count: u32,

    pub frame_timeout_ms: u64,

    pub recreates_on_size_change: bool,
}

impl WgcFramePoolContract {
    pub const fn one_frame(frame_timeout_ms: u64) -> Self {
        Self {
            buffer_count: 1,
            frame_timeout_ms,
            recreates_on_size_change: true,
        }
    }

    pub fn missing_requirements(self) -> Vec<WgcContractRequirement> {
        vec![WgcContractRequirement::FramePoolCreated]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub struct WgcSessionContract {
    pub requires_user_consent: bool,

    pub cursor_capture_enabled: bool,

    pub border_required: bool,
}

impl WgcSessionContract {
    pub const fn one_frame() -> Self {
        Self {
            requires_user_consent: true,
            cursor_capture_enabled: false,
            border_required: false,
        }
    }

    pub fn missing_requirements(self) -> Vec<WgcContractRequirement> {
        vec![
            WgcContractRequirement::CaptureItemConsent,
            WgcContractRequirement::SessionCreated,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub struct WgcOneFrameProbeContract {
    pub default: WgcOneFrameProbeDefault,

    pub requires_explicit_opt_in: bool,

    pub may_call_real_wgc_api: bool,

    pub device: WgcDeviceContract,

    pub framepool: WgcFramePoolContract,

    pub session: WgcSessionContract,
}

impl WgcOneFrameProbeContract {
    pub const fn guarded_one_frame(frame_timeout_ms: u64) -> Self {
        Self {
            default: WgcOneFrameProbeDefault::Disabled,

            requires_explicit_opt_in: true,

            may_call_real_wgc_api: false,

            device: WgcDeviceContract::one_frame_readback(),

            framepool: WgcFramePoolContract::one_frame(frame_timeout_ms),

            session: WgcSessionContract::one_frame(),
        }
    }

    pub const fn validates_no_default_enable(self) -> bool {
        !self.default.is_enabled() && self.requires_explicit_opt_in && !self.may_call_real_wgc_api
    }

    pub fn missing_requirements(
        self,
        api_probe: &WgcNativeApiProbe,
    ) -> Vec<WgcContractRequirement> {
        let mut requirements = Vec::new();

        if !api_probe.is_supported {
            requirements.push(WgcContractRequirement::WindowsGraphicsCaptureApi);
        }

        requirements.extend(
            self.device
                .missing_requirements()
                .into_iter()
                .filter(|requirement| {
                    *requirement != WgcContractRequirement::WindowsGraphicsCaptureApi
                }),
        );

        requirements.extend(self.framepool.missing_requirements());

        requirements.extend(self.session.missing_requirements());

        requirements
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

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct WgcOneFrameProbeDiagnostics {
    pub api_probe: WgcNativeApiProbe,

    pub missing_requirements: Vec<WgcContractRequirement>,

    pub next_stage: Option<WgcContractStage>,
}

impl WgcOneFrameProbeDiagnostics {
    pub fn from_contract(contract: WgcOneFrameProbeContract, api_probe: WgcNativeApiProbe) -> Self {
        let missing_requirements = contract.missing_requirements(&api_probe);

        let next_stage = missing_requirements
            .first()
            .map(|requirement| requirement.stage());

        Self {
            api_probe,
            missing_requirements,
            next_stage,
        }
    }

    pub fn first_contract_error(&self) -> Option<WgcOneFrameProbeError> {
        self.missing_requirements
            .first()
            .copied()
            .map(|requirement| WgcOneFrameProbeError::ContractNotReady { requirement })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct WgcOneFrameProbePlan {
    pub contract: WgcOneFrameProbeContract,

    pub status: WgcOneFrameProbeStatus,

    pub should_attempt_probe: bool,

    pub fallback: WgcOneFrameProbeFallback,

    pub error: Option<WgcOneFrameProbeError>,

    pub diagnostics: WgcOneFrameProbeDiagnostics,

    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgcOneFrameSmokeReport {
    pub status: WgcOneFrameSmokeStatus,
    pub plan_status: WgcOneFrameProbeStatus,
    pub attempted_real_wgc_api: bool,
    pub frame_capture_attempted: bool,
    pub frame_capture_confirmed: bool,
    pub should_attempt_probe: bool,
    pub fallback: WgcOneFrameProbeFallback,
    pub error: Option<WgcOneFrameProbeError>,
    pub reason: String,
}

impl WgcOneFrameSmokeReport {
    pub fn from_plan(plan: WgcOneFrameProbePlan) -> Self {
        let status = match plan.status {
            WgcOneFrameProbeStatus::Disabled => WgcOneFrameSmokeStatus::NotRun,
            WgcOneFrameProbeStatus::InvalidRequest => WgcOneFrameSmokeStatus::InvalidRequest,
            WgcOneFrameProbeStatus::ProbeReady if plan.should_attempt_probe => {
                WgcOneFrameSmokeStatus::ReadyToAttempt
            }
            WgcOneFrameProbeStatus::GuardedDiagnosticsReady => {
                WgcOneFrameSmokeStatus::ReadinessOnly
            }
            WgcOneFrameProbeStatus::FallbackPlanned | WgcOneFrameProbeStatus::ProbeReady => {
                WgcOneFrameSmokeStatus::FallbackRequired
            }
        };
        Self {
            status,
            plan_status: plan.status,
            attempted_real_wgc_api: false,
            frame_capture_attempted: false,
            frame_capture_confirmed: false,
            should_attempt_probe: plan.should_attempt_probe,
            fallback: plan.fallback,
            error: plan.error,
            reason: plan.reason,
        }
    }
}

impl WgcOneFrameProbePlan {
    pub fn fallback(
        contract: WgcOneFrameProbeContract,

        diagnostics: WgcOneFrameProbeDiagnostics,

        status: WgcOneFrameProbeStatus,

        fallback: WgcOneFrameProbeFallback,

        error: Option<WgcOneFrameProbeError>,

        reason: impl Into<String>,
    ) -> Self {
        Self {
            contract,
            status,
            should_attempt_probe: false,
            fallback,
            error,
            diagnostics,
            reason: reason.into(),
        }
    }

    pub fn uses_fallback(&self) -> bool {
        !self.should_attempt_probe
            || !matches!(self.fallback, WgcOneFrameProbeFallback::Unavailable)
    }
}

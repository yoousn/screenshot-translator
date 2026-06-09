pub use super::wgc_contract::{
    WgcContractStage, WgcNativeApiProbe, WgcOneFrameProbeContract, WgcOneFrameProbeDiagnostics,
    WgcOneFrameProbeError, WgcOneFrameProbeFallback, WgcOneFrameProbePlan, WgcOneFrameProbeRequest,
    WgcOneFrameProbeStatus, WgcOneFrameSmokeReport, WgcOneFrameSmokeStatus,
};

#[cfg(target_os = "windows")]
pub fn probe_wgc_native_api_support() -> WgcNativeApiProbe {
    match windows::Graphics::Capture::GraphicsCaptureSession::IsSupported() {
        Ok(true) => WgcNativeApiProbe::supported(),
        Ok(false) => WgcNativeApiProbe::unavailable(
            true,
            "GraphicsCaptureSession::IsSupported returned false",
        ),
        Err(error) => WgcNativeApiProbe::unavailable(
            true,
            format!("GraphicsCaptureSession::IsSupported failed: {error}"),
        ),
    }
}
#[cfg(not(target_os = "windows"))]
pub fn probe_wgc_native_api_support() -> WgcNativeApiProbe {
    WgcNativeApiProbe::unavailable(false, "Windows Graphics Capture requires Windows")
}

pub const fn default_wgc_one_frame_probe_contract() -> WgcOneFrameProbeContract {
    WgcOneFrameProbeContract::guarded_one_frame(0)
}
pub fn default_wgc_one_frame_probe_plan() -> WgcOneFrameProbePlan {
    resolve_wgc_one_frame_probe_plan(WgcOneFrameProbeRequest::disabled())
}

pub fn resolve_wgc_one_frame_probe_plan(request: WgcOneFrameProbeRequest) -> WgcOneFrameProbePlan {
    let contract = WgcOneFrameProbeContract::guarded_one_frame(request.frame_timeout_ms);
    let api_probe = if request.allow_real_wgc_api {
        probe_wgc_native_api_support()
    } else {
        WgcNativeApiProbe::unavailable(true, "WGC API support probe skipped until guarded opt-in")
    };
    let diagnostics = WgcOneFrameProbeDiagnostics::from_contract(contract, api_probe.clone());
    if !contract.validates_no_default_enable() {
        return WgcOneFrameProbePlan::fallback(
            contract,
            diagnostics,
            WgcOneFrameProbeStatus::InvalidRequest,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            Some(WgcOneFrameProbeError::DefaultEnableRejected),
            "WGC one-frame probe contract rejected a default-enabled configuration.",
        );
    }
    if !request.explicit_opt_in {
        return WgcOneFrameProbePlan::fallback(
            contract,
            diagnostics,
            WgcOneFrameProbeStatus::Disabled,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            None,
            "WGC one-frame probe is disabled by default; keep using the existing screenshot path.",
        );
    }
    if request.frame_timeout_ms == 0 {
        return WgcOneFrameProbePlan::fallback(
            contract,
            diagnostics,
            WgcOneFrameProbeStatus::InvalidRequest,
            WgcOneFrameProbeFallback::ExistingScreenshotPath,
            Some(WgcOneFrameProbeError::InvalidFrameTimeoutMs {
                timeout_ms: request.frame_timeout_ms,
            }),
            "WGC one-frame probe needs a positive frame timeout before it can be scheduled.",
        );
    }
    if request.allow_real_wgc_api {
        if !api_probe.is_supported {
            return WgcOneFrameProbePlan::fallback(
                contract,
                diagnostics,
                WgcOneFrameProbeStatus::FallbackPlanned,
                WgcOneFrameProbeFallback::DesktopDuplicationPlaceholder,
                Some(api_probe.fallback_error()),
                "Native WGC support is unavailable; use fallback capture.",
            );
        }
        return WgcOneFrameProbePlan {
            contract,
            status: WgcOneFrameProbeStatus::ProbeReady,
            should_attempt_probe: true,
            fallback: WgcOneFrameProbeFallback::DesktopDuplicationPlaceholder,
            error: None,
            diagnostics,
            reason: "Native WGC API support is present; framepool/session wiring can attempt a guarded one-frame probe.".to_string(),
        };
    }
    WgcOneFrameProbePlan::fallback(contract, diagnostics.clone(), WgcOneFrameProbeStatus::GuardedDiagnosticsReady, WgcOneFrameProbeFallback::ExistingScreenshotPath, diagnostics.first_contract_error(), "WGC one-frame probe diagnostics resolved device/framepool/session contracts; real API calls remain blocked until guarded opt-in.")
}

pub fn planned_wgc_one_frame_smoke_report() -> WgcOneFrameSmokeReport {
    WgcOneFrameSmokeReport::from_plan(default_wgc_one_frame_probe_plan())
}

pub fn resolve_wgc_one_frame_smoke_report(
    request: WgcOneFrameProbeRequest,
) -> WgcOneFrameSmokeReport {
    WgcOneFrameSmokeReport::from_plan(resolve_wgc_one_frame_probe_plan(request))
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

    const REAL_API_PROBE_ENV: &str = "YSN_WGC_REAL_API_PROBE_SMOKE";

    fn real_api_probe_enabled() -> bool {
        std::env::var(REAL_API_PROBE_ENV).ok().as_deref() == Some("1")
    }

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
        assert_eq!(plan.diagnostics.next_stage, Some(WgcContractStage::Device));
    }

    #[test]
    fn default_smoke_report_never_claims_frame_capture() {
        let report = planned_wgc_one_frame_smoke_report();
        assert_eq!(report.status, WgcOneFrameSmokeStatus::NotRun);
        assert!(!report.attempted_real_wgc_api);
        assert!(!report.frame_capture_attempted);
        assert!(!report.frame_capture_confirmed);
    }

    #[test]
    fn ready_to_attempt_still_does_not_claim_frame_capture() {
        let report = resolve_wgc_one_frame_smoke_report(WgcOneFrameProbeRequest {
            explicit_opt_in: true,
            allow_real_wgc_api: true,
            frame_timeout_ms: 500,
        });
        if report.should_attempt_probe {
            assert_eq!(report.status, WgcOneFrameSmokeStatus::ReadyToAttempt);
        }
        assert!(!report.attempted_real_wgc_api);
        assert!(!report.frame_capture_attempted);
        assert!(!report.frame_capture_confirmed);
    }

    #[test]
    fn contract_splits_device_framepool_and_session_requirements() {
        let contract = WgcOneFrameProbeContract::guarded_one_frame(250);
        let diagnostics =
            WgcOneFrameProbeDiagnostics::from_contract(contract, WgcNativeApiProbe::supported());
        assert_eq!(contract.framepool.frame_timeout_ms, 250);
        for stage in [
            WgcContractStage::Device,
            WgcContractStage::FramePool,
            WgcContractStage::Session,
        ] {
            assert!(diagnostics
                .missing_requirements
                .iter()
                .any(|requirement| requirement.stage() == stage));
        }
    }

    #[test]
    fn explicit_placeholder_resolves_diagnostics_without_real_api() {
        let plan =
            resolve_wgc_one_frame_probe_plan(WgcOneFrameProbeRequest::explicit_placeholder(500));
        assert_eq!(plan.status, WgcOneFrameProbeStatus::GuardedDiagnosticsReady);
        assert!(!plan.should_attempt_probe);
        assert!(matches!(
            plan.error,
            Some(WgcOneFrameProbeError::ContractNotReady { .. })
        ));
        assert!(plan
            .diagnostics
            .api_probe
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("skipped"));
    }

    #[test]
    #[ignore = "requires real WGC IsSupported probe and YSN_WGC_REAL_API_PROBE_SMOKE=1"]
    fn explicit_real_api_probe_preserves_recoverable_fallback() {
        if !real_api_probe_enabled() {
            eprintln!("skipping WGC real API probe smoke; set {REAL_API_PROBE_ENV}=1 to run");
            return;
        }

        let plan = resolve_wgc_one_frame_probe_plan(WgcOneFrameProbeRequest {
            explicit_opt_in: true,
            allow_real_wgc_api: true,
            frame_timeout_ms: 500,
        });
        assert_eq!(
            plan.fallback,
            WgcOneFrameProbeFallback::DesktopDuplicationPlaceholder
        );
        if plan.should_attempt_probe {
            assert_eq!(plan.status, WgcOneFrameProbeStatus::ProbeReady);
            assert!(plan.error.is_none());
        } else {
            assert_eq!(plan.status, WgcOneFrameProbeStatus::FallbackPlanned);
            assert!(matches!(
                plan.error,
                Some(WgcOneFrameProbeError::NativeApiUnavailable { .. })
            ));
        }
    }
}

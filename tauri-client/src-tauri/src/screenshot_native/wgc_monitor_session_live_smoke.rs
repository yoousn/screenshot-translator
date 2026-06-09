use crate::screenshot_diagnostics_requests::{
    NativeDxgiSelectedReadbackSmokeRequest, NativeWgcMonitorSessionSmokeRequest,
};
use crate::screenshot_wgc_diagnostic_commands::run_native_wgc_monitor_session_smoke;

const LIVE_SMOKE_ENV: &str = "YSN_WGC_MONITOR_SESSION_LIVE_SMOKE";
const STRICT_SMOKE_ENV: &str = "YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE";

fn env_enabled(name: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some("1")
}

fn live_smoke_request() -> NativeWgcMonitorSessionSmokeRequest {
    NativeWgcMonitorSessionSmokeRequest {
        bounds: Some(NativeDxgiSelectedReadbackSmokeRequest {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            explicit_opt_in: Some(true),
            allow_real_dxgi_api: Some(false),
        }),
        explicit_opt_in: Some(true),
        allow_real_wgc_api: Some(true),
        frame_timeout_ms: Some(500),
        include_cursor: Some(false),
        require_border: Some(false),
        buffer_count: Some(1),
        validate_target: Some(true),
    }
}

#[test]
#[ignore = "requires real WGC monitor session and YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1"]
fn native_wgc_monitor_session_live_smoke_env_guarded() {
    if !env_enabled(LIVE_SMOKE_ENV) {
        eprintln!("skipping WGC monitor session live smoke; set {LIVE_SMOKE_ENV}=1 to run");
        return;
    }

    let response = run_native_wgc_monitor_session_smoke(Some(live_smoke_request()))
        .expect("WGC monitor session smoke response");
    eprintln!(
        "WGC monitor session live smoke response: {}",
        serde_json::to_string_pretty(&response).expect("response json")
    );

    assert_eq!(response["diagnosticOnly"], true);
    assert_eq!(response["persistentHandleExposed"], false);
    assert_eq!(response["readinessChanged"], false);
    assert_eq!(response["attemptedRealWgcApi"], true);
    assert_eq!(response["frameCaptureAttempted"], true);
    assert_eq!(response["selectedReadbackPlan"]["diagnosticOnly"], true);
    assert_eq!(response["selectedReadbackPlan"]["readinessChanged"], false);
    assert_eq!(response["selectedReadbackPlan"]["status"], "planned");

    if response["ok"].as_bool().unwrap_or(false) {
        assert_eq!(response["frameCaptureConfirmed"], true);
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["diagnosticOnly"],
            true
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["frameMatchesTargetMonitorBounds"],
            true
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["selectedCropWithinFrame"],
            true
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["selectedPngProduced"],
            true
        );
        assert_eq!(
            response["session"]["selectedMonitorFrameEvidence"]["readbackBytesPresent"],
            true
        );
        let png_evidence = &response["session"]["selectedPngEvidence"];
        assert!(
            png_evidence.is_object(),
            "selected PNG evidence must be present"
        );
        assert_eq!(png_evidence["selectedOnlyPng"], true);
        assert_eq!(png_evidence["dimensionsMatchCrop"], true);
        assert!(png_evidence["pngWidth"].as_u64().unwrap_or(0) > 0);
        assert!(png_evidence["pngHeight"].as_u64().unwrap_or(0) > 0);
        assert!(png_evidence["pngByteLen"].as_u64().unwrap_or(0) > 8);
        assert_eq!(png_evidence["crop"]["width"], png_evidence["pngWidth"]);
        assert_eq!(png_evidence["crop"]["height"], png_evidence["pngHeight"]);
        assert_eq!(response["session"]["selectedPngProduced"], true);
        assert_eq!(response["session"]["persistentHandleExposed"], false);
        assert_eq!(response["session"]["readinessChanged"], false);
        let fake_sink = &response["selectedOutputFakeSinkAcceptance"];
        assert_eq!(fake_sink["ok"], true);
        assert_eq!(fake_sink["diagnosticOnly"], true);
        assert_eq!(fake_sink["readinessChanged"], false);
        assert_eq!(fake_sink["altAChanged"], false);
        assert_eq!(fake_sink["persistentHandleExposed"], false);
        assert_eq!(fake_sink["wgcSelectedPngEvidencePresent"], true);
        assert_eq!(fake_sink["fakeSinkCopyAccepted"], true);
        assert_eq!(fake_sink["sink"], "provided-fake-sink");
        assert_eq!(fake_sink["sinkCalls"], 1);
        assert_eq!(fake_sink["selectedOnlyPng"], true);
        assert_eq!(fake_sink["pngByteLen"], fake_sink["copiedPngByteLen"]);
        assert_eq!(fake_sink["effect"]["copyOnly"], true);
        assert_eq!(
            response["session"]["selectedOutputFakeSinkAcceptance"]["ok"],
            true
        );
    }

    if env_enabled(STRICT_SMOKE_ENV) {
        assert!(
            response["ok"].as_bool().unwrap_or(false),
            "strict WGC monitor session live smoke requires ok=true"
        );
    }
}

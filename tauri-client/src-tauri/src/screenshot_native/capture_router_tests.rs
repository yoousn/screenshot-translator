use super::*;
use crate::screenshot_native::gpu::{GpuCaptureCapability, GpuTextureInterop};
use crate::screenshot_native::gpu_device::{
    D3d11AdapterPreference, D3d11DeviceDiagnostics, D3d11FeatureLevel,
};

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
    let d3d11_probe = ready_d3d11_probe();
    let wgc_probe = WgcOneFrameProbePlan {
        status: WgcOneFrameProbeStatus::ProbeReady,
        should_attempt_probe: true,
        ..super::super::wgc_probe::default_wgc_one_frame_probe_plan()
    };
    let decision = resolve_diagnostics_capture_route(CaptureDiagnosticsRouteOptions::prefer_wgc(
        &d3d11_probe,
        &wgc_probe,
        None,
    ));

    assert_eq!(
        decision.requested,
        CaptureBackendKind::WindowsGraphicsCapture
    );
    assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
    assert_eq!(
        decision.fallback_reason.as_str(),
        "wgc-frame-acquisition-contract-only"
    );
    assert!(decision.fallback_reason.is_probe_or_bridge_reason());
}

#[test]
fn dxgi_capture_reason_can_drive_route_decision() {
    let d3d11_probe = ready_d3d11_probe();
    let mut dxgi_contract = DxgiDesktopDuplicationContract::placeholder();
    dxgi_contract.readiness = DxgiCaptureReadiness::Ready;
    let decision = resolve_diagnostics_capture_route(CaptureDiagnosticsRouteOptions::prefer_dxgi(
        &d3d11_probe,
        &dxgi_contract,
    ));

    assert_eq!(decision.requested, CaptureBackendKind::DesktopDuplication);
    assert_eq!(decision.selected, CaptureBackendKind::ExistingCpu);
    assert_eq!(
        decision.fallback_reason.as_str(),
        "dxgi-frame-acquisition-contract-only"
    );
    assert!(decision.fallback_reason.is_probe_or_bridge_reason());
}

fn ready_d3d11_probe() -> D3d11GpuProbeReport {
    D3d11GpuProbeReport {
        capability: GpuCaptureCapability::ready(
            GpuCaptureBackend::WindowsGraphicsCapture,
            GpuTextureInterop::D3d11Texture,
        ),
        diagnostics: D3d11DeviceDiagnostics {
            adapter_preference: D3d11AdapterPreference::Default,
            adapter_label: "test-adapter".to_string(),
            feature_level: D3d11FeatureLevel::V11_0,
            debug_layer_requested: false,
            used_default_adapter: true,
            fallback_reason: Some("test fixture".to_string()),
        },
    }
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

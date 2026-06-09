use super::capture::{CaptureBackendKind, MonitorCaptureBounds};
use super::dxgi_capture::{
    DxgiCaptureError, DxgiCaptureFallbackTarget, DxgiDesktopDuplicationContract,
};
use super::dxgi_output::DxgiDesktopCoordinates;
use super::dxgi_probe::DxgiNativeApiProbe;
use super::dxgi_session::{DxgiDuplicationSessionContract, DxgiDuplicationSessionState};
use super::gpu::{GpuCaptureFallback, GpuCaptureStatus};

#[test]
fn placeholder_contract_falls_back_to_cpu() {
    let contract = DxgiDesktopDuplicationContract::placeholder();

    assert_eq!(
        contract.capture_contract.backend,
        CaptureBackendKind::DesktopDuplication
    );
    assert_eq!(contract.fallback(), GpuCaptureFallback::CpuScreenshot);
    assert_eq!(contract.capability().status, GpuCaptureStatus::Unsupported);
}

#[test]
fn empty_bounds_are_not_fallback_safe() {
    let error = DxgiCaptureError::InvalidBounds(MonitorCaptureBounds::new(0, 0, 0, 100));

    assert_eq!(error.fallback(), DxgiCaptureFallbackTarget::Unavailable);
}

#[test]
fn dxgi_native_probe_retains_optional_desktop_coordinates() {
    let probe = DxgiNativeApiProbe::available_with_desktop_coordinates(
        Some(DxgiDesktopCoordinates::new(0, 0, 1920, 1080)),
        None,
    );

    assert!(probe.supports_duplication_probe());
    assert_eq!(
        probe
            .desktop_coordinates
            .and_then(DxgiDesktopCoordinates::bounds),
        Some(MonitorCaptureBounds::new(0, 0, 1920, 1080))
    );
}

#[test]
fn session_tracks_duplicate_output_readiness() {
    let mut session = DxgiDuplicationSessionContract::native_api_available();
    session.mark_duplicate_output_ready();
    assert_eq!(
        session.state,
        DxgiDuplicationSessionState::DuplicateOutputReady
    );
    session.mark_frame_acquired();
    assert!(session.requires_release_before_next_acquire);
    session.mark_frame_released();
    assert_eq!(
        session.state,
        DxgiDuplicationSessionState::DuplicateOutputReady
    );
}

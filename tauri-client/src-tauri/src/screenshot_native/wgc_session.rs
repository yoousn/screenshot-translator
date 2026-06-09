use std::time::{Duration, Instant};

use super::wgc_contract::{
    WgcNativeApiProbe, WgcOneFrameProbeDiagnostics, WgcOneFrameProbeRequest,
};
use super::wgc_probe::{default_wgc_one_frame_probe_contract, probe_wgc_native_api_support};
#[cfg(windows)]
use super::wgc_readback::build_selected_png_contract_from_wgc_texture;
pub use super::wgc_session_types::{
    WgcCaptureTarget, WgcOneFrameSessionOptions, WgcOneFrameSessionReport,
    WgcSelectedMonitorFrameEvidence, WgcSessionError, WgcSessionState,
};
use super::wgc_target::validate_wgc_capture_target_basics;

#[cfg(windows)]
use super::gpu_device::D3d11DeviceCreateOptions;
#[cfg(windows)]
use super::wgc_device::{
    create_wgc_direct3d_device, d3d11_texture_from_wgc_surface, describe_wgc_d3d11_texture_2d,
    WgcAcquiredTextureFrame, WgcDeviceBridgeError,
};

#[cfg(windows)]
use windows::core::factory;
#[cfg(windows)]
use windows::Graphics::{
    Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
    DirectX::DirectXPixelFormat,
    SizeInt32,
};
#[cfg(windows)]
use windows::Win32::{
    Foundation::HWND, Graphics::Gdi::HMONITOR,
    System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
};

pub fn default_wgc_one_frame_session_options(
    target: WgcCaptureTarget,
    width: u32,
    height: u32,
) -> WgcOneFrameSessionOptions {
    WgcOneFrameSessionOptions {
        request: WgcOneFrameProbeRequest::disabled(),
        target,
        width,
        height,
        requested_bounds: None,
        target_bounds: None,
        include_cursor: false,
        require_border: false,
        buffer_count: 1,
    }
}

pub fn guarded_wgc_one_frame_session(
    options: WgcOneFrameSessionOptions,
) -> WgcOneFrameSessionReport {
    let started_at = Instant::now();
    if let Err(error) = validate_wgc_session_request_basics(&options) {
        let diagnostics = WgcOneFrameProbeDiagnostics::from_contract(
            default_wgc_one_frame_probe_contract(),
            WgcNativeApiProbe::unavailable(
                true,
                "WGC session API support probe skipped until local request validation passes",
            ),
        );
        return WgcOneFrameSessionReport::failed(
            state_for_validation_error(&error),
            false,
            options.width,
            options.height,
            options.requested_bounds,
            options.target_bounds,
            started_at,
            diagnostics,
            error,
        );
    }

    let api_probe = probe_wgc_native_api_support();
    let diagnostics = WgcOneFrameProbeDiagnostics::from_contract(
        default_wgc_one_frame_probe_contract(),
        api_probe.clone(),
    );

    match validate_wgc_session_api_support(&api_probe) {
        Ok(()) => run_guarded_wgc_one_frame_session(options, diagnostics, started_at),
        Err(error) => WgcOneFrameSessionReport::failed(
            state_for_validation_error(&error),
            false,
            options.width,
            options.height,
            options.requested_bounds,
            options.target_bounds,
            started_at,
            diagnostics,
            error,
        ),
    }
}

#[cfg(windows)]
pub fn guarded_wgc_one_frame_texture_session(
    options: WgcOneFrameSessionOptions,
) -> Result<WgcAcquiredTextureFrame, WgcSessionError> {
    validate_wgc_session_request_basics(&options)?;
    let api_probe = probe_wgc_native_api_support();
    validate_wgc_session_api_support(&api_probe)?;
    run_windows_wgc_one_frame_session(&options)
}

fn validate_wgc_session_request_basics(
    options: &WgcOneFrameSessionOptions,
) -> Result<(), WgcSessionError> {
    if !options.request.explicit_opt_in {
        return Err(WgcSessionError::ExplicitOptInRequired);
    }
    if !options.request.allow_real_wgc_api {
        return Err(WgcSessionError::RealApiNotAllowed);
    }
    if options.request.frame_timeout_ms == 0 {
        return Err(WgcSessionError::InvalidFrameTimeoutMs {
            timeout_ms: options.request.frame_timeout_ms,
        });
    }
    if options.width == 0 || options.height == 0 {
        return Err(WgcSessionError::InvalidDimensions {
            width: options.width,
            height: options.height,
        });
    }
    if options.buffer_count < 1 {
        return Err(WgcSessionError::InvalidBufferCount {
            buffer_count: options.buffer_count,
        });
    }
    validate_wgc_capture_target_basics(options.target).map_err(|error| {
        WgcSessionError::InvalidTarget {
            reason: error.to_string(),
        }
    })?;
    Ok(())
}

fn validate_wgc_session_api_support(api_probe: &WgcNativeApiProbe) -> Result<(), WgcSessionError> {
    if !api_probe.is_supported {
        return Err(WgcSessionError::NativeApiUnavailable {
            reason: api_probe.reason.clone().unwrap_or_else(|| {
                "Windows Graphics Capture support probe returned false".to_string()
            }),
        });
    }
    Ok(())
}

fn state_for_validation_error(error: &WgcSessionError) -> WgcSessionState {
    match error {
        WgcSessionError::ExplicitOptInRequired | WgcSessionError::RealApiNotAllowed => {
            WgcSessionState::Disabled
        }
        WgcSessionError::NativeApiUnavailable { .. } => WgcSessionState::ApiUnavailable,
        WgcSessionError::InvalidFrameTimeoutMs { .. }
        | WgcSessionError::InvalidDimensions { .. }
        | WgcSessionError::InvalidBufferCount { .. }
        | WgcSessionError::InvalidTarget { .. } => WgcSessionState::InvalidRequest,
        _ => WgcSessionState::Failed,
    }
}

impl WgcOneFrameSessionReport {
    fn failed(
        state: WgcSessionState,
        attempted_real_wgc_api: bool,
        width: u32,
        height: u32,
        requested_bounds: Option<super::MonitorCaptureBounds>,
        target_bounds: Option<super::MonitorCaptureBounds>,
        started_at: Instant,
        diagnostics: WgcOneFrameProbeDiagnostics,
        error: WgcSessionError,
    ) -> Self {
        Self {
            state,
            attempted_real_wgc_api,
            created_device: false,
            created_item: false,
            created_frame_pool: false,
            created_session: false,
            started_capture: false,
            acquired_frame: false,
            frame_id: 0,
            width,
            height,
            elapsed_ms: elapsed_ms(started_at),
            diagnostics,
            selected_monitor_frame_evidence: WgcSelectedMonitorFrameEvidence::from_session(
                requested_bounds,
                target_bounds,
                None,
                None,
                false,
                false,
            ),
            frame: None,
            selected_image: None,
            error: Some(error),
        }
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}

#[cfg(not(windows))]
fn run_guarded_wgc_one_frame_session(
    options: WgcOneFrameSessionOptions,
    diagnostics: WgcOneFrameProbeDiagnostics,
    started_at: Instant,
) -> WgcOneFrameSessionReport {
    WgcOneFrameSessionReport::failed(
        WgcSessionState::ApiUnavailable,
        true,
        options.width,
        options.height,
        options.requested_bounds,
        options.target_bounds,
        started_at,
        diagnostics,
        WgcSessionError::UnsupportedPlatform {
            reason: "WGC one-frame session requires Windows".to_string(),
        },
    )
}

#[cfg(windows)]
fn run_guarded_wgc_one_frame_session(
    options: WgcOneFrameSessionOptions,
    diagnostics: WgcOneFrameProbeDiagnostics,
    started_at: Instant,
) -> WgcOneFrameSessionReport {
    match run_windows_wgc_one_frame_session(&options) {
        Ok(acquired) => {
            let frame = acquired.frame_contract().clone();
            let selected_image = acquired.selected_image().cloned();
            let selected_monitor_frame_evidence = WgcSelectedMonitorFrameEvidence::from_session(
                options.requested_bounds,
                options.target_bounds,
                Some(frame.metadata.width),
                Some(frame.metadata.height),
                frame.readback_bytes.is_some() || selected_image.is_some(),
                selected_image
                    .as_ref()
                    .map(|image| image.is_selected_only_png())
                    .unwrap_or(false),
            );
            WgcOneFrameSessionReport {
                state: WgcSessionState::FrameAcquired,
                attempted_real_wgc_api: true,
                created_device: true,
                created_item: true,
                created_frame_pool: true,
                created_session: true,
                started_capture: true,
                acquired_frame: true,
                frame_id: frame.metadata.frame_id,
                width: frame.metadata.width,
                height: frame.metadata.height,
                elapsed_ms: elapsed_ms(started_at),
                diagnostics,
                selected_monitor_frame_evidence,
                frame: Some(frame),
                selected_image,
                error: None,
            }
        }
        Err(error) => WgcOneFrameSessionReport::failed(
            runtime_state_for_error(&error),
            true,
            options.width,
            options.height,
            options.requested_bounds,
            options.target_bounds,
            started_at,
            diagnostics,
            error,
        ),
    }
}

#[cfg(windows)]
fn runtime_state_for_error(error: &WgcSessionError) -> WgcSessionState {
    match error {
        WgcSessionError::FrameTimeout { .. } => WgcSessionState::TimedOut,
        WgcSessionError::DeviceBridge { .. } => WgcSessionState::Failed,
        WgcSessionError::CaptureItem { .. } => WgcSessionState::DeviceReady,
        WgcSessionError::FramePool { .. } => WgcSessionState::CaptureItemReady,
        WgcSessionError::CaptureSession { .. } => WgcSessionState::FramePoolReady,
        WgcSessionError::StartCapture { .. } => WgcSessionState::SessionReady,
        WgcSessionError::FrameSurface { .. } | WgcSessionError::TextureContract { .. } => {
            WgcSessionState::CaptureStarted
        }
        _ => WgcSessionState::Failed,
    }
}

#[cfg(windows)]
fn run_windows_wgc_one_frame_session(
    options: &WgcOneFrameSessionOptions,
) -> Result<WgcAcquiredTextureFrame, WgcSessionError> {
    let bridge = create_wgc_direct3d_device(D3d11DeviceCreateOptions::default())?;
    let item = create_wgc_capture_item(options.target)?;
    let size = SizeInt32 {
        Width: options.width as i32,
        Height: options.height as i32,
    };
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &bridge.direct3d,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        options.buffer_count,
        size,
    )
    .map_err(|error| WgcSessionError::FramePool {
        reason: error.to_string(),
    })?;
    let session = frame_pool.CreateCaptureSession(&item).map_err(|error| {
        WgcSessionError::CaptureSession {
            reason: error.to_string(),
        }
    })?;
    let _ = session.SetIsCursorCaptureEnabled(options.include_cursor);
    let _ = session.SetIsBorderRequired(options.require_border);
    session
        .StartCapture()
        .map_err(|error| WgcSessionError::StartCapture {
            reason: error.to_string(),
        })?;
    let frame = poll_next_wgc_frame(&frame_pool, options.request.frame_timeout_ms)?;
    let surface = frame
        .Surface()
        .map_err(|error| WgcSessionError::FrameSurface {
            reason: error.to_string(),
        })?;
    let texture = d3d11_texture_from_wgc_surface(&surface).map_err(|error| {
        WgcSessionError::TextureContract {
            reason: error.to_string(),
        }
    })?;
    let mut acquired = describe_wgc_d3d11_texture_2d(texture, 1).map_err(|error| {
        WgcSessionError::TextureContract {
            reason: error.to_string(),
        }
    })?;
    let selected_image = build_selected_png_contract_from_wgc_texture(
        &bridge.d3d11.device,
        &bridge.d3d11.immediate_context,
        acquired.texture(),
        options.requested_bounds,
        options.target_bounds,
    )
    .map_err(|error| WgcSessionError::TextureContract {
        reason: error.to_string(),
    })?;
    acquired.set_selected_image(selected_image);
    let _ = session.Close();
    let _ = frame_pool.Close();
    Ok(acquired)
}

#[cfg(windows)]
fn create_wgc_capture_item(
    target: WgcCaptureTarget,
) -> Result<GraphicsCaptureItem, WgcSessionError> {
    let interop =
        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|error| {
            WgcSessionError::CaptureItem {
                reason: error.to_string(),
            }
        })?;
    match target {
        WgcCaptureTarget::Monitor { hmonitor } => unsafe {
            interop.CreateForMonitor::<_, GraphicsCaptureItem>(HMONITOR(hmonitor as *mut _))
        },
        WgcCaptureTarget::Window { hwnd } => unsafe {
            interop.CreateForWindow::<_, GraphicsCaptureItem>(HWND(hwnd as *mut _))
        },
    }
    .map_err(|error| WgcSessionError::CaptureItem {
        reason: error.to_string(),
    })
}

#[cfg(windows)]
fn poll_next_wgc_frame(
    frame_pool: &Direct3D11CaptureFramePool,
    timeout_ms: u64,
) -> Result<windows::Graphics::Capture::Direct3D11CaptureFrame, WgcSessionError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match frame_pool.TryGetNextFrame() {
            Ok(frame) => return Ok(frame),
            Err(error) => {
                let _ = error;
                if Instant::now() >= deadline {
                    return Err(WgcSessionError::FrameTimeout { timeout_ms });
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}

#[cfg(windows)]
impl From<WgcDeviceBridgeError> for WgcSessionError {
    fn from(error: WgcDeviceBridgeError) -> Self {
        Self::DeviceBridge {
            reason: error.to_string(),
        }
    }
}

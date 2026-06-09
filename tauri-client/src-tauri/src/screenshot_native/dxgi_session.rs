use super::dxgi_capture::{DxgiCaptureError, DxgiCaptureResult};
use super::dxgi_output::{
    rank_dxgi_outputs_for_selection, DxgiDesktopCoordinates, DxgiOutputCandidate,
    DxgiOutputRankingEvidence,
};
use super::dxgi_probe::DxgiNativeApiProbe;
use super::MonitorCaptureBounds;

#[cfg(windows)]
use super::gpu_device::{
    adapter_handle, create_d3d11_capture_device_for_adapter, D3d11DeviceCreateOptions,
    D3d11DeviceHandle,
};

#[cfg(windows)]
use windows::core::Interface;
#[cfg(windows)]
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
#[cfg(windows)]
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput1, IDXGIOutputDuplication,
    DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiDuplicationSessionState {
    Uninitialized,
    NativeApiAvailable,
    DuplicateOutputReady,
    FrameAcquired,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiDuplicationSessionContract {
    pub state: DxgiDuplicationSessionState,
    pub frame_id: u64,
    pub owns_acquired_frame: bool,
    pub requires_release_before_next_acquire: bool,
    pub reason: Option<String>,
}

impl DxgiDuplicationSessionContract {
    pub const fn idle(state: DxgiDuplicationSessionState, reason: Option<String>) -> Self {
        Self {
            state,
            frame_id: 0,
            owns_acquired_frame: false,
            requires_release_before_next_acquire: false,
            reason,
        }
    }

    pub fn from_probe(api_probe: &DxgiNativeApiProbe) -> Self {
        if api_probe.supports_duplication_probe() {
            Self::native_api_available()
        } else {
            Self::idle(
                DxgiDuplicationSessionState::Failed,
                Some(api_probe.reason.clone().unwrap_or_else(|| {
                    "DXGI factory, adapter, or output probe failed".to_string()
                })),
            )
        }
    }

    pub fn native_api_available() -> Self {
        Self::idle(DxgiDuplicationSessionState::NativeApiAvailable, None)
    }

    pub fn mark_duplicate_output_ready(&mut self) {
        self.state = DxgiDuplicationSessionState::DuplicateOutputReady;
        self.reason = None;
    }

    pub fn mark_frame_acquired(&mut self) {
        self.frame_id = self.frame_id.saturating_add(1);
        self.state = DxgiDuplicationSessionState::FrameAcquired;
        self.owns_acquired_frame = true;
        self.requires_release_before_next_acquire = true;
    }

    pub fn mark_frame_released(&mut self) {
        self.state = DxgiDuplicationSessionState::DuplicateOutputReady;
        self.owns_acquired_frame = false;
        self.requires_release_before_next_acquire = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiOutduplPointerPositionDiagnostics {
    pub visible: bool,
    pub position_x: i32,
    pub position_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiOutduplFrameInfoDiagnostics {
    pub last_present_time_qpc: i64,
    pub last_mouse_update_time_qpc: i64,
    pub accumulated_frames: u32,
    pub rects_coalesced: bool,
    pub protected_content_masked_out: bool,
    pub pointer_position: DxgiOutduplPointerPositionDiagnostics,
    pub total_metadata_buffer_size: u32,
    pub pointer_shape_buffer_size: u32,
}

#[cfg(windows)]
impl From<DXGI_OUTDUPL_FRAME_INFO> for DxgiOutduplFrameInfoDiagnostics {
    fn from(info: DXGI_OUTDUPL_FRAME_INFO) -> Self {
        Self {
            last_present_time_qpc: info.LastPresentTime,
            last_mouse_update_time_qpc: info.LastMouseUpdateTime,
            accumulated_frames: info.AccumulatedFrames,
            rects_coalesced: info.RectsCoalesced.as_bool(),
            protected_content_masked_out: info.ProtectedContentMaskedOut.as_bool(),
            pointer_position: DxgiOutduplPointerPositionDiagnostics {
                visible: info.PointerPosition.Visible.as_bool(),
                position_x: info.PointerPosition.Position.x,
                position_y: info.PointerPosition.Position.y,
            },
            total_metadata_buffer_size: info.TotalMetadataBufferSize,
            pointer_shape_buffer_size: info.PointerShapeBufferSize,
        }
    }
}

#[cfg(windows)]
pub(crate) struct DxgiAcquiredTextureWithInfo {
    pub(crate) texture: ID3D11Texture2D,
    pub(crate) frame_info: DxgiOutduplFrameInfoDiagnostics,
}

#[cfg(windows)]
#[derive(Debug, Clone)]
pub(crate) struct DxgiDuplicationSession {
    pub(crate) device: D3d11DeviceHandle,
    duplication: IDXGIOutputDuplication,
    output_bounds: Option<MonitorCaptureBounds>,
    adapter_index: u32,
    output_index: u32,
    output_ranking: Option<DxgiOutputRankingEvidence>,
}

#[cfg(windows)]
impl DxgiDuplicationSession {
    pub(crate) fn open() -> DxgiCaptureResult<Self> {
        Self::open_for_bounds(None)
    }

    pub(crate) fn open_for_selection(selection: MonitorCaptureBounds) -> DxgiCaptureResult<Self> {
        Self::open_for_bounds(Some(selection))
    }

    fn open_for_bounds(selection: Option<MonitorCaptureBounds>) -> DxgiCaptureResult<Self> {
        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }.map_err(|error| {
            DxgiCaptureError::adapter_unavailable(format!("CreateDXGIFactory1 failed: {error}"))
        })?;
        let selected_output = select_dxgi_output(&factory, selection)?;
        let output = selected_output.output;
        let output_bounds = unsafe { output.GetDesc() }.ok().and_then(|desc| {
            DxgiDesktopCoordinates::new(
                desc.DesktopCoordinates.left,
                desc.DesktopCoordinates.top,
                desc.DesktopCoordinates.right,
                desc.DesktopCoordinates.bottom,
            )
            .bounds()
        });
        let output1: IDXGIOutput1 = output.cast().map_err(|error| {
            DxgiCaptureError::adapter_unavailable(format!("IDXGIOutput1 cast failed: {error}"))
        })?;
        let device_adapter = adapter_handle(selected_output.adapter)
            .map_err(|error| DxgiCaptureError::adapter_unavailable(error.to_string()))?;
        let device = create_d3d11_capture_device_for_adapter(
            device_adapter,
            D3d11DeviceCreateOptions::default(),
        )
        .map_err(|error| DxgiCaptureError::adapter_unavailable(error.to_string()))?;
        let duplication = unsafe { output1.DuplicateOutput(&device.device) }.map_err(|error| {
            DxgiCaptureError::adapter_unavailable(format!(
                "IDXGIOutput1::DuplicateOutput failed: {error}"
            ))
        })?;
        Ok(Self {
            device,
            duplication,
            output_bounds,
            adapter_index: selected_output.adapter_index,
            output_index: selected_output.output_index,
            output_ranking: selected_output.output_ranking,
        })
    }

    pub(crate) fn output_bounds(&self) -> Option<MonitorCaptureBounds> {
        self.output_bounds
    }

    pub(crate) fn output_identity(&self) -> (u32, u32) {
        (self.adapter_index, self.output_index)
    }

    pub(crate) fn output_ranking(&self) -> Option<&DxgiOutputRankingEvidence> {
        self.output_ranking.as_ref()
    }

    pub(crate) fn acquire_next_frame(&self, timeout_ms: u32) -> DxgiCaptureResult<ID3D11Texture2D> {
        self.acquire_next_frame_with_info(timeout_ms)
            .map(|frame| frame.texture)
    }

    pub(crate) fn acquire_next_frame_with_info(
        &self,
        timeout_ms: u32,
    ) -> DxgiCaptureResult<DxgiAcquiredTextureWithInfo> {
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource = None;
        unsafe {
            self.duplication
                .AcquireNextFrame(timeout_ms, &mut info, &mut resource)
        }
        .map_err(map_acquire_frame_error)?;
        let frame_info = DxgiOutduplFrameInfoDiagnostics::from(info);
        let result = resource
            .ok_or_else(|| {
                DxgiCaptureError::frame_unavailable("DXGI AcquireNextFrame returned no resource")
            })
            .and_then(|resource| {
                resource.cast().map_err(|error| {
                    DxgiCaptureError::frame_unavailable(format!(
                        "acquired DXGI resource is not ID3D11Texture2D: {error}"
                    ))
                })
            })
            .map(|texture| DxgiAcquiredTextureWithInfo {
                texture,
                frame_info,
            });
        if result.is_err() {
            let _ = self.release_frame();
        }
        result
    }

    pub(crate) fn release_frame(&self) -> DxgiCaptureResult<()> {
        unsafe { self.duplication.ReleaseFrame() }.map_err(|error| {
            DxgiCaptureError::frame_unavailable(format!(
                "IDXGIOutputDuplication::ReleaseFrame failed: {error}"
            ))
        })
    }
}

#[cfg(windows)]
struct RankedDxgiOutput {
    adapter_index: u32,
    output_index: u32,
    output_ranking: Option<DxgiOutputRankingEvidence>,
    adapter: windows::Win32::Graphics::Dxgi::IDXGIAdapter1,
    output: windows::Win32::Graphics::Dxgi::IDXGIOutput,
}

#[cfg(windows)]
fn select_dxgi_output(
    factory: &IDXGIFactory1,
    selection: Option<MonitorCaptureBounds>,
) -> DxgiCaptureResult<RankedDxgiOutput> {
    let mut ranked_outputs = Vec::new();
    let mut adapter_index = 0;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(adapter_index) } {
        let mut output_index = 0;
        while let Ok(output) = unsafe { adapter.EnumOutputs(output_index) } {
            let desktop_bounds = unsafe { output.GetDesc() }.ok().and_then(|desc| {
                DxgiDesktopCoordinates::new(
                    desc.DesktopCoordinates.left,
                    desc.DesktopCoordinates.top,
                    desc.DesktopCoordinates.right,
                    desc.DesktopCoordinates.bottom,
                )
                .bounds()
            });
            if let Some(desktop_bounds) = desktop_bounds {
                ranked_outputs.push((
                    DxgiOutputCandidate::new(adapter_index, output_index, desktop_bounds),
                    adapter.clone(),
                    output,
                ));
            }
            output_index += 1;
        }
        adapter_index += 1;
    }

    if ranked_outputs.is_empty() {
        return Err(DxgiCaptureError::adapter_unavailable(
            "DXGI output enumeration returned no outputs with valid DesktopCoordinates",
        ));
    }

    let output_ranking = if let Some(selection) = selection {
        let candidates = ranked_outputs
            .iter()
            .map(|(candidate, _, _)| *candidate)
            .collect::<Vec<_>>();
        Some(rank_dxgi_outputs_for_selection(selection, &candidates))
    } else {
        None
    };

    let selected = if let Some(output_ranking) = &output_ranking {
        output_ranking.selected_output.ok_or_else(|| {
            DxgiCaptureError::adapter_unavailable(
                "requested selection does not intersect any DXGI output DesktopCoordinates",
            )
        })?
    } else {
        ranked_outputs[0].0
    };

    ranked_outputs
        .into_iter()
        .find(|(candidate, _, _)| *candidate == selected)
        .map(|(candidate, adapter, output)| RankedDxgiOutput {
            adapter_index: candidate.adapter_index,
            output_index: candidate.output_index,
            output_ranking,
            adapter,
            output,
        })
        .ok_or_else(|| DxgiCaptureError::adapter_unavailable("ranked DXGI output disappeared"))
}

#[cfg(windows)]
fn map_acquire_frame_error(error: windows::core::Error) -> DxgiCaptureError {
    let code = error.code();
    if code == DXGI_ERROR_WAIT_TIMEOUT {
        DxgiCaptureError::frame_timeout(format!("DXGI AcquireNextFrame was not ready: {error}"))
    } else if code == DXGI_ERROR_ACCESS_LOST {
        DxgiCaptureError::access_lost(format!(
            "DXGI AcquireNextFrame access was lost and duplication must be recreated: {error}"
        ))
    } else {
        DxgiCaptureError::frame_unavailable(format!(
            "IDXGIOutputDuplication::AcquireNextFrame failed: {error}"
        ))
    }
}

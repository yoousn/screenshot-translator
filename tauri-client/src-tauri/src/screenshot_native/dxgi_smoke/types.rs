use super::super::capture::MonitorCaptureBounds;
use super::super::d3d11_frame::D3d11TextureFrameFormat;
use super::super::dxgi_session::DxgiDuplicationSessionState;
use super::super::output::CropRect;
use super::super::selected_readback_plan::SelectedReadbackPlan;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiTextureSmokeStage {
    NotStarted,
    Started,
    TextureAcquired,
    FrameReleased,
    Stopped,
    Failed,
}

impl DxgiTextureSmokeStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::Started => "started",
            Self::TextureAcquired => "texture-acquired",
            Self::FrameReleased => "frame-released",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiTextureSmokeReport {
    pub attempted: bool,
    pub ok: bool,
    pub stage: DxgiTextureSmokeStage,
    pub elapsed_ms: u128,
    pub frame_id: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<D3d11TextureFrameFormat>,
    pub session_state: DxgiDuplicationSessionState,
    pub released_frame: bool,
    pub stopped: bool,
    pub error: Option<String>,
}

impl DxgiTextureSmokeReport {
    pub(super) fn failed(
        stage: DxgiTextureSmokeStage,
        elapsed_ms: u128,
        session_state: DxgiDuplicationSessionState,
        error: impl ToString,
    ) -> Self {
        Self {
            attempted: true,
            ok: false,
            stage: DxgiTextureSmokeStage::Failed,
            elapsed_ms,
            frame_id: None,
            width: None,
            height: None,
            format: None,
            session_state,
            released_frame: false,
            stopped: false,
            error: Some(format!("{}: {}", stage.as_str(), error.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiSelectedReadbackSmokeStage {
    NotStarted,
    Started,
    TextureAcquired,
    SelectedReadback,
    FrameReleased,
    Stopped,
    Failed,
}

impl DxgiSelectedReadbackSmokeStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::Started => "started",
            Self::TextureAcquired => "texture-acquired",
            Self::SelectedReadback => "selected-readback",
            Self::FrameReleased => "frame-released",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiSelectedReadbackSmokeReport {
    pub attempted: bool,
    pub ok: bool,
    pub stage: DxgiSelectedReadbackSmokeStage,
    pub elapsed_ms: u128,
    pub frame_id: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub requested_bounds: MonitorCaptureBounds,
    pub output_bounds: Option<MonitorCaptureBounds>,
    pub adapter_index: Option<u32>,
    pub output_index: Option<u32>,
    pub crop: Option<CropRect>,
    pub selected_readback_plan: Option<SelectedReadbackPlan>,
    pub selected_output_ready_planning_only: bool,
    pub format: Option<D3d11TextureFrameFormat>,
    pub selected_only: bool,
    pub bounded_crop_valid: bool,
    pub copy_subresource_region: bool,
    pub bgra_to_rgba: bool,
    pub png_signature_valid: bool,
    pub released_frame: bool,
    pub stopped: bool,
    pub error: Option<String>,
}

impl DxgiSelectedReadbackSmokeReport {
    pub(super) fn failed(
        stage: DxgiSelectedReadbackSmokeStage,
        elapsed_ms: u128,
        error: impl ToString,
    ) -> Self {
        Self {
            attempted: true,
            ok: false,
            stage: DxgiSelectedReadbackSmokeStage::Failed,
            elapsed_ms,
            frame_id: None,
            width: None,
            height: None,
            requested_bounds: MonitorCaptureBounds::new(0, 0, 0, 0),
            output_bounds: None,
            adapter_index: None,
            output_index: None,
            crop: None,
            selected_readback_plan: None,
            selected_output_ready_planning_only: false,
            format: None,
            selected_only: false,
            bounded_crop_valid: false,
            copy_subresource_region: false,
            bgra_to_rgba: false,
            png_signature_valid: false,
            released_frame: false,
            stopped: false,
            error: Some(format!("{}: {}", stage.as_str(), error.to_string())),
        }
    }
}

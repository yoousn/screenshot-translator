use std::fmt;

#[cfg(windows)]
use super::d3d11_frame::{D3d11TextureFrame, D3d11TextureFrameFormat};
use super::{
    CaptureBackendContract, CaptureBackendKind, GpuCapabilityRequirement, GpuCaptureBackend,
    GpuCaptureCapability, GpuCaptureFallback, GpuCaptureStatus, GpuTextureInterop,
    MonitorCaptureBounds, RgbaFrame,
};

use super::dxgi_probe::{probe_dxgi_native_api_support, DxgiNativeApiProbe};
#[cfg(windows)]
use super::dxgi_readback::readback_dxgi_d3d11_texture_2d;
#[cfg(windows)]
use super::dxgi_session::DxgiDuplicationSession;
use super::dxgi_session::{DxgiDuplicationSessionContract, DxgiDuplicationSessionState};
#[cfg(windows)]
use super::dxgi_texture::{describe_dxgi_d3d11_texture_2d, DxgiAcquiredTextureFrame};

pub const DXGI_PLACEHOLDER_REASON: &str =
    "DXGI Desktop Duplication backend is a placeholder; native API wiring is intentionally pending.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiCaptureFallbackTarget {
    ExistingCpuScreenshot,
    Unavailable,
}

impl DxgiCaptureFallbackTarget {
    pub const fn as_gpu_fallback(self) -> GpuCaptureFallback {
        match self {
            Self::ExistingCpuScreenshot => GpuCaptureFallback::CpuScreenshot,
            Self::Unavailable => GpuCaptureFallback::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxgiCaptureReadiness {
    PlaceholderOnly,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiDesktopDuplicationContract {
    pub capture_contract: CaptureBackendContract,
    pub backend: GpuCaptureBackend,
    pub texture_interop: GpuTextureInterop,
    pub required_capabilities: Vec<GpuCapabilityRequirement>,
    pub readiness: DxgiCaptureReadiness,
    pub fallback_target: DxgiCaptureFallbackTarget,
    pub session: DxgiDuplicationSessionContract,
    pub reason: String,
}

impl DxgiDesktopDuplicationContract {
    pub fn placeholder() -> Self {
        Self::from_native_probe(probe_dxgi_native_api_support())
    }

    pub fn from_native_probe(api_probe: DxgiNativeApiProbe) -> Self {
        let mut required_capabilities = vec![
            GpuCapabilityRequirement::D3d11Device,
            GpuCapabilityRequirement::D3d11SharedTexture,
            GpuCapabilityRequirement::CompatibleAdapter,
        ];
        if !api_probe.supports_duplication_probe() {
            required_capabilities.insert(0, GpuCapabilityRequirement::DxgiOutputDuplicationApi);
        }

        let session = DxgiDuplicationSessionContract::from_probe(&api_probe);
        Self {
            capture_contract: CaptureBackendContract::dxgi(),
            backend: GpuCaptureBackend::DxgiDesktopDuplication,
            texture_interop: GpuTextureInterop::D3d11Texture,
            required_capabilities,
            readiness: DxgiCaptureReadiness::Blocked,
            fallback_target: DxgiCaptureFallbackTarget::ExistingCpuScreenshot,
            session,
            reason: api_probe.reason.unwrap_or_else(|| {
                "DXGI factory/adapter/output are available; DuplicateOutput, D3D11 device, frame acquire, and readback wiring are pending.".to_string()
            }),
        }
    }

    pub fn capability(&self) -> GpuCaptureCapability {
        match self.readiness {
            DxgiCaptureReadiness::Ready => {
                GpuCaptureCapability::ready(self.backend, self.texture_interop)
            }
            DxgiCaptureReadiness::PlaceholderOnly | DxgiCaptureReadiness::Blocked => {
                GpuCaptureCapability {
                    backend: self.backend,
                    texture_interop: self.texture_interop,
                    status: GpuCaptureStatus::Unsupported,
                    missing_requirements: self.required_capabilities.clone(),
                    fallback: self.fallback_target.as_gpu_fallback(),
                    reason: Some(self.reason.clone()),
                }
            }
        }
    }

    pub fn fallback(&self) -> GpuCaptureFallback {
        self.fallback_target.as_gpu_fallback()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DxgiCaptureError {
    PlaceholderOnly,
    Unsupported { reason: String },
    InvalidBounds(MonitorCaptureBounds),
    AdapterUnavailable { reason: String },
    FrameUnavailable { reason: String },
    FrameTimeout { reason: String },
    AccessLost { reason: String },
    ProtectedContent,
}

impl DxgiCaptureError {
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported {
            reason: reason.into(),
        }
    }

    pub fn adapter_unavailable(reason: impl Into<String>) -> Self {
        Self::AdapterUnavailable {
            reason: reason.into(),
        }
    }

    pub fn frame_unavailable(reason: impl Into<String>) -> Self {
        Self::FrameUnavailable {
            reason: reason.into(),
        }
    }

    pub fn frame_timeout(reason: impl Into<String>) -> Self {
        Self::FrameTimeout {
            reason: reason.into(),
        }
    }

    pub fn access_lost(reason: impl Into<String>) -> Self {
        Self::AccessLost {
            reason: reason.into(),
        }
    }

    pub fn fallback(&self) -> DxgiCaptureFallbackTarget {
        match self {
            Self::InvalidBounds(_) => DxgiCaptureFallbackTarget::Unavailable,
            Self::ProtectedContent => DxgiCaptureFallbackTarget::Unavailable,
            Self::PlaceholderOnly
            | Self::Unsupported { .. }
            | Self::AdapterUnavailable { .. }
            | Self::AccessLost { .. }
            | Self::FrameTimeout { .. }
            | Self::FrameUnavailable { .. } => DxgiCaptureFallbackTarget::ExistingCpuScreenshot,
        }
    }
}

impl fmt::Display for DxgiCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaceholderOnly => formatter.write_str(DXGI_PLACEHOLDER_REASON),
            Self::Unsupported { reason } => {
                write!(formatter, "DXGI capture is unsupported: {reason}")
            }
            Self::InvalidBounds(bounds) => write!(
                formatter,
                "invalid DXGI capture bounds: x={}, y={}, width={}, height={}",
                bounds.origin_x, bounds.origin_y, bounds.width, bounds.height
            ),
            Self::AdapterUnavailable { reason } => {
                write!(formatter, "DXGI adapter is unavailable: {reason}")
            }
            Self::FrameUnavailable { reason } => {
                write!(formatter, "DXGI frame is unavailable: {reason}")
            }
            Self::FrameTimeout { reason } => {
                write!(formatter, "DXGI frame timed out: {reason}")
            }
            Self::AccessLost { reason } => {
                write!(formatter, "DXGI duplication access was lost: {reason}")
            }
            Self::ProtectedContent => {
                formatter.write_str("DXGI capture returned protected content")
            }
        }
    }
}

impl std::error::Error for DxgiCaptureError {}

pub type DxgiCaptureResult<T> = Result<T, DxgiCaptureError>;

#[cfg(windows)]
const DXGI_FIRST_FRAME_TIMEOUT_BUDGET_MS: u32 = 500;
#[cfg(windows)]
const DXGI_FIRST_FRAME_ATTEMPT_TIMEOUT_MS: u32 = 50;

#[derive(Debug, Clone)]
pub struct DxgiDesktopDuplicationBackend {
    session: DxgiDuplicationSessionContract,
    #[cfg(windows)]
    native_session: Option<DxgiDuplicationSession>,
}

impl DxgiDesktopDuplicationBackend {
    pub fn new() -> Self {
        Self {
            session: DxgiDuplicationSessionContract {
                state: DxgiDuplicationSessionState::Uninitialized,
                frame_id: 0,
                owns_acquired_frame: false,
                requires_release_before_next_acquire: false,
                reason: None,
            },
            #[cfg(windows)]
            native_session: None,
        }
    }

    pub fn session_contract(&self) -> &DxgiDuplicationSessionContract {
        &self.session
    }

    pub fn output_bounds(&self) -> Option<MonitorCaptureBounds> {
        #[cfg(windows)]
        {
            return self
                .native_session
                .as_ref()
                .and_then(|session| session.output_bounds());
        }
        #[cfg(not(windows))]
        None
    }

    pub fn output_identity(&self) -> Option<(u32, u32)> {
        #[cfg(windows)]
        {
            return self
                .native_session
                .as_ref()
                .map(|session| session.output_identity());
        }
        #[cfg(not(windows))]
        None
    }

    pub fn output_ranking(
        &self,
    ) -> Option<&crate::screenshot_native::dxgi_output::DxgiOutputRankingEvidence> {
        #[cfg(windows)]
        {
            return self
                .native_session
                .as_ref()
                .and_then(|session| session.output_ranking());
        }
        #[cfg(not(windows))]
        None
    }

    #[cfg(windows)]
    pub(crate) fn native_session_for_diagnostics(
        &self,
    ) -> Option<&crate::screenshot_native::dxgi_session::DxgiDuplicationSession> {
        self.native_session.as_ref()
    }

    pub fn contract(&self) -> DxgiDesktopDuplicationContract {
        DxgiDesktopDuplicationContract::placeholder()
    }

    pub fn capture_backend_kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::DesktopDuplication
    }

    pub fn capability(&self) -> GpuCaptureCapability {
        self.contract().capability()
    }

    pub fn start(&mut self) -> DxgiCaptureResult<()> {
        self.start_for_selection(None)
    }

    pub fn start_for_bounds(&mut self, selection: MonitorCaptureBounds) -> DxgiCaptureResult<()> {
        self.start_for_selection(Some(selection))
    }

    fn start_for_selection(
        &mut self,
        selection: Option<MonitorCaptureBounds>,
    ) -> DxgiCaptureResult<()> {
        let api_probe = probe_dxgi_native_api_support();
        if !api_probe.supports_duplication_probe() {
            self.session = DxgiDuplicationSessionContract::from_probe(&api_probe);
            return Err(DxgiCaptureError::adapter_unavailable(
                api_probe
                    .reason
                    .unwrap_or_else(|| "DXGI factory, adapter, or output probe failed".to_string()),
            ));
        }

        self.session = DxgiDuplicationSessionContract::from_probe(&api_probe);
        #[cfg(windows)]
        {
            let native_session = if let Some(selection) = selection {
                DxgiDuplicationSession::open_for_selection(selection)?
            } else {
                DxgiDuplicationSession::open()?
            };
            self.native_session = Some(native_session);
            self.session.mark_duplicate_output_ready();
            return Ok(());
        }
        #[cfg(not(windows))]
        Err(DxgiCaptureError::unsupported(
            "DXGI DuplicateOutput requires Windows",
        ))
    }

    pub fn stop(&mut self) -> DxgiCaptureResult<()> {
        if self.session.owns_acquired_frame {
            self.release_acquired_frame()?;
        }
        self.session.state = DxgiDuplicationSessionState::Stopped;
        self.session.owns_acquired_frame = false;
        self.session.requires_release_before_next_acquire = false;
        #[cfg(windows)]
        {
            self.native_session = None;
        }
        Ok(())
    }

    pub fn release_acquired_frame(&mut self) -> DxgiCaptureResult<()> {
        if self.session.owns_acquired_frame {
            let mut release_error = None;
            #[cfg(windows)]
            if let Some(native_session) = &self.native_session {
                release_error = native_session.release_frame().err();
            }
            self.session.mark_frame_released();
            if let Some(error) = release_error {
                return Err(error);
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn acquire_texture_with_first_frame_retry(
        native_session: &DxgiDuplicationSession,
    ) -> DxgiCaptureResult<windows::Win32::Graphics::Direct3D11::ID3D11Texture2D> {
        let mut elapsed_budget = 0u32;
        let mut attempts = 0u32;
        let mut last_timeout = None;
        while elapsed_budget < DXGI_FIRST_FRAME_TIMEOUT_BUDGET_MS {
            attempts = attempts.saturating_add(1);
            let remaining = DXGI_FIRST_FRAME_TIMEOUT_BUDGET_MS.saturating_sub(elapsed_budget);
            let timeout = remaining.min(DXGI_FIRST_FRAME_ATTEMPT_TIMEOUT_MS).max(1);
            match native_session.acquire_next_frame(timeout) {
                Ok(texture) => return Ok(texture),
                Err(DxgiCaptureError::FrameTimeout { reason }) => {
                    elapsed_budget = elapsed_budget.saturating_add(timeout);
                    last_timeout = Some(reason);
                }
                Err(error) => return Err(error),
            }
        }
        Err(DxgiCaptureError::frame_timeout(format!(
            "DXGI first-frame warmup exhausted {DXGI_FIRST_FRAME_TIMEOUT_BUDGET_MS} ms after {attempts} attempts: {}",
            last_timeout.unwrap_or_else(|| "no frame became available".to_string())
        )))
    }

    #[cfg(windows)]
    pub fn capture_texture_frame(
        &mut self,
        bounds: MonitorCaptureBounds,
    ) -> DxgiCaptureResult<DxgiAcquiredTextureFrame> {
        if bounds.is_empty() {
            return Err(DxgiCaptureError::InvalidBounds(bounds));
        }
        if self.session.requires_release_before_next_acquire {
            self.release_acquired_frame()?;
        }
        let native_session = self.native_session.as_ref().ok_or_else(|| {
            DxgiCaptureError::frame_unavailable("DXGI DuplicateOutput session has not been started")
        })?;
        let texture = Self::acquire_texture_with_first_frame_retry(native_session)?;
        self.session.mark_frame_acquired();
        let result = describe_dxgi_d3d11_texture_2d(texture, self.session.frame_id)
            .map_err(|error| DxgiCaptureError::frame_unavailable(error.to_string()));
        if result.is_err() {
            let _ = self.release_acquired_frame();
        }
        result
    }

    #[cfg(not(windows))]
    pub fn capture_texture_frame(
        &mut self,
        bounds: MonitorCaptureBounds,
    ) -> DxgiCaptureResult<D3d11TextureFrame> {
        if bounds.is_empty() {
            return Err(DxgiCaptureError::InvalidBounds(bounds));
        }
        Err(DxgiCaptureError::PlaceholderOnly)
    }

    pub fn capture_frame(&mut self, bounds: MonitorCaptureBounds) -> DxgiCaptureResult<RgbaFrame> {
        if bounds.is_empty() {
            return Err(DxgiCaptureError::InvalidBounds(bounds));
        }
        #[cfg(windows)]
        {
            if self.session.requires_release_before_next_acquire {
                self.release_acquired_frame()?;
            }
            let native_session = self.native_session.as_ref().ok_or_else(|| {
                DxgiCaptureError::frame_unavailable(
                    "DXGI DuplicateOutput session has not been started",
                )
            })?;
            let texture = Self::acquire_texture_with_first_frame_retry(native_session)?;
            self.session.mark_frame_acquired();
            let result = readback_dxgi_d3d11_texture_2d(
                &native_session.device.device,
                &native_session.device.immediate_context,
                &texture,
                self.session.frame_id,
            )
            .map_err(|error| DxgiCaptureError::frame_unavailable(error.to_string()))
            .and_then(|frame| {
                texture_frame_to_rgba(frame, bounds)
                    .map_err(|error| DxgiCaptureError::frame_unavailable(error.to_string()))
            });
            let release_result = self.release_acquired_frame();
            return match (result, release_result) {
                (Ok(frame), Ok(())) => Ok(frame),
                (Err(error), Ok(())) => Err(error),
                (Ok(_), Err(release_error)) => Err(release_error),
                (Err(error), Err(release_error)) => Err(DxgiCaptureError::frame_unavailable(
                    format!("{error}; additionally failed to release DXGI frame: {release_error}"),
                )),
            };
        }
        #[cfg(not(windows))]
        Err(DxgiCaptureError::PlaceholderOnly)
    }
}

#[cfg(windows)]
fn texture_frame_to_rgba(
    frame: D3d11TextureFrame,
    bounds: MonitorCaptureBounds,
) -> Result<RgbaFrame, String> {
    let width = frame.metadata.width;
    let height = frame.metadata.height;
    if width != bounds.width || height != bounds.height {
        return Err(format!(
            "DXGI readback size {}x{} does not match requested bounds {}x{}",
            width, height, bounds.width, bounds.height
        ));
    }
    let mut bytes = frame
        .compact_readback_bytes()
        .map_err(|error| error.to_string())?;
    if matches!(frame.metadata.format, D3d11TextureFrameFormat::Bgra8Unorm) {
        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    Ok(RgbaFrame {
        bytes,
        width,
        height,
    })
}

impl Default for DxgiDesktopDuplicationBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DxgiDesktopDuplicationBackend {
    fn drop(&mut self) {
        if self.session.owns_acquired_frame {
            let _ = self.release_acquired_frame();
        }
    }
}

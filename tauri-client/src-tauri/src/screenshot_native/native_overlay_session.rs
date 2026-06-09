use std::sync::{Mutex, OnceLock};

use super::overlay_renderer::render_rgba_frame_to_overlay;
use super::{
    create_win32_overlay, destroy_win32_overlay, show_win32_overlay, MonitorCaptureBounds,
    OverlayRenderReceipt, OverlayRenderTarget, RgbaFrame, Win32OverlayConfig, Win32OverlayWindow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySessionRuntime {
    CpuRgbaWin32,
}

impl NativeOverlaySessionRuntime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuRgbaWin32 => "cpu-rgba-win32",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOverlaySessionState {
    Empty,
    Created,
    Rendered,
    Visible,
    Cancelled,
    Failed,
}

impl NativeOverlaySessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Created => "created",
            Self::Rendered => "rendered",
            Self::Visible => "visible",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOverlaySessionError {
    InvalidBounds,
    OverlayCreate(String),
    OverlayRender(String),
    OverlayShow(String),
    StorePoisoned(String),
}

impl std::fmt::Display for NativeOverlaySessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBounds => formatter.write_str("native overlay session bounds are empty"),
            Self::OverlayCreate(error) => {
                write!(formatter, "native overlay create failed: {error}")
            }
            Self::OverlayRender(error) => {
                write!(formatter, "native overlay render failed: {error}")
            }
            Self::OverlayShow(error) => write!(formatter, "native overlay show failed: {error}"),
            Self::StorePoisoned(error) => {
                write!(formatter, "native overlay store poisoned: {error}")
            }
        }
    }
}

impl std::error::Error for NativeOverlaySessionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOverlaySessionDiagnostics {
    pub active: bool,
    pub session_id: Option<String>,
    pub generation: Option<u64>,
    pub runtime: Option<NativeOverlaySessionRuntime>,
    pub state: NativeOverlaySessionState,
    pub hwnd: Option<isize>,
    pub bounds: Option<MonitorCaptureBounds>,
    pub rendered: bool,
    pub visible: bool,
    pub fallback_reason: Option<String>,
}

impl NativeOverlaySessionDiagnostics {
    pub const fn empty() -> Self {
        Self {
            active: false,
            session_id: None,
            generation: None,
            runtime: None,
            state: NativeOverlaySessionState::Empty,
            hwnd: None,
            bounds: None,
            rendered: false,
            visible: false,
            fallback_reason: None,
        }
    }
}

#[derive(Debug)]
pub struct CpuNativeOverlaySession {
    session_id: String,
    generation: u64,
    bounds: MonitorCaptureBounds,
    window: Win32OverlayWindow,
    render_receipt: Option<OverlayRenderReceipt>,
    state: NativeOverlaySessionState,
    fallback_reason: Option<String>,
}

impl CpuNativeOverlaySession {
    fn diagnostics(&self) -> NativeOverlaySessionDiagnostics {
        NativeOverlaySessionDiagnostics {
            active: matches!(
                self.state,
                NativeOverlaySessionState::Created
                    | NativeOverlaySessionState::Rendered
                    | NativeOverlaySessionState::Visible
            ),
            session_id: Some(self.session_id.clone()),
            generation: Some(self.generation),
            runtime: Some(NativeOverlaySessionRuntime::CpuRgbaWin32),
            state: self.state,
            hwnd: Some(self.window.handle().hwnd()),
            bounds: Some(self.bounds),
            rendered: self.render_receipt.is_some(),
            visible: matches!(self.state, NativeOverlaySessionState::Visible),
            fallback_reason: self.fallback_reason.clone(),
        }
    }
}

static CPU_NATIVE_OVERLAY_SESSION: OnceLock<Mutex<Option<CpuNativeOverlaySession>>> =
    OnceLock::new();

fn session_store() -> &'static Mutex<Option<CpuNativeOverlaySession>> {
    CPU_NATIVE_OVERLAY_SESSION.get_or_init(|| Mutex::new(None))
}

pub fn begin_cpu_native_overlay_session(
    session_id: String,
    generation: u64,
    bounds: MonitorCaptureBounds,
    frame: &RgbaFrame,
) -> Result<NativeOverlaySessionDiagnostics, NativeOverlaySessionError> {
    if bounds.is_empty() {
        return Err(NativeOverlaySessionError::InvalidBounds);
    }

    let width =
        i32::try_from(bounds.width).map_err(|_| NativeOverlaySessionError::InvalidBounds)?;
    let height =
        i32::try_from(bounds.height).map_err(|_| NativeOverlaySessionError::InvalidBounds)?;
    let config = Win32OverlayConfig {
        x: bounds.origin_x,
        y: bounds.origin_y,
        width,
        height,
        ..Win32OverlayConfig::fullscreen_like(width, height)
    };
    let mut window = create_win32_overlay(&config)
        .map_err(|error| NativeOverlaySessionError::OverlayCreate(error.to_string()))?;
    let hwnd = window.handle().hwnd();
    let render_target = OverlayRenderTarget::hwnd(hwnd, bounds.width, bounds.height);
    let render_receipt = render_rgba_frame_to_overlay(render_target, frame)
        .map_err(|error| NativeOverlaySessionError::OverlayRender(error.to_string()))?;
    show_win32_overlay(&mut window, &config)
        .map_err(|error| NativeOverlaySessionError::OverlayShow(error.to_string()))?;

    let session = CpuNativeOverlaySession {
        session_id,
        generation,
        bounds,
        window,
        render_receipt: Some(render_receipt),
        state: NativeOverlaySessionState::Visible,
        fallback_reason: None,
    };
    let diagnostics = session.diagnostics();
    let mut guard = session_store()
        .lock()
        .map_err(|error| NativeOverlaySessionError::StorePoisoned(error.to_string()))?;
    *guard = Some(session);
    Ok(diagnostics)
}

pub fn cancel_cpu_native_overlay_session(
    reason: impl Into<String>,
) -> NativeOverlaySessionDiagnostics {
    let reason = reason.into();
    let Ok(mut guard) = session_store().lock() else {
        return NativeOverlaySessionDiagnostics {
            state: NativeOverlaySessionState::Failed,
            fallback_reason: Some("native overlay session store lock failed".to_string()),
            ..NativeOverlaySessionDiagnostics::empty()
        };
    };
    let Some(mut session) = guard.take() else {
        return NativeOverlaySessionDiagnostics::empty();
    };
    let destroy_error = destroy_win32_overlay(&mut session.window).err();
    session.state = if destroy_error.is_some() {
        NativeOverlaySessionState::Failed
    } else {
        NativeOverlaySessionState::Cancelled
    };
    session.fallback_reason = destroy_error
        .map(|error| format!("{}; destroy failed: {error}", reason))
        .or(Some(reason));
    session.diagnostics()
}

pub fn cpu_native_overlay_session_diagnostics() -> NativeOverlaySessionDiagnostics {
    let Ok(guard) = session_store().lock() else {
        return NativeOverlaySessionDiagnostics {
            state: NativeOverlaySessionState::Failed,
            fallback_reason: Some("native overlay session store lock failed".to_string()),
            ..NativeOverlaySessionDiagnostics::empty()
        };
    };
    guard
        .as_ref()
        .map(CpuNativeOverlaySession::diagnostics)
        .unwrap_or_else(NativeOverlaySessionDiagnostics::empty)
}

pub fn cleanup_stale_cpu_native_overlay_session(
    generation: u64,
) -> NativeOverlaySessionDiagnostics {
    let Ok(guard) = session_store().lock() else {
        return NativeOverlaySessionDiagnostics {
            state: NativeOverlaySessionState::Failed,
            fallback_reason: Some("native overlay session store lock failed".to_string()),
            ..NativeOverlaySessionDiagnostics::empty()
        };
    };
    let should_cancel = guard
        .as_ref()
        .map(|session| session.generation != generation)
        .unwrap_or(false);
    drop(guard);
    if should_cancel {
        cancel_cpu_native_overlay_session("stale-generation")
    } else {
        cpu_native_overlay_session_diagnostics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diagnostics_are_stable() {
        let diagnostics = NativeOverlaySessionDiagnostics::empty();
        assert!(!diagnostics.active);
        assert_eq!(diagnostics.state, NativeOverlaySessionState::Empty);
    }

    #[test]
    fn runtime_and_state_labels_are_stable() {
        assert_eq!(
            NativeOverlaySessionRuntime::CpuRgbaWin32.as_str(),
            "cpu-rgba-win32"
        );
        assert_eq!(NativeOverlaySessionState::Visible.as_str(), "visible");
    }
}

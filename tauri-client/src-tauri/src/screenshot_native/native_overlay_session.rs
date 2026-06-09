use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::overlay_renderer::render_rgba_frame_to_overlay;
use super::{
    create_win32_overlay, destroy_win32_overlay, set_win32_overlay_bitmap, show_win32_overlay,
    MonitorCaptureBounds, OverlayRenderReceipt, OverlayRenderTarget, RgbaFrame, Win32OverlayConfig,
};

const OVERLAY_THREAD_START_TIMEOUT_MS: u64 = 500;
const OVERLAY_THREAD_COMMAND_TIMEOUT_MS: u64 = 500;
const OVERLAY_THREAD_IDLE_TIMEOUT_MS: u64 = 5_000;

type NativeOverlayThreadReply = Sender<Result<(), String>>;

enum NativeOverlayThreadCommand {
    Raise {
        reason: String,
        reply: NativeOverlayThreadReply,
    },
    Cancel {
        reason: String,
        reply: NativeOverlayThreadReply,
    },
}

#[derive(Debug)]
struct NativeOverlayThreadStarted {
    hwnd: isize,
    render_receipt: Option<OverlayRenderReceipt>,
}

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
    hwnd: isize,
    command_sender: Sender<NativeOverlayThreadCommand>,
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
            hwnd: Some(self.hwnd),
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

    let (command_sender, command_receiver) = mpsc::channel::<NativeOverlayThreadCommand>();
    let (started_sender, started_receiver) =
        mpsc::channel::<Result<NativeOverlayThreadStarted, String>>();
    let thread_config = config.clone();
    let thread_bounds = bounds;
    let thread_frame = frame.clone();
    std::thread::Builder::new()
        .name("ysn-first-frame-shield".to_string())
        .spawn(move || {
            run_cpu_native_overlay_thread(
                thread_config,
                thread_bounds,
                thread_frame,
                started_sender,
                command_receiver,
            );
        })
        .map_err(|error| NativeOverlaySessionError::OverlayCreate(error.to_string()))?;

    let started = started_receiver
        .recv_timeout(Duration::from_millis(OVERLAY_THREAD_START_TIMEOUT_MS))
        .map_err(|error| NativeOverlaySessionError::OverlayCreate(error.to_string()))?
        .map_err(NativeOverlaySessionError::OverlayCreate)?;

    let session = CpuNativeOverlaySession {
        session_id,
        generation,
        bounds,
        hwnd: started.hwnd,
        command_sender,
        render_receipt: started.render_receipt,
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

fn run_cpu_native_overlay_thread(
    config: Win32OverlayConfig,
    bounds: MonitorCaptureBounds,
    frame: RgbaFrame,
    started_sender: Sender<Result<NativeOverlayThreadStarted, String>>,
    command_receiver: Receiver<NativeOverlayThreadCommand>,
) {
    let mut window = match create_win32_overlay(&config) {
        Ok(window) => window,
        Err(error) => {
            let _ = started_sender.send(Err(format!("native overlay create failed: {error}")));
            return;
        }
    };
    let hwnd = window.handle().hwnd();
    let started_result = (|| -> Result<NativeOverlayThreadStarted, String> {
        set_win32_overlay_bitmap(window.handle(), &frame.bytes, frame.width, frame.height)
            .map_err(|error| format!("native overlay render failed: {error}"))?;
        let render_target = OverlayRenderTarget::hwnd(hwnd, bounds.width, bounds.height);
        show_win32_overlay(&mut window, &config)
            .map_err(|error| format!("native overlay show failed: {error}"))?;
        let render_receipt = render_rgba_frame_to_overlay(render_target, &frame).ok();
        Ok(NativeOverlayThreadStarted {
            hwnd,
            render_receipt,
        })
    })();
    if started_sender.send(started_result).is_err() {
        let _ = destroy_win32_overlay(&mut window);
        return;
    }

    let started_at = std::time::Instant::now();
    loop {
        pump_overlay_thread_messages(hwnd);
        match command_receiver.recv_timeout(Duration::from_millis(4)) {
            Ok(NativeOverlayThreadCommand::Raise { reason, reply }) => {
                let result = show_win32_overlay(&mut window, &config)
                    .map_err(|error| format!("{reason}; raise failed: {error}"));
                let _ = reply.send(result);
            }
            Ok(NativeOverlayThreadCommand::Cancel { reason, reply }) => {
                let result = destroy_win32_overlay(&mut window)
                    .map_err(|error| format!("{reason}; destroy failed: {error}"));
                let _ = reply.send(result);
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if started_at.elapsed() > Duration::from_millis(OVERLAY_THREAD_IDLE_TIMEOUT_MS) {
                    let _ = destroy_win32_overlay(&mut window);
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = destroy_win32_overlay(&mut window);
                return;
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn pump_overlay_thread_messages(hwnd: isize) {
    while let Some(message) = super::win32_overlay_dispatch::peek_overlay_message(hwnd) {
        super::win32_overlay_dispatch::translate_and_dispatch_overlay_message(&message);
    }
}

#[cfg(not(target_os = "windows"))]
fn pump_overlay_thread_messages(_hwnd: isize) {}

fn send_overlay_thread_command(
    sender: &Sender<NativeOverlayThreadCommand>,
    command_builder: impl FnOnce(NativeOverlayThreadReply) -> NativeOverlayThreadCommand,
) -> Result<(), String> {
    let (reply_sender, reply_receiver) = mpsc::channel();
    sender
        .send(command_builder(reply_sender))
        .map_err(|error| error.to_string())?;
    reply_receiver
        .recv_timeout(Duration::from_millis(OVERLAY_THREAD_COMMAND_TIMEOUT_MS))
        .map_err(|error| error.to_string())?
}

pub fn raise_cpu_native_overlay_session(
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
    let Some(session) = guard.as_mut() else {
        return NativeOverlaySessionDiagnostics::empty();
    };
    if !matches!(
        session.state,
        NativeOverlaySessionState::Created
            | NativeOverlaySessionState::Rendered
            | NativeOverlaySessionState::Visible
    ) {
        return session.diagnostics();
    }
    match send_overlay_thread_command(&session.command_sender, |reply| {
        NativeOverlayThreadCommand::Raise {
            reason: reason.clone(),
            reply,
        }
    }) {
        Ok(()) => {
            session.state = NativeOverlaySessionState::Visible;
            session.fallback_reason = None;
        }
        Err(error) => {
            session.state = NativeOverlaySessionState::Failed;
            session.fallback_reason = Some(error);
        }
    }
    session.diagnostics()
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
    cancel_session_on_owner_thread(&mut session, reason);
    session.diagnostics()
}

pub fn cancel_cpu_native_overlay_session_if_matches(
    session_id: Option<&str>,
    reason: impl Into<String>,
) -> Option<NativeOverlaySessionDiagnostics> {
    let reason = reason.into();
    let Ok(mut guard) = session_store().lock() else {
        return Some(NativeOverlaySessionDiagnostics {
            state: NativeOverlaySessionState::Failed,
            fallback_reason: Some("native overlay session store lock failed".to_string()),
            ..NativeOverlaySessionDiagnostics::empty()
        });
    };
    let Some(session) = guard.as_ref() else {
        return None;
    };
    if let Some(expected_session_id) = session_id {
        if session.session_id != expected_session_id {
            return None;
        }
    }
    let Some(mut session) = guard.take() else {
        return None;
    };
    cancel_session_on_owner_thread(&mut session, reason);
    Some(session.diagnostics())
}

fn cancel_session_on_owner_thread(session: &mut CpuNativeOverlaySession, reason: String) {
    match send_overlay_thread_command(&session.command_sender, |reply| {
        NativeOverlayThreadCommand::Cancel {
            reason: reason.clone(),
            reply,
        }
    }) {
        Ok(()) => {
            session.state = NativeOverlaySessionState::Cancelled;
            session.fallback_reason = Some(reason);
        }
        Err(error) => {
            session.state = NativeOverlaySessionState::Failed;
            session.fallback_reason = Some(error);
        }
    }
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

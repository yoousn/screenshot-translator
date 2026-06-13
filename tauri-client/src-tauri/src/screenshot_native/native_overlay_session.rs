use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use super::overlay_renderer::render_rgba_frame_to_overlay;
use super::{
    create_win32_overlay, destroy_win32_overlay, set_win32_overlay_bitmap,
    set_win32_overlay_candidate, show_win32_overlay, MonitorCaptureBounds, OverlayRenderReceipt,
    OverlayRenderTarget, RgbaFrame, Win32OverlayConfig, Win32OverlayHandle,
    Win32OverlayNativeInputPhase, Win32OverlaySelectionRect,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOverlaySelectionSnapshot {
    pub hwnd: isize,
    pub bounds: MonitorCaptureBounds,
    pub selection: Option<crate::screenshot_native::Win32OverlaySelectionRect>,
    pub input_started: bool,
    pub mouse_captured: bool,
    pub completed: bool,
    pub cancelled: bool,
    pub phase: Win32OverlayNativeInputPhase,
    pub event_seq: u64,
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
    let thread_session_id = session_id.clone();
    std::thread::Builder::new()
        .name("ysn-native-first-frame-session".to_string())
        .spawn(move || {
            run_cpu_native_overlay_thread(
                thread_session_id,
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
    session_id: String,
    config: Win32OverlayConfig,
    bounds: MonitorCaptureBounds,
    frame: RgbaFrame,
    started_sender: Sender<Result<NativeOverlayThreadStarted, String>>,
    command_receiver: Receiver<NativeOverlayThreadCommand>,
) {
    let thread_started_at = std::time::Instant::now();
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
        let initial_candidate = current_native_candidate_rect(bounds, window.handle());
        set_win32_overlay_candidate(window.handle(), initial_candidate);
        if let Some(candidate) = initial_candidate {
            log_native_overlay_thread_event(
                &session_id,
                "native_candidate_first_rect",
                &thread_started_at,
                &format!(
                    "rect={},{},{},{}",
                    candidate.left,
                    candidate.top,
                    candidate.right.saturating_sub(candidate.left),
                    candidate.bottom.saturating_sub(candidate.top)
                ),
            );
        }
        let render_target = OverlayRenderTarget::hwnd(hwnd, bounds.width, bounds.height);
        show_win32_overlay(&mut window, &config)
            .map_err(|error| format!("native overlay show failed: {error}"))?;
        log_native_overlay_thread_event(
            &session_id,
            "native_overlay_first_paint",
            &thread_started_at,
            &format!("hwnd={hwnd} size={}x{}", bounds.width, bounds.height),
        );
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
    let mut last_selection: Option<(i32, i32, i32, i32)> = None;
    let mut last_candidate = current_native_candidate_rect(bounds, window.handle());
    let mut last_candidate_refresh: Option<std::time::Instant> = None;
    let mut last_native_input_phase: Option<Win32OverlayNativeInputPhase> = None;
    let mut native_handoff_ready_logged = false;
    loop {
        pump_overlay_thread_messages(hwnd);

        let native_input_snapshot =
            crate::screenshot_native::win32_overlay_native_input_snapshot(window.handle());
        let native_input_started = native_input_snapshot
            .map(|snapshot| snapshot.native_input_started)
            .unwrap_or(false);
        if let Some(snapshot) =
            native_input_snapshot.filter(|snapshot| snapshot.native_input_started)
        {
            if last_native_input_phase.is_none() {
                log_native_overlay_thread_event(
                    &session_id,
                    "native_input_ready",
                    &thread_started_at,
                    &format!(
                        "phase={} event_seq={} captured={}",
                        snapshot.phase.as_str(),
                        snapshot.event_seq,
                        snapshot.mouse_captured
                    ),
                );
            }
            if snapshot.phase.handoff_ready() && !native_handoff_ready_logged {
                native_handoff_ready_logged = true;
                log_native_overlay_thread_event(
                    &session_id,
                    "native_selection_handoff_ready",
                    &thread_started_at,
                    &format!(
                        "phase={} event_seq={} has_selection={}",
                        snapshot.phase.as_str(),
                        snapshot.event_seq,
                        snapshot.selection.is_some()
                    ),
                );
            }
            if Some(snapshot.phase) != last_native_input_phase {
                match snapshot.phase {
                    Win32OverlayNativeInputPhase::Completed => {
                        log_native_overlay_thread_event(
                            &session_id,
                            "native_selection_completed",
                            &thread_started_at,
                            &native_input_snapshot_detail(snapshot),
                        );
                    }
                    Win32OverlayNativeInputPhase::Cancelled => {
                        log_native_overlay_thread_event(
                            &session_id,
                            "native_selection_cancelled",
                            &thread_started_at,
                            &native_input_snapshot_detail(snapshot),
                        );
                    }
                    Win32OverlayNativeInputPhase::Idle
                    | Win32OverlayNativeInputPhase::Started
                    | Win32OverlayNativeInputPhase::Selecting => {}
                }
                last_native_input_phase = Some(snapshot.phase);
            }
        }
        if native_input_started {
            if last_candidate.is_some() {
                last_candidate = None;
                set_win32_overlay_candidate(window.handle(), None);
            }
        } else if last_candidate_refresh
            .map(|instant| instant.elapsed() >= Duration::from_millis(16))
            .unwrap_or(true)
        {
            last_candidate_refresh = Some(std::time::Instant::now());
            let current_candidate = current_native_candidate_rect(bounds, window.handle());
            if current_candidate != last_candidate {
                last_candidate = current_candidate;
                set_win32_overlay_candidate(window.handle(), current_candidate);
            }
        }

        if !native_input_started {
            let current_selection =
                crate::screenshot_commands::read_screenshot_pointer_pre_capture_selection(
                    &session_id,
                );
            if current_selection != last_selection {
                last_selection = current_selection;
                let new_selection_rect = current_selection.map(|(x, y, w, h)| {
                    crate::screenshot_native::Win32OverlaySelectionRect {
                        left: x - bounds.origin_x,
                        top: y - bounds.origin_y,
                        right: x - bounds.origin_x + w,
                        bottom: y - bounds.origin_y + h,
                    }
                });
                crate::screenshot_native::set_win32_overlay_selection(
                    window.handle(),
                    new_selection_rect,
                );
            }
        }

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
    let (session_id, command_sender) = {
        let Ok(guard) = session_store().lock() else {
            return NativeOverlaySessionDiagnostics {
                state: NativeOverlaySessionState::Failed,
                fallback_reason: Some("native overlay session store lock failed".to_string()),
                ..NativeOverlaySessionDiagnostics::empty()
            };
        };
        let Some(session) = guard.as_ref() else {
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
        (session.session_id.clone(), session.command_sender.clone())
    };
    let command_result =
        send_overlay_thread_command(&command_sender, |reply| NativeOverlayThreadCommand::Raise {
            reason: reason.clone(),
            reply,
        });
    let Ok(mut guard) = session_store().lock() else {
        return NativeOverlaySessionDiagnostics {
            state: NativeOverlaySessionState::Failed,
            fallback_reason: Some("native overlay session store lock failed".to_string()),
            ..NativeOverlaySessionDiagnostics::empty()
        };
    };
    let Some(session) = guard
        .as_mut()
        .filter(|session| session.session_id == session_id)
    else {
        return NativeOverlaySessionDiagnostics::empty();
    };
    match command_result {
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
    let mut session = {
        let Ok(mut guard) = session_store().lock() else {
            return NativeOverlaySessionDiagnostics {
                state: NativeOverlaySessionState::Failed,
                fallback_reason: Some("native overlay session store lock failed".to_string()),
                ..NativeOverlaySessionDiagnostics::empty()
            };
        };
        let Some(session) = guard.take() else {
            return NativeOverlaySessionDiagnostics::empty();
        };
        session
    };
    cancel_session_on_owner_thread(&mut session, reason);
    session.diagnostics()
}

pub fn cancel_cpu_native_overlay_session_if_matches(
    session_id: Option<&str>,
    reason: impl Into<String>,
) -> Option<NativeOverlaySessionDiagnostics> {
    let reason = reason.into();
    let mut session = {
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
        guard.take()?
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

pub fn cpu_native_overlay_selection_snapshot(
    session_id: Option<&str>,
) -> Option<NativeOverlaySelectionSnapshot> {
    let guard = session_store().lock().ok()?;
    let session = guard.as_ref()?;
    if let Some(expected_session_id) = session_id {
        if session.session_id != expected_session_id {
            return None;
        }
    }
    if !matches!(
        session.state,
        NativeOverlaySessionState::Created
            | NativeOverlaySessionState::Rendered
            | NativeOverlaySessionState::Visible
    ) {
        return None;
    }
    let input = crate::screenshot_native::win32_overlay_native_input_snapshot(
        crate::screenshot_native::Win32OverlayHandle::new(session.hwnd),
    )?;
    if !input.native_input_started {
        return None;
    }
    Some(NativeOverlaySelectionSnapshot {
        hwnd: session.hwnd,
        bounds: session.bounds,
        selection: input.selection,
        input_started: input.native_input_started,
        mouse_captured: input.mouse_captured,
        completed: input.completed,
        cancelled: input.cancelled,
        phase: input.phase,
        event_seq: input.event_seq,
    })
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

#[cfg(target_os = "windows")]
fn current_native_candidate_rect(
    bounds: MonitorCaptureBounds,
    overlay_handle: Win32OverlayHandle,
) -> Option<Win32OverlaySelectionRect> {
    let (cursor_x, cursor_y) = crate::window_targets::get_cursor_position()?;

    for hwnd in crate::window_targets::taskbar_windows_at_cursor(cursor_x, cursor_y) {
        if let Some(rect) = crate::window_targets::hwnd_rect(hwnd, false)
            .and_then(|rect| rect_to_overlay_candidate(rect, bounds, 12))
        {
            return Some(rect);
        }
    }

    let excluded_hwnds = vec![overlay_handle.hwnd()];
    let windows =
        crate::window_targets::top_level_windows_at_cursor(cursor_x, cursor_y, excluded_hwnds);
    if let Some(hwnd) = windows.first().copied() {
        for child in crate::window_targets::child_windows_at_cursor(hwnd, cursor_x, cursor_y)
            .into_iter()
            .rev()
            .take(1)
        {
            if let Some(rect) = crate::window_targets::hwnd_rect(child, false)
                .and_then(|rect| rect_to_overlay_candidate(rect, bounds, 12))
            {
                return Some(rect);
            }
        }
        if let Some(rect) = crate::window_targets::hwnd_rect(hwnd, true)
            .and_then(|rect| rect_to_overlay_candidate(rect, bounds, 50))
        {
            return Some(rect);
        }
    }

    Some(Win32OverlaySelectionRect {
        left: 0,
        top: 0,
        right: bounds.width.min(i32::MAX as u32) as i32,
        bottom: bounds.height.min(i32::MAX as u32) as i32,
    })
}

#[cfg(not(target_os = "windows"))]
fn current_native_candidate_rect(
    _bounds: MonitorCaptureBounds,
    _overlay_handle: Win32OverlayHandle,
) -> Option<Win32OverlaySelectionRect> {
    None
}

#[cfg(target_os = "windows")]
fn rect_to_overlay_candidate(
    rect: crate::win32::RECT,
    bounds: MonitorCaptureBounds,
    min_size: i32,
) -> Option<Win32OverlaySelectionRect> {
    let bounds_right = bounds
        .origin_x
        .saturating_add(bounds.width.min(i32::MAX as u32) as i32);
    let bounds_bottom = bounds
        .origin_y
        .saturating_add(bounds.height.min(i32::MAX as u32) as i32);
    let left = rect.left.max(bounds.origin_x);
    let top = rect.top.max(bounds.origin_y);
    let right = rect.right.min(bounds_right);
    let bottom = rect.bottom.min(bounds_bottom);
    if right.saturating_sub(left) < min_size || bottom.saturating_sub(top) < min_size {
        return None;
    }
    Some(Win32OverlaySelectionRect {
        left: left.saturating_sub(bounds.origin_x),
        top: top.saturating_sub(bounds.origin_y),
        right: right.saturating_sub(bounds.origin_x),
        bottom: bottom.saturating_sub(bounds.origin_y),
    })
}

fn log_native_overlay_thread_event(
    session_id: &str,
    phase: &str,
    started_at: &std::time::Instant,
    detail: &str,
) {
    println!(
        "[screenshot-baseline] session={} phase={} elapsed_ms={} {}",
        session_id,
        phase,
        started_at.elapsed().as_millis(),
        detail
    );
}

fn native_input_snapshot_detail(
    snapshot: crate::screenshot_native::Win32OverlayNativeInputSnapshot,
) -> String {
    let rect = snapshot
        .selection
        .map(|selection| {
            format!(
                "{},{},{},{}",
                selection.left,
                selection.top,
                selection.right.saturating_sub(selection.left),
                selection.bottom.saturating_sub(selection.top)
            )
        })
        .unwrap_or_else(|| "none".to_string());
    format!(
        "phase={} event_seq={} captured={} completed={} cancelled={} rect={}",
        snapshot.phase.as_str(),
        snapshot.event_seq,
        snapshot.mouse_captured,
        snapshot.completed,
        snapshot.cancelled,
        rect
    )
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

    #[cfg(target_os = "windows")]
    #[test]
    fn rect_to_overlay_candidate_clips_to_capture_bounds() {
        let rect = crate::win32::RECT {
            left: -10,
            top: 10,
            right: 120,
            bottom: 140,
        };
        let bounds = MonitorCaptureBounds::new(0, 0, 100, 100);

        assert_eq!(
            rect_to_overlay_candidate(rect, bounds, 12),
            Some(Win32OverlaySelectionRect {
                left: 0,
                top: 10,
                right: 100,
                bottom: 100,
            })
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rect_to_overlay_candidate_rejects_tiny_rects() {
        let rect = crate::win32::RECT {
            left: 10,
            top: 10,
            right: 16,
            bottom: 16,
        };
        let bounds = MonitorCaptureBounds::new(0, 0, 100, 100);

        assert_eq!(rect_to_overlay_candidate(rect, bounds, 12), None);
    }
}

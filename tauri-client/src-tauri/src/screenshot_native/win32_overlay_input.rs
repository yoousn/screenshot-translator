#[cfg(target_os = "windows")]
use super::output::SelectionRect;
#[cfg(target_os = "windows")]
use super::selection_state::{SelectionState, SelectionTransition};
#[cfg(target_os = "windows")]
use super::win32_input::{
    screenshot_input_event_from_win32_message, Win32KeyState, MK_LBUTTON, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_RWIN, VK_SHIFT, WM_CANCELMODE, WM_CAPTURECHANGED, WM_KEYDOWN, WM_KILLFOCUS,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN,
};
use super::win32_overlay::{Win32OverlayHandle, Win32OverlaySelectionRect};
#[cfg(target_os = "windows")]
use crate::win32;
#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32OverlayNativeInputSnapshot {
    pub native_input_started: bool,
    pub mouse_captured: bool,
    pub completed: bool,
    pub cancelled: bool,
    pub phase: Win32OverlayNativeInputPhase,
    pub event_seq: u64,
    pub selection: Option<Win32OverlaySelectionRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32OverlayNativeInputPhase {
    Idle,
    Started,
    Selecting,
    Completed,
    Cancelled,
}

impl Win32OverlayNativeInputPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Started => "started",
            Self::Selecting => "selecting",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn handoff_ready(self) -> bool {
        matches!(self, Self::Selecting | Self::Completed | Self::Cancelled)
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
struct Win32OverlayInputState {
    selection_state: SelectionState,
    native_input_started: bool,
    mouse_captured: bool,
    completed: bool,
    cancelled: bool,
    event_seq: u64,
    latest_selection: Option<Win32OverlaySelectionRect>,
}

#[cfg(target_os = "windows")]
impl Default for Win32OverlayInputState {
    fn default() -> Self {
        Self {
            selection_state: SelectionState::new(),
            native_input_started: false,
            mouse_captured: false,
            completed: false,
            cancelled: false,
            event_seq: 0,
            latest_selection: None,
        }
    }
}

#[cfg(target_os = "windows")]
impl Win32OverlayInputState {
    const fn input_phase(self) -> Win32OverlayNativeInputPhase {
        if self.cancelled {
            return Win32OverlayNativeInputPhase::Cancelled;
        }
        if self.completed {
            return Win32OverlayNativeInputPhase::Completed;
        }
        if self.mouse_captured || self.latest_selection.is_some() {
            return Win32OverlayNativeInputPhase::Selecting;
        }
        if self.native_input_started {
            return Win32OverlayNativeInputPhase::Started;
        }
        Win32OverlayNativeInputPhase::Idle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Win32OverlayInputDispatch {
    pub handled: bool,
    pub set_capture: bool,
    pub release_capture: bool,
    pub selection: Option<Option<Win32OverlaySelectionRect>>,
}

#[cfg(target_os = "windows")]
impl Win32OverlayInputDispatch {
    const fn ignored() -> Self {
        Self {
            handled: false,
            set_capture: false,
            release_capture: false,
            selection: None,
        }
    }
}

#[cfg(target_os = "windows")]
static WIN32_OVERLAY_INPUT_STATES: OnceLock<Mutex<HashMap<isize, Win32OverlayInputState>>> =
    OnceLock::new();

#[cfg(target_os = "windows")]
fn overlay_input_store() -> &'static Mutex<HashMap<isize, Win32OverlayInputState>> {
    WIN32_OVERLAY_INPUT_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "windows")]
pub(crate) fn initialize_win32_overlay_input_state(handle: Win32OverlayHandle) {
    if !handle.is_valid() {
        return;
    }
    if let Ok(mut guard) = overlay_input_store().lock() {
        guard.insert(handle.hwnd(), Win32OverlayInputState::default());
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn initialize_win32_overlay_input_state(_handle: Win32OverlayHandle) {}

#[cfg(target_os = "windows")]
pub(crate) fn clear_win32_overlay_input_state(handle: Win32OverlayHandle) {
    if let Ok(mut guard) = overlay_input_store().lock() {
        guard.remove(&handle.hwnd());
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn clear_win32_overlay_input_state(_handle: Win32OverlayHandle) {}

#[cfg(target_os = "windows")]
pub fn win32_overlay_native_input_started(handle: Win32OverlayHandle) -> bool {
    if !handle.is_valid() {
        return false;
    }
    overlay_input_store()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&handle.hwnd()).copied())
        .map(|state| state.native_input_started)
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn win32_overlay_native_input_started(_handle: Win32OverlayHandle) -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn win32_overlay_native_input_snapshot(
    handle: Win32OverlayHandle,
) -> Option<Win32OverlayNativeInputSnapshot> {
    if !handle.is_valid() {
        return None;
    }
    overlay_input_store()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&handle.hwnd()).copied())
        .map(|state| Win32OverlayNativeInputSnapshot {
            native_input_started: state.native_input_started,
            mouse_captured: state.mouse_captured,
            completed: state.completed,
            cancelled: state.cancelled,
            phase: state.input_phase(),
            event_seq: state.event_seq,
            selection: state.latest_selection,
        })
}

#[cfg(not(target_os = "windows"))]
pub fn win32_overlay_native_input_snapshot(
    _handle: Win32OverlayHandle,
) -> Option<Win32OverlayNativeInputSnapshot> {
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn apply_win32_overlay_input(
    hwnd: isize,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> Win32OverlayInputDispatch {
    if hwnd == 0 {
        return Win32OverlayInputDispatch::ignored();
    }
    let Some(event) = screenshot_input_event_from_win32_message(
        message,
        w_param,
        l_param,
        current_win32_overlay_key_state(),
    ) else {
        return Win32OverlayInputDispatch::ignored();
    };

    let Ok(mut guard) = overlay_input_store().lock() else {
        return Win32OverlayInputDispatch::ignored();
    };
    let state = guard.entry(hwnd).or_default();
    if message == WM_MOUSEMOVE
        && !state.mouse_captured
        && !state.selection_state.is_active()
        && (w_param & MK_LBUTTON as usize) == 0
    {
        return Win32OverlayInputDispatch::ignored();
    }

    let mut dispatch = Win32OverlayInputDispatch {
        handled: true,
        set_capture: false,
        release_capture: false,
        selection: None,
    };
    match message {
        WM_LBUTTONDOWN => {
            state.native_input_started = true;
            state.mouse_captured = true;
            dispatch.set_capture = true;
        }
        WM_MOUSEMOVE => {
            state.native_input_started = true;
        }
        WM_LBUTTONUP => {
            state.native_input_started = true;
            state.mouse_captured = false;
            dispatch.release_capture = true;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KILLFOCUS | WM_CANCELMODE | WM_CAPTURECHANGED => {}
        _ => return Win32OverlayInputDispatch::ignored(),
    }

    let transition = state.selection_state.handle_event(event);
    dispatch.selection = selection_update_from_transition(transition);
    if matches!(transition, SelectionTransition::Cancelled)
        || matches!(message, WM_KILLFOCUS | WM_CANCELMODE | WM_CAPTURECHANGED)
    {
        state.native_input_started = true;
    }
    apply_input_snapshot_transition(state, transition);
    if state.native_input_started {
        state.event_seq = state.event_seq.saturating_add(1);
    }
    if state.selection_state.is_terminal()
        || matches!(message, WM_KILLFOCUS | WM_CANCELMODE | WM_CAPTURECHANGED)
    {
        state.mouse_captured = false;
        dispatch.release_capture = true;
    }
    dispatch
}

#[cfg(target_os = "windows")]
fn selection_update_from_transition(
    transition: SelectionTransition,
) -> Option<Option<Win32OverlaySelectionRect>> {
    match transition {
        SelectionTransition::Ignored => None,
        SelectionTransition::Armed { .. } | SelectionTransition::Cancelled => Some(None),
        SelectionTransition::Updated { rect } | SelectionTransition::Completed { rect } => {
            Some(Some(selection_rect_to_overlay_rect(rect)))
        }
    }
}

#[cfg(target_os = "windows")]
fn apply_input_snapshot_transition(
    state: &mut Win32OverlayInputState,
    transition: SelectionTransition,
) {
    match transition {
        SelectionTransition::Ignored => {}
        SelectionTransition::Armed { .. } => {
            state.completed = false;
            state.cancelled = false;
            state.latest_selection = None;
        }
        SelectionTransition::Updated { rect } => {
            state.completed = false;
            state.cancelled = false;
            state.latest_selection = Some(selection_rect_to_overlay_rect(rect));
        }
        SelectionTransition::Completed { rect } => {
            state.completed = true;
            state.cancelled = false;
            state.latest_selection = Some(selection_rect_to_overlay_rect(rect));
        }
        SelectionTransition::Cancelled => {
            state.completed = false;
            state.cancelled = true;
            state.latest_selection = None;
        }
    }
}

#[cfg(target_os = "windows")]
fn selection_rect_to_overlay_rect(rect: SelectionRect) -> Win32OverlaySelectionRect {
    let rect = rect.normalized();
    Win32OverlaySelectionRect {
        left: rect.x,
        top: rect.y,
        right: rect.x.saturating_add(rect.width),
        bottom: rect.y.saturating_add(rect.height),
    }
}

#[cfg(target_os = "windows")]
fn current_win32_overlay_key_state() -> Win32KeyState {
    Win32KeyState {
        shift: win32_overlay_key_is_down(VK_SHIFT),
        ctrl: win32_overlay_key_is_down(VK_CONTROL),
        alt: win32_overlay_key_is_down(VK_MENU),
        meta: win32_overlay_key_is_down(VK_LWIN) || win32_overlay_key_is_down(VK_RWIN),
    }
}

#[cfg(target_os = "windows")]
fn win32_overlay_key_is_down(virtual_key: u16) -> bool {
    unsafe { win32::GetAsyncKeyState(virtual_key as i32) & i16::MIN != 0 }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    fn test_lparam(x: i32, y: i32) -> isize {
        let x = (x as i16 as u16) as u32;
        let y = (y as i16 as u16) as u32;
        ((y << 16) | x) as isize
    }

    #[test]
    fn selection_transition_maps_to_overlay_rect_updates() {
        assert_eq!(
            selection_update_from_transition(SelectionTransition::Armed {
                anchor_x: 10,
                anchor_y: 10,
            }),
            Some(None)
        );
        assert_eq!(
            selection_update_from_transition(SelectionTransition::Completed {
                rect: SelectionRect::new(10, 12, 70, 48),
            }),
            Some(Some(Win32OverlaySelectionRect {
                left: 10,
                top: 12,
                right: 80,
                bottom: 60,
            }))
        );
    }

    #[test]
    fn native_input_dispatch_tracks_drag_selection() {
        let hwnd = 42_269;
        initialize_win32_overlay_input_state(Win32OverlayHandle::new(hwnd));

        let down = apply_win32_overlay_input(
            hwnd,
            WM_LBUTTONDOWN,
            MK_LBUTTON as usize,
            test_lparam(10, 10),
        );
        assert!(down.handled);
        assert!(down.set_capture);
        assert_eq!(down.selection, Some(None));
        assert!(win32_overlay_native_input_started(Win32OverlayHandle::new(
            hwnd
        )));

        let move_event =
            apply_win32_overlay_input(hwnd, WM_MOUSEMOVE, MK_LBUTTON as usize, test_lparam(80, 70));
        assert!(move_event.handled);
        assert_eq!(
            move_event.selection,
            Some(Some(Win32OverlaySelectionRect {
                left: 10,
                top: 10,
                right: 80,
                bottom: 70,
            }))
        );

        let up = apply_win32_overlay_input(hwnd, WM_LBUTTONUP, 0, test_lparam(80, 70));
        assert!(up.handled);
        assert!(up.release_capture);
        assert_eq!(
            up.selection,
            Some(Some(Win32OverlaySelectionRect {
                left: 10,
                top: 10,
                right: 80,
                bottom: 70,
            }))
        );
        assert_eq!(
            win32_overlay_native_input_snapshot(Win32OverlayHandle::new(hwnd)),
            Some(Win32OverlayNativeInputSnapshot {
                native_input_started: true,
                mouse_captured: false,
                completed: true,
                cancelled: false,
                phase: Win32OverlayNativeInputPhase::Completed,
                event_seq: 3,
                selection: Some(Win32OverlaySelectionRect {
                    left: 10,
                    top: 10,
                    right: 80,
                    bottom: 70,
                }),
            })
        );

        clear_win32_overlay_input_state(Win32OverlayHandle::new(hwnd));
    }

    #[test]
    fn native_input_dispatch_surfaces_cancel_without_selection() {
        let hwnd = 42_270;
        initialize_win32_overlay_input_state(Win32OverlayHandle::new(hwnd));

        let cancel = apply_win32_overlay_input(
            hwnd,
            WM_KEYDOWN,
            super::super::win32_input::VK_ESCAPE as usize,
            0,
        );

        assert!(cancel.handled);
        assert!(cancel.release_capture);
        assert_eq!(cancel.selection, Some(None));
        assert_eq!(
            win32_overlay_native_input_snapshot(Win32OverlayHandle::new(hwnd)),
            Some(Win32OverlayNativeInputSnapshot {
                native_input_started: true,
                mouse_captured: false,
                completed: false,
                cancelled: true,
                phase: Win32OverlayNativeInputPhase::Cancelled,
                event_seq: 1,
                selection: None,
            })
        );

        clear_win32_overlay_input_state(Win32OverlayHandle::new(hwnd));
    }
}

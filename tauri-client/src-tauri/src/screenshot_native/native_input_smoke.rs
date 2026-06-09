use super::selection_state::{SelectionState, SelectionStateStatus, SelectionTransition};
use super::win32_input::{
    screenshot_input_event_from_win32_message, Win32KeyState, MK_LBUTTON, VK_ESCAPE, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};
use super::SelectionRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInputSmokeStatus {
    Completed,
    Cancelled,
    Incomplete,
    DecodeFailed,
}

impl NativeInputSmokeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Incomplete => "incomplete",
            Self::DecodeFailed => "decode-failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInputSmokeReport {
    pub status: NativeInputSmokeStatus,
    pub transitions: Vec<&'static str>,
    pub completed_rect: Option<SelectionRect>,
    pub final_state: SelectionStateStatus,
    pub decoded_events: usize,
    pub error: Option<String>,
}

fn lparam(x: i32, y: i32) -> isize {
    let x = (x as i16 as u16) as u32;
    let y = (y as i16 as u16) as u32;
    ((y << 16) | x) as isize
}

fn transition_label(transition: SelectionTransition) -> &'static str {
    match transition {
        SelectionTransition::Ignored => "ignored",
        SelectionTransition::Armed { .. } => "armed",
        SelectionTransition::Updated { .. } => "updated",
        SelectionTransition::Completed { .. } => "completed",
        SelectionTransition::Cancelled => "cancelled",
    }
}

pub fn run_synthetic_native_drag_smoke() -> NativeInputSmokeReport {
    let mut state = SelectionState::new();
    let messages = [
        (WM_LBUTTONDOWN, MK_LBUTTON as usize, lparam(10, 10)),
        (WM_MOUSEMOVE, MK_LBUTTON as usize, lparam(80, 70)),
        (WM_LBUTTONUP, 0, lparam(80, 70)),
    ];
    let mut transitions = Vec::new();
    let mut decoded_events = 0;

    for (message, wparam, lparam) in messages {
        let Some(event) = screenshot_input_event_from_win32_message(
            message,
            wparam,
            lparam,
            Win32KeyState::empty(),
        ) else {
            return NativeInputSmokeReport {
                status: NativeInputSmokeStatus::DecodeFailed,
                transitions,
                completed_rect: None,
                final_state: state.status(),
                decoded_events,
                error: Some(format!("failed to decode win32 message {message}")),
            };
        };
        decoded_events += 1;
        let transition = state.handle_event(event);
        transitions.push(transition_label(transition));
    }

    let completed_rect = state.completed_selection();
    NativeInputSmokeReport {
        status: if completed_rect.is_some() {
            NativeInputSmokeStatus::Completed
        } else {
            NativeInputSmokeStatus::Incomplete
        },
        transitions,
        completed_rect,
        final_state: state.status(),
        decoded_events,
        error: None,
    }
}

pub fn run_synthetic_native_cancel_smoke() -> NativeInputSmokeReport {
    let mut state = SelectionState::new();
    let messages = [
        (WM_LBUTTONDOWN, MK_LBUTTON as usize, lparam(10, 10)),
        (WM_KEYDOWN, VK_ESCAPE as usize, 0),
    ];
    let mut transitions = Vec::new();
    let mut decoded_events = 0;

    for (message, wparam, lparam) in messages {
        let Some(event) = screenshot_input_event_from_win32_message(
            message,
            wparam,
            lparam,
            Win32KeyState::empty(),
        ) else {
            return NativeInputSmokeReport {
                status: NativeInputSmokeStatus::DecodeFailed,
                transitions,
                completed_rect: None,
                final_state: state.status(),
                decoded_events,
                error: Some(format!("failed to decode win32 message {message}")),
            };
        };
        decoded_events += 1;
        let transition = state.handle_event(event);
        transitions.push(transition_label(transition));
    }

    NativeInputSmokeReport {
        status: if matches!(state.status(), SelectionStateStatus::Cancelled) {
            NativeInputSmokeStatus::Cancelled
        } else {
            NativeInputSmokeStatus::Incomplete
        },
        transitions,
        completed_rect: state.completed_selection(),
        final_state: state.status(),
        decoded_events,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_drag_smoke_completes_selection() {
        let report = run_synthetic_native_drag_smoke();
        assert_eq!(report.status, NativeInputSmokeStatus::Completed);
        assert_eq!(report.decoded_events, 3);
        assert_eq!(
            report.completed_rect,
            Some(SelectionRect::new(10, 10, 70, 60))
        );
        assert!(report.transitions.contains(&"completed"));
    }

    #[test]
    fn synthetic_cancel_smoke_cancels_selection() {
        let report = run_synthetic_native_cancel_smoke();
        assert_eq!(report.status, NativeInputSmokeStatus::Cancelled);
        assert_eq!(report.decoded_events, 2);
        assert!(report.completed_rect.is_none());
        assert!(report.transitions.contains(&"cancelled"));
    }
}

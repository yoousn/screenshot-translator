use super::win32_input::{screenshot_input_event_from_win32_message, Win32KeyState};
use super::{
    ScreenshotInputEvent, Win32OverlayHandle, Win32OverlayPumpDiagnostics,
    Win32OverlayPumpExitReason, Win32OverlayPumpOptions,
};

#[cfg(target_os = "windows")]
const PM_REMOVE: u32 = 0x0001;
#[cfg(target_os = "windows")]
const QS_ALLINPUT: u32 = 0x04FF;
#[cfg(target_os = "windows")]
const MWMO_INPUTAVAILABLE: u32 = 0x0004;
#[cfg(target_os = "windows")]
const WAIT_OBJECT_0: u32 = 0x00000000;
#[cfg(target_os = "windows")]
const WAIT_TIMEOUT: u32 = 0x00000102;
#[cfg(target_os = "windows")]
const WAIT_FAILED: u32 = 0xFFFFFFFF;
#[cfg(target_os = "windows")]
const WM_DESTROY: u32 = 0x0002;
#[cfg(target_os = "windows")]
const WM_NCDESTROY: u32 = 0x0082;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32OverlayWaitResult {
    InputAvailable,
    Timeout,
    Failed,
    Unexpected(u32),
}

pub const fn input_event_label(event: ScreenshotInputEvent) -> &'static str {
    match event {
        ScreenshotInputEvent::MouseDown { .. } => "mouse-down",
        ScreenshotInputEvent::MouseMove { .. } => "mouse-move",
        ScreenshotInputEvent::MouseUp { .. } => "mouse-up",
        ScreenshotInputEvent::PointerDown { .. } => "pointer-down",
        ScreenshotInputEvent::PointerMove { .. } => "pointer-move",
        ScreenshotInputEvent::PointerUp { .. } => "pointer-up",
        ScreenshotInputEvent::Wheel { .. } => "wheel",
        ScreenshotInputEvent::Key { .. } => "key",
        ScreenshotInputEvent::LostFocus => "lost-focus",
        ScreenshotInputEvent::Confirm => "confirm",
        ScreenshotInputEvent::Cancel => "cancel",
        ScreenshotInputEvent::RepeatHotkey => "repeat-hotkey",
    }
}

pub fn run_win32_overlay_message_tuple_diagnostic_pump(
    handle: Win32OverlayHandle,
    options: Win32OverlayPumpOptions,
    message: u32,
    wparam: usize,
    lparam: isize,
    key_state: Win32KeyState,
) -> Win32OverlayPumpDiagnostics {
    match screenshot_input_event_from_win32_message(message, wparam, lparam, key_state) {
        Some(event) => Win32OverlayPumpDiagnostics::from_event(handle, options, event),
        None => Win32OverlayPumpDiagnostics {
            active: false,
            hwnd: handle.hwnd(),
            session_id: options.session_id,
            generation: options.generation,
            started: true,
            events_dispatched: 0,
            last_event: None,
            exit_reason: Win32OverlayPumpExitReason::NonTerminalInput,
            error: Some(format!(
                "Win32 message {message:#06x} did not map to screenshot input"
            )),
        },
    }
}

#[cfg(target_os = "windows")]
pub const fn is_overlay_destroy_message(message: u32) -> bool {
    matches!(message, WM_DESTROY | WM_NCDESTROY)
}

#[cfg(target_os = "windows")]
pub fn peek_overlay_message(hwnd: isize) -> Option<crate::win32::MSG> {
    let mut message = std::mem::MaybeUninit::<crate::win32::MSG>::uninit();
    let has_message =
        unsafe { crate::win32::PeekMessageW(message.as_mut_ptr(), hwnd, 0, 0, PM_REMOVE) != 0 };
    if has_message {
        Some(unsafe { message.assume_init() })
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn translate_and_dispatch_overlay_message(message: &crate::win32::MSG) {
    unsafe {
        crate::win32::TranslateMessage(message);
        crate::win32::DispatchMessageW(message);
    }
}

#[cfg(target_os = "windows")]
pub fn has_pending_overlay_message(hwnd: isize) -> bool {
    let mut probe = std::mem::MaybeUninit::<crate::win32::MSG>::uninit();
    unsafe { crate::win32::PeekMessageW(probe.as_mut_ptr(), hwnd, 0, 0, 0) != 0 }
}

#[cfg(target_os = "windows")]
pub fn wait_for_overlay_input(wait_ms: u32) -> Win32OverlayWaitResult {
    let wait_result = unsafe {
        crate::win32::MsgWaitForMultipleObjectsEx(
            0,
            std::ptr::null(),
            wait_ms,
            QS_ALLINPUT,
            MWMO_INPUTAVAILABLE,
        )
    };

    match wait_result {
        WAIT_OBJECT_0 => Win32OverlayWaitResult::InputAvailable,
        WAIT_TIMEOUT => Win32OverlayWaitResult::Timeout,
        WAIT_FAILED => Win32OverlayWaitResult::Failed,
        other => Win32OverlayWaitResult::Unexpected(other),
    }
}

#[cfg(target_os = "windows")]
pub fn current_win32_key_state() -> Win32KeyState {
    Win32KeyState {
        shift: is_key_down(super::win32_input::VK_SHIFT),
        ctrl: is_key_down(super::win32_input::VK_CONTROL),
        alt: is_key_down(super::win32_input::VK_MENU),
        meta: is_key_down(super::win32_input::VK_LWIN) || is_key_down(super::win32_input::VK_RWIN),
    }
}

#[cfg(target_os = "windows")]
fn is_key_down(virtual_key: u16) -> bool {
    unsafe { crate::win32::GetAsyncKeyState(virtual_key as i32) & i16::MIN != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_win32_message_tuple_converts_to_pump_diagnostics() {
        let diagnostics = run_win32_overlay_message_tuple_diagnostic_pump(
            Win32OverlayHandle::new(42),
            Win32OverlayPumpOptions::diagnostic("session", 7),
            super::super::win32_input::WM_KEYDOWN,
            super::super::win32_input::VK_ESCAPE as usize,
            0,
            super::super::win32_input::Win32KeyState::empty(),
        );

        assert_eq!(diagnostics.hwnd, 42);
        assert_eq!(diagnostics.events_dispatched, 1);
        assert_eq!(diagnostics.last_event, Some("cancel"));
        assert_eq!(diagnostics.exit_reason, Win32OverlayPumpExitReason::Cancel);
    }

    #[test]
    fn unmapped_win32_message_tuple_reports_non_terminal_input() {
        let diagnostics = run_win32_overlay_message_tuple_diagnostic_pump(
            Win32OverlayHandle::new(42),
            Win32OverlayPumpOptions::diagnostic("session", 7),
            0xFFFF,
            0,
            0,
            super::super::win32_input::Win32KeyState::empty(),
        );

        assert_eq!(diagnostics.events_dispatched, 0);
        assert_eq!(
            diagnostics.exit_reason,
            Win32OverlayPumpExitReason::NonTerminalInput
        );
        assert!(diagnostics.error.unwrap().contains("did not map"));
    }

    #[test]
    fn dispatch_labels_terminal_events_stably() {
        assert_eq!(input_event_label(ScreenshotInputEvent::Confirm), "confirm");
        assert_eq!(input_event_label(ScreenshotInputEvent::Cancel), "cancel");
        assert_eq!(
            input_event_label(ScreenshotInputEvent::RepeatHotkey),
            "repeat-hotkey"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn destroy_messages_are_terminal_window_messages() {
        assert!(is_overlay_destroy_message(WM_DESTROY));
        assert!(is_overlay_destroy_message(WM_NCDESTROY));
        assert!(!is_overlay_destroy_message(
            super::super::win32_input::WM_KEYDOWN
        ));
    }
}

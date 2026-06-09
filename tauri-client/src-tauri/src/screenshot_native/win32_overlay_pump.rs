use super::input::KeyCommand;
use super::win32_overlay_dispatch::{
    current_win32_key_state, has_pending_overlay_message, input_event_label,
    is_overlay_destroy_message, peek_overlay_message,
    run_win32_overlay_message_tuple_diagnostic_pump, translate_and_dispatch_overlay_message,
    wait_for_overlay_input, Win32OverlayWaitResult,
};
use super::{ScreenshotInputEvent, Win32OverlayHandle};

#[cfg(target_os = "windows")]
const ACTIVE_PUMP_SLEEP_MS: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32OverlayPumpContract {
    pub owns_thread: bool,
    pub dispatches_input: bool,
    pub blocks_until_terminal: bool,
    pub supports_timeout: bool,
    pub restores_focus_on_exit: bool,
}

impl Win32OverlayPumpContract {
    pub const fn diagnostic_message_pump() -> Self {
        Self {
            owns_thread: false,
            dispatches_input: true,
            blocks_until_terminal: true,
            supports_timeout: true,
            restores_focus_on_exit: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32OverlayPumpOptions {
    pub session_id: String,
    pub generation: u64,
    pub timeout_ms: u64,
    pub allow_repeat_hotkey_cancel: bool,
}

impl Win32OverlayPumpOptions {
    pub fn diagnostic(session_id: impl Into<String>, generation: u64) -> Self {
        Self {
            session_id: session_id.into(),
            generation,
            timeout_ms: 1_500,
            allow_repeat_hotkey_cancel: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32OverlayPumpExitReason {
    Confirm,
    Cancel,
    RepeatHotkey,
    LostFocus,
    Timeout,
    StaleGeneration,
    WindowDestroyed,
    DispatchError,
    UnsupportedPlatform,
    NonTerminalInput,
}

impl Win32OverlayPumpExitReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirm => "confirm",
            Self::Cancel => "cancel",
            Self::RepeatHotkey => "repeat-hotkey",
            Self::LostFocus => "lost-focus",
            Self::Timeout => "timeout",
            Self::StaleGeneration => "stale-generation",
            Self::WindowDestroyed => "window-destroyed",
            Self::DispatchError => "dispatch-error",
            Self::UnsupportedPlatform => "unsupported-platform",
            Self::NonTerminalInput => "non-terminal-input",
        }
    }

    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::NonTerminalInput)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32OverlayPumpDiagnostics {
    pub active: bool,
    pub hwnd: isize,
    pub session_id: String,
    pub generation: u64,
    pub started: bool,
    pub events_dispatched: usize,
    pub last_event: Option<&'static str>,
    pub exit_reason: Win32OverlayPumpExitReason,
    pub error: Option<String>,
}

impl Win32OverlayPumpDiagnostics {
    pub fn unsupported(handle: Win32OverlayHandle, options: Win32OverlayPumpOptions) -> Self {
        Self {
            active: false,
            hwnd: handle.hwnd(),
            session_id: options.session_id,
            generation: options.generation,
            started: false,
            events_dispatched: 0,
            last_event: None,
            exit_reason: Win32OverlayPumpExitReason::UnsupportedPlatform,
            error: Some("Win32 overlay message pump is only available on Windows".into()),
        }
    }

    pub fn from_event(
        handle: Win32OverlayHandle,
        options: Win32OverlayPumpOptions,
        event: ScreenshotInputEvent,
    ) -> Self {
        let exit_reason = classify_pump_event(event, options.allow_repeat_hotkey_cancel);
        Self {
            active: false,
            hwnd: handle.hwnd(),
            session_id: options.session_id,
            generation: options.generation,
            started: true,
            events_dispatched: 1,
            last_event: Some(input_event_label(event)),
            exit_reason,
            error: None,
        }
    }
}

pub const fn win32_overlay_pump_contract() -> Win32OverlayPumpContract {
    Win32OverlayPumpContract::diagnostic_message_pump()
}

pub const fn classify_pump_event(
    event: ScreenshotInputEvent,
    allow_repeat_hotkey_cancel: bool,
) -> Win32OverlayPumpExitReason {
    match event {
        ScreenshotInputEvent::Confirm => Win32OverlayPumpExitReason::Confirm,
        ScreenshotInputEvent::Cancel => Win32OverlayPumpExitReason::Cancel,
        ScreenshotInputEvent::LostFocus => Win32OverlayPumpExitReason::LostFocus,
        ScreenshotInputEvent::RepeatHotkey if allow_repeat_hotkey_cancel => {
            Win32OverlayPumpExitReason::RepeatHotkey
        }
        ScreenshotInputEvent::Key {
            command: KeyCommand::Confirm,
            ..
        } => Win32OverlayPumpExitReason::Confirm,
        ScreenshotInputEvent::Key {
            command: KeyCommand::Cancel,
            ..
        } => Win32OverlayPumpExitReason::Cancel,
        ScreenshotInputEvent::Key {
            command: KeyCommand::RepeatHotkey,
            ..
        } if allow_repeat_hotkey_cancel => Win32OverlayPumpExitReason::RepeatHotkey,
        _ => Win32OverlayPumpExitReason::NonTerminalInput,
    }
}

pub fn run_win32_overlay_diagnostic_pump(
    handle: Win32OverlayHandle,
    options: Win32OverlayPumpOptions,
) -> Win32OverlayPumpDiagnostics {
    #[cfg(target_os = "windows")]
    {
        run_windows_overlay_diagnostic_pump(handle, options)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Win32OverlayPumpDiagnostics::unsupported(handle, options)
    }
}

#[cfg(target_os = "windows")]
fn run_windows_overlay_diagnostic_pump(
    handle: Win32OverlayHandle,
    options: Win32OverlayPumpOptions,
) -> Win32OverlayPumpDiagnostics {
    use std::thread;
    use std::time::{Duration, Instant};

    let hwnd = handle.hwnd();
    let started_at = Instant::now();
    let timeout_ms = options.timeout_ms;
    let mut events_dispatched = 0;
    let mut last_event = None;
    let mut last_error = None;

    loop {
        while let Some(message) = peek_overlay_message(hwnd) {
            if is_overlay_destroy_message(message.message) {
                return finish_windows_pump(
                    handle,
                    options,
                    events_dispatched,
                    last_event,
                    Win32OverlayPumpExitReason::WindowDestroyed,
                    None,
                );
            }

            let diagnostics = run_win32_overlay_message_tuple_diagnostic_pump(
                handle,
                options.clone(),
                message.message,
                message.w_param,
                message.l_param,
                current_win32_key_state(),
            );

            translate_and_dispatch_overlay_message(&message);

            if diagnostics.events_dispatched > 0 {
                events_dispatched += diagnostics.events_dispatched;
                last_event = diagnostics.last_event;
            } else if diagnostics.error.is_some() {
                last_error = diagnostics.error.clone();
            }

            if diagnostics.exit_reason.is_terminal() {
                return finish_windows_pump(
                    handle,
                    options,
                    events_dispatched,
                    last_event,
                    diagnostics.exit_reason,
                    diagnostics.error,
                );
            }
        }

        let elapsed_ms = elapsed_since(started_at);
        if elapsed_ms >= timeout_ms {
            return finish_windows_pump(
                handle,
                options,
                events_dispatched,
                last_event,
                Win32OverlayPumpExitReason::Timeout,
                last_error,
            );
        }

        let wait_ms = bounded_wait_ms(timeout_ms - elapsed_ms);
        let wait_result = wait_for_overlay_input(wait_ms);

        match wait_result {
            Win32OverlayWaitResult::InputAvailable => {
                if !has_pending_overlay_message(hwnd) {
                    thread::sleep(Duration::from_millis(ACTIVE_PUMP_SLEEP_MS));
                }
            }
            Win32OverlayWaitResult::Timeout => {}
            Win32OverlayWaitResult::Failed => {
                return finish_windows_pump(
                    handle,
                    options,
                    events_dispatched,
                    last_event,
                    Win32OverlayPumpExitReason::DispatchError,
                    Some("MsgWaitForMultipleObjectsEx failed in Win32 overlay pump".into()),
                );
            }
            Win32OverlayWaitResult::Unexpected(other) => {
                return finish_windows_pump(
                    handle,
                    options,
                    events_dispatched,
                    last_event,
                    Win32OverlayPumpExitReason::DispatchError,
                    Some(format!(
                        "unexpected Win32 overlay wait result {other:#010x}"
                    )),
                );
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn finish_windows_pump(
    handle: Win32OverlayHandle,
    options: Win32OverlayPumpOptions,
    events_dispatched: usize,
    last_event: Option<&'static str>,
    exit_reason: Win32OverlayPumpExitReason,
    error: Option<String>,
) -> Win32OverlayPumpDiagnostics {
    Win32OverlayPumpDiagnostics {
        active: false,
        hwnd: handle.hwnd(),
        session_id: options.session_id,
        generation: options.generation,
        started: true,
        events_dispatched,
        last_event,
        exit_reason,
        error,
    }
}

#[cfg(target_os = "windows")]
fn elapsed_since(started_at: std::time::Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(target_os = "windows")]
fn bounded_wait_ms(remaining_ms: u64) -> u32 {
    remaining_ms.min(u32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::super::input::InputModifiers;
    use super::super::win32_overlay_dispatch::run_win32_overlay_message_tuple_diagnostic_pump;
    use super::*;

    #[test]
    fn pump_contract_requires_dispatch_and_timeout_without_owning_window_yet() {
        let contract = win32_overlay_pump_contract();
        assert!(!contract.owns_thread);
        assert!(contract.dispatches_input);
        assert!(contract.blocks_until_terminal);
        assert!(contract.supports_timeout);
        assert!(!contract.restores_focus_on_exit);
    }

    #[test]
    fn classifies_terminal_events_and_repeat_hotkey_gate() {
        assert_eq!(
            classify_pump_event(ScreenshotInputEvent::Confirm, true),
            Win32OverlayPumpExitReason::Confirm
        );
        assert_eq!(
            classify_pump_event(ScreenshotInputEvent::Cancel, true),
            Win32OverlayPumpExitReason::Cancel
        );
        assert_eq!(
            classify_pump_event(ScreenshotInputEvent::LostFocus, true),
            Win32OverlayPumpExitReason::LostFocus
        );
        assert_eq!(
            classify_pump_event(ScreenshotInputEvent::RepeatHotkey, true),
            Win32OverlayPumpExitReason::RepeatHotkey
        );
        assert_eq!(
            classify_pump_event(ScreenshotInputEvent::RepeatHotkey, false),
            Win32OverlayPumpExitReason::NonTerminalInput
        );
    }

    #[test]
    fn key_commands_share_terminal_exit_mapping() {
        let modifiers = InputModifiers::none();
        assert_eq!(
            classify_pump_event(
                ScreenshotInputEvent::Key {
                    command: KeyCommand::Confirm,
                    modifiers,
                },
                true,
            ),
            Win32OverlayPumpExitReason::Confirm
        );
        assert_eq!(
            classify_pump_event(
                ScreenshotInputEvent::Key {
                    command: KeyCommand::RepeatHotkey,
                    modifiers,
                },
                false,
            ),
            Win32OverlayPumpExitReason::NonTerminalInput
        );
    }

    #[test]
    fn diagnostics_preserve_handle_and_event_label() {
        let diagnostics = Win32OverlayPumpDiagnostics::from_event(
            Win32OverlayHandle::new(42),
            Win32OverlayPumpOptions::diagnostic("session", 7),
            ScreenshotInputEvent::Cancel,
        );
        assert_eq!(diagnostics.hwnd, 42);
        assert_eq!(diagnostics.session_id, "session");
        assert_eq!(diagnostics.generation, 7);
        assert_eq!(diagnostics.events_dispatched, 1);
        assert_eq!(diagnostics.last_event, Some("cancel"));
        assert_eq!(diagnostics.exit_reason.as_str(), "cancel");
    }

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
}

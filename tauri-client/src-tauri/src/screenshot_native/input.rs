use super::output::SelectionRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl InputModifiers {
    pub const fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }

    pub const fn any(self) -> bool {
        self.shift || self.ctrl || self.alt || self.meta
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Confirm,
    Cancel,
    RepeatHotkey,
    Nudge { dx: i32, dy: i32 },
    Resize { dx: i32, dy: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    Idle,
    Armed { anchor_x: i32, anchor_y: i32 },
    Dragging { rect: SelectionRect },
    Completed { rect: SelectionRect },
    Cancelled,
}

impl DragPhase {
    pub fn selection(self) -> Option<SelectionRect> {
        match self {
            Self::Dragging { rect } | Self::Completed { rect } => Some(rect.normalized()),
            _ => None,
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Armed { .. } | Self::Dragging { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotInputEvent {
    MouseDown {
        x: i32,
        y: i32,
    },
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseUp {
        x: i32,
        y: i32,
    },
    PointerDown {
        x: i32,
        y: i32,
        button: PointerButton,
        modifiers: InputModifiers,
    },
    PointerMove {
        x: i32,
        y: i32,
        modifiers: InputModifiers,
    },
    PointerUp {
        x: i32,
        y: i32,
        button: PointerButton,
        modifiers: InputModifiers,
    },
    Wheel {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
        modifiers: InputModifiers,
    },
    Key {
        command: KeyCommand,
        modifiers: InputModifiers,
    },
    LostFocus,
    Confirm,
    Cancel,
    RepeatHotkey,
}

impl ScreenshotInputEvent {
    pub fn pointer_position(self) -> Option<(i32, i32)> {
        match self {
            Self::MouseDown { x, y }
            | Self::MouseMove { x, y }
            | Self::MouseUp { x, y }
            | Self::PointerDown { x, y, .. }
            | Self::PointerMove { x, y, .. }
            | Self::PointerUp { x, y, .. }
            | Self::Wheel { x, y, .. } => Some((x, y)),
            Self::Key { .. }
            | Self::LostFocus
            | Self::Confirm
            | Self::Cancel
            | Self::RepeatHotkey => None,
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Confirm | Self::Cancel | Self::LostFocus)
    }

    pub const fn key_command(self) -> Option<KeyCommand> {
        match self {
            Self::Confirm => Some(KeyCommand::Confirm),
            Self::Cancel => Some(KeyCommand::Cancel),
            Self::RepeatHotkey => Some(KeyCommand::RepeatHotkey),
            Self::Key { command, .. } => Some(command),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputContract {
    pub allow_repeat_hotkey_cancel: bool,
    pub allow_keyboard_adjustment: bool,
    pub minimum_drag_pixels: u32,
}

impl InputContract {
    pub const fn screenshot_selection() -> Self {
        Self {
            allow_repeat_hotkey_cancel: true,
            allow_keyboard_adjustment: true,
            minimum_drag_pixels: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_selection_lifecycle_events() {
        assert!(ScreenshotInputEvent::Confirm.is_terminal());
        assert!(ScreenshotInputEvent::Cancel.is_terminal());
        assert!(ScreenshotInputEvent::LostFocus.is_terminal());
        assert!(!ScreenshotInputEvent::RepeatHotkey.is_terminal());
    }

    #[test]
    fn exposes_key_commands_for_terminal_and_repeat_events() {
        assert_eq!(
            ScreenshotInputEvent::Confirm.key_command(),
            Some(KeyCommand::Confirm)
        );
        assert_eq!(
            ScreenshotInputEvent::Cancel.key_command(),
            Some(KeyCommand::Cancel)
        );
        assert_eq!(
            ScreenshotInputEvent::RepeatHotkey.key_command(),
            Some(KeyCommand::RepeatHotkey)
        );
    }

    #[test]
    fn screenshot_selection_contract_keeps_repeat_and_keyboard_ready() {
        let contract = InputContract::screenshot_selection();

        assert!(contract.allow_repeat_hotkey_cancel);
        assert!(contract.allow_keyboard_adjustment);
        assert_eq!(contract.minimum_drag_pixels, 2);
    }
}

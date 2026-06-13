use super::input::{InputModifiers, KeyCommand, PointerButton, ScreenshotInputEvent};

pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_XBUTTONUP: u32 = 0x020C;
pub const WM_MOUSEHWHEEL: u32 = 0x020E;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_CANCELMODE: u32 = 0x001F;
pub const WM_KILLFOCUS: u32 = 0x0008;
pub const WM_CAPTURECHANGED: u32 = 0x0215;

pub const MK_LBUTTON: u16 = 0x0001;
pub const MK_RBUTTON: u16 = 0x0002;
pub const MK_SHIFT: u16 = 0x0004;
pub const MK_CONTROL: u16 = 0x0008;
pub const MK_MBUTTON: u16 = 0x0010;
pub const MK_XBUTTON1: u16 = 0x0020;
pub const MK_XBUTTON2: u16 = 0x0040;

pub const XBUTTON1: u16 = 0x0001;
pub const XBUTTON2: u16 = 0x0002;

pub const VK_RETURN: u16 = 0x0D;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;
pub const VK_SNAPSHOT: u16 = 0x2C;
pub const VK_A: u16 = 0x41;
pub const VK_C: u16 = 0x43;
pub const VK_S: u16 = 0x53;
pub const VK_T: u16 = 0x54;

pub const WHEEL_DELTA: i32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32KeyState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Win32KeyState {
    pub const fn empty() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }

    pub const fn to_modifiers(self) -> InputModifiers {
        InputModifiers {
            shift: self.shift,
            ctrl: self.ctrl,
            alt: self.alt,
            meta: self.meta,
        }
    }
}

pub fn screenshot_input_event_from_win32_message(
    message: u32,
    wparam: usize,
    lparam: isize,
    key_state: Win32KeyState,
) -> Option<ScreenshotInputEvent> {
    let (x, y) = lparam_point(lparam);
    let modifiers = modifiers_from_mouse_wparam(wparam, key_state);

    match message {
        WM_LBUTTONDOWN => Some(ScreenshotInputEvent::PointerDown {
            x,
            y,
            button: PointerButton::Primary,
            modifiers,
        }),
        WM_RBUTTONDOWN => Some(ScreenshotInputEvent::PointerDown {
            x,
            y,
            button: PointerButton::Secondary,
            modifiers,
        }),
        WM_MBUTTONDOWN => Some(ScreenshotInputEvent::PointerDown {
            x,
            y,
            button: PointerButton::Middle,
            modifiers,
        }),
        WM_XBUTTONDOWN => Some(ScreenshotInputEvent::PointerDown {
            x,
            y,
            button: PointerButton::Other(xbutton_from_wparam(wparam)),
            modifiers,
        }),
        WM_MOUSEMOVE => Some(ScreenshotInputEvent::PointerMove { x, y, modifiers }),
        WM_LBUTTONUP => Some(ScreenshotInputEvent::PointerUp {
            x,
            y,
            button: PointerButton::Primary,
            modifiers,
        }),
        WM_RBUTTONUP => Some(ScreenshotInputEvent::PointerUp {
            x,
            y,
            button: PointerButton::Secondary,
            modifiers,
        }),
        WM_MBUTTONUP => Some(ScreenshotInputEvent::PointerUp {
            x,
            y,
            button: PointerButton::Middle,
            modifiers,
        }),
        WM_XBUTTONUP => Some(ScreenshotInputEvent::PointerUp {
            x,
            y,
            button: PointerButton::Other(xbutton_from_wparam(wparam)),
            modifiers,
        }),
        WM_MOUSEWHEEL => Some(ScreenshotInputEvent::Wheel {
            x,
            y,
            delta_x: 0,
            delta_y: wheel_delta_from_wparam(wparam),
            modifiers,
        }),
        WM_MOUSEHWHEEL => Some(ScreenshotInputEvent::Wheel {
            x,
            y,
            delta_x: wheel_delta_from_wparam(wparam),
            delta_y: 0,
            modifiers,
        }),
        WM_KEYDOWN | WM_SYSKEYDOWN => key_event_from_virtual_key(low_word(wparam), key_state),
        WM_KILLFOCUS | WM_CANCELMODE | WM_CAPTURECHANGED => Some(ScreenshotInputEvent::LostFocus),
        _ => None,
    }
}

pub fn lparam_point(lparam: isize) -> (i32, i32) {
    (lparam_x(lparam), lparam_y(lparam))
}

pub fn lparam_x(lparam: isize) -> i32 {
    sign_extend_word(low_word(lparam as usize))
}

pub fn lparam_y(lparam: isize) -> i32 {
    sign_extend_word(high_word(lparam as usize))
}

pub fn wheel_delta_from_wparam(wparam: usize) -> i32 {
    sign_extend_word(high_word(wparam))
}

pub fn xbutton_from_wparam(wparam: usize) -> u16 {
    match high_word(wparam) {
        XBUTTON1 => 1,
        XBUTTON2 => 2,
        other => other,
    }
}

pub fn modifiers_from_mouse_wparam(wparam: usize, key_state: Win32KeyState) -> InputModifiers {
    let mouse_state = low_word(wparam);

    InputModifiers {
        shift: key_state.shift || has_mouse_key(mouse_state, MK_SHIFT),
        ctrl: key_state.ctrl || has_mouse_key(mouse_state, MK_CONTROL),
        alt: key_state.alt,
        meta: key_state.meta,
    }
}

pub fn key_event_from_virtual_key(
    virtual_key: u16,
    key_state: Win32KeyState,
) -> Option<ScreenshotInputEvent> {
    let modifiers = key_state.to_modifiers();
    let command = match virtual_key {
        VK_RETURN => KeyCommand::Confirm,
        VK_ESCAPE => KeyCommand::Cancel,
        VK_SNAPSHOT => KeyCommand::RepeatHotkey,
        VK_LEFT => arrow_key_command(-1, 0, modifiers),
        VK_RIGHT => arrow_key_command(1, 0, modifiers),
        VK_UP => arrow_key_command(0, -1, modifiers),
        VK_DOWN => arrow_key_command(0, 1, modifiers),
        _ => return None,
    };

    Some(match command {
        KeyCommand::Confirm => ScreenshotInputEvent::Confirm,
        KeyCommand::Cancel => ScreenshotInputEvent::Cancel,
        KeyCommand::RepeatHotkey => ScreenshotInputEvent::RepeatHotkey,
        command => ScreenshotInputEvent::Key { command, modifiers },
    })
}

pub const fn virtual_key_to_key_state(virtual_key: u16) -> Option<KeyStateField> {
    match virtual_key {
        VK_SHIFT => Some(KeyStateField::Shift),
        VK_CONTROL => Some(KeyStateField::Ctrl),
        VK_MENU => Some(KeyStateField::Alt),
        VK_LWIN | VK_RWIN => Some(KeyStateField::Meta),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStateField {
    Shift,
    Ctrl,
    Alt,
    Meta,
}

pub const fn low_word(value: usize) -> u16 {
    (value & 0xFFFF) as u16
}

pub const fn high_word(value: usize) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

pub const fn has_mouse_key(mouse_state: u16, flag: u16) -> bool {
    mouse_state & flag != 0
}

const fn sign_extend_word(value: u16) -> i32 {
    value as i16 as i32
}

const fn arrow_key_command(dx: i32, dy: i32, modifiers: InputModifiers) -> KeyCommand {
    if modifiers.shift {
        KeyCommand::Resize { dx, dy }
    } else {
        KeyCommand::Nudge { dx, dy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_signed_lparam_coordinates() {
        let lparam = ((20_u32 << 16) | 0xFFF6) as isize;

        assert_eq!(lparam_point(lparam), (-10, 20));
    }

    #[test]
    fn maps_primary_pointer_down_with_modifiers() {
        let event = screenshot_input_event_from_win32_message(
            WM_LBUTTONDOWN,
            MK_CONTROL as usize,
            ((7_u32 << 16) | 5) as isize,
            Win32KeyState {
                shift: true,
                ctrl: false,
                alt: true,
                meta: false,
            },
        );

        assert_eq!(
            event,
            Some(ScreenshotInputEvent::PointerDown {
                x: 5,
                y: 7,
                button: PointerButton::Primary,
                modifiers: InputModifiers {
                    shift: true,
                    ctrl: true,
                    alt: true,
                    meta: false,
                },
            })
        );
    }

    #[test]
    fn maps_shift_arrow_to_resize() {
        let event = key_event_from_virtual_key(
            VK_RIGHT,
            Win32KeyState {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            },
        );

        assert_eq!(
            event,
            Some(ScreenshotInputEvent::Key {
                command: KeyCommand::Resize { dx: 1, dy: 0 },
                modifiers: InputModifiers {
                    shift: true,
                    ctrl: false,
                    alt: false,
                    meta: false,
                },
            })
        );
    }

    #[test]
    fn maps_selection_lifecycle_keys_to_terminal_events() {
        assert_eq!(
            key_event_from_virtual_key(VK_RETURN, Win32KeyState::empty()),
            Some(ScreenshotInputEvent::Confirm)
        );
        assert_eq!(
            key_event_from_virtual_key(VK_ESCAPE, Win32KeyState::empty()),
            Some(ScreenshotInputEvent::Cancel)
        );
    }

    #[test]
    fn maps_repeat_hotkey_to_non_terminal_cancel_event() {
        let event = key_event_from_virtual_key(VK_SNAPSHOT, Win32KeyState::empty());

        assert_eq!(event, Some(ScreenshotInputEvent::RepeatHotkey));
        assert_eq!(
            event.and_then(ScreenshotInputEvent::key_command),
            Some(KeyCommand::RepeatHotkey)
        );
        assert!(!event.unwrap().is_terminal());
    }

    #[test]
    fn leaves_app_level_shortcuts_unclaimed_by_native_selection_input() {
        let ctrl = Win32KeyState {
            shift: false,
            ctrl: true,
            alt: false,
            meta: false,
        };
        let alt = Win32KeyState {
            shift: false,
            ctrl: false,
            alt: true,
            meta: false,
        };

        assert_eq!(key_event_from_virtual_key(VK_C, ctrl), None);
        assert_eq!(key_event_from_virtual_key(VK_S, ctrl), None);
        assert_eq!(key_event_from_virtual_key(VK_A, alt), None);
        assert_eq!(key_event_from_virtual_key(VK_T, alt), None);
    }

    #[test]
    fn maps_wheel_delta_from_high_word() {
        let wparam = (0xFF88_u32 << 16) as usize;

        assert_eq!(wheel_delta_from_wparam(wparam), -WHEEL_DELTA);
    }
}

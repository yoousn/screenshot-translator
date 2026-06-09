use super::input::{
    DragPhase, InputContract, InputModifiers, KeyCommand, PointerButton, ScreenshotInputEvent,
};
use super::output::SelectionRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionTransition {
    Ignored,
    Armed { anchor_x: i32, anchor_y: i32 },
    Updated { rect: SelectionRect },
    Completed { rect: SelectionRect },
    Cancelled,
}

impl SelectionTransition {
    pub fn selection(self) -> Option<SelectionRect> {
        match self {
            Self::Updated { rect } | Self::Completed { rect } => Some(rect.normalized()),
            Self::Ignored | Self::Armed { .. } | Self::Cancelled => None,
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStateStatus {
    Idle,
    Selecting,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionState {
    contract: InputContract,
    phase: DragPhase,
    last_pointer: Option<(i32, i32)>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionState {
    pub fn new() -> Self {
        Self::with_contract(InputContract::screenshot_selection())
    }

    pub const fn with_contract(contract: InputContract) -> Self {
        Self {
            contract,
            phase: DragPhase::Idle,
            last_pointer: None,
        }
    }

    pub const fn contract(self) -> InputContract {
        self.contract
    }

    pub const fn phase(self) -> DragPhase {
        self.phase
    }

    pub fn status(self) -> SelectionStateStatus {
        match self.phase {
            DragPhase::Idle => SelectionStateStatus::Idle,
            DragPhase::Armed { .. } | DragPhase::Dragging { .. } => SelectionStateStatus::Selecting,
            DragPhase::Completed { .. } => SelectionStateStatus::Completed,
            DragPhase::Cancelled => SelectionStateStatus::Cancelled,
        }
    }

    pub fn selection(self) -> Option<SelectionRect> {
        self.phase.selection()
    }

    pub fn completed_selection(self) -> Option<SelectionRect> {
        match self.phase {
            DragPhase::Completed { rect } => Some(rect.normalized()),
            DragPhase::Idle
            | DragPhase::Armed { .. }
            | DragPhase::Dragging { .. }
            | DragPhase::Cancelled => None,
        }
    }

    pub fn is_active(self) -> bool {
        self.phase.is_active()
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self.phase,
            DragPhase::Completed { .. } | DragPhase::Cancelled
        )
    }

    pub fn reset(&mut self) {
        self.phase = DragPhase::Idle;
        self.last_pointer = None;
    }

    pub fn handle_event(&mut self, event: ScreenshotInputEvent) -> SelectionTransition {
        if let Some(pointer) = event.pointer_position() {
            self.last_pointer = Some(pointer);
        }

        match event {
            ScreenshotInputEvent::MouseDown { x, y } => self.begin_drag(x, y),
            ScreenshotInputEvent::MouseMove { x, y } => self.update_drag(x, y),
            ScreenshotInputEvent::MouseUp { x, y } => self.end_drag(x, y),
            ScreenshotInputEvent::PointerDown { x, y, button, .. } => {
                self.handle_pointer_down(x, y, button)
            }
            ScreenshotInputEvent::PointerMove { x, y, .. } => self.update_drag(x, y),
            ScreenshotInputEvent::PointerUp { x, y, button, .. } => {
                self.handle_pointer_up(x, y, button)
            }
            ScreenshotInputEvent::Key { command, modifiers } => {
                self.handle_key_command(command, modifiers)
            }
            ScreenshotInputEvent::Confirm => self.confirm(),
            ScreenshotInputEvent::Cancel | ScreenshotInputEvent::LostFocus => self.cancel(),
            ScreenshotInputEvent::RepeatHotkey if self.contract.allow_repeat_hotkey_cancel => {
                self.cancel()
            }
            ScreenshotInputEvent::RepeatHotkey | ScreenshotInputEvent::Wheel { .. } => {
                SelectionTransition::Ignored
            }
        }
    }

    pub fn confirm(&mut self) -> SelectionTransition {
        match self.phase.selection() {
            Some(rect) if rect.is_valid() => {
                self.phase = DragPhase::Completed { rect };
                SelectionTransition::Completed { rect }
            }
            _ => SelectionTransition::Ignored,
        }
    }

    pub fn cancel(&mut self) -> SelectionTransition {
        self.phase = DragPhase::Cancelled;
        SelectionTransition::Cancelled
    }

    fn handle_pointer_down(
        &mut self,
        x: i32,
        y: i32,
        button: PointerButton,
    ) -> SelectionTransition {
        if button == PointerButton::Primary {
            self.begin_drag(x, y)
        } else {
            SelectionTransition::Ignored
        }
    }

    fn handle_pointer_up(&mut self, x: i32, y: i32, button: PointerButton) -> SelectionTransition {
        if button == PointerButton::Primary {
            self.end_drag(x, y)
        } else {
            SelectionTransition::Ignored
        }
    }

    fn handle_key_command(
        &mut self,
        command: KeyCommand,
        modifiers: InputModifiers,
    ) -> SelectionTransition {
        match command {
            KeyCommand::Confirm => self.confirm(),
            KeyCommand::Cancel => self.cancel(),
            KeyCommand::RepeatHotkey if self.contract.allow_repeat_hotkey_cancel => self.cancel(),
            KeyCommand::Nudge { dx, dy }
                if self.contract.allow_keyboard_adjustment && !modifiers.any() =>
            {
                self.adjust_selection(dx, dy, false)
            }
            KeyCommand::Resize { dx, dy }
                if self.contract.allow_keyboard_adjustment && !modifiers.any() =>
            {
                self.adjust_selection(dx, dy, true)
            }
            KeyCommand::RepeatHotkey | KeyCommand::Nudge { .. } | KeyCommand::Resize { .. } => {
                SelectionTransition::Ignored
            }
        }
    }

    fn begin_drag(&mut self, x: i32, y: i32) -> SelectionTransition {
        self.phase = DragPhase::Armed {
            anchor_x: x,
            anchor_y: y,
        };
        SelectionTransition::Armed {
            anchor_x: x,
            anchor_y: y,
        }
    }

    fn update_drag(&mut self, x: i32, y: i32) -> SelectionTransition {
        match self.phase {
            DragPhase::Armed { anchor_x, anchor_y } => {
                if !drag_exceeds_threshold(
                    anchor_x,
                    anchor_y,
                    x,
                    y,
                    self.contract.minimum_drag_pixels,
                ) {
                    return SelectionTransition::Ignored;
                }

                let rect = rect_from_points(anchor_x, anchor_y, x, y);
                self.phase = DragPhase::Dragging { rect };
                SelectionTransition::Updated {
                    rect: rect.normalized(),
                }
            }
            DragPhase::Dragging { rect } => {
                let anchor_x = rect.x;
                let anchor_y = rect.y;
                let rect = rect_from_points(anchor_x, anchor_y, x, y);
                self.phase = DragPhase::Dragging { rect };
                SelectionTransition::Updated {
                    rect: rect.normalized(),
                }
            }
            DragPhase::Idle | DragPhase::Completed { .. } | DragPhase::Cancelled => {
                SelectionTransition::Ignored
            }
        }
    }

    fn end_drag(&mut self, x: i32, y: i32) -> SelectionTransition {
        match self.phase {
            DragPhase::Armed { anchor_x, anchor_y } => {
                if !drag_exceeds_threshold(
                    anchor_x,
                    anchor_y,
                    x,
                    y,
                    self.contract.minimum_drag_pixels,
                ) {
                    self.phase = DragPhase::Idle;
                    return SelectionTransition::Ignored;
                }

                self.complete_rect(rect_from_points(anchor_x, anchor_y, x, y))
            }
            DragPhase::Dragging { rect } => {
                self.complete_rect(rect_from_points(rect.x, rect.y, x, y))
            }
            DragPhase::Idle | DragPhase::Completed { .. } | DragPhase::Cancelled => {
                SelectionTransition::Ignored
            }
        }
    }

    fn complete_rect(&mut self, rect: SelectionRect) -> SelectionTransition {
        let rect = rect.normalized();
        if rect.is_valid() {
            self.phase = DragPhase::Completed { rect };
            SelectionTransition::Completed { rect }
        } else {
            self.phase = DragPhase::Idle;
            SelectionTransition::Ignored
        }
    }

    fn adjust_selection(&mut self, dx: i32, dy: i32, resize: bool) -> SelectionTransition {
        let Some(rect) = self.phase.selection() else {
            return SelectionTransition::Ignored;
        };

        let next = if resize {
            SelectionRect::new(
                rect.x,
                rect.y,
                rect.width.saturating_add(dx),
                rect.height.saturating_add(dy),
            )
        } else {
            SelectionRect::new(
                rect.x.saturating_add(dx),
                rect.y.saturating_add(dy),
                rect.width,
                rect.height,
            )
        }
        .normalized();

        if !next.is_valid() {
            return SelectionTransition::Ignored;
        }

        self.phase = match self.phase {
            DragPhase::Completed { .. } => DragPhase::Completed { rect: next },
            DragPhase::Dragging { .. } => DragPhase::Dragging { rect: next },
            DragPhase::Idle | DragPhase::Armed { .. } | DragPhase::Cancelled => self.phase,
        };

        SelectionTransition::Updated { rect: next }
    }
}

fn rect_from_points(anchor_x: i32, anchor_y: i32, x: i32, y: i32) -> SelectionRect {
    SelectionRect::new(
        anchor_x,
        anchor_y,
        x.saturating_sub(anchor_x),
        y.saturating_sub(anchor_y),
    )
}

fn drag_exceeds_threshold(anchor_x: i32, anchor_y: i32, x: i32, y: i32, threshold: u32) -> bool {
    let dx = i64::from(x).saturating_sub(i64::from(anchor_x)).abs();
    let dy = i64::from(y).saturating_sub(i64::from(anchor_y)).abs();

    dx.max(dy) >= i64::from(threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_completes_normalized_selection() {
        let mut state = SelectionState::new();

        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseDown { x: 30, y: 40 }),
            SelectionTransition::Armed {
                anchor_x: 30,
                anchor_y: 40,
            }
        );
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseMove { x: 10, y: 15 }),
            SelectionTransition::Updated {
                rect: SelectionRect::new(10, 15, 20, 25),
            }
        );
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseUp { x: 10, y: 15 }),
            SelectionTransition::Completed {
                rect: SelectionRect::new(10, 15, 20, 25),
            }
        );
        assert_eq!(
            state.completed_selection(),
            Some(SelectionRect::new(10, 15, 20, 25))
        );
    }

    #[test]
    fn ignores_clicks_under_drag_threshold() {
        let mut state = SelectionState::new();

        state.handle_event(ScreenshotInputEvent::MouseDown { x: 10, y: 10 });
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseMove { x: 11, y: 11 }),
            SelectionTransition::Ignored
        );
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseUp { x: 11, y: 11 }),
            SelectionTransition::Ignored
        );
        assert_eq!(state.status(), SelectionStateStatus::Idle);
    }

    #[test]
    fn confirm_completes_current_drag() {
        let mut state = SelectionState::new();

        state.handle_event(ScreenshotInputEvent::MouseDown { x: 5, y: 5 });
        state.handle_event(ScreenshotInputEvent::MouseMove { x: 20, y: 25 });

        assert_eq!(
            state.handle_event(ScreenshotInputEvent::Confirm),
            SelectionTransition::Completed {
                rect: SelectionRect::new(5, 5, 15, 20),
            }
        );
        assert_eq!(state.status(), SelectionStateStatus::Completed);
    }

    #[test]
    fn escape_cancels_selection() {
        let mut state = SelectionState::new();

        state.handle_event(ScreenshotInputEvent::MouseDown { x: 5, y: 5 });
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::Key {
                command: KeyCommand::Cancel,
                modifiers: InputModifiers::none(),
            }),
            SelectionTransition::Cancelled
        );
        assert_eq!(state.status(), SelectionStateStatus::Cancelled);
    }

    #[test]
    fn repeat_hotkey_cleans_active_selection_and_allows_fresh_drag() {
        let mut state = SelectionState::new();

        state.handle_event(ScreenshotInputEvent::MouseDown { x: -120, y: 80 });
        state.handle_event(ScreenshotInputEvent::MouseMove { x: 320, y: 260 });

        assert_eq!(
            state.handle_event(ScreenshotInputEvent::RepeatHotkey),
            SelectionTransition::Cancelled
        );
        assert_eq!(state.status(), SelectionStateStatus::Cancelled);
        assert_eq!(state.selection(), None);

        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseDown { x: 10, y: 10 }),
            SelectionTransition::Armed {
                anchor_x: 10,
                anchor_y: 10,
            }
        );
        assert_eq!(
            state.handle_event(ScreenshotInputEvent::MouseUp { x: 40, y: 35 }),
            SelectionTransition::Completed {
                rect: SelectionRect::new(10, 10, 30, 25),
            }
        );
    }

    #[test]
    fn repeat_hotkey_contract_can_leave_selection_active() {
        let mut state = SelectionState::with_contract(InputContract {
            allow_repeat_hotkey_cancel: false,
            allow_keyboard_adjustment: true,
            minimum_drag_pixels: 2,
        });

        state.handle_event(ScreenshotInputEvent::MouseDown { x: 5, y: 5 });
        state.handle_event(ScreenshotInputEvent::MouseMove { x: 30, y: 25 });

        assert_eq!(
            state.handle_event(ScreenshotInputEvent::Key {
                command: KeyCommand::RepeatHotkey,
                modifiers: InputModifiers::none(),
            }),
            SelectionTransition::Ignored
        );
        assert_eq!(state.status(), SelectionStateStatus::Selecting);
        assert_eq!(state.selection(), Some(SelectionRect::new(5, 5, 25, 20)));
    }
}

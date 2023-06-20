use core::{cmp::Ordering, num::Wrapping};

use crate::data::INPUT;

/// Describes the input button that was pressed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inputs {
    pub up: StateTracker,
    pub down: StateTracker,
    /// Doubles as Back or Cancel
    pub pos1: StateTracker,
    /// Doubles as Ok or Store
    pub pos2: StateTracker,
}

impl Inputs {
    pub const fn new() -> Self {
        Self {
            up: StateTracker::new(),
            down: StateTracker::new(),
            pos1: StateTracker::new(),
            pos2: StateTracker::new(),
        }
    }
    pub fn changed_since(&self, other: &Self) -> StateChanges {
        StateChanges {
            up: self.up.changed_since(&other.up),
            down: self.down.changed_since(&other.down),
            pos1: self.pos1.changed_since(&other.pos1),
            pos2: self.pos2.changed_since(&other.pos2),
        }
    }

    async fn wait_for_change<F, T>(&mut self, mut check: F) -> T
    where
        F: FnMut(StateChanges) -> Option<T>,
    {
        loop {
            let input = INPUT.lock().await.clone();
            let changes = input.changed_since(self);
            *self = input;
            match check(changes) {
                Some(res) => break res,
                None => embassy_futures::yield_now().await,
            }
        }
    }

    pub async fn wait_for_press(&mut self) -> Button {
        self.wait_for_change(|changes| changes.pressed_exclusive())
            .await
    }

    pub async fn wait_for_release(&mut self, button: Button) {
        self.wait_for_change(|changes| changes.released(button).then_some(()))
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateTracker {
    changes: Wrapping<u8>,
}

impl StateTracker {
    pub const fn new() -> Self {
        Self {
            changes: Wrapping(0),
        }
    }

    fn is_pressed(&self) -> bool {
        self.changes.0 % 2 == 1
    }

    pub fn press(&mut self) {
        if !self.is_pressed() {
            self.changes += 1;
        }
    }

    pub fn release(&mut self) {
        if self.is_pressed() {
            self.changes += 1;
        }
    }

    pub fn changed_since(&self, other: &Self) -> StateChange {
        match self.changes.cmp(&other.changes) {
            Ordering::Equal => StateChange {
                state: if self.is_pressed() {
                    State::StillPressed
                } else {
                    State::StillReleased
                },
                missed_updates: false,
            },
            ordering @ (Ordering::Greater | Ordering::Less) => {
                let state = match (other.is_pressed(), self.is_pressed()) {
                    (false, false) => State::StillReleased,
                    (true, true) => State::StillPressed,
                    (true, false) => State::Released,
                    (false, true) => State::Pressed,
                };

                StateChange {
                    state,
                    missed_updates: ordering == Ordering::Less
                        || other.changes + Wrapping(1) != self.changes,
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Pressed,
    Released,
    StillPressed,
    StillReleased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateChange {
    state: State,
    missed_updates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateChanges {
    pub up: StateChange,
    pub down: StateChange,
    pub pos1: StateChange,
    pub pos2: StateChange,
}

impl StateChanges {
    pub fn pressed_exclusive(&self) -> Option<Button> {
        let map = [
            (Button::Up, self.up),
            (Button::Down, self.down),
            (Button::Pos1, self.pos1),
            (Button::Pos2, self.pos2),
        ];

        let mut pressed = None;

        for (button, state) in map {
            match state.state {
                State::Pressed if pressed.is_none() => pressed = Some(button),
                State::StillReleased => {}
                _ => return None,
            }
        }

        pressed
    }

    pub fn released(&self, button: Button) -> bool {
        let button = match button {
            Button::Up => self.up,
            Button::Down => self.down,
            Button::Pos1 => self.pos1,
            Button::Pos2 => self.pos2,
        };

        matches!(button.state, State::Released | State::StillReleased)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Up,
    Down,
    Pos1,
    Pos2,
}

use bitflags::bitflags;
use embassy_time::{Duration, Timer};

use crate::data::INPUT;

/// Describes the input button that was pressed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inputs {
    pub up: State,
    pub down: State,
    /// Doubles as Back or Cancel
    pub pos1: State,
    /// Doubles as Ok or Store
    pub pos2: State,
}

impl Inputs {
    pub const fn new() -> Self {
        Self {
            up: State::Inactive,
            down: State::Inactive,
            pos1: State::Inactive,
            pos2: State::Inactive,
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

    async fn is_unchanged_after<F, T>(
        &self,
        expected: &T,
        wait_time: Duration,
        mut check: F,
    ) -> bool
    where
        F: FnMut(StateChanges) -> Option<T>,
        T: PartialEq,
        T: core::fmt::Debug,
    {
        Timer::after(wait_time).await;
        let input = INPUT.lock().await.clone();
        let changes = input.changed_since(self);
        let new = check(changes);

        Some(expected) == new.as_ref()
    }

    pub async fn wait_for_press(&mut self) -> Button {
        loop {
            let button = self.wait_for_change(StateChanges::pressed).await;
            log::trace!("registered button event: {button:?}");
            let unchanged = self
                .is_unchanged_after(&button, Duration::from_millis(80), StateChanges::pressed)
                .await;
            if unchanged {
                log::debug!("detected button press: {button:?}");
                break button;
            } else {
                log::trace!("button changed before timeout. Ignoring event.");
            }
        }
    }

    pub async fn wait_for_single_press(&mut self) -> Button {
        let button = self.wait_for_change(StateChanges::pressed).await;
        log::debug!("detected button press: {button:?}");
        button
    }

    pub async fn wait_all_released(&mut self) {
        log::trace!("waiting for release of all buttons");
        self.wait_for_release(Button::all()).await;
        log::trace!("all buttons released");
    }

    pub async fn wait_for_release(&mut self, button: Button) {
        self.wait_for_change(|changes| changes.released().contains(button).then_some(()))
            .await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum State {
    Active,
    #[default]
    Inactive,
}

impl State {
    pub fn press(&mut self) {
        *self = Self::Active;
    }

    pub fn release(&mut self) {
        *self = Self::Inactive;
    }

    pub fn changed_since(&self, other: &Self) -> StateChange {
        use State::*;
        match (other, self) {
            (Inactive, Inactive) => StateChange::StillReleased,
            (Active, Active) => StateChange::StillPressed,
            (Active, Inactive) => StateChange::Released,
            (Inactive, Active) => StateChange::Pressed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateChange {
    Pressed,
    Released,
    StillPressed,
    StillReleased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateChanges {
    pub up: StateChange,
    pub down: StateChange,
    pub pos1: StateChange,
    pub pos2: StateChange,
}

impl StateChanges {
    fn button_map(self) -> impl Iterator<Item = (Button, StateChange)> {
        [
            (Button::Up, self.up),
            (Button::Down, self.down),
            (Button::Pos1, self.pos1),
            (Button::Pos2, self.pos2),
        ]
        .into_iter()
    }

    pub fn pressed(self) -> Option<Button> {
        let button: Button = self
            .button_map()
            .filter_map(|(btn, state)| {
                matches!(state, StateChange::Pressed | StateChange::StillPressed).then_some(btn)
            })
            .collect();

        (!button.is_empty()).then_some(button)
    }

    pub fn released(self) -> Button {
        self.button_map()
            .filter_map(|(btn, state)| {
                matches!(state, StateChange::Released | StateChange::StillReleased).then_some(btn)
            })
            .collect()
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Button: u8 {
        const Up = 0b0001;
        const Down = 0b0010;
        const Pos1 = 0b0100;
        const Pos2 = 0b1000;

        const UpAndDown = Self::Up.bits() | Self::Down.bits();
    }
}

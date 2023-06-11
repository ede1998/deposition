use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

use self::start::Start;

mod start;

#[derive(Debug)]
pub enum Menu {
    Start(Start),
    Options(Options),
}

impl Default for Menu {
    fn default() -> Self {
        Self::Start(Start::default())
    }
}

impl Menu {
    pub async fn update(&mut self, input: Option<Input>) {
        let res = match self {
            Self::Start(start) => start.update(input).await,
            _ => unimplemented!(),
        };
        if let Some(res) = res {
            *self = res;
        }
    }

    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        match self {
            Menu::Start(start) => start.display(display).await,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum Options {
    Calibration(Calibration),
    StoreHeight1,
    StoreHeight2,
    ResetDrive,
}

pub enum ResetDrive {
    Selected,
    Running,
    Finished,
}

#[derive(Debug)]
pub struct Calibration {
    // List height points
}

pub struct CalibrationHeight {
    // current
}

/// Describes the input button that was pressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Button {
    Up,
    Down,
    /// Doubles as Back or Cancel
    Height1,
    /// Doubles as Ok or Store
    Height2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Pressed,
    Held,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    button: Button,
    action: Action,
}

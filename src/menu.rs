use core::{cmp::Ordering, num::Wrapping};

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

pub enum Menu {
    Options(Options),
}

impl Default for Menu {
    fn default() -> Self {
        Self::Start(Start::default())
    }
}

impl Menu {
    pub async fn update(&mut self) {
        let res = match self {
            Self::Start(start) => start.update().await,
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
            Self::Start(start) => start.display(display).await,
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

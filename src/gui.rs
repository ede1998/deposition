use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

mod calibration;
mod options;
mod start;
mod widgets;

pub use calibration::{CalibrationOptions, CalibrationMenu, Selected};
pub use options::{OptionItem, Options, ResetDrive};
pub use start::Start;
pub use widgets::{Menu, MenuContent};

pub enum MainMenu {
    Start(Start),
    Options(Options),
    ResetDrive(ResetDrive),
    Calibration(CalibrationOptions),
}

impl MainMenu {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        match self {
            MainMenu::Start(start) => start.display(display).await,
            MainMenu::Options(options) => options.display(display).await,
            MainMenu::ResetDrive(reset_drive) => reset_drive.display(display).await,
            MainMenu::Calibration(calibration) => calibration.display(display).await,
        }
    }
}

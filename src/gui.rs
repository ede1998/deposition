use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

mod calibration;
mod calibration_point;
mod options;
mod start;
mod widgets;

pub use calibration::{CalibrationMenu, CalibrationOptions, Selected};
pub use calibration_point::CalibrationPoint;
pub use options::{OptionItem, Options, ResetDrive};
pub use start::Start;
pub use widgets::{Menu, MenuContent};

pub enum MainMenu {
    Start(Start),
    Options(Options),
    ResetDrive(ResetDrive),
    Calibration(CalibrationOptions),
    CalibrationPoint(CalibrationPoint),
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
            MainMenu::CalibrationPoint(point) => point.display(display).await,
        }
    }
}

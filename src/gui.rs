use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

mod options;
mod start;
mod widgets;

pub use options::{OptionItem, Options, ResetDrive};
pub use start::Start;
pub use widgets::{Menu, MenuItem};

pub enum MainMenu {
    Start(Start),
    Options(Options),
    ResetDrive(ResetDrive),
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
        }
    }
}

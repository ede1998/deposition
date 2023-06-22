use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

mod options;
mod start;

pub use options::Options;
pub use start::Start;

pub enum Menu {
    Start(Start),
    Options(Options),
}

impl Menu {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        match self {
            Menu::Start(start) => start.display(display).await,
            Menu::Options(options) => options.display(display).await,
        }
    }
}

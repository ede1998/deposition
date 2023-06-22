use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

mod start;
pub use start::Start;

pub enum Menu {
    Start(Start),
}

impl Menu {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        match self {
            Menu::Start(start) => start.display(display).await,
        }
    }
}

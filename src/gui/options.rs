use core::fmt::Write;

use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{
        ascii::{FONT_10X20, FONT_7X13},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};
use heapless::String;

pub struct Options {
    selected: MenuItem,
}

impl Options {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        footer(display).await?;
        Ok(())
    }

    async fn menu<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();

        let build_str = || {
            let mut string = String::<50>::new();
            for item in MenuItem::ALL {
                if item == self.selected {
                    string.push_str("-> ")?;
                } else {
                    string.push_str("   ")?;
                }
                writeln!(string, "{item}").map_err(|_| ())?;
            }
            string.pop();
            Ok(string)
        };

        let string = build_str().map_err(|_: ()| "failed to render menu string")?;

        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::TopLeft),
            text_style,
            Alignment::Left,
        );

        text.draw(display).map_err(|_| "failed to draw text")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    SavePos1,
    SavePos2,
    Calibration,
    ResetDrive,
}

impl MenuItem {
    const ALL: [Self; 4] = [
        Self::SavePos1,
        Self::SavePos2,
        Self::Calibration,
        Self::ResetDrive,
    ];
}

impl core::fmt::Display for MenuItem {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let string = match self {
            MenuItem::SavePos1 => "Store position 1",
            MenuItem::SavePos2 => "Store position 2",
            MenuItem::Calibration => "Height calibration",
            MenuItem::ResetDrive => "Start reset drive",
        };

        f.write_str(string)
    }
}

async fn footer<D>(display: &mut D) -> Result<(), &'static str>
where
    D: DrawTarget<Color = BinaryColor> + Dimensions,
{
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_7X13)
        .text_color(BinaryColor::On)
        .build();

    let string = "up/down nav | pos1 exit | pos2 select";

    let text = Text::with_alignment(
        string,
        display.bounding_box().anchor_point(AnchorPoint::BottomLeft),
        text_style,
        Alignment::Center,
    );

    text.draw(display).map_err(|_| "failed to draw text")?;
    Ok(())
}

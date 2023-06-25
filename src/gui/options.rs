use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};

use super::{
    widgets::{footer, Menu, MenuContent},
    MainMenu,
};

pub struct Options {
    pub menu: Menu<OptionItem>,
}

impl Options {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        self.menu
            .display::<{ OptionItem::MENU_STRING_LENGTH }>(display)
            .await?;
        let string = "+- nav | pos1 exit | pos2 sel";
        footer(display, string).await?;
        Ok(())
    }
}

impl From<Options> for MainMenu {
    fn from(value: Options) -> Self {
        Self::Options(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionItem {
    SavePos1,
    SavePos2,
    Calibration,
    ResetDrive,
}

impl MenuContent for OptionItem {
    const MENU_STRING_LENGTH: usize = 83;

    type Iter = core::array::IntoIter<OptionItem, 4>;
    type IterItem = OptionItem;

    fn iter(&self) -> Self::Iter {
        [
            OptionItem::SavePos1,
            OptionItem::SavePos2,
            OptionItem::Calibration,
            OptionItem::ResetDrive,
        ]
        .into_iter()
    }

    fn next(&mut self) {
        *self = match self {
            OptionItem::SavePos1 => OptionItem::SavePos2,
            OptionItem::SavePos2 => OptionItem::Calibration,
            OptionItem::Calibration => OptionItem::ResetDrive,
            OptionItem::ResetDrive => OptionItem::SavePos1,
        }
    }

    fn prev(&mut self) {
        *self = match self {
            OptionItem::SavePos1 => OptionItem::ResetDrive,
            OptionItem::SavePos2 => OptionItem::SavePos1,
            OptionItem::Calibration => OptionItem::SavePos2,
            OptionItem::ResetDrive => OptionItem::Calibration,
        }
    }

    fn is_selected(&self, content: &Self) -> bool {
        self == content
    }
}

impl core::fmt::Display for OptionItem {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let string = match self {
            OptionItem::SavePos1 => "Store position 1",
            OptionItem::SavePos2 => "Store position 2",
            OptionItem::Calibration => "Height calibration",
            OptionItem::ResetDrive => "Start reset drive",
        };

        f.write_str(string)
    }
}

pub struct ResetDrive;

impl From<ResetDrive> for MainMenu {
    fn from(value: ResetDrive) -> Self {
        Self::ResetDrive(value)
    }
}

impl ResetDrive {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        let text = Text::with_alignment(
            "Reset drive active.\nPress any button\nto stop/finish.",
            display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(0, 6),
            text_style,
            Alignment::Left,
        );

        text.draw(display).map_err(|_| "failed to draw text")?;
        Ok(())
    }
}

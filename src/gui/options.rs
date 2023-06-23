use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

use super::widgets::{footer, Menu, MenuItem};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionItem {
    SavePos1,
    SavePos2,
    Calibration,
    ResetDrive,
}

impl MenuItem for OptionItem {
    const MENU_STRING_LENGTH: usize = 83;

    type Iter = core::array::IntoIter<OptionItem, 4>;

    fn iter() -> Self::Iter {
        [
            OptionItem::SavePos1,
            OptionItem::SavePos2,
            OptionItem::Calibration,
            OptionItem::ResetDrive,
        ]
        .into_iter()
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
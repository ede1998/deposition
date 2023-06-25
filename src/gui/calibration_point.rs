use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{
        ascii::{FONT_10X20, FONT_6X12},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};

use crate::{data::Millimeters, format};

use super::{widgets::footer, MainMenu};

pub struct CalibrationPoint {
    pub adc: u16,
    pub height: Millimeters,
}

impl CalibrationPoint {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();
        let cm = self.height.as_cm();
        let mm = self.height.as_mm() % 10;
        let string = format!(20, "{cm:>3},{mm}cm");
        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::Center),
            text_style,
            Alignment::Center,
        );
        text.draw(display).map_err(|_| "failed to draw height")?;

        let text_style_small = MonoTextStyleBuilder::new()
            .font(&FONT_6X12)
            .text_color(BinaryColor::On)
            .build();
        let string = format!(20, "ADC: {}", self.adc);
        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::TopCenter) + Point::new(0, 6),
            text_style_small,
            Alignment::Center,
        );
        text.draw(display).map_err(|_| "failed to draw ADC text")?;

        let string = "+ inc | - dec | pos1 exit | pos2 sav";
        footer(display, string).await?;
        Ok(())
    }
}

impl From<CalibrationPoint> for MainMenu {
    fn from(value: CalibrationPoint) -> Self {
        Self::CalibrationPoint(value)
    }
}

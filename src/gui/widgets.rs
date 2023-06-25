use core::fmt::Display;
use core::fmt::Write;

use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{
        ascii::{FONT_4X6, FONT_6X10},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, StyledDrawable},
    text::{Alignment, Text},
};
use heapless::String;

pub async fn footer<D>(display: &mut D, string: &str) -> Result<(), &'static str>
where
    D: DrawTarget<Color = BinaryColor> + Dimensions,
{
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_4X6)
        .text_color(BinaryColor::On)
        .build();
    let prim_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    let text = Text::with_alignment(
        string,
        display
            .bounding_box()
            .anchor_point(AnchorPoint::BottomCenter),
        text_style,
        Alignment::Center,
    );

    let top_left = text.bounding_box().anchor_point(AnchorPoint::TopLeft);
    let top_right = text.bounding_box().anchor_point(AnchorPoint::TopRight);
    let separator = Line::new(top_left, top_right).translate(Point::new(0, -2));

    text.draw(display).map_err(|_| "failed to draw text")?;
    separator
        .draw_styled(&prim_style, display)
        .map_err(|_| "failed to draw separator line")?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct Menu<T> {
    pub content: T,
}

pub trait MenuContent {
    fn iter(&self) -> Self::Iter;
    type IterItem: Display + Copy;
    type Iter: Iterator<Item = Self::IterItem>;
    const MENU_STRING_LENGTH: usize;
    fn next(&mut self);
    fn prev(&mut self);
    fn is_selected(&self, item: &Self::IterItem) -> bool;
}

impl<T: MenuContent> Menu<T> {
    pub fn new(content: T) -> Self {
        Self { content }
    }

    pub async fn display<const MENU_STRING_LENGTH: usize>(
        &self,
        display: &mut (impl DrawTarget<Color = BinaryColor> + Dimensions),
    ) -> Result<(), &'static str> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        let build_str = || {
            let mut string = String::<MENU_STRING_LENGTH>::new();
            for item in self.content.iter() {
                if self.content.is_selected(&item) {
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
            display.bounding_box().anchor_point(AnchorPoint::TopLeft) + Point::new(0, 6),
            text_style,
            Alignment::Left,
        );

        text.draw(display).map_err(|_| "failed to draw text")?;
        Ok(())
    }
}

use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable, Triangle},
    text::{Alignment, Text},
};

use crate::{data::Millimeters, format, history::Direction};

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

pub struct Start {
    pub height: Option<Millimeters>,
    pub direction: Direction,
}

impl Start {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();
        let prim_style = PrimitiveStyle::with_fill(BinaryColor::On);
        let string = match self.height {
            Some(height) => format!(10, "{:>3}cm", height.as_cm()),
            None => format!(10, "???cm"),
        };

        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::Center),
            text_style,
            Alignment::Center,
        );

        text.draw(display).map_err(|_| "failed to draw text")?;

        let rect = Rectangle::new(
            text.bounding_box()
                .anchor_point(AnchorPoint::TopLeft)
                .y_axis(),
            Size::new_equal(text.bounding_box().size.height),
        );

        match self.direction {
            Direction::Up => triangle(rect, true).draw_styled(&prim_style, display),
            Direction::Stopped => rect.draw_styled(&prim_style, display),
            Direction::Down => triangle(rect, false).draw_styled(&prim_style, display),
            Direction::ResetDrive => unimplemented!(),
        }
        .map_err(|_| "failed to draw direction indicator")?;
        Ok(())
    }
}

fn triangle(bounding_box: Rectangle, point_up: bool) -> Triangle {
    let anchors = if point_up {
        [
            AnchorPoint::BottomLeft,
            AnchorPoint::BottomRight,
            AnchorPoint::TopCenter,
        ]
    } else {
        [
            AnchorPoint::BottomCenter,
            AnchorPoint::TopLeft,
            AnchorPoint::TopRight,
        ]
    };

    let v = anchors.map(|a| bounding_box.anchor_point(a));
    Triangle::new(v[0], v[1], v[2])
}

use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{ascii::FONT_10X20, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Dimensions, DrawTarget, Size},
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable, Triangle},
    text::{Alignment, Text},
    Drawable,
};
use esp_println::println;

use crate::{
    data::{Millimeters, DIRECTION, HEIGHT},
    format,
    history::Direction,
};

use super::{Action, Button, Input, Menu};

#[derive(Debug)]
pub struct Start {
    text_style: MonoTextStyle<'static, BinaryColor>,
    prim_style: PrimitiveStyle<BinaryColor>,
    current_height: Option<Millimeters>,
    current_direction: Direction,
}

impl Default for Start {
    fn default() -> Self {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();
        let prim_style = PrimitiveStyle::with_fill(BinaryColor::On);
        Self {
            text_style,
            prim_style,
            current_height: None,
            current_direction: Direction::Stopped,
        }
    }
}

impl Start {
    pub async fn update(&mut self, input: Option<Input>) -> Option<Menu> {
        self.current_height = Some(HEIGHT.wait().await);
        self.current_direction = DIRECTION.get().await;
        println!("input: {input:?}");
        let input = input?;
        let pot_direction = match input.button {
            Button::Up => Direction::Up,
            Button::Down => Direction::Down,
            _ => return None,
        };

        match input.action {
            Action::Pressed => {
                DIRECTION.request(pot_direction).await;
            }
            Action::Released => {
                DIRECTION.request(pot_direction).await;
            }
            _ => {}
        }

        None
    }

    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        let string = match self.current_height {
            Some(height) => format!(10, "{:>3}cm", height.as_cm()),
            None => format!(10, "???cm"),
        };

        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::Center),
            self.text_style,
            Alignment::Center,
        );

        text.draw(display).map_err(|_| "failed to draw text")?;

        let rect = Rectangle::new(
            text.bounding_box()
                .anchor_point(AnchorPoint::TopLeft)
                .y_axis(),
            Size::new_equal(text.bounding_box().size.height),
        );

        match self.current_direction {
            Direction::Up => triangle(rect, true).draw_styled(&self.prim_style, display),
            Direction::Stopped => rect.draw_styled(&self.prim_style, display),
            Direction::Down => triangle(rect, false).draw_styled(&self.prim_style, display),
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

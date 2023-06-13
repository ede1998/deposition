use embedded_graphics::{
    geometry::AnchorPoint,
    mono_font::{ascii::FONT_10X20, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Dimensions, DrawTarget, Size},
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable, Triangle},
    text::{Alignment, Text},
    Drawable,
};

use crate::{
    data::{Millimeters, DIRECTION, HEIGHT, INPUT},
    format,
    history::Direction,
};

use super::{Button, Inputs, Menu};

#[derive(Debug)]
pub struct Start {
    text_style: MonoTextStyle<'static, BinaryColor>,
    prim_style: PrimitiveStyle<BinaryColor>,
    current_height: Option<Millimeters>,
    current_direction: Direction,
    current_input: Inputs,
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
            current_input: Inputs::default(),
        }
    }
}

impl Start {
    pub async fn update(&mut self) -> Option<Menu> {
        self.current_height = Some(*HEIGHT.lock().await);
        self.current_direction = DIRECTION.get().await;
        let inputs = INPUT.lock().await.clone();

        let changes = inputs.changed_since(&self.current_input);

        self.current_input = inputs;

        match changes.pressed_exclusive() {
            Some(Button::Up) => {
                DIRECTION.request(Direction::Up).await;
            }
            Some(Button::Down) => {
                DIRECTION.request(Direction::Down).await;
            }
            _ => {}
        }

        if changes.released(Button::Up) && self.current_direction == Direction::Up {
            DIRECTION.request(Direction::Stopped).await;
        }
        if changes.released(Button::Down) && self.current_direction == Direction::Down {
            DIRECTION.request(Direction::Stopped).await;
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

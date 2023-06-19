use core::convert::Infallible;

use embassy_futures::select::select;
use embassy_time::{Duration, Timer};
use esp_println::println;

use crate::{
    data::{Millimeters, DIRECTION, GUI_MENU, HEIGHT},
    gui::{Menu, Start},
    history::Direction,
    input::{Button, Inputs},
};

enum OperationMode {
    Start,
}

type Result<T = OperationMode> = core::result::Result<T, &'static str>;

pub async fn run() -> Result<Infallible> {
    let mut mode = OperationMode::Start;
    let mut inputs = Inputs::new();
    loop {
        mode = match mode {
            OperationMode::Start => run_start(&mut inputs).await?,
        };
    }
}

async fn run_start(inputs: &mut Inputs) -> Result {
    loop {
        println!("running start");
        update_start_gui(Direction::Stopped).await;
        match inputs.wait_for_press().await {
            Button::Up => {
                drive_direction(inputs, Direction::Up, Button::Up).await;
            }
            Button::Down => {
                drive_direction(inputs, Direction::Down, Button::Down).await;
            }
            Button::Pos1 => todo!(),
            Button::Pos2 => todo!(),
        }
    }
}

async fn drive_direction(inputs: &mut Inputs, direction: Direction, button: Button) {
    DIRECTION.request(direction).await;
    select(inputs.wait_for_release(button), async {
        loop {
            update_start_gui(direction).await;
            Timer::after(Duration::from_millis(100)).await;
        }
    })
    .await;
    DIRECTION.request(Direction::Stopped).await;
}

async fn update_start_gui(direction: Direction) {
    let height = *HEIGHT.lock().await;
    GUI_MENU.signal(Menu::Start(Start {
        height: Some(height),
        direction,
    }));
}

async fn drive_to(pos: Millimeters) {}

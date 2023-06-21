use core::{cmp::Ordering, convert::Infallible, future::Future};

use embassy_futures::select::{select, select3};
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
        start_gui(Direction::Stopped).await;
        match inputs.wait_for_press().await {
            Button::UpAndDown => {
                println!("show options menu")
            }
            Button::Up => {
                drive_direction(inputs, Direction::Up, Button::Up).await;
            }
            Button::Down => {
                drive_direction(inputs, Direction::Down, Button::Down).await;
            }
            Button::Pos1 => {
                let target_height = Millimeters::from_mm(0);
                drive_to_position(inputs, target_height).await;
            }
            Button::Pos2 => {
                let target_height = Millimeters::from_mm(80);
                drive_to_position(inputs, target_height).await;
            }
            _ => {},
        }
    }
}

async fn drive_direction(inputs: &mut Inputs, direction: Direction, button: Button) {
    DIRECTION.request(direction).await;
    select(
        inputs.wait_for_release(button),
        refresh_gui(|| start_gui(direction)),
    )
    .await;
    DIRECTION.request(Direction::Stopped).await;
}

async fn drive_to_position(inputs: &mut Inputs, target_height: Millimeters) {
    let current_height = *HEIGHT.lock().await;
    let direction;
    let on_the_way: fn(Millimeters, Millimeters) -> bool;
    match current_height.cmp_fuzzy_eq(target_height) {
        Ordering::Equal => return,
        Ordering::Less => {
            direction = Direction::Up;
            on_the_way =
                |current_height, target_height| current_height.cmp_fuzzy_eq(target_height).is_lt();
        }
        Ordering::Greater => {
            direction = Direction::Down;
            on_the_way =
                |current_height, target_height| current_height.cmp_fuzzy_eq(target_height).is_gt();
        }
    };

    let check_height = || async move {
        let mut current_height = current_height;
        while on_the_way(current_height, target_height) {
            embassy_futures::yield_now().await;
            current_height = *HEIGHT.lock().await;
        }
    };
    DIRECTION.request(direction).await;
    select3(
        check_height(),
        inputs.wait_for_press(),
        refresh_gui(|| start_gui(direction)),
    )
    .await;
    DIRECTION.request(Direction::Stopped).await;
}

async fn refresh_gui<F, O>(mut updater: F)
where
    F: FnMut() -> O,
    O: Future<Output = ()>,
{
    loop {
        updater().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}

async fn start_gui(direction: Direction) {
    let height = *HEIGHT.lock().await;
    GUI_MENU.signal(Menu::Start(Start {
        height: Some(height),
        direction,
    }));
}

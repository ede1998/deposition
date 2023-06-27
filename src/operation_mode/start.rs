use core::cmp::Ordering;

use embassy_futures::select::{select, select3};
use embassy_time::{Duration, Ticker};

use crate::{
    data::{Direction, Millimeters, DIRECTION, GUI_MENU, HEIGHT},
    gui::Start,
    input::{Button, Inputs},
    storage::CONFIGURATION,
};

use super::{options, refresh_gui, Result};

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running start screen");
        start_gui(Direction::Stopped).await;
        inputs.wait_all_released().await;
        match inputs.wait_for_press().await {
            Button::UpAndDown => {
                options::run(inputs).await?;
            }
            Button::Up => {
                drive_direction(inputs, Direction::Up, Button::Up).await;
            }
            Button::Down => {
                drive_direction(inputs, Direction::Down, Button::Down).await;
            }
            Button::Pos1 => {
                let Some(target_height) = CONFIGURATION.lock().await.get().position_1 else {
                    log::debug!("position 1 not saved.");
                    continue;
                };
                drive_to_position(inputs, target_height).await;
            }
            Button::Pos2 => {
                let Some(target_height) = CONFIGURATION.lock().await.get().position_2 else {
                    log::debug!("position 2 not saved.");
                    continue;
                };
                drive_to_position(inputs, target_height).await;
            }
            _ => {}
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
            Ticker::every(Duration::from_millis(10)).next().await;
            current_height = *HEIGHT.lock().await;
        }
    };
    DIRECTION.request(direction).await;

    inputs.wait_all_released().await;
    select3(
        check_height(),
        inputs.wait_for_single_press(),
        refresh_gui(|| start_gui(direction)),
    )
    .await;
    DIRECTION.request(Direction::Stopped).await;
}

async fn start_gui(direction: Direction) {
    let height = *HEIGHT.lock().await;
    GUI_MENU.signal(
        Start {
            height: Some(height),
            direction,
        }
        .into(),
    );
}

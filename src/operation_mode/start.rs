use core::cmp::Ordering;

use embassy_futures::select::{select, select3};
use embassy_time::{Duration, Ticker, Timer};

use crate::{
    data::{Direction, Millimeters, Mutex, DIRECTION, GUI_MENU, HEIGHT},
    gui::Start,
    input::{Button, Inputs},
    storage::CONFIGURATION,
};

use super::{options, refresh_gui, Result};

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running start screen");
        wait_for_first_measurement().await;
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
    const ALLOWED_DELTA_IN_STANDSTILL: Millimeters = Millimeters::from_mm(2);
    const ALLOWED_DELTA_IN_MOVEMENT: Millimeters = Millimeters::from_mm(18);
    let current_height = *HEIGHT.lock().await;
    let direction;
    let on_the_way = match current_height.cmp_fuzzy_eq(target_height, ALLOWED_DELTA_IN_STANDSTILL) {
        Ordering::Equal => return,
        Ordering::Less => {
            direction = Direction::Up;
            |current_height: Millimeters, target_height| {
                current_height
                    .cmp_fuzzy_eq(target_height, ALLOWED_DELTA_IN_MOVEMENT)
                    .is_lt()
            }
        }
        Ordering::Greater => {
            direction = Direction::Down;
            |current_height: Millimeters, target_height| {
                current_height
                    .cmp_fuzzy_eq(target_height, ALLOWED_DELTA_IN_MOVEMENT)
                    .is_gt()
            }
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

async fn wait_for_first_measurement() {
    let timeout = Duration::from_secs(2);
    let check_height = async {
        fn not_updated(h: &Mutex<Millimeters>) -> bool {
            h.try_lock().map_or(true, |guard| guard.is_zero())
        }

        while not_updated(&HEIGHT) {
            Ticker::every(Duration::from_hz(10)).next().await;
        }
    };

    select(Timer::after(timeout), check_height).await;
}

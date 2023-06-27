use embassy_futures::select::select;
use embassy_time::{Duration, Instant, Ticker, Timer};

use crate::{
    data::{Calibration, Millimeters, CALIBRATION, GUI_MENU, RAW_HEIGHT},
    gui::{CalibrationMenu, CalibrationOptions, CalibrationPoint, Menu, MenuContent, Selected},
    input::{Button, Inputs},
    storage::CONFIGURATION,
};

use super::Result;

pub async fn run(inputs: &mut Inputs) -> Result {
    let mut menu = CalibrationMenu::new(Calibration::new());
    loop {
        log::info!("running calibration screen");
        let calibration = CONFIGURATION.lock().await.get().calibration.clone();
        let no_more_points = calibration.is_full();
        menu.update_calibration(&calibration);

        GUI_MENU.signal(
            CalibrationOptions {
                menu: Menu::new(menu.clone()),
            }
            .into(),
        );

        inputs.wait_all_released().await;
        match inputs.wait_for_single_press().await {
            Button::Up => menu.prev(),
            Button::Down => menu.next(),
            Button::Pos1 => return Ok(()),
            Button::Pos2 => match menu.selected() {
                Selected::AddNew => {
                    if no_more_points {
                        continue;
                    }
                    add_calibration_point(inputs).await?;
                }
                Selected::RemoveAll => {
                    CONFIGURATION.lock().await.update(|data| {
                        data.calibration.clear();
                    });
                }
                Selected::ShowOne => {
                    let Some(index) = menu.shown_index() else {continue;};
                    CONFIGURATION.lock().await.update(|data| {
                        data.calibration.remove(index);
                    });
                }
            },
            _ => {}
        }
    }
}

async fn add_calibration_point(inputs: &mut Inputs) -> Result {
    log::info!("running add calibration point screen");

    let adc = RAW_HEIGHT.wait().await;

    let mut height = Millimeters::from_mm(1000);
    loop {
        GUI_MENU.signal(CalibrationPoint { adc, height }.into());
        inputs.wait_all_released().await;
        match inputs.wait_for_single_press().await {
            Button::Up => height = button_held(adc, height, Button::Up, inputs).await,
            Button::Down => height = button_held(adc, height, Button::Down, inputs).await,
            Button::Pos1 => return Ok(()),
            Button::Pos2 => break,
            _ => {}
        }
    }

    let mut res = Ok(());
    let cali = CONFIGURATION
        .lock()
        .await
        .update(|data| {
            res = data.calibration.insert(adc, height);
        })
        .calibration
        .clone();

    res?;

    CALIBRATION.signal(cali);

    Ok(())
}

async fn button_held(
    adc: u16,
    mut height: Millimeters,
    btn: Button,
    inputs: &mut Inputs,
) -> Millimeters {
    let mut update_height = || match btn {
        Button::Up => {
            height = height.increase();
            height
        }
        Button::Down => {
            height = height.decrease();
            height
        }
        _ => panic!("Invalid argument: Expected up or down, got {btn:?}"),
    };

    update_height();
    select(inputs.wait_for_release(btn), async {
        Timer::after(Duration::from_millis(500)).await;
        let mut ladder = Ladder::new([1, 10, 50, 100], Duration::from_secs(2));
        loop {
            let height = ladder.repeat(&mut update_height);

            GUI_MENU.signal(CalibrationPoint { adc, height }.into());

            ladder.accelerate();
            Ticker::every(Duration::from_hz(10)).next().await;
        }
    })
    .await;

    height
}

struct Ladder {
    steps: core::array::IntoIter<u16, 3>,
    current_step: u16,
    speed_up_after: Duration,
    start: Instant,
}

impl Ladder {
    fn new(steps: [u16; 4], speed_up_after: Duration) -> Self {
        let [current, rest @ ..] = steps;
        Self {
            steps: rest.into_iter(),
            current_step: current,
            speed_up_after,
            start: Instant::now(),
        }
    }

    fn repeat<F, T>(&self, mut f: F) -> T
    where
        F: FnMut() -> T,
    {
        let mut res = f();
        for _ in 1..self.current_step {
            res = f();
        }
        res
    }

    fn accelerate(&mut self) {
        if self.start.elapsed() < self.speed_up_after {
            return;
        }

        let Some(next_step) = self.steps.next() else { return; };

        self.current_step = next_step;
        self.start = Instant::now();

        log::debug!("Scroll speed is now {}", self.current_step);
    }
}

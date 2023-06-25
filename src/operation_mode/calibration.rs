use crate::{
    data::{Millimeters, CALIBRATION, GUI_MENU, RAW_HEIGHT},
    gui::{CalibrationMenu, CalibrationOptions, CalibrationPoint, Menu, MenuContent, Selected},
    input::{Button, Inputs},
    storage::CONFIGURATION,
};

use super::Result;

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running calibration screen");
        let calibration = CONFIGURATION.lock().await.get().calibration.clone();
        let no_more_points = calibration.is_full();
        let mut menu = CalibrationMenu::new(calibration);

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

pub async fn add_calibration_point(inputs: &mut Inputs) -> Result {
    log::info!("running add calibration point screen");

    let adc = RAW_HEIGHT.wait().await;

    let mut height = Millimeters::from_mm(1000);
    loop {
        GUI_MENU.signal(CalibrationPoint { adc, height }.into());
        inputs.wait_all_released().await;
        match inputs.wait_for_single_press().await {
            Button::Up => height = height.increase(),
            Button::Down => height = height.decrease(),
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

use crate::{
    data::GUI_MENU,
    gui::{CalibrationMenu, CalibrationOptions, Menu, MenuContent, Selected},
    input::{Button, Inputs},
    storage::CONFIGURATION,
};

use super::Result;

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running calibration screen");
        let calibration = CONFIGURATION.lock().await.get().calibration.clone();
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
    //CONFIGURATION.lock().await.update(|data| {
    //    data.calibration.insert();
    //});
    Ok(())
}

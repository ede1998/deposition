use crate::{
    data::{Direction, Millimeters, DIRECTION, GUI_MENU, HEIGHT},
    gui::{Menu, MenuContent, OptionItem, Options, ResetDrive},
    input::{Button, Inputs},
    storage::{InnerData, CONFIGURATION},
};

use super::{calibration, Result};

pub async fn run(inputs: &mut Inputs) -> Result {
    let mut selected = OptionItem::SavePos1;
    loop {
        log::info!("running options screen");

        GUI_MENU.signal(
            Options {
                menu: Menu::new(selected),
            }
            .into(),
        );

        inputs.wait_all_released().await;
        match inputs.wait_for_single_press().await {
            Button::Up => selected.prev(),
            Button::Down => selected.next(),
            Button::Pos1 => return Ok(()),
            Button::Pos2 => match selected {
                OptionItem::SavePos1 => save_pos(1, |d| &mut d.position_1).await,
                OptionItem::SavePos2 => save_pos(2, |d| &mut d.position_2).await,
                OptionItem::Calibration => calibration::run(inputs).await?,
                OptionItem::ResetDrive => reset_drive(inputs).await,
            },
            _ => {}
        }
    }
}

async fn reset_drive(inputs: &mut Inputs) {
    DIRECTION.request(Direction::ResetDrive).await;

    inputs.wait_all_released().await;
    GUI_MENU.signal(ResetDrive.into());
    inputs.wait_for_single_press().await;
    DIRECTION.request(Direction::Stopped).await;
}

async fn save_pos<F>(pos_num: u8, f: F)
where
    F: Fn(&mut InnerData) -> &mut Option<Millimeters>,
{
    let height = *HEIGHT.lock().await;
    log::info!("saving position {pos_num} with height {}mm", height.as_mm());
    let mut conf = CONFIGURATION.lock().await;
    conf.update(|data| *f(data) = Some(height));
}

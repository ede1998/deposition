use crate::{
    data::{DIRECTION, GUI_MENU},
    gui::{Menu, MenuItem, OptionItem, Options, ResetDrive},
    history::Direction,
    input::{Button, Inputs},
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
            Button::Up => selected = selected.prev(),
            Button::Down => selected = selected.next(),
            Button::Pos1 => return Ok(()),
            Button::Pos2 => match selected {
                OptionItem::SavePos1 => todo!(),
                OptionItem::SavePos2 => todo!(),
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

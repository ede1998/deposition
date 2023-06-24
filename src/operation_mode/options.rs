use crate::{
    data::GUI_MENU,
    gui::{Menu, MenuItem, OptionItem, Options},
    input::{Button, Inputs},
};

use super::Result;

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
        match inputs.wait_for_press().await {
            Button::Up => selected = selected.prev(),
            Button::Down => selected = selected.prev(),
            Button::Pos1 => return Ok(()),
            Button::Pos2 => match selected {
                OptionItem::SavePos1 => todo!(),
                OptionItem::SavePos2 => todo!(),
                OptionItem::Calibration => todo!(),
                OptionItem::ResetDrive => todo!(),
            },
            _ => {}
        }

        embassy_futures::yield_now().await;
    }
}

use crate::{
    data::GUI_MENU,
    gui::{MainMenu, Menu, OptionItem, Options},
    input::Inputs,
};

use super::Result;

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running options screen");
        inputs.wait_all_released().await;
        GUI_MENU.signal(MainMenu::Options(Options {
            menu: Menu::new(OptionItem::SavePos1),
        }));
        embassy_futures::yield_now().await;
    }
}

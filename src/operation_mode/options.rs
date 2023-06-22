use crate::input::Inputs;

use super::Result;

pub async fn run(inputs: &mut Inputs) -> Result {
    loop {
        log::info!("running options screen");
        inputs.wait_all_released().await;
    }
}

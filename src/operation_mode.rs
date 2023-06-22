use core::{convert::Infallible, future::Future};

use embassy_time::{Duration, Timer};

use crate::input::Inputs;

pub enum OperationMode {
    Start,
    Options,
}

type Result<T = OperationMode> = core::result::Result<T, &'static str>;

mod start;
mod options;

pub async fn run() -> Result<Infallible> {
    let mut mode = OperationMode::Start;
    let mut inputs = Inputs::new();
    loop {
        mode = match mode {
            OperationMode::Start => start::run(&mut inputs).await?,
            // Do I even need this?
            OperationMode::Options => options::run(&mut inputs).await?,
        };
    }
}

async fn refresh_gui<F, O>(mut updater: F)
where
    F: FnMut() -> O,
    O: Future<Output = ()>,
{
    loop {
        updater().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}

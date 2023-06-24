use core::{convert::Infallible, future::Future};

use embassy_time::{Duration, Timer};

use crate::input::Inputs;

type Result<T = ()> = core::result::Result<T, &'static str>;

mod options;
mod start;

pub async fn run() -> Result<Infallible> {
    let mut inputs = Inputs::new();
    loop {
        start::run(&mut inputs).await?;
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

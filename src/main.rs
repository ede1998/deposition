#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use debouncr::DebouncerStateful;
use embassy_time::{Duration, Ticker, Timer};
use embedded_graphics::{draw_target::DrawTarget, pixelcolor::BinaryColor};
use esp_backtrace as _;
use esp_println::logger::init_logger;
use hal::{
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    clock::ClockControl,
    gpio::{Gpio34, Io, Level, Pull},
    i2c::I2C,
    peripherals::{Peripherals, ADC1},
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
};
use heapless::String;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

mod data;
mod gui;
mod input;
mod operation_mode;
mod storage;
mod string_format;

use crate::{
    data::{Direction, CALIBRATION, DIRECTION, GUI_MENU, HEIGHT, INPUT, RAW_HEIGHT},
    input::{Inputs, State},
    storage::CONFIGURATION,
};

async fn poll<T, E>(mut f: impl FnMut() -> nb::Result<T, E>) -> Result<T, E> {
    loop {
        match f() {
            Ok(ok) => break Ok(ok),
            Err(nb::Error::Other(err)) => break Err(err),
            Err(nb::Error::WouldBlock) => {} // todo: do we want to keep a busy loop here? the measure task starved with this: `embassy_futures::yield_now().await,`
        }
    }
}

fn compute_median(samples: &mut [u16]) -> u16 {
    samples.sort_unstable();
    let len = samples.len();
    if len % 2 == 0 {
        let right_mid = samples[len / 2];
        let left_mid = samples[(len / 2) - 1];
        (right_mid + left_mid) / 2
    } else {
        samples[len / 2]
    }
}

const SAMPLE_COUNT: usize = if cfg!(debug_assertions) { 32 } else { 64 };

type InputPin = hal::gpio::AnyInput<'static>;
type OutputPin = hal::gpio::AnyOutput<'static>;

#[embassy_executor::task]
async fn drive(mut up: OutputPin, mut down: OutputPin) {
    up.set_low();
    down.set_low();
    loop {
        Timer::after(Duration::from_millis(5)).await;
        let Some(direction) = DIRECTION.planned().await else {
            continue;
        };
        log::info!("starting to drive in direction {direction}");
        match direction {
            Direction::Up => {
                down.set_low();
                up.set_high();
            }
            Direction::Down => {
                up.set_low();
                down.set_high();
            }
            Direction::Stopped => {
                up.set_low();
                down.set_low();
            }
            Direction::ResetDrive => {
                up.set_high();
                down.set_high();
            }
        }
        DIRECTION.acknowledge(direction).await;
    }
}

#[embassy_executor::task]
async fn read_input(up: InputPin, down: InputPin, pos1: InputPin, pos2: InputPin) {
    struct DebouncedPin {
        pin: InputPin,
        debouncer: DebouncerStateful<u8, debouncr::Repeat2>,
    }

    impl DebouncedPin {
        fn new(pin: InputPin) -> Self {
            let debouncer = debouncr::debounce_stateful_2(false);
            Self { pin, debouncer }
        }

        fn update_input(&mut self, input: &mut State) {
            let active = self.pin.is_low();
            let Some(state) = self.debouncer.update(active) else {
                return;
            };
            match state {
                debouncr::Edge::Rising => input.press(),
                debouncr::Edge::Falling => input.release(),
            }
        }
    }

    let mut up = DebouncedPin::new(up);
    let mut down = DebouncedPin::new(down);
    let mut pos1 = DebouncedPin::new(pos1);
    let mut pos2 = DebouncedPin::new(pos2);

    let mut inputs = Inputs::default();

    loop {
        up.update_input(&mut inputs.up);
        down.update_input(&mut inputs.down);
        pos1.update_input(&mut inputs.pos1);
        pos2.update_input(&mut inputs.pos2);

        *INPUT.lock().await = inputs.clone();

        Timer::after(Duration::from_millis(5)).await;
    }
}

#[embassy_executor::task]
async fn measure_task(gpio34: Gpio34, adc: ADC1) {
    measure(gpio34, adc).await.expect("measure task failed");
}

async fn measure(pin: Gpio34, adc: ADC1) -> Result<(), &'static str> {
    let mut adc1_config = AdcConfig::new();
    let mut pin34 = adc1_config.enable_pin(pin, Attenuation::Attenuation11dB);
    let mut adc1 = Adc::<ADC1>::new(adc, adc1_config);

    let mut calibration = CONFIGURATION.lock().await.get().calibration.clone();

    loop {
        if CALIBRATION.signaled() {
            calibration = CALIBRATION.wait().await;
        }

        log::trace!("starting measurement");

        let pin25_value = read_sample(&mut adc1, &mut pin34).await?;

        let value = calibration.transform(pin25_value);

        log::trace!("new height {} (={pin25_value}) measured", value.as_mm());
        *HEIGHT.lock().await = value;
        RAW_HEIGHT.signal(pin25_value);
        Ticker::every(Duration::from_millis(5)).next().await;
    }
}

async fn read_sample<'a>(
    adc1: &mut Adc<'a, ADC1>,
    pin34: &mut AdcPin<Gpio34, ADC1>,
) -> Result<u16, &'static str> {
    let mut samples = heapless::Vec::<_, SAMPLE_COUNT>::new();
    for _ in 0..samples.capacity() {
        let sample = poll(|| adc1.read_oneshot(pin34))
            .await
            .map_err(|_| "failed to read ADC value")?;

        samples.push(sample).map_err(|_| "failed to store sample")?;
    }

    Ok(compute_median(&mut samples))
}

#[embassy_executor::task]
async fn display_task(i2c: I2C<'static, hal::peripherals::I2C0, hal::Blocking>) {
    display(i2c).await.expect("display task failed");
}

fn str_to_owned<const N: usize>(text: &str) -> String<N> {
    text.try_into()
        .expect("Length of str exceeds String capacity")
}

async fn display(
    i2c: I2C<'static, hal::peripherals::I2C0, hal::Blocking>,
) -> Result<(), String<150>> {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display
        .init()
        .map_err(|e| format!(150, "display initialization failed: {e:?}"))?;

    loop {
        let menu = GUI_MENU.wait().await;

        display
            .clear(BinaryColor::Off)
            .map_err(|e| format!(150, "clearing display failed: {e:?}"))?;
        menu.display(&mut display).await.map_err(str_to_owned)?;
        display
            .flush()
            .map_err(|e| format!(150, "flushing failed: {e:?}"))?;
    }
}

#[embassy_executor::task]
async fn run() {
    operation_mode::run().await.expect("run task failed");
}

#[main]
async fn main(spawner: embassy_executor::Spawner) {
    init_logger(log::LevelFilter::Trace);
    log::info!("init!");
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let timer_group0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timer_group0);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let adc = peripherals.ADC1;
    // Create a new peripheral object with the described wiring
    // and standard I2C clock speed
    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio32,
        io.pins.gpio27,
        100u32.kHz(),
        &clocks,
        None,
    );
    let btn_up = InputPin::new(io.pins.gpio18, Pull::Up);
    let btn_down = InputPin::new(io.pins.gpio19, Pull::Up);
    let btn_pos1 = InputPin::new(io.pins.gpio4, Pull::Up);
    let btn_pos2 = InputPin::new(io.pins.gpio5, Pull::Up);
    let height_meter = io.pins.gpio34;

    let up = OutputPin::new(io.pins.gpio26, Level::Low);
    let down = OutputPin::new(io.pins.gpio25, Level::Low);

    spawner.spawn(measure_task(height_meter, adc)).unwrap();
    spawner.spawn(display_task(i2c)).unwrap();
    spawner
        .spawn(read_input(btn_up, btn_down, btn_pos1, btn_pos2))
        .unwrap();
    spawner.spawn(drive(up, down)).unwrap();
    spawner.spawn(run()).unwrap();
}

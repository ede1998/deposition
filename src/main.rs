#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use debouncr::DebouncerStateful;
use embassy_executor::Executor;
use embassy_time::{Duration, Timer};
//use embassy_time::Instant;
use esp_backtrace as _;
use esp_println::logger::init_logger;
use hal::{
    adc::{AdcConfig, AdcPin, Attenuation, ADC, ADC2},
    analog::AvailableAnalog,
    clock::ClockControl,
    gpio::{Analog, Gpio25, Unknown},
    i2c::I2C,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc, IO,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use static_cell::StaticCell;
use unwrap_infallible::UnwrapInfallible;

mod data;
mod gui;
mod history;
mod input;
mod operation_mode;
mod string_format;

use data::{init_calibration, CALIBRATION, DIRECTION, GUI_MENU, HEIGHT};
use history::{lin_reg, Direction, History};

use crate::{
    data::INPUT,
    input::{Inputs, State},
};

async fn poll<T, E>(mut f: impl FnMut() -> nb::Result<T, E>) -> Result<T, E> {
    loop {
        match f() {
            Ok(ok) => break Ok(ok),
            Err(nb::Error::Other(err)) => break Err(err),
            Err(nb::Error::WouldBlock) => embassy_futures::yield_now().await,
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
const HISTORY_COUNT: usize = 32;

type InputPin = hal::gpio::AnyPin<hal::gpio::Input<hal::gpio::PullUp>>;
type OutputPin = hal::gpio::AnyPin<hal::gpio::Output<hal::gpio::PushPull>>;

#[embassy_executor::task]
async fn drive(mut up: OutputPin, mut down: OutputPin) {
    up.set_low().unwrap();
    down.set_low().unwrap();
    loop {
        Timer::after(Duration::from_millis(5)).await;
        let Some(direction) = DIRECTION.planned().await else {
            continue;
        };
        log::info!("starting to drive in direction {direction}");
        match direction {
            Direction::Up => {
                down.set_low().unwrap();
                up.set_high().unwrap();
            }
            Direction::Down => {
                up.set_low().unwrap();
                down.set_high().unwrap();
            }
            Direction::Stopped => {
                up.set_low().unwrap();
                down.set_low().unwrap();
            }
            Direction::ResetDrive => {
                up.set_high().unwrap();
                down.set_high().unwrap();
            }
        }
        DIRECTION.acknowledge(direction).await;
    }
}

#[embassy_executor::task]
async fn read_input(up: InputPin, down: InputPin, pos1: InputPin, pos2: InputPin) {
    struct DebouncedPin {
        pin: InputPin,
        debouncer: DebouncerStateful<u8, debouncr::Repeat4>,
    }

    impl DebouncedPin {
        fn new(pin: InputPin) -> Self {
            let debouncer = debouncr::debounce_stateful_4(false);
            Self { pin, debouncer }
        }

        fn update_input(&mut self, input: &mut State) {
            let active = self.pin.is_low().unwrap_infallible();
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
async fn measure_task(gpio25: Gpio25<Unknown>, analog: AvailableAnalog) {
    measure(gpio25, analog).await.expect("measure task failed");
}

async fn measure(gpio25: Gpio25<Unknown>, analog: AvailableAnalog) -> Result<(), &'static str> {
    let mut adc2_config = AdcConfig::new();
    let mut pin25 = adc2_config.enable_pin(gpio25.into_analog(), Attenuation::Attenuation11dB);
    let mut adc2 =
        ADC::<ADC2>::adc(analog.adc2, adc2_config).map_err(|_| "ADC initialization failed")?;

    init_calibration();

    let mut history = History::<_, HISTORY_COUNT>::new();

    let mut calibration = CALIBRATION.wait().await;

    loop {
        if CALIBRATION.signaled() {
            calibration = CALIBRATION.wait().await;
        }

        // let start = Instant::now();
        let pin25_value = read_sample(&mut adc2, &mut pin25).await?;
        // let duration = start.elapsed();

        let value = calibration.transform(pin25_value);

        history.add(pin25_value);
        let (slope, intercept) = lin_reg(&history);
        let dir = Direction::estimate_from_slope(slope);

        *HEIGHT.lock().await = value;

        //println!(
        //    "{value:?} = PIN25 ADC reading = {pin25_value}, waited {}",
        //    0 // duration.as_millis()
        //);
        //println!("slope = {slope}, intercept = {intercept}, dir = {dir}");
    }
}

async fn read_sample<'a>(
    adc2: &mut ADC<'a, ADC2>,
    pin25: &mut AdcPin<Gpio25<Analog>, ADC2>,
) -> Result<u16, &'static str> {
    let mut samples = heapless::Vec::<_, SAMPLE_COUNT>::new();
    for _ in 0..samples.capacity() {
        let sample = poll(|| adc2.read(pin25))
            .await
            .map_err(|_| "failed to read ADC value")?;

        samples.push(sample).map_err(|_| "failed to store sample")?;
    }

    Ok(compute_median(&mut samples))
}

#[embassy_executor::task]
async fn display_task(i2c: I2C<'static, hal::peripherals::I2C0>) {
    display(i2c).await.expect("display task failed");
}

async fn display(i2c: I2C<'static, hal::peripherals::I2C0>) -> Result<(), &'static str> {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display
        .init()
        .map_err(|_| "display initialization failed")?;

    loop {
        let menu = GUI_MENU.wait().await;

        display.clear();
        menu.display(&mut display).await?;
        display.flush().map_err(|_| "flushing failed")?;
    }
}

#[embassy_executor::task]
async fn run() {
    operation_mode::run().await.expect("run task failed");
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);
    log::info!("init!");
    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    // Disable watchdog timers
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    hal::embassy::init(&clocks, timer_group0.timer0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let analog = peripherals.SENS.split();
    // Create a new peripheral object with the described wiring
    // and standard I2C clock speed
    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio32,
        io.pins.gpio27,
        100u32.kHz(),
        &mut system.peripheral_clock_control,
        &clocks,
    );
    let btn_up = io.pins.gpio18.into_pull_up_input().degrade();
    let btn_down = io.pins.gpio19.into_pull_up_input().degrade();
    let btn_pos1 = io.pins.gpio4.into_pull_up_input().degrade();
    let btn_pos2 = io.pins.gpio2.into_pull_up_input().degrade();

    let up = io.pins.gpio14.into_push_pull_output().degrade();
    let down = io.pins.gpio12.into_push_pull_output().degrade();

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(measure_task(io.pins.gpio25, analog)).unwrap();
        spawner.spawn(display_task(i2c)).unwrap();
        spawner
            .spawn(read_input(btn_up, btn_down, btn_pos1, btn_pos2))
            .unwrap();
        spawner.spawn(drive(up, down)).unwrap();
        spawner.spawn(run()).unwrap();
    });
}

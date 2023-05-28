#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;
use embassy_executor::Executor;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;
use hal::{
    adc::{AdcConfig, Attenuation, ADC, ADC2},
    analog::AvailableAnalog,
    clock::ClockControl,
    i2c::I2C,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc, IO,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use static_cell::StaticCell;

async fn poll<T, E>(mut f: impl FnMut() -> nb::Result<T, E>) -> Result<T, E> {
    loop {
        match f() {
            Ok(ok) => break Ok(ok),
            Err(nb::Error::Other(err)) => break Err(err),
            Err(nb::Error::WouldBlock) => embassy_futures::yield_now().await,
        }
    }
}

type Mapping = (u16, Millimeters);
const FIX_POINTS: [Mapping; 5] = [
    (952, Millimeters(86)),
    (1432, Millimeters(172)),
    (1893, Millimeters(258)),
    (2204, Millimeters(316)),
    (2572, Millimeters(386)),
];

#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
struct Millimeters(u16);

impl Millimeters {
    pub fn _from_adc_reading_simple(reading: u16) -> Self {
        const FACTOR: u64 = 256;
        const SLOPE: u64 = (0.185185185185185 * FACTOR as f64) as _;
        const OFFSET: u64 = (90.2962962962963 * FACTOR as f64) as _;
        let reading: u64 = reading.into();
        let length = (SLOPE * reading - OFFSET) / FACTOR;
        Self(length.try_into().unwrap_or(u16::MAX))
    }

    pub fn from_adc_reading(reading: u16) -> Self {
        match FIX_POINTS.binary_search_by_key(&reading, |x| x.0) {
            Ok(i) => FIX_POINTS[i].1,
            Err(i) => {
                let section = if i == 0 {
                    (FIX_POINTS.first(), FIX_POINTS.get(1))
                } else if i == FIX_POINTS.len() {
                    (FIX_POINTS.get(FIX_POINTS.len() - 2), FIX_POINTS.last())
                } else {
                    (FIX_POINTS.get(i - 1), FIX_POINTS.get(i))
                };

                let &(left_adc, Millimeters(left_mm)) = section.0.unwrap();
                let &(right_adc, Millimeters(right_mm)) = section.1.unwrap();
                let section_height = f64::from(right_mm.abs_diff(left_mm));
                let section_length = f64::from(right_adc.abs_diff(left_adc));
                let slope = section_height / section_length;
                let distance_reading_to_left = f64::from(reading.abs_diff(left_adc));
                let mm_from_left = slope * distance_reading_to_left;
                let mm_from_left = mm_from_left as u16;
                let abs_mm = if reading < left_adc {
                    left_mm - mm_from_left
                } else {
                    left_mm + mm_from_left
                };

                Millimeters(abs_mm)
            }
        }
    }

    pub fn as_cm(self) -> u16 {
        self.0 / 10
    }
}

static HEIGHT: Signal<CriticalSectionRawMutex, Millimeters> = Signal::new();

#[embassy_executor::task]
async fn measure(gpio25: hal::gpio::Gpio25<hal::gpio::Unknown>, analog: AvailableAnalog) {
    let mut adc2_config = AdcConfig::new();
    let mut pin25 = adc2_config.enable_pin(gpio25.into_analog(), Attenuation::Attenuation11dB);
    let mut adc2 = ADC::<ADC2>::adc(analog.adc2, adc2_config).unwrap();

    loop {
        let pin25_value: u16 = poll(|| adc2.read(&mut pin25)).await.unwrap();
        let value = Millimeters::from_adc_reading(pin25_value);
        HEIGHT.signal(value);
        println!("{value:?} = PIN25 ADC reading = {pin25_value}");
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn display(i2c: I2C<'static, hal::peripherals::I2C0>) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // Specify different text styles
    let text_style_big = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    loop {
        let height = HEIGHT.wait().await;
        let mut string = heapless::String::<10>::new();
        write!(&mut string, "{}cm", height.as_cm()).unwrap();
        
        Text::with_alignment(
            &string,
            display.bounding_box().center(),
            text_style_big,
            Alignment::Center,
        )
        .draw(&mut display)
        .unwrap();

        display.flush().unwrap();
        display.clear();

        Timer::after(Duration::from_millis(100)).await;
    }
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    esp_println::println!("Init!");
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

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(measure(io.pins.gpio25, analog)).unwrap();
        spawner.spawn(display(i2c)).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_conversion() {
        //(952, Millimeters(86)),
        //(1432, Millimeters(172)),
        //(1893, Millimeters(258)),
        //(2204, Millimeters(316)),
        //(2572, Millimeters(386)),
    }
}

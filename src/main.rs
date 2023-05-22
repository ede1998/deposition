#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Executor;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_println::println;
use hal::{
    adc::{AdcConfig, Attenuation, ADC, ADC2},
    analog::AvailableAnalog,
    clock::ClockControl,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc, IO,
};
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

const FIX_POINTS: [(u16, Millimeters); 5] = [
    (952, Millimeters(86)),
    (1432, Millimeters(172)),
    (1893, Millimeters(258)),
    (2204, Millimeters(316)),
    (2572, Millimeters(386)),
];

#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
struct Millimeters(u16);

impl Millimeters {
    pub fn from_adc_reading(reading: u16) -> Self {
        match FIX_POINTS.binary_search_by_key(&reading, |x| x.0) {
            Ok(i) => FIX_POINTS[i].1,
            Err(i) => {
                let neighbors = if i == 0 {
                    (None, FIX_POINTS.get(0))
                } else if i == FIX_POINTS.len() {
                    (FIX_POINTS.get(i), None)
                } else {
                    (FIX_POINTS.get(i), FIX_POINTS.get(i + 1))
                };

                let scale = |left, right| {
                    let &(left_adc, Millimeters(left_mm)) = left;
                    let &(right_adc, Millimeters(right_mm)) = right;
                    let slope = (right_mm - left_mm) / (right_adc - left_adc);
                    left_mm + slope * reading
                };

                match neighbors {
                    (Some(left), Some(right)) => Millimeters(scale(left, right)),
                    (Some(left), None) => {
                        let predecessor_left = &FIX_POINTS[i - 1];
                        Millimeters(scale(predecessor_left, left))
                    }
                    (None, Some(right)) => {
                        let successor_right = &FIX_POINTS[1];
                        Millimeters(scale(right, successor_right))
                    }
                    (None, None) => panic!("Missing FIX POINTS!"),
                }
            }
        }
    }
}

static HEIGHT: Signal<CriticalSectionRawMutex, Millimeters> = Signal::new();

#[embassy_executor::task]
async fn measure(io: IO, analog: AvailableAnalog) {
    let mut adc2_config = AdcConfig::new();
    let mut pin25 =
        adc2_config.enable_pin(io.pins.gpio25.into_analog(), Attenuation::Attenuation11dB);
    let mut adc2 = ADC::<ADC2>::adc(analog.adc2, adc2_config).unwrap();

    loop {
        let pin25_value: u16 = poll(|| adc2.read(&mut pin25)).await.unwrap();
        let value = Millimeters::from_adc_reading(pin25_value);
        HEIGHT.signal(value);
        println!("{value:?} = PIN25 ADC reading = {pin25_value}");
        Timer::after(Duration::from_millis(100)).await;
    }
}

//#[embassy_executor::task]
//async fn display() {
//    // Create a new peripheral object with the described wiring
//    // and standard I2C clock speed
//    let i2c = I2C::new(
//        peripherals.I2C0,
//        io.pins.gpio32,
//        io.pins.gpio27,
//        100u32.kHz(),
//        &mut system.peripheral_clock_control,
//        &clocks,
//    );
//
//    // Start timer (5 second interval)
//    timer0.start(5u64.secs());
//
//    // Initialize display
//    let interface = I2CDisplayInterface::new(i2c);
//    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
//        .into_buffered_graphics_mode();
//    display.init().unwrap();
//
//    // Specify different text styles
//    let text_style = MonoTextStyleBuilder::new()
//        .font(&FONT_6X10)
//        .text_color(BinaryColor::On)
//        .build();
//    let text_style_big = MonoTextStyleBuilder::new()
//        .font(&FONT_9X18_BOLD)
//        .text_color(BinaryColor::On)
//        .build();
//
//    loop {
//        // Fill display buffer with a centered text with two lines (and two text
//        // styles)
//        Text::with_alignment(
//            "esp-hal",
//            display.bounding_box().center() + Point::new(0, 0),
//            text_style_big,
//            Alignment::Center,
//        )
//        .draw(&mut display)
//        .unwrap();
//
//        Text::with_alignment(
//            "Chip: ESP32",
//            display.bounding_box().center() + Point::new(0, 14),
//            text_style,
//            Alignment::Center,
//        )
//        .draw(&mut display)
//        .unwrap();
//
//        // Write buffer to display
//        display.flush().unwrap();
//        // Clear display buffer
//        display.clear();
//
//        // Wait 5 seconds
//        block!(timer0.wait()).unwrap();
//
//        // Write single-line centered text "Hello World" to buffer
//        Text::with_alignment(
//            "Hello World!",
//            display.bounding_box().center(),
//            text_style_big,
//            Alignment::Center,
//        )
//        .draw(&mut display)
//        .unwrap();
//
//        // Write buffer to display
//        display.flush().unwrap();
//        // Clear display buffer
//        display.clear();
//
//        // Wait 5 seconds
//        block!(timer0.wait()).unwrap();
//    }
//}

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

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(measure(io, analog)).ok();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_conversion() {
        
    }
}
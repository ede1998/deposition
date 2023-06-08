#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::fmt::Write;
use embassy_executor::Executor;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable, Triangle};
use embedded_graphics::text::{Alignment, Text};
use esp_backtrace as _;
use esp_println::println;
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

fn format<const N: usize>(args: core::fmt::Arguments) -> heapless::String<N> {
    fn format_inner<const N: usize>(args: core::fmt::Arguments) -> heapless::String<N> {
        let mut output = heapless::String::new();
        output
            .write_fmt(args)
            .expect("a formatting trait implementation returned an error");
        output
    }

    args.as_str()
        .map_or_else(|| format_inner(args), heapless::String::from)
}
macro_rules! format {
    ($max:literal, $($arg:tt)*) => {{
        let res = format::<$max>(core::format_args!($($arg)*));
        res
    }}
}

async fn poll<T, E>(mut f: impl FnMut() -> nb::Result<T, E>) -> Result<T, E> {
    loop {
        match f() {
            Ok(ok) => break Ok(ok),
            Err(nb::Error::Other(err)) => break Err(err),
            Err(nb::Error::WouldBlock) => embassy_futures::yield_now().await,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Up,
    Stopped,
    Down,
}

impl Direction {
    pub fn estimate_from_slope(slope: f32) -> Self {
        const STOPPED: f32 = 1.3;
        if slope < -STOPPED {
            Self::Down
        } else if slope > STOPPED {
            Self::Up
        } else {
            Self::Stopped
        }
    }
}

impl core::fmt::Display for Direction {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str(match self {
            Direction::Up => "▲",
            Direction::Stopped => "■",
            Direction::Down => "▼",
        })
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

#[derive(Debug)]
struct History<T, const N: usize>(heapless::Deque<T, N>);

impl<T, const N: usize> History<T, N> {
    pub fn new() -> Self {
        Self(heapless::Deque::new())
    }

    pub fn add(&mut self, value: T) {
        if self.0.len() >= self.0.capacity() {
            self.0.pop_back();
        }
        if self.0.push_front(value).is_err() {
            panic!("Failed to add value?");
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + ExactSizeIterator {
        self.0.iter()
    }
}

/// Compute trend line from history.
///
/// ![formula](https://laas.vercel.app/api/svg?input=slope%20=%20%7Bn%5Csum(xy)%20-%20%5Csum%20x%20%5Csum%20y%20%5Cover%20n%5Csum%20x%5E2%20-%20(%5Csum%20x)%5E2%7D%20%5C%5C%20intercept%20=%20%7B%5Csum%20y%20-%20slope%20*%20%5Csum%20x%20%5Cover%20n%7D)
// slope = {n\sum(xy) - \sum x \sum y \over n\sum x^2 - (\sum x)^2}
// intercept = {\sum y - slope * \sum x \over n}
fn lin_reg<const N: usize>(history: &History<u16, N>) -> (f32, f32) {
    let len = history.iter().len() as f32;
    let x = || (0..history.iter().len()).map(|x| x as f32);
    let y = || history.iter().map(|y| *y as f32);
    let sum_x: f32 = x().sum();
    let sum_y: f32 = y().sum();
    let sum_xx: f32 = x().map(|x| x * x).sum();
    let sum_xy: f32 = x().zip(y()).map(|(x, y)| x * y).sum();

    let slope = (len * sum_xy - sum_x * sum_y) / (len * sum_xx - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / len;

    (slope, intercept)
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

static HEIGHT: Signal<CriticalSectionRawMutex, (Millimeters, Direction)> = Signal::new();

const SAMPLE_COUNT: usize = if cfg!(debug_assertions) { 32 } else { 64 };
const HISTORY_COUNT: usize = 32;

#[embassy_executor::task]
async fn measure_task(gpio25: Gpio25<Unknown>, analog: AvailableAnalog) {
    measure(gpio25, analog).await.expect("measure task failed");
}

async fn measure(gpio25: Gpio25<Unknown>, analog: AvailableAnalog) -> Result<(), &'static str> {
    let mut adc2_config = AdcConfig::new();
    let mut pin25 = adc2_config.enable_pin(gpio25.into_analog(), Attenuation::Attenuation11dB);
    let mut adc2 =
        ADC::<ADC2>::adc(analog.adc2, adc2_config).map_err(|_| "ADC initialization failed")?;

    let mut history = History::<_, HISTORY_COUNT>::new();

    loop {
        let start = Instant::now();
        let pin25_value = read_sample(&mut adc2, &mut pin25).await?;
        let duration = start.elapsed();

        let value = Millimeters::from_adc_reading(pin25_value);

        history.add(pin25_value);
        let (slope, intercept) = lin_reg(&history);
        let dir = Direction::estimate_from_slope(slope);

        HEIGHT.signal((value, dir));

        println!(
            "{value:?} = PIN25 ADC reading = {pin25_value}, waited {}",
            duration.as_millis()
        );
        println!("slope = {slope}, intercept = {intercept}, dir = {dir}");
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

    let text_style_big = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(BinaryColor::On)
        .build();

    let prim_style = PrimitiveStyle::with_fill(BinaryColor::On);

    loop {
        let (height, dir) = HEIGHT.wait().await;
        let string = format!(10, "{:>3}cm", height.as_cm());

        let text = Text::with_alignment(
            &string,
            display.bounding_box().anchor_point(AnchorPoint::Center),
            text_style_big,
            Alignment::Center,
        );

        text.draw(&mut display).map_err(|_| "failed to draw text")?;

        let rect = Rectangle::new(
            text.bounding_box()
                .anchor_point(AnchorPoint::TopLeft)
                .y_axis(),
            Size::new_equal(text.bounding_box().size.height),
        );
        match dir {
            Direction::Up => triangle(rect, true).draw_styled(&prim_style, &mut display),
            Direction::Stopped => rect.draw_styled(&prim_style, &mut display),
            Direction::Down => triangle(rect, false).draw_styled(&prim_style, &mut display),
        }
        .map_err(|_| "failed to draw direction indicator")?;

        display.flush().unwrap();
        display.clear();

        Timer::after(Duration::from_millis(100)).await;
    }
}

fn triangle(bounding_box: Rectangle, point_up: bool) -> Triangle {
    let anchors = if point_up {
        [
            AnchorPoint::BottomLeft,
            AnchorPoint::BottomRight,
            AnchorPoint::TopCenter,
        ]
    } else {
        [
            AnchorPoint::BottomCenter,
            AnchorPoint::TopLeft,
            AnchorPoint::TopRight,
        ]
    };

    let v = anchors.map(|a| bounding_box.anchor_point(a));
    Triangle::new(v[0], v[1], v[2])
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
        spawner.spawn(measure_task(io.pins.gpio25, analog)).unwrap();
        spawner.spawn(display_task(i2c)).unwrap();
    });
}

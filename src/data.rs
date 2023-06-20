use core::cmp::Ordering;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use esp_println::println;
use heapless::Vec;

use crate::{gui::Menu, history::Direction, input::Inputs};

pub static HEIGHT: Mutex<CriticalSectionRawMutex, Millimeters> = Mutex::new(Millimeters(0));
pub static INPUT: Mutex<CriticalSectionRawMutex, Inputs> = Mutex::new(Inputs::new());

pub static GUI_MENU: Signal<CriticalSectionRawMutex, Menu> = Signal::new();

pub static DIRECTION: DirectionControl = DirectionControl::new();

pub struct DirectionControl {
    requested: Mutex<CriticalSectionRawMutex, Direction>,
    current: Mutex<CriticalSectionRawMutex, Direction>,
}

impl DirectionControl {
    pub const fn new() -> Self {
        Self {
            requested: Mutex::new(Direction::Stopped),
            current: Mutex::new(Direction::Stopped),
        }
    }

    pub async fn request(&self, new_direction: Direction) {
        println!("Req: {new_direction}");
        *self.requested.lock().await = new_direction;
    }

    pub async fn get(&self) -> Direction {
        *self.current.lock().await
    }

    pub async fn planned(&self) -> Option<Direction> {
        let (cur, req) = {
            let cur_guard = self.current.lock().await;
            let req_guard = self.requested.lock().await;
            (*cur_guard, *req_guard)
        };

        Some(req).filter(|&req| req != cur)
    }

    pub async fn acknowledge(&self, direction: Direction) {
        *self.current.lock().await = direction;
    }
}

pub static CALIBRATION: Signal<CriticalSectionRawMutex, Calibration> = Signal::new();

pub fn init_calibration() {
    let values = [
        (952, Millimeters::from_mm(86)),
        (1432, Millimeters::from_mm(172)),
        (1893, Millimeters::from_mm(258)),
        (2204, Millimeters::from_mm(316)),
        (2572, Millimeters::from_mm(386)),
    ]
    .into_iter()
    .collect();

    CALIBRATION.signal(Calibration { fix_points: values });
}

type Mapping = (u16, Millimeters);

pub struct Calibration {
    fix_points: Vec<Mapping, 20>,
}

impl Calibration {
    pub fn transform(&self, reading: u16) -> Millimeters {
        match self.fix_points.binary_search_by_key(&reading, |x| x.0) {
            Ok(i) => self.fix_points[i].1,
            Err(i) => {
                let section = if i == 0 {
                    (self.fix_points.first(), self.fix_points.get(1))
                } else if i == self.fix_points.len() {
                    (
                        self.fix_points.get(self.fix_points.len() - 2),
                        self.fix_points.last(),
                    )
                } else {
                    (self.fix_points.get(i - 1), self.fix_points.get(i))
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
                    left_mm.saturating_sub(mm_from_left)
                } else {
                    left_mm.saturating_add(mm_from_left)
                };

                Millimeters(abs_mm)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct Millimeters(u16);

impl Millimeters {
    pub const fn from_mm(value: u16) -> Self {
        Self(value)
    }

    pub const fn cmp_fuzzy_eq(self, other: Self) -> core::cmp::Ordering {
        const ALLOWED_DELTA: u16 = 2;
        let left = self.0;
        let right = other.0;
        if left.abs_diff(right) < ALLOWED_DELTA {
            Ordering::Equal
        } else if left < right {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    //pub fn _from_adc_reading_simple(reading: u16) -> Self {
    //    const FACTOR: u64 = 256;
    //    const SLOPE: u64 = (0.185185185185185 * FACTOR as f64) as _;
    //    const OFFSET: u64 = (90.2962962962963 * FACTOR as f64) as _;
    //    let reading: u64 = reading.into();
    //    let length = (SLOPE * reading - OFFSET) / FACTOR;
    //    Self(length.try_into().unwrap_or(u16::MAX))
    //}

    pub fn as_cm(self) -> u16 {
        self.0 / 10
    }
}

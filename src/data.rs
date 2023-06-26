use core::cmp::Ordering;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::{gui::MainMenu, input::Inputs};

pub static HEIGHT: Mutex<CriticalSectionRawMutex, Millimeters> = Mutex::new(Millimeters(0));
pub static RAW_HEIGHT: Signal<CriticalSectionRawMutex, u16> = Signal::new();
pub static INPUT: Mutex<CriticalSectionRawMutex, Inputs> = Mutex::new(Inputs::new());

pub static GUI_MENU: Signal<CriticalSectionRawMutex, MainMenu> = Signal::new();

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
        log::debug!("driving in direction {new_direction} requested.");
        *self.requested.lock().await = new_direction;
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

type Mapping = (u16, Millimeters);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Calibration {
    fix_points: Vec<Mapping, 20>,
}

impl core::ops::Deref for Calibration {
    type Target = Vec<Mapping, 20>;

    fn deref(&self) -> &Self::Target {
        &self.fix_points
    }
}

impl Calibration {
    pub const fn new() -> Self {
        Self {
            fix_points: Vec::new(),
        }
    }

    pub fn insert(&mut self, adc: u16, height: Millimeters) -> Result<(), &'static str> {
        let position = self.fix_points.binary_search_by_key(&adc, |(adc, _)| *adc);
        match position {
            Ok(index) => {
                self.fix_points[index] = (adc, height);
            }
            Err(index) => {
                self.fix_points
                    .insert(index, (adc, height))
                    .map_err(|_| "too many calibration points")?;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.fix_points.clear();
    }

    pub fn remove(&mut self, index: usize) {
        self.fix_points.remove(index);
    }
    pub fn transform(&self, reading: u16) -> Millimeters {
        if self.fix_points.is_empty() {
            return Millimeters::from_mm(0);
        }

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

#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Serialize, Deserialize)]
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

    pub fn increase(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub fn decrease(self) -> Self {
        Self(self.0.saturating_sub(1))
    }

    pub fn as_cm(self) -> u16 {
        self.0 / 10
    }

    pub fn as_mm(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Stopped,
    Down,
    ResetDrive,
}

impl core::fmt::Display for Direction {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_str(match self {
            Direction::Up => "+",
            Direction::Stopped => "0",
            Direction::Down => "-",
            Direction::ResetDrive => "R",
        })
    }
}

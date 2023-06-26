use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};

use crate::data::{Calibration, Millimeters};

use super::{
    widgets::{footer, MenuContent},
    MainMenu, Menu,
};

pub struct CalibrationOptions {
    pub menu: Menu<CalibrationMenu>,
}

impl CalibrationOptions {
    pub async fn display<D>(&self, display: &mut D) -> Result<(), &'static str>
    where
        D: DrawTarget<Color = BinaryColor> + Dimensions,
    {
        self.menu
            .display::<{ CalibrationMenu::MENU_STRING_LENGTH }>(display)
            .await?;
        let string = match self.menu.content.selected {
            Selected::AddNew => "+- nav | pos1 exit | pos2 sel",
            Selected::RemoveAll | Selected::ShowOne => "+- nav | pos1 exit | pos2 del",
        };
        footer(display, string).await?;
        Ok(())
    }
}

impl From<CalibrationOptions> for MainMenu {
    fn from(value: CalibrationOptions) -> Self {
        Self::Calibration(value)
    }
}

#[derive(Debug, Clone)]
pub struct CalibrationMenu {
    items: Calibration,
    shown_index: u8,
    selected: Selected,
}

impl CalibrationMenu {
    pub fn new(items: Calibration) -> Self {
        Self {
            items,
            shown_index: 0,
            selected: Selected::AddNew,
        }
    }

    pub fn update_calibration(&mut self, items: &Calibration) {
        self.items = items.clone();

        let max_index = self
            .items
            .len()
            .try_into()
            .unwrap_or(u8::MAX)
            .saturating_sub(1);
        if self.shown_index > max_index {
            self.shown_index = max_index;
        }

        if self.items.is_empty() {
            self.selected = Selected::AddNew;
        }
    }

    pub fn selected(&self) -> Selected {
        self.selected
    }

    pub fn shown_index(&self) -> Option<usize> {
        let non_empty = !self.items.is_empty();
        non_empty.then_some(self.shown_index.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selected {
    AddNew,
    RemoveAll,
    ShowOne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationItem {
    AddNew,
    RemoveAll,
    ShowOne {
        index: u8,
        adc: u16,
        height: Millimeters,
    },
}

impl core::fmt::Display for CalibrationItem {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CalibrationItem::AddNew => f.write_str("Add new calibration point"),
            CalibrationItem::RemoveAll => f.write_str("Remove all calibration points"),
            CalibrationItem::ShowOne { index, adc, height } => {
                write!(f, "{index}) {adc} <=> {}mm", height.as_mm())
            }
        }
    }
}

impl MenuContent for CalibrationMenu {
    const MENU_STRING_LENGTH: usize = 120;

    type Iter = <heapless::Vec<CalibrationItem, 3> as core::iter::IntoIterator>::IntoIter;
    type IterItem = CalibrationItem;

    fn iter(&self) -> Self::Iter {
        let inner = || {
            let mut items = heapless::Vec::new();
            items.push(CalibrationItem::AddNew)?;

            if !self.items.is_empty() {
                items.push(CalibrationItem::RemoveAll)?;
                let (adc, height) = self.items[usize::from(self.shown_index)];
                items.push(CalibrationItem::ShowOne {
                    index: self.shown_index,
                    adc,
                    height,
                })?;
            }
            Ok::<_, CalibrationItem>(items.into_iter())
        };

        inner().expect("push of at most 3 items into vec with capacity 3")
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = match self.selected {
            Selected::AddNew => Selected::RemoveAll,
            Selected::RemoveAll => {
                self.shown_index = 0;
                Selected::ShowOne
            }
            Selected::ShowOne => {
                let new_index = self.shown_index + 1;
                if self.items.len() == new_index.into() {
                    Selected::AddNew
                } else {
                    self.shown_index = new_index;
                    Selected::ShowOne
                }
            }
        };
    }

    fn prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = match self.selected {
            Selected::AddNew => {
                self.shown_index = self.items.len().try_into().unwrap_or(u8::MAX) - 1;
                Selected::ShowOne
            }
            Selected::RemoveAll => Selected::AddNew,
            Selected::ShowOne => {
                if self.shown_index == 0 {
                    Selected::RemoveAll
                } else {
                    self.shown_index -= 1;
                    Selected::ShowOne
                }
            }
        };
    }

    fn is_selected(&self, item: &Self::IterItem) -> bool {
        matches!(
            (item, &self.selected),
            (CalibrationItem::AddNew, Selected::AddNew)
                | (CalibrationItem::RemoveAll, Selected::RemoveAll)
                | (CalibrationItem::ShowOne { .. }, Selected::ShowOne)
        )
    }
}

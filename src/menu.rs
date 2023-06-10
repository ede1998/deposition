

pub enum Menu {
    Start(Start),
    Options(Options),
}

pub struct Start {

}

pub enum Options {
    Calibration(Calibration),
    StoreHeight1,
    StoreHeight2,
    ResetDrive,
}

pub enum ResetDrive {
    Selected,
    Running,
    Finished,
}


pub struct Calibration {
// List height points
}

pub struct CalibrationHeight {
    // current

}

/// Describes the input button that was pressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Up,
    Down,
    /// Doubles as Back or Cancel
    Height1,
    /// Doubles as Ok or Store
    Height2,
}

pub trait Navigate {
    type Output: Navigate;
    fn run(self, input: Input) -> Self::Output;
}
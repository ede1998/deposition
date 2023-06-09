
type Mapping = (u16, Millimeters);
const FIX_POINTS: [Mapping; 5] = [
    (952, Millimeters::from_mm(86)),
    (1432, Millimeters::from_mm(172)),
    (1893, Millimeters::from_mm(258)),
    (2204, Millimeters::from_mm(316)),
    (2572, Millimeters::from_mm(386)),
];


#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq)]
pub struct Millimeters(u16);

impl Millimeters {
    pub const fn from_mm(value: u16) -> Self {
        Self(value)
    }

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
                    left_mm.saturating_sub(mm_from_left)
                } else {
                    left_mm.saturating_add(mm_from_left)
                };

                Millimeters(abs_mm)
            }
        }
    }

    pub fn as_cm(self) -> u16 {
        self.0 / 10
    }
}

#[derive(Debug)]
pub struct History<T, const N: usize>(heapless::Deque<T, N>);

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
pub fn lin_reg<const N: usize>(history: &History<u16, N>) -> (f32, f32) {
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


#[derive(Debug, Clone, Copy)]
pub enum Direction {
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

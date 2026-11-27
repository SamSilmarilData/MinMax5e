//! One value per target AC, for every AC from 10 to 25.

use std::ops::{Index, RangeInclusive};

/// The target ACs that get a lane.
pub const LANE_ACS: RangeInclusive<i32> = 10..=25;

/// The number of lanes: one per AC in `LANE_ACS`.
pub const LANES: usize = (*LANE_ACS.end() - *LANE_ACS.start() + 1).unsigned_abs() as usize;

/// One value per target AC in `LANE_ACS`: a value at each Lane of one kit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcLanes<T>([T; LANES]);

impl<T> AcLanes<T> {
    /// The lanes holding `value(ac)` for each AC.
    pub fn from_fn(mut value: impl FnMut(i32) -> T) -> AcLanes<T> {
        AcLanes(std::array::from_fn(|i| value(lane_ac(i))))
    }

    /// The value at `ac`, or `None` outside `LANE_ACS`.
    pub fn get(&self, ac: i32) -> Option<&T> {
        let i = usize::try_from(ac - LANE_ACS.start()).ok()?;
        self.0.get(i)
    }

    /// `(ac, value)` for each lane, in increasing AC.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &T)> {
        self.0.iter().enumerate().map(|(i, v)| (lane_ac(i), v))
    }

    /// The lanes holding `f(value)`.
    #[must_use]
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> AcLanes<U> {
        AcLanes(self.0.map(f))
    }
}

impl<T> Index<i32> for AcLanes<T> {
    type Output = T;

    /// # Panics
    /// If `ac` is outside `LANE_ACS`.
    fn index(&self, ac: i32) -> &T {
        self.get(ac)
            .unwrap_or_else(|| panic!("AC {ac} has no lane; lanes cover {LANE_ACS:?}"))
    }
}

fn lane_ac(i: usize) -> i32 {
    LANE_ACS.start() + i32::try_from(i).expect("a lane index fits in i32")
}

//! Exact damage distributions over integer totals.

use crate::Moments;

/// A Damage Distribution: the probability of every integer damage total.
///
/// The masses need not sum to 1. A partial distribution (one branch of an attack, say)
/// holds only that branch's probability, and `mix` with weight 1 adds branches back
/// together. The statistics read the masses as probabilities, so they are the usual ones
/// when the mass is 1; on a partial distribution `mean` is the partial expectation.
///
/// Every operation runs in a fixed order, so the same inputs give the same bits on every
/// target (ADR-0037, ADR-0050).
#[derive(Debug, Clone, PartialEq)]
pub struct Pmf {
    /// The damage total of `mass[0]`.
    offset: i32,
    /// The mass at `offset`, `offset + 1`, …. Never empty; the ends are non-zero unless the
    /// whole distribution is a single zero entry.
    mass: Box<[f64]>,
}

impl Pmf {
    /// Certain damage `value`.
    pub fn point(value: i32) -> Pmf {
        Pmf {
            offset: value,
            mass: Box::new([1.0]),
        }
    }

    /// One die with faces 1 to `sides`, each equally likely.
    ///
    /// # Panics
    /// If `sides` is 0.
    pub fn die(sides: u32) -> Pmf {
        assert!(sides > 0, "a die has at least one face");
        let p = 1.0 / f64::from(sides);
        Pmf {
            offset: 1,
            mass: vec![p; to_index(sides)].into_boxed_slice(),
        }
    }

    /// The sum of `count` independent copies of this distribution; `count = 0` gives 0.
    #[must_use]
    pub fn sum_of(&self, count: u32) -> Pmf {
        (0..count).fold(Pmf::point(0), |acc, _| acc.convolve(self))
    }

    /// The distribution of the sum of two independent damage totals.
    #[must_use]
    pub fn convolve(&self, other: &Pmf) -> Pmf {
        let mut mass = vec![0.0; self.mass.len() + other.mass.len() - 1];
        for (i, &a) in self.mass.iter().enumerate() {
            for (j, &b) in other.mass.iter().enumerate() {
                mass[i + j] += a * b;
            }
        }
        Pmf::from_parts(self.offset + other.offset, &mass)
    }

    /// The weighted sum of the branches: `Σ weight · branch`.
    ///
    /// With weights that sum to 1 over full distributions this is a mixture; with weight 1
    /// over partial distributions it adds the branches together.
    ///
    /// # Panics
    /// If `branches` is empty.
    pub fn mix(branches: &[(f64, &Pmf)]) -> Pmf {
        let lo = branches.iter().map(|(_, b)| b.offset).min();
        let hi = branches.iter().map(|(_, b)| b.max()).max();
        let (Some(lo), Some(hi)) = (lo, hi) else {
            panic!("a mix needs at least one branch");
        };
        let mut mass = vec![0.0; span(lo, hi)];
        for &(weight, branch) in branches {
            let start = to_index(branch.offset - lo);
            for (slot, &p) in mass[start..].iter_mut().zip(branch.mass.iter()) {
                *slot += weight * p;
            }
        }
        Pmf::from_parts(lo, &mass)
    }

    /// Every total moved by the flat modifier `by`.
    #[must_use]
    pub fn shift(&self, by: i32) -> Pmf {
        Pmf {
            offset: self.offset + by,
            mass: self.mass.clone(),
        }
    }

    /// Every total below `min` counted as `min`.
    #[must_use]
    pub fn floor_at(&self, min: i32) -> Pmf {
        self.clamp(min, self.max().max(min))
    }

    /// Every total above `max` counted as `max`, as when damage is capped at a target's HP.
    #[must_use]
    pub fn cap_at(&self, max: i32) -> Pmf {
        self.clamp(self.offset.min(max), max)
    }

    /// The total mass: 1 for a full distribution.
    pub fn mass(&self) -> f64 {
        self.mass.iter().sum()
    }

    /// The probability of exactly `value`.
    pub fn prob(&self, value: i32) -> f64 {
        usize::try_from(i64::from(value) - i64::from(self.offset))
            .ok()
            .and_then(|i| self.mass.get(i))
            .copied()
            .unwrap_or(0.0)
    }

    /// The lowest and highest totals that can occur.
    pub fn support(&self) -> (i32, i32) {
        (self.offset, self.max())
    }

    /// The expected total, `Σ x · p(x)`.
    pub fn mean(&self) -> f64 {
        self.values().map(|(x, p)| f64::from(x) * p).sum()
    }

    /// The variance, `Σ (x − mean)² · p(x)`, taken about the mean so that a large flat
    /// modifier costs no precision.
    pub fn variance(&self) -> f64 {
        let mean = self.mean();
        self.values()
            .map(|(x, p)| {
                let d = f64::from(x) - mean;
                d * d * p
            })
            .sum()
    }

    /// The mean and second moment.
    pub fn moments(&self) -> Moments {
        let second = self
            .values()
            .map(|(x, p)| {
                let x = f64::from(x);
                x * x * p
            })
            .sum();
        Moments::new(self.mean(), second)
    }

    /// The probability that the total is at least `value`, summed from the top so that
    /// small tails keep their precision.
    pub fn tail(&self, value: i32) -> f64 {
        self.values()
            .rev()
            .take_while(|&(x, _)| x >= value)
            .map(|(_, p)| p)
            .sum()
    }

    /// The smallest total whose cumulative probability reaches `q`; the highest total if
    /// the mass never reaches it.
    pub fn quantile(&self, q: f64) -> i32 {
        let mut cumulative = 0.0;
        for (x, p) in self.values() {
            cumulative += p;
            if cumulative >= q {
                return x;
            }
        }
        self.max()
    }

    /// `(total, mass)` pairs in increasing order of total.
    fn values(&self) -> impl DoubleEndedIterator<Item = (i32, f64)> + '_ {
        self.mass
            .iter()
            .enumerate()
            .map(|(i, &p)| (self.offset + to_i32(i), p))
    }

    fn max(&self) -> i32 {
        self.offset + to_i32(self.mass.len() - 1)
    }

    /// Totals below `lo` counted as `lo`, above `hi` as `hi`. Requires `lo <= hi`.
    fn clamp(&self, lo: i32, hi: i32) -> Pmf {
        let mut mass = vec![0.0; span(lo, hi)];
        for (x, p) in self.values() {
            mass[to_index(x.clamp(lo, hi) - lo)] += p;
        }
        Pmf::from_parts(lo, &mass)
    }

    /// Trims zero mass from both ends, keeping at least one entry.
    fn from_parts(offset: i32, mass: &[f64]) -> Pmf {
        let first = mass.iter().position(|&p| p != 0.0);
        let Some(first) = first else {
            return Pmf {
                offset,
                mass: Box::new([0.0]),
            };
        };
        let last = mass.iter().rposition(|&p| p != 0.0).unwrap_or(first);
        Pmf {
            offset: offset + to_i32(first),
            mass: mass[first..=last].into(),
        }
    }
}

/// The number of totals from `lo` to `hi` inclusive.
fn span(lo: i32, hi: i32) -> usize {
    to_index(hi - lo) + 1
}

fn to_index<T: TryInto<usize>>(n: T) -> usize {
    n.try_into()
        .unwrap_or_else(|_| panic!("a damage index is a non-negative usize"))
}

fn to_i32(n: usize) -> i32 {
    i32::try_from(n).expect("a damage support fits in i32")
}

//! The exact oracle for `mm-dice` (ADR-0023): damage distributions and d20 odds in
//! `BigRational` arithmetic.
//!
//! It is written independently of `mm-dice`. Distributions are maps from total to mass,
//! and the d20 odds come from classifying every equally likely face (or pair of faces)
//! rather than from counting formulas.

use std::collections::BTreeMap;

use mm_dice::{AttackRoll, RollMode};
use num_bigint::BigInt;
use num_traits::{Signed as _, Zero as _};

/// An exact rational number.
pub type Ratio = num_rational::BigRational;

fn int(n: i32) -> Ratio {
    Ratio::from_integer(BigInt::from(n))
}

/// A float as the exact rational it denotes.
///
/// # Panics
/// If `x` is not finite.
pub fn exact(x: f64) -> Ratio {
    Ratio::from_float(x).unwrap_or_else(|| panic!("{x} is not finite"))
}

/// Whether `got` is within `relative` of `exact`, measured in exact arithmetic. When the
/// exact value is 0, `got` must be within `relative` of 0.
pub fn within_relative(got: f64, exact_value: &Ratio, relative: f64) -> bool {
    let error = (exact(got) - exact_value).abs();
    let scale = if exact_value.is_zero() {
        int(1)
    } else {
        exact_value.abs()
    };
    error <= exact(relative) * scale
}

/// The exact odds `[miss, hit, crit]` of an attack roll, `hit` excluding crits.
///
/// Every face is classified by the rules directly: a natural 1 misses, a natural 20 or a
/// face at or above `crit_on` crits (a crit always hits; research 03 Open Q1), and any
/// other face hits if face + bonus reaches the AC. With advantage the better of two faces
/// counts, with disadvantage the worse.
pub fn exact_odds(roll: AttackRoll) -> [Ratio; 3] {
    // Ordered so that the better of two faces is the larger outcome.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Outcome {
        Miss,
        Hit,
        Crit,
    }
    let outcome = |face: i32| {
        if face == 1 {
            Outcome::Miss
        } else if face == 20 || face >= roll.crit_on {
            Outcome::Crit
        } else if face + roll.bonus >= roll.ac {
            Outcome::Hit
        } else {
            Outcome::Miss
        }
    };
    let rolls: Vec<Outcome> = match roll.mode {
        RollMode::Normal => (1..=20).map(outcome).collect(),
        RollMode::Advantage | RollMode::Disadvantage => (1..=20)
            .flat_map(|a| (1..=20).map(move |b| (outcome(a), outcome(b))))
            .map(|(a, b)| {
                if roll.mode == RollMode::Advantage {
                    a.max(b)
                } else {
                    a.min(b)
                }
            })
            .collect(),
    };
    let total = BigInt::from(rolls.len());
    [Outcome::Miss, Outcome::Hit, Outcome::Crit].map(|o| {
        let count = rolls.iter().filter(|&&r| r == o).count();
        Ratio::new(BigInt::from(count), total.clone())
    })
}

/// An exact damage distribution: the mass at each integer total.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactPmf(BTreeMap<i32, Ratio>);

impl ExactPmf {
    /// Certain damage `value`.
    pub fn point(value: i32) -> ExactPmf {
        ExactPmf(BTreeMap::from([(value, int(1))]))
    }

    /// One die with faces 1 to `sides`.
    ///
    /// # Panics
    /// If `sides` is 0 or does not fit in `i32`.
    pub fn die(sides: u32) -> ExactPmf {
        let n = i32::try_from(sides).expect("die size fits in i32");
        assert!(n > 0, "a die has at least one face");
        ExactPmf(
            (1..=n)
                .map(|face| (face, Ratio::new(BigInt::from(1), BigInt::from(n))))
                .collect(),
        )
    }

    /// The sum of `count` independent copies.
    #[must_use]
    pub fn sum_of(&self, count: u32) -> ExactPmf {
        (0..count).fold(ExactPmf::point(0), |acc, _| acc.convolve(self))
    }

    /// The distribution of the sum of two independent totals.
    #[must_use]
    pub fn convolve(&self, other: &ExactPmf) -> ExactPmf {
        let mut out = BTreeMap::new();
        for (x, p) in &self.0 {
            for (y, q) in &other.0 {
                *out.entry(x + y).or_insert_with(Ratio::zero) += p * q;
            }
        }
        ExactPmf(out)
    }

    /// `Σ weight · branch`.
    pub fn mix(branches: &[(Ratio, &ExactPmf)]) -> ExactPmf {
        let mut out = BTreeMap::new();
        for (weight, branch) in branches {
            for (x, p) in &branch.0 {
                *out.entry(*x).or_insert_with(Ratio::zero) += weight * p;
            }
        }
        ExactPmf(out)
    }

    /// Every total moved by `by`.
    #[must_use]
    pub fn shift(&self, by: i32) -> ExactPmf {
        self.map_totals(|x| x + by)
    }

    /// Totals below `min` counted as `min`.
    #[must_use]
    pub fn at_least(&self, min: i32) -> ExactPmf {
        self.map_totals(|x| x.max(min))
    }

    /// Totals above `max` counted as `max`.
    #[must_use]
    pub fn at_most(&self, max: i32) -> ExactPmf {
        self.map_totals(|x| x.min(max))
    }

    pub fn mass(&self) -> Ratio {
        self.0.values().sum()
    }

    pub fn prob(&self, value: i32) -> Ratio {
        self.0.get(&value).cloned().unwrap_or_else(Ratio::zero)
    }

    /// The lowest and highest totals with non-zero mass.
    ///
    /// # Panics
    /// If every mass is zero.
    pub fn support(&self) -> (i32, i32) {
        let mut nonzero = self.0.iter().filter(|(_, p)| !p.is_zero()).map(|(x, _)| *x);
        let lo = nonzero.next().expect("some total has mass");
        (lo, nonzero.next_back().unwrap_or(lo))
    }

    /// `Σ x · p(x)`.
    pub fn mean(&self) -> Ratio {
        self.0.iter().map(|(x, p)| int(*x) * p).sum()
    }

    /// `Σ x² · p(x)`.
    pub fn second(&self) -> Ratio {
        self.0.iter().map(|(x, p)| int(*x) * int(*x) * p).sum()
    }

    /// `Σ (x − mean)² · p(x)`.
    pub fn variance(&self) -> Ratio {
        let mean = self.mean();
        self.0
            .iter()
            .map(|(x, p)| {
                let d = int(*x) - &mean;
                &d * &d * p
            })
            .sum()
    }

    /// The mass at totals of at least `value`.
    pub fn tail(&self, value: i32) -> Ratio {
        self.0.range(value..).map(|(_, p)| p).sum()
    }

    /// The smallest total whose cumulative mass reaches `q`; the highest total if none.
    pub fn quantile(&self, q: f64) -> i32 {
        let q = exact(q);
        let mut cumulative = Ratio::zero();
        for (x, p) in &self.0 {
            cumulative += p;
            if cumulative >= q {
                return *x;
            }
        }
        self.support().1
    }

    /// Whether some cumulative mass is within `relative` of `q`, where a float quantile
    /// may round to either side.
    pub fn cumulative_near(&self, q: f64, relative: f64) -> bool {
        let mut cumulative = Ratio::zero();
        self.0.values().any(|p| {
            cumulative += p;
            within_relative(q, &cumulative, relative)
        })
    }

    fn map_totals(&self, f: impl Fn(i32) -> i32) -> ExactPmf {
        let mut out = BTreeMap::new();
        for (x, p) in &self.0 {
            *out.entry(f(*x)).or_insert_with(Ratio::zero) += p;
        }
        ExactPmf(out)
    }
}

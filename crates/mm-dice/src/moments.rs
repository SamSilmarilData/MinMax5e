//! The first two moments of a damage total, for when the full distribution is not needed.

/// The mean `E[X]` and second moment `E[X²]` of a damage total. Get them from
/// `Pmf::moments` or `Moments::point`, then combine with `add_independent` and `mix`.
///
/// Like `Pmf`, the moments may describe a partial distribution: `mix` with weight 1 adds
/// branches. `add_independent` is for independent totals of full distributions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moments {
    mean: f64,
    second: f64,
}

impl Moments {
    /// The moments `E[X] = mean` and `E[X²] = second`; `Pmf::moments` keeps them consistent.
    pub(crate) fn new(mean: f64, second: f64) -> Moments {
        Moments { mean, second }
    }

    /// Certain damage `value`.
    pub fn point(value: i32) -> Moments {
        let x = f64::from(value);
        Moments::new(x, x * x)
    }

    /// The moments of the sum of two independent totals:
    /// `E[(X+Y)²] = E[X²] + 2·E[X]·E[Y] + E[Y²]`.
    ///
    /// Ported from `fold` in the tape spike (`spikes/tape-spike/src/main.rs`).
    #[must_use]
    pub fn add_independent(self, other: Moments) -> Moments {
        Moments::new(
            self.mean + other.mean,
            self.second + 2.0 * self.mean * other.mean + other.second,
        )
    }

    /// The weighted sum of the branches' moments: `Σ weight · branch`.
    pub fn mix(branches: &[(f64, Moments)]) -> Moments {
        branches
            .iter()
            .fold(Moments::new(0.0, 0.0), |acc, &(weight, branch)| {
                Moments::new(
                    acc.mean + weight * branch.mean,
                    acc.second + weight * branch.second,
                )
            })
    }

    /// `E[X]`.
    pub fn mean(self) -> f64 {
        self.mean
    }

    /// `E[X²]`.
    pub fn second(self) -> f64 {
        self.second
    }

    /// `E[X²] − E[X]²`. This loses precision when the mean is large against the spread;
    /// `Pmf::variance` does not.
    pub fn variance(self) -> f64 {
        self.second - self.mean * self.mean
    }
}

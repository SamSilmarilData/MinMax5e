//! The distribution kernel: exact damage arithmetic below the engine (architecture §8,
//! build-order step 1).
//!
//! The public seam:
//! - [`Pmf`]: an exact Damage Distribution over integer totals. Build one from dice and
//!   flat modifiers (`die`, `point`, `sum_of`, `convolve`, `shift`, `at_least`, `at_most`),
//!   combine branches with `mix`, and read `mass`, `mean`, `variance`, `quantile` and
//!   `tail`.
//! - [`Moments`] (from `Pmf::moments`): `E[X]` and `E[X²]` alone, with `add_independent` and `mix`.
//! - [`AttackRoll::odds`]: the miss, hit and crit odds of a d20 attack roll, counted
//!   exactly in integers and divided once.
//! - [`AcLanes`]: one value per target AC from 10 to 25 ([`LANE_ACS`]).
//!
//! Values are `f64`, using IEEE 754 basic operations only, in a fixed order, so results
//! are bit-identical across targets (ADR-0023, ADR-0037, ADR-0050). The crate has no
//! dependencies (ADR-0034). `mm-testkit`'s `BigRational` oracle checks it from the
//! integration tests.

mod d20;
mod lanes;
mod moments;
mod pmf;

pub use d20::{AttackRoll, OutcomeOdds, RollMode};
pub use lanes::{AcLanes, LANE_ACS, LANES};
pub use moments::Moments;
pub use pmf::Pmf;

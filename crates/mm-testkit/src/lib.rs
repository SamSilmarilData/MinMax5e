//! Test oracles and fixtures for `MinMax5e`, used only from integration tests (ADR-0034,
//! ADR-0056; `cargo xtask deps` enforces it).
//!
//! - [`rational`]: exact `BigRational` arithmetic, the oracle `mm-dice` is checked against
//!   (ADR-0023).
//! - [`fixtures`]: where the synthetic fixtures live (source `tst`, ADR-0028).
//! - [`tolerance`]: the float bounds tests compare within, and their sources.
//!
//! `TestRules` arrives with #27. Until then `mm-engine` compiles Builds only under test
//! through `mm-cli`, because `cargo xtask deps` keeps rules crates out of `mm-engine`,
//! dev-dependencies included; its own integration tests evaluate raw Kit Shapes and Kit
//! Params. The Monte Carlo oracle and the proptest strategies arrive with #25 and #26.

pub mod fixtures;
pub mod rational;
pub mod tolerance;

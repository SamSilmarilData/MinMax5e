//! Test oracles and fixtures for `MinMax5e`, used only from integration tests (ADR-0034,
//! ADR-0056; `cargo xtask deps` enforces it).
//!
//! - [`rational`]: exact `BigRational` arithmetic, the oracle `mm-dice` is checked against
//!   (ADR-0023).
//!
//! The fixtures, the `TestRules` ruleset, the Monte Carlo oracle and the proptest
//! strategies for Kit Shapes arrive with the engine (architecture §11.1, steps 2–3).

pub mod rational;

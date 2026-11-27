//! Helpers shared by the integration tests.

/// The bound spec #12 sets for float results against exact arithmetic. Hand-computed
/// literals here are exact fractions, so the same bound applies to them.
pub const RELATIVE: f64 = 1e-12;

/// `got` is within `RELATIVE` of `expected`.
#[track_caller]
#[allow(
    dead_code,
    reason = "each test crate compiles this module; the oracle test does not call it"
)]
pub fn assert_close(got: f64, expected: f64) {
    assert!(
        (got - expected).abs() <= RELATIVE * expected.abs(),
        "got {got}, expected {expected}"
    );
}

//! Helpers shared by the integration tests.

/// The bound spec #12 sets for float results against exact arithmetic. Hand-computed
/// literals here are exact fractions, so the same bound applies to them.
pub const RELATIVE: f64 = 1e-12;

/// `got` is within `RELATIVE` of `expected`.
#[track_caller]
#[allow(
    dead_code,
    reason = "each test crate compiles this module; not every one calls this"
)]
pub fn assert_close(got: f64, expected: f64) {
    assert!(
        (got - expected).abs() <= RELATIVE * expected.abs(),
        "got {got}, expected {expected}"
    );
}

/// `got` rounds to `expected`, a published figure whose last stated digit is in the place
/// of `unit`: it is within half a unit.
#[track_caller]
#[allow(
    dead_code,
    reason = "each test crate compiles this module; not every one calls this"
)]
pub fn assert_rounds_to(what: &str, got: f64, expected: f64, unit: f64) {
    assert!(
        (got - expected).abs() <= unit / 2.0,
        "{what}: got {got}, expected {expected} to the nearest {unit}"
    );
}

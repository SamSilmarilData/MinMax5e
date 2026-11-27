//! Float tolerances for tests, with the source of each bound (spec #12, "Testing
//! Decisions": floats compare bit for bit, or within a stated bound; never an ad-hoc
//! epsilon).

/// The bound spec #12 sets for float results against exact arithmetic. Hand-computed
/// literals here are exact fractions, so the same bound applies to them.
pub const RELATIVE: f64 = 1e-12;

/// Asserts `got` is within `RELATIVE` of `expected`.
///
/// # Panics
/// If it is not.
#[track_caller]
pub fn assert_close(got: f64, expected: f64) {
    assert!(
        (got - expected).abs() <= RELATIVE * expected.abs(),
        "got {got}, expected {expected}"
    );
}

/// `got` rounds to `expected`, a published figure whose last stated digit is in the place
/// of `unit`: it is within half a unit.
///
/// # Panics
/// If it does not.
#[track_caller]
pub fn assert_rounds_to(what: &str, got: f64, expected: f64, unit: f64) {
    assert!(
        (got - expected).abs() <= unit / 2.0,
        "{what}: got {got}, expected {expected} to the nearest {unit}"
    );
}

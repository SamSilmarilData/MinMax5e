//! The synthetic fixtures (`crates/mm-testkit/fixtures/`, described in its README).

use std::path::PathBuf;

/// The fixtures directory: `pack/` is a pack directory, `builds/` holds Build files, and
/// `malformed/` holds one broken pack directory per kind of mistake.
pub fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

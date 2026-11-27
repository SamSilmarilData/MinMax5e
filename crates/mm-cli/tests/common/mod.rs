//! Helpers the `mm-cli` process tests share.

use std::process::Output;

/// The stdout of a run that must succeed.
pub fn stdout_of(output: &Output) -> String {
    assert!(
        output.status.success(),
        "mm-cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

/// The stderr of a run that must fail.
pub fn stderr_of_failure(output: &Output) -> String {
    assert!(!output.status.success(), "mm-cli should have failed");
    String::from_utf8(output.stderr.clone()).expect("UTF-8 output")
}

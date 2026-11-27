//! `cargo xtask xhash` as a process (ADR-0050).
//!
//! These build `mm-cli` twice and need `wasmtime` on PATH, so they are `#[ignore]`d in
//! the ordinary test run. CI's macOS `xhash` job runs them with `--ignored`.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::xtask;

/// The `native` and `wasm32-wasip1` hashes `xhash` printed, from its two summary lines.
fn hashes(stdout: &str) -> (String, String) {
    let hash_on = |label: &str| {
        stdout
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap_or_else(|| panic!("no {label:?} line in:\n{stdout}"))
            .trim()
            .to_owned()
    };
    (hash_on("native "), hash_on("wasm32-wasip1 "))
}

#[test]
#[ignore = "builds mm-cli for two targets and needs wasmtime; CI's xhash job runs it"]
fn native_and_wasm_hash_alike_and_xhash_passes() {
    let run = xtask(&["xhash"]);
    run.assert_ok();
    let (native, wasm) = hashes(&run.stdout());
    assert_eq!(native, wasm);
    assert_eq!(native.len(), 64, "a full BLAKE3 hash in hex: {native}");
}

#[test]
#[ignore = "builds mm-cli for two targets and needs wasmtime; CI's xhash job runs it"]
fn a_mismatch_fails_with_exit_code_1_and_prints_both_hashes() {
    let run = xtask(&["xhash", "--mismatch"]);
    // 1 means the hashes differ; 2 would mean xhash itself could not run.
    assert_eq!(
        run.0.status.code(),
        Some(1),
        "stdout:\n{}\nstderr:\n{}",
        run.stdout(),
        run.stderr()
    );
    let (native, wasm) = hashes(&run.stdout());
    assert_ne!(native, wasm);
    run.assert_fails_with(&["hashes differ", "builds/warrior-1.ron"]);
}

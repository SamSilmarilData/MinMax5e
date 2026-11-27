//! `cargo xtask xhash` as a process (ADR-0050).
//!
//! The process tests build `mm-cli` twice and need `wasmtime` on PATH, so they are
//! `#[ignore]`d in the ordinary test run. CI's macOS test job runs them with `--ignored`.
//! The hashed-set test needs neither and runs everywhere.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use std::collections::BTreeSet;

use common::{repo_root, xtask};

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
#[ignore = "builds mm-cli for two targets and needs wasmtime; CI's macOS test job runs it"]
fn native_and_wasm_hash_alike_and_xhash_passes() {
    let run = xtask(&["xhash"]);
    run.assert_ok();
    let (native, wasm) = hashes(&run.stdout());
    assert_eq!(native, wasm);
    assert_eq!(native.len(), 64, "a full BLAKE3 hash in hex: {native}");
}

#[test]
#[ignore = "builds mm-cli for two targets and needs wasmtime; CI's macOS test job runs it"]
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
    run.assert_fails_with(&["[target-mismatch]", "builds/warrior-1.ron"]);
}

/// `xtask/xhash.toml`'s Build paths.
#[derive(serde::Deserialize)]
struct HashedSet {
    build: Vec<HashedBuild>,
}

#[derive(serde::Deserialize)]
struct HashedBuild {
    path: String,
}

#[test]
fn every_fixture_build_is_in_the_hashed_set() {
    let root = repo_root();
    let builds_dir = "crates/mm-testkit/fixtures/builds";
    let on_disk: BTreeSet<String> = std::fs::read_dir(root.join(builds_dir))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("ron"))
        })
        .map(|name| format!("{builds_dir}/{name}"))
        .collect();
    let set: HashedSet =
        toml::from_str(&std::fs::read_to_string(root.join("xtask/xhash.toml")).unwrap()).unwrap();
    let hashed: BTreeSet<String> = set.build.into_iter().map(|build| build.path).collect();

    let missing: Vec<_> = on_disk.difference(&hashed).collect();
    let stale: Vec<_> = hashed.difference(&on_disk).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "xtask/xhash.toml must list every Build in {builds_dir} (ADR-0050).\n\
         Add a [[build]] entry for: {missing:?}\nRemove the entry for: {stale:?}"
    );
}

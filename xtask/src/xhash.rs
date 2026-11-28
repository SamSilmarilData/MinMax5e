//! `xhash`: the cross-target result hash (ADR-0050).
//!
//! Builds `mm-cli` in release for the host and for `wasm32-wasip1`, evaluates every
//! Build in the hashed set (`xtask/xhash.toml`) with each, and hashes the results. The
//! WebAssembly build runs under wasmtime, which must be on PATH.
//!
//! What is hashed is `mm-cli evaluate --format ron` output (with `--hp` where the entry
//! gives one). RON writes each float in
//! Rust's shortest round-trip form, so the text carries every bit of every finite
//! result. The Builds are hashed in set order, each as its path and its output, each
//! preceded by its length, with BLAKE3.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cargo_metadata::MetadataCommand;
use serde::Deserialize;

use crate::Outcome;
use crate::report::{Check as _, Report};

/// The hashed set, relative to the repository root.
const HASHED_SET: &str = "xtask/xhash.toml";
/// The pack every Build in the set is evaluated on, relative to the repository root.
const PACK: &str = "crates/mm-testkit/fixtures/pack";
const WASM_TARGET: &str = "wasm32-wasip1";
/// Where the repository root is mounted inside wasmtime's sandbox.
const GUEST_ROOT: &str = "/repo";

/// The one check `xhash` runs.
#[derive(Debug, Clone, Copy)]
struct TargetMismatch;

impl crate::report::Check for TargetMismatch {
    fn name(self) -> &'static str {
        "target-mismatch"
    }
}

/// `xtask/xhash.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HashedSet {
    build: Vec<Entry>,
}

/// One Build in the hashed set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    /// The Build file, relative to the repository root.
    path: String,
    /// The target AC it is evaluated against.
    ac: i32,
    /// The target's hit points, passed as `--hp` so Time-to-kill and Kill probability are
    /// hashed too.
    hp: Option<u32>,
}

/// Runs the cross-target hash for the repository at `root`. With `mismatch`, the last
/// digit of the first WebAssembly result is changed before hashing (the smallest change
/// to the printed text), to show that a difference fails.
pub fn run(root: &Path, mismatch: bool) -> Result<Outcome> {
    // wasmtime mounts the root by path, so give it one without `..`.
    let root = &root
        .canonicalize()
        .with_context(|| format!("resolving {}", root.display()))?;
    let set = read_set(root)?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    build_cli(root, &cargo, None)?;
    build_cli(root, &cargo, Some(WASM_TARGET))?;
    let target_dir = target_dir(root)?;
    let native_bin = target_dir
        .join("release")
        .join(format!("mm-cli{}", std::env::consts::EXE_SUFFIX));
    let wasm_bin = target_dir
        .join(WASM_TARGET)
        .join("release")
        .join("mm-cli.wasm");

    let mut native_hash = blake3::Hasher::new();
    let mut wasm_hash = blake3::Hasher::new();
    let mut report = Report::new("xhash");
    for (index, entry) in set.build.iter().enumerate() {
        let native = evaluate(Command::new(&native_bin), root, entry)?;
        let mut wasmtime = Command::new("wasmtime");
        wasmtime
            .arg("run")
            .arg("--dir")
            .arg(format!("{}::{GUEST_ROOT}", root.display()))
            .arg(&wasm_bin);
        let mut wasm = evaluate(wasmtime, Path::new(GUEST_ROOT), entry)
            .context("running the wasm32-wasip1 build (is wasmtime installed and on PATH?)")?;
        if mismatch && index == 0 {
            corrupt_last_digit(&mut wasm)?;
        }
        if native != wasm {
            report.push(
                TargetMismatch.problem(&entry.path, "native and wasm32-wasip1 results differ"),
            );
        }
        absorb(&mut native_hash, entry, &native);
        absorb(&mut wasm_hash, entry, &wasm);
    }

    println!("native        {}", native_hash.finalize().to_hex());
    println!("{WASM_TARGET} {}", wasm_hash.finalize().to_hex());
    Ok(report.finish(
        &format!("{} Build(s) hash alike on both targets", set.build.len()),
        "The hashes differ: a scored value is not bit-identical across targets. Check the \
         ADR-0050 rules for scored code.",
    ))
}

fn read_set(root: &Path) -> Result<HashedSet> {
    let path = root.join(HASHED_SET);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let set: HashedSet = toml::from_str(&text).with_context(|| format!("parsing {HASHED_SET}"))?;
    if set.build.is_empty() {
        bail!("{HASHED_SET} lists no Builds");
    }
    Ok(set)
}

/// `cargo build --release --locked -p mm-cli`, for `target` or the host.
fn build_cli(root: &Path, cargo: &OsString, target: Option<&str>) -> Result<()> {
    let mut command = Command::new(cargo);
    command
        .current_dir(root)
        .args(["build", "--release", "--locked", "--package", "mm-cli"]);
    if let Some(target) = target {
        command.args(["--target", target]);
    }
    let status = command
        .status()
        .with_context(|| format!("running {command:?}"))?;
    if !status.success() {
        bail!("{command:?} failed with {status}");
    }
    Ok(())
}

fn target_dir(root: &Path) -> Result<PathBuf> {
    let metadata = MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .context("running cargo metadata")?;
    Ok(metadata.target_directory.into_std_path_buf())
}

/// Runs `mm-cli evaluate` for `entry` through `command` (the binary, or wasmtime and its
/// arguments), with paths under `root` as that command sees the repository. Returns stdout.
fn evaluate(mut command: Command, root: &Path, entry: &Entry) -> Result<Vec<u8>> {
    command
        .arg("evaluate")
        .arg(root.join(&entry.path))
        .args([
            "--ac",
            &entry.ac.to_string(),
            "--format",
            "ron",
            "--pack-dir",
        ])
        .arg(root.join(PACK));
    if let Some(hp) = entry.hp {
        command.args(["--hp", &hp.to_string()]);
    }
    let output = command
        .output()
        .with_context(|| format!("running {command:?}"))?;
    if !output.status.success() {
        bail!(
            "{command:?} failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

fn absorb(hasher: &mut blake3::Hasher, entry: &Entry, output: &[u8]) {
    hasher.update(&(entry.path.len() as u64).to_le_bytes());
    hasher.update(entry.path.as_bytes());
    hasher.update(&(output.len() as u64).to_le_bytes());
    hasher.update(output);
}

/// Flips the low bit of the last ASCII digit in `output`, which keeps it a digit
/// (`0`↔`1`, …, `8`↔`9`): the smallest change to a printed number.
fn corrupt_last_digit(output: &mut [u8]) -> Result<()> {
    let digit = output
        .iter_mut()
        .rev()
        .find(|byte| byte.is_ascii_digit())
        .context("--mismatch: the WebAssembly result has no digit to change")?;
    *digit ^= 1;
    Ok(())
}

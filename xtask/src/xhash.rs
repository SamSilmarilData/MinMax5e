//! `xhash`: the cross-target result hash (ADR-0050).
//!
//! Builds `mm-cli` in release for the host and for `wasm32-wasip1`, evaluates every
//! Build in the hashed set (`xtask/xhash.toml`) with each, and hashes the results. The
//! WebAssembly build runs under wasmtime, which must be on PATH.
//!
//! What is hashed is `mm-cli evaluate --format ron` output, with `--assumptions`, `--ac`
//! and `--hp` where the entry gives them; an entry with none is evaluated against the
//! Encounter Baseline. RON writes each float in Rust's shortest round-trip form, so the
//! text carries every bit of every finite result. The entries are hashed in set order,
//! each as its path, its target flags and its output, each preceded by its length, with
//! BLAKE3.

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
    /// An Assumption Set file, relative to the repository root, passed as `--assumptions`.
    assumptions: Option<String>,
    /// The target AC, passed as `--ac`.
    ac: Option<i32>,
    /// The target's hit points, passed as `--hp`.
    hp: Option<u32>,
}

impl Entry {
    /// The target flags `mm-cli evaluate` gets, with paths under `root`.
    fn target_args(&self, root: &Path) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(assumptions) = &self.assumptions {
            args.push("--assumptions".into());
            args.push(root.join(assumptions).into_os_string());
        }
        if let Some(ac) = self.ac {
            args.extend(["--ac".into(), ac.to_string().into()]);
        }
        if let Some(hp) = self.hp {
            args.extend(["--hp".into(), hp.to_string().into()]);
        }
        args
    }
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
        &format!(
            "{} evaluation(s) hash alike on both targets",
            set.build.len()
        ),
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
        .args(entry.target_args(root))
        .args(["--format", "ron", "--pack-dir"])
        .arg(root.join(PACK));
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
    // The flags as written in the set, so both targets absorb the same bytes whatever
    // root they see.
    let flags = format!("{:?} {:?} {:?}", entry.assumptions, entry.ac, entry.hp);
    for part in [entry.path.as_bytes(), flags.as_bytes(), output] {
        hasher.update(&(part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
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

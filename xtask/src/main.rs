//! Repository guards, run as `cargo xtask <command>`.
//!
//! - `deps`: the crate dependency rules of ADR-0034, checked against `cargo metadata`.
//! - `leak-scan`: no upstream rules data in tracked files (ADR-0028).
//! - `adr-check`: accepted ADRs change only by dated amendment.
//! - `glossary`: Content ID kinds use no term GLOSSARY.md avoids.
//! - `xhash`: evaluation gives the same bits natively and in WebAssembly (ADR-0050).

mod adr;
mod deps;
mod git;
mod glossary;
mod leak;
mod report;
mod xhash;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "MinMax5e repository guards")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check the crate dependency rules (ADR-0034) against `cargo metadata`.
    Deps {
        /// The workspace manifest to check; defaults to this repository's.
        #[arg(long)]
        manifest_path: Option<PathBuf>,
    },
    /// Check that accepted ADRs changed only by dated amendment since `base`.
    AdrCheck {
        /// The repository to check; defaults to this one.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Compare against the merge base of this revision and HEAD.
        #[arg(long, default_value = "main")]
        base: String,
    },
    /// Scan tracked files for upstream rules data (ADR-0028).
    LeakScan {
        /// The git working tree to scan; defaults to this repository.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Check that no Content ID kind is a term GLOSSARY.md says to avoid.
    Glossary {
        /// The git working tree to check; defaults to this repository.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Check that evaluation gives the same bits natively and under wasmtime (ADR-0050).
    ///
    /// Builds mm-cli for the host and for wasm32-wasip1, evaluates every Build in the
    /// hashed set with each, and prints both hashes. Needs wasmtime on PATH. To add a
    /// fixture Build to the hashed set, append a `[[build]]` entry to xtask/xhash.toml.
    Xhash {
        /// Change the last digit of the first WebAssembly result before hashing, to show
        /// that a difference fails.
        #[arg(long)]
        mismatch: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Deps { manifest_path } => {
            deps::run(&manifest_path.unwrap_or_else(|| repo_root().join("Cargo.toml")))
        }
        Command::LeakScan { root } => leak::run(&root.unwrap_or_else(repo_root)),
        Command::AdrCheck { root, base } => adr::run(&root.unwrap_or_else(repo_root), &base),
        Command::Glossary { root } => glossary::run(&root.unwrap_or_else(repo_root)),
        Command::Xhash { mismatch } => xhash::run(&repo_root(), mismatch),
    };
    match result {
        Ok(Outcome::Clean) => ExitCode::SUCCESS,
        Ok(Outcome::Violations) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

/// Whether a guard found violations. Errors running the guard itself are `Err`.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Clean,
    Violations,
}

/// The repository root: the parent of this crate's directory.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

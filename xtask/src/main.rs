//! Repository guards, run as `cargo xtask <command>`.
//!
//! - `deps`: the crate dependency rules of ADR-0034, checked against `cargo metadata`.
//! - `leak-scan`: no upstream rules data in tracked files (ADR-0028).

mod deps;
mod git;
mod leak;

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
    /// Scan tracked files for upstream rules data (ADR-0028).
    LeakScan {
        /// The git working tree to scan; defaults to this repository.
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Deps { manifest_path } => {
            deps::run(&manifest_path.unwrap_or_else(|| repo_root().join("Cargo.toml")))
        }
        Command::LeakScan { root } => leak::run(&root.unwrap_or_else(repo_root)),
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

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or(manifest_dir.clone(), std::path::Path::to_path_buf)
}

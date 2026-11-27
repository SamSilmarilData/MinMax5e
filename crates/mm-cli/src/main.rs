//! `mm-cli`: evaluate a Build from the command line (architecture §11.1).
//!
//! `mm-cli evaluate <build.ron> --ac <AC> [--pack-dir <dir>]… [--format text|ron]`
//!
//! The Ruleset is inferred from the Build's Content ID prefixes. The pack is the
//! Ruleset's embedded encodings plus every `.ron` file under each `--pack-dir`: files
//! under a `records/` directory are records, the rest are encodings. The CLI calls the
//! `mm-engine` facade directly until the Session protocol arrives (ADR-0051 amendment).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use miette::NarratableReportHandler;
use mm_core::{AssumptionSet, Build, Ruleset, Source, SourceRole, Sources};
use mm_engine::Evaluation;
use mm_rules_2024::Rules2024;

#[derive(Debug, Parser)]
#[command(
    name = "mm-cli",
    about = "Evaluate 5th-edition character Builds exactly."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate a Build: Nova DPR and its SD at every AC from 10 to 25.
    Evaluate(EvaluateArgs),
}

#[derive(Debug, clap::Args)]
struct EvaluateArgs {
    /// The Build file (RON).
    build: PathBuf,
    /// The target's Armor Class; its row is highlighted.
    #[arg(long)]
    ac: i32,
    /// A directory of records and encodings to load into the pack. Repeatable.
    #[arg(long = "pack-dir")]
    pack_dirs: Vec<PathBuf>,
    /// The output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Ron,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Evaluate(args) => evaluate(&args),
    };
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn evaluate(args: &EvaluateArgs) -> Result<String> {
    let text = fs::read_to_string(&args.build)
        .with_context(|| format!("reading {}", args.build.display()))?;
    let build: Build = ron::Options::default()
        .from_str(&text)
        .with_context(|| format!("parsing the Build in {}", args.build.display()))?;
    let evaluation = match build.ruleset()? {
        Rules2024::PREFIX => evaluate_in::<Rules2024>(&build, args)?,
        other => bail!("the Build's Content IDs use Ruleset `{other}`, which mm-cli does not know"),
    };
    match args.format {
        Format::Text => Ok(render_text(&evaluation)),
        Format::Ron => {
            let mut ron =
                ron::ser::to_string_pretty(&evaluation, ron::ser::PrettyConfig::default())
                    .context("serialising the Evaluation")?;
            ron.push('\n');
            Ok(ron)
        }
    }
}

fn evaluate_in<R: Ruleset>(build: &Build, args: &EvaluateArgs) -> Result<Evaluation> {
    let mut sources = R::encodings();
    for dir in &args.pack_dirs {
        sources = sources.chain(read_pack_dir(dir)?);
    }
    let pack = mm_load::load::<R>(&sources).map_err(|error| {
        let mut rendered = String::new();
        NarratableReportHandler::new()
            .render_report(&mut rendered, &error)
            .map_or_else(|_| anyhow!("{error}"), |()| anyhow!("{rendered}"))
    })?;
    let kit = mm_engine::compile(&pack, build)?;
    Ok(mm_engine::evaluate(&kit, &AssumptionSet::against(args.ac)))
}

/// Every `.ron` file under `dir`, in path order.
fn read_pack_dir(dir: &Path) -> Result<Sources> {
    let mut paths = Vec::new();
    collect_ron(dir, &mut paths)
        .with_context(|| format!("reading pack directory {}", dir.display()))?;
    paths.sort();
    let files = paths
        .into_iter()
        .map(|path| {
            let relative = path.strip_prefix(dir).unwrap_or(&path);
            let role = if relative.components().any(|c| c.as_os_str() == "records") {
                SourceRole::Records
            } else {
                SourceRole::Encoding
            };
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            Ok(Source {
                name: path.display().to_string(),
                role,
                text,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Sources { files })
}

fn collect_ron(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_ron(&path, paths)?;
        } else if path.extension().is_some_and(|e| e == "ron") {
            paths.push(path);
        }
    }
    Ok(())
}

fn render_text(evaluation: &Evaluation) -> String {
    let nova = &evaluation.nova;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Nova DPR, rounds 1–{}, target AC {} (marked >)",
        nova.rounds, evaluation.target_ac
    );
    let _ = writeln!(out, "     AC      DPR       SD");
    for lane in &nova.lanes {
        let mark = if lane.ac == evaluation.target_ac {
            '>'
        } else {
            ' '
        };
        let _ = writeln!(
            out,
            "{mark}    {:>2} {:>8.3} {:>8.3}",
            lane.ac, lane.dpr, lane.sd
        );
    }
    let _ = writeln!(out, "ShapeKey {}", evaluation.shape_key);
    let _ = writeln!(out, "ParamsFp {}", evaluation.params_fp);
    out
}

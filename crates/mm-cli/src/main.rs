//! `mm-cli`: evaluate a Build, or check encodings, from the command line (architecture
//! §11.1).
//!
//! - `mm-cli evaluate <build.ron> [--assumptions <file.ron>] [--ac <AC>] [--hp <HP>] [--nova-rounds <N>] [--pack-dir <dir>]… [--format text|ron]`
//! - `mm-cli validate-packs [--pack-dir <dir>]…`
//!
//! `evaluate` starts from the default Assumption Set, whose target is the Encounter
//! Baseline for the Build's level (ADR-0024), or from the `--assumptions` file in its
//! place. `--ac`, `--hp` and `--nova-rounds` then override what either gives.
//!
//! `evaluate` infers the Ruleset from the Build's Content ID prefixes; `validate-packs`
//! checks against the 2024 Ruleset, the only one `mm-cli` knows. The pack is the
//! Ruleset's embedded encodings plus every `.ron` file under each `--pack-dir`: files
//! under a `records/` directory are records, the rest are encodings. Load problems are
//! printed as miette diagnostics naming the file and the offending text. The CLI calls
//! the `mm-engine` facade directly until the Session protocol arrives (ADR-0051
//! amendment).

use std::fmt::{self, Write as _};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use miette::NarratableReportHandler;
use mm_core::{AssumptionSet, Build, EncounterProfile, Ruleset, Source, SourceRole, Sources};
use mm_engine::Evaluation;
use mm_load::Pack;
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
    /// Evaluate a Build: Nova DPR, its SD, the least a round deals and the Reliability Band
    /// at every AC from 10 to 25, and Time-to-kill and Kill probability against the
    /// target's HP. The target is the Encounter Baseline for the Build's level unless
    /// `--assumptions`, `--ac` or `--hp` say otherwise.
    Evaluate(EvaluateArgs),
    /// Load every pack and report problems, then list which in-scope records are
    /// encoded. Fails on any problem or unencoded record (ADR-0002).
    ValidatePacks(ValidatePacksArgs),
}

#[derive(Debug, clap::Args)]
struct EvaluateArgs {
    /// The Build file (RON).
    build: PathBuf,
    /// An Assumption Set file (RON) to use in place of the defaults; the flags below
    /// override it.
    #[arg(long)]
    assumptions: Option<PathBuf>,
    /// The target's Armor Class; its row is highlighted. Defaults to the Encounter
    /// Baseline's for the Build's level.
    #[arg(long)]
    ac: Option<i32>,
    /// The target's hit points, for Time-to-kill and Kill probability over the horizon.
    /// Defaults to the Encounter Baseline's for the Build's level.
    #[arg(long)]
    hp: Option<NonZeroU32>,
    /// The Nova horizon: rounds 1 to this. 1 gives a single round's DPR, SD and Band.
    /// Defaults to 2.
    #[arg(long)]
    nova_rounds: Option<NonZeroU32>,
    /// A directory of records and encodings to load into the pack. Repeatable.
    #[arg(long = "pack-dir")]
    pack_dirs: Vec<PathBuf>,
    /// The output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(Debug, clap::Args)]
struct ValidatePacksArgs {
    /// A directory of records and encodings to load into the pack. Repeatable.
    #[arg(long = "pack-dir")]
    pack_dirs: Vec<PathBuf>,
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
        Command::ValidatePacks(args) => validate_packs::<Rules2024>(&args.pack_dirs),
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
    let ruleset = build.ruleset()?;
    let target = Target::resolve(&build, args)?;
    let evaluation = match ruleset {
        Rules2024::PREFIX => evaluate_in::<Rules2024>(&build, &target.assumptions, args)?,
        other => bail!("the Build's Content IDs use Ruleset `{other}`, which mm-cli does not know"),
    };
    match args.format {
        Format::Text => Ok(render_text(&evaluation, &target)),
        Format::Ron => {
            let mut ron =
                ron::ser::to_string_pretty(&evaluation, ron::ser::PrettyConfig::default())
                    .context("serialising the Evaluation")?;
            ron.push('\n');
            Ok(ron)
        }
    }
}

fn evaluate_in<R: Ruleset>(
    build: &Build,
    assumptions: &AssumptionSet,
    args: &EvaluateArgs,
) -> Result<Evaluation> {
    let pack = load_pack::<R>(&args.pack_dirs)?;
    let kit = mm_engine::compile(&pack, build)?;
    Ok(mm_engine::evaluate(&kit, assumptions))
}

/// The Assumption Set an evaluation runs under, and where its target came from.
struct Target {
    assumptions: AssumptionSet,
    ac_from: Origin,
    hp_from: Origin,
}

/// Where a target figure came from, as the text output names it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Origin {
    Baseline { cr: usize },
    File(PathBuf),
    Flag(&'static str),
}

impl Target {
    /// The defaults, or the `--assumptions` file in their place, with the flags on top.
    /// The Encounter Baseline is only consulted for a figure no file or flag gives.
    fn resolve(build: &Build, args: &EvaluateArgs) -> Result<Target> {
        let (mut assumptions, origin) = match (&args.assumptions, args.ac, args.hp) {
            (Some(path), _, _) => (read_assumptions(path)?, Origin::File(path.clone())),
            // Both figures are the flags', so the origin is replaced below.
            (None, Some(target_ac), Some(target_hp)) => (
                AssumptionSet::against(EncounterProfile {
                    target_ac,
                    target_hp,
                }),
                Origin::Flag("--ac"),
            ),
            (None, _, _) => (
                AssumptionSet::baseline(build.level())
                    .context("the Build has no default target; give --ac and --hp")?,
                Origin::Baseline { cr: build.level() },
            ),
        };
        let profile = &mut assumptions.encounter_profile;
        let ac_from = match args.ac {
            Some(ac) => {
                profile.target_ac = ac;
                Origin::Flag("--ac")
            }
            None => origin.clone(),
        };
        let hp_from = match args.hp {
            Some(hp) => {
                profile.target_hp = hp;
                Origin::Flag("--hp")
            }
            None => origin,
        };
        if let Some(rounds) = args.nova_rounds {
            assumptions.nova_rounds = rounds;
        }
        Ok(Target {
            assumptions,
            ac_from,
            hp_from,
        })
    }

    /// The target line of the text output.
    fn describe(&self) -> String {
        let EncounterProfile {
            target_ac,
            target_hp,
        } = self.assumptions.encounter_profile;
        if self.ac_from == self.hp_from {
            format!("Target: AC {target_ac}, {target_hp} HP ({})", self.ac_from)
        } else {
            format!(
                "Target: AC {target_ac} ({}), {target_hp} HP ({})",
                self.ac_from, self.hp_from
            )
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Baseline { cr } => write!(f, "Encounter Baseline, CR {cr}"),
            Origin::File(path) => write!(f, "{}", path.display()),
            Origin::Flag(flag) => f.write_str(flag),
        }
    }
}

/// The Assumption Set in the RON file at `path`.
fn read_assumptions(path: &Path) -> Result<AssumptionSet> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    ron::Options::default()
        .from_str(&text)
        .with_context(|| format!("parsing the Assumption Set in {}", path.display()))
}

/// Loads every pack under Ruleset `R` and reports its coverage; an error if any record
/// in scope has no encoding.
fn validate_packs<R: Ruleset>(pack_dirs: &[PathBuf]) -> Result<String> {
    let coverage = load_pack::<R>(pack_dirs)?.coverage();
    let mut report = String::new();
    let _ = writeln!(
        report,
        "Coverage, Ruleset {}: {} of {} in-scope records encoded",
        R::PREFIX,
        coverage.encoded.len(),
        coverage.encoded.len() + coverage.missing.len()
    );
    for id in &coverage.encoded {
        let _ = writeln!(report, "  encoded  {id}");
    }
    for id in &coverage.missing {
        let _ = writeln!(report, "  missing  {id}");
    }
    if coverage.missing.is_empty() {
        Ok(report)
    } else {
        bail!(
            "{} in-scope record(s) have no encoding (ADR-0002)\n{report}",
            coverage.missing.len()
        )
    }
}

/// The Ruleset's embedded encodings plus every pack directory, loaded under `R`.
fn load_pack<R: Ruleset>(pack_dirs: &[PathBuf]) -> Result<Pack<R>> {
    let mut sources = R::encodings();
    for dir in pack_dirs {
        sources = sources.chain(read_pack_dir(dir)?);
    }
    mm_load::load::<R>(&sources).map_err(|error| {
        let mut rendered = String::new();
        NarratableReportHandler::new()
            .render_report(&mut rendered, &error)
            .map_or_else(|_| anyhow!("{error}"), |()| anyhow!("{rendered}"))
    })
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

fn render_text(evaluation: &Evaluation, target: &Target) -> String {
    let nova = &evaluation.nova;
    let mut out = String::new();
    let _ = writeln!(out, "{}", target.describe());
    let rounds = match nova.rounds {
        1 => "round 1".to_owned(),
        rounds => format!("rounds 1–{rounds}"),
    };
    let _ = writeln!(
        out,
        "Nova DPR, {rounds}, target AC {} (marked >)",
        evaluation.target_ac
    );
    let _ = writeln!(
        out,
        "     AC      DPR       SD    Min    p10    p50    p90  Floor"
    );
    for lane in &nova.lanes {
        let mark = if lane.ac == evaluation.target_ac {
            '>'
        } else {
            ' '
        };
        let band = &lane.band;
        let _ = writeln!(
            out,
            "{mark}    {:>2} {:>8.3} {:>8.3} {:>6.1} {:>6.1} {:>6.1} {:>6.1} {:>6.1}",
            lane.ac, lane.dpr, lane.sd, lane.minimum, band.p10, band.p50, band.p90, band.floor
        );
    }
    let target_lane = nova
        .lanes
        .iter()
        .find(|lane| lane.ac == evaluation.target_ac);
    if let Some(kill) = target_lane.map(|lane| &lane.kill) {
        let hp = evaluation.target_hp;
        let _ = writeln!(
            out,
            "Against {hp} HP at AC {}, by round:",
            evaluation.target_ac
        );
        let _ = writeln!(out, "  round  Time-to-kill  Kill probability");
        for (i, (first, by_now)) in kill
            .time_to_kill
            .iter()
            .zip(&kill.kill_probability)
            .enumerate()
        {
            let _ = writeln!(
                out,
                "  {:>5} {:>12.2}% {:>16.2}%",
                i + 1,
                first * 100.0,
                by_now * 100.0
            );
        }
        let survives = 1.0 - kill.kill_probability.last().copied().unwrap_or(0.0);
        let _ = writeln!(out, "  survives the horizon: {:.2}%", survives * 100.0);
    }
    let _ = writeln!(out, "ShapeKey {}", evaluation.shape_key);
    let _ = writeln!(out, "ParamsFp {}", evaluation.params_fp);
    out
}

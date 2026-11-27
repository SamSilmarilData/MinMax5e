//! `deps`: the crate dependency rules of ADR-0034, checked against `cargo metadata`.
//!
//! This is the authority for the rules. `deny.toml` repeats the rules-crate and `mm-host`
//! bans as a backstop. The table covers every crate in the architecture's crate map,
//! including ones not yet created, so a new crate lands into rules that already exist. A
//! workspace crate missing from the table fails the check.
//!
//! Checked per crate:
//! - its normal and build dependencies on workspace crates are in its allowed set;
//! - its dev-dependencies are too, plus `mm-testkit`;
//! - its third-party normal and build dependencies, where the table limits them;
//! - `mm-testkit` is used only from integration tests (`<crate>/tests/`), never from
//!   unit-test modules, which would make a dev-dependency cycle with two copies of the
//!   engine's types. Comments are ignored; `mm-testkit` may not be renamed, so the scan
//!   can find it;
//! - `publish = false` and the MIT licence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use cargo_metadata::{Dependency, DependencyKind, MetadataCommand, Package};

use crate::Outcome;
use crate::report::{Check as _, Report, at_line};

/// The checks `deps` runs; each names the problems it reports.
#[derive(Debug, Clone, Copy)]
enum Check {
    /// `publish = false` and the MIT licence.
    Package,
    /// Every workspace crate is in the table.
    CrateTable,
    /// Workspace dependencies are in the crate's allowed set.
    AllowedDeps,
    /// Third-party dependencies, where the table limits them.
    ThirdParty,
    /// `mm-testkit` is used only from integration tests.
    Testkit,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::Package => "package",
            Check::CrateTable => "crate-table",
            Check::AllowedDeps => "allowed-deps",
            Check::ThirdParty => "third-party",
            Check::Testkit => "testkit",
        }
    }
}

/// Every crate in ADR-0034's crate map, including ones not yet created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Crate {
    Dice,
    Core,
    Load,
    Import,
    Engine,
    Solver,
    Cache,
    Rules2024,
    Rules2014,
    AppCore,
    Host,
    Testkit,
    Gui,
    Worker,
    Cli,
    Xtask,
}

impl Crate {
    const ALL: [Crate; 16] = [
        Crate::Dice,
        Crate::Core,
        Crate::Load,
        Crate::Import,
        Crate::Engine,
        Crate::Solver,
        Crate::Cache,
        Crate::Rules2024,
        Crate::Rules2014,
        Crate::AppCore,
        Crate::Host,
        Crate::Testkit,
        Crate::Gui,
        Crate::Worker,
        Crate::Cli,
        Crate::Xtask,
    ];

    /// The package name.
    fn name(self) -> &'static str {
        match self {
            Crate::Dice => "mm-dice",
            Crate::Core => "mm-core",
            Crate::Load => "mm-load",
            Crate::Import => "mm-import",
            Crate::Engine => "mm-engine",
            Crate::Solver => "mm-solver",
            Crate::Cache => "mm-cache",
            Crate::Rules2024 => "mm-rules-2024",
            Crate::Rules2014 => "mm-rules-2014",
            Crate::AppCore => "mm-app-core",
            Crate::Host => "mm-host",
            Crate::Testkit => "mm-testkit",
            Crate::Gui => "mm-gui",
            Crate::Worker => "mm-worker",
            Crate::Cli => "mm-cli",
            Crate::Xtask => "xtask",
        }
    }

    fn from_name(name: &str) -> Option<Crate> {
        Crate::ALL.into_iter().find(|c| c.name() == name)
    }

    /// This crate's row in the table.
    fn rule(self) -> &'static CrateRule {
        TABLE
            .iter()
            .find(|row| row.krate == self)
            .expect("every crate has a row in the table")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Library,
    /// A `mm-rules-*` crate: depends on `mm-core` only.
    Rules,
    /// `mm-testkit`: oracles and fixtures, a dev-dependency only.
    Testkit,
    /// `mm-host`: the platform adapters, used only by binaries.
    Host,
    /// A binary: the shells and `xtask`. Nothing depends on one.
    Binary,
}

/// Which third-party crates a crate may use in normal and build dependencies.
#[derive(Debug, Clone, Copy)]
enum ThirdParty {
    Any,
    None,
    Only(&'static [&'static str]),
}

impl ThirdParty {
    /// Why the third-party crate `dep` is rejected, or `None` if it is allowed.
    fn rejection(self, dep: &str) -> Option<String> {
        match self {
            ThirdParty::Any => None,
            ThirdParty::Only(allowed) if allowed.contains(&dep) => None,
            ThirdParty::Only(allowed) => Some(format!(
                "may use only these third-party crates: {}",
                allowed.join(", ")
            )),
            ThirdParty::None => Some("may have no third-party dependencies".to_owned()),
        }
    }
}

#[derive(Debug)]
struct CrateRule {
    krate: Crate,
    role: Role,
    may_use: Vec<Crate>,
    third_party: ThirdParty,
}

impl CrateRule {
    /// Whether this crate may depend on the workspace crate `dep`. Any crate may take
    /// `mm-testkit` as a dev-dependency.
    fn permits(&self, dep: Crate, dev: bool) -> bool {
        self.may_use.contains(&dep) || (dev && dep == Crate::Testkit)
    }

    /// Why this crate may not depend on the workspace crate `dep`, phrased by the rule broken.
    fn workspace_dep_rejection(&self, dep: Crate) -> String {
        let name = self.krate.name();
        match dep.rule().role {
            Role::Rules => {
                "only binaries and mm-testkit may depend on a rules crate (ADR-0034)".to_owned()
            }
            Role::Host => "only binaries may depend on mm-host (ADR-0034)".to_owned(),
            Role::Testkit => {
                "mm-testkit is used only as a dev-dependency, from integration tests (ADR-0034)"
                    .to_owned()
            }
            Role::Binary => "nothing may depend on a binary (ADR-0034)".to_owned(),
            Role::Library if self.may_use.is_empty() => {
                format!("{name} may depend on no workspace crate (ADR-0034)")
            }
            Role::Library => {
                let allowed: Vec<_> = self.may_use.iter().map(|c| c.name()).collect();
                format!(
                    "{name} may depend only on {} (ADR-0034)",
                    allowed.join(", ")
                )
            }
        }
    }
}

/// ADR-0034 as amended 2026-11-27, and the crate map in architecture §8.
static TABLE: LazyLock<Vec<CrateRule>> = LazyLock::new(|| {
    use Crate::{
        AppCore, Cache, Cli, Core, Dice, Engine, Gui, Host, Import, Load, Rules2014, Rules2024,
        Solver, Testkit, Worker, Xtask,
    };

    let plus = |base: &[Crate], more: &[Crate]| -> Vec<Crate> {
        base.iter().chain(more).copied().collect()
    };
    // Each layer may use everything below it.
    let below_app = [Dice, Core, Load, Import, Engine, Solver, Cache];
    let below_host = plus(&below_app, &[AppCore]);
    let shell = plus(&below_host, &[Host, Rules2024, Rules2014]);

    let table_row = |krate, role, may_use: &[Crate], third_party| CrateRule {
        krate,
        role,
        may_use: may_use.to_vec(),
        third_party,
    };
    vec![
        table_row(Dice, Role::Library, &[], ThirdParty::None),
        table_row(
            Core,
            Role::Library,
            &[],
            ThirdParty::Only(&["blake3", "serde", "thiserror"]),
        ),
        table_row(Load, Role::Library, &[Core], ThirdParty::Any),
        table_row(Import, Role::Library, &[Core, Load], ThirdParty::Any),
        table_row(Engine, Role::Library, &[Core, Load, Dice], ThirdParty::Any),
        table_row(
            Solver,
            Role::Library,
            &[Core, Load, Dice, Engine],
            ThirdParty::Any,
        ),
        table_row(
            Cache,
            Role::Library,
            &[Core, Load, Dice, Engine, Solver],
            ThirdParty::Any,
        ),
        table_row(Rules2024, Role::Rules, &[Core], ThirdParty::Any),
        table_row(Rules2014, Role::Rules, &[Core], ThirdParty::Any),
        table_row(AppCore, Role::Library, &below_app, ThirdParty::Any),
        table_row(Host, Role::Host, &below_host, ThirdParty::Any),
        table_row(
            Testkit,
            Role::Testkit,
            &[Dice, Core, Load, Engine, Solver, Rules2024, Rules2014],
            ThirdParty::Any,
        ),
        table_row(Gui, Role::Binary, &shell, ThirdParty::Any),
        table_row(Worker, Role::Binary, &shell, ThirdParty::Any),
        table_row(Cli, Role::Binary, &shell, ThirdParty::Any),
        table_row(Xtask, Role::Binary, &[], ThirdParty::Any),
    ]
});

pub fn run(manifest_path: &Path) -> Result<Outcome> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .with_context(|| format!("running cargo metadata on {}", manifest_path.display()))?;
    let packages = metadata.workspace_packages();
    let mut scan = DepsScan {
        members: packages.iter().map(|p| p.name.as_str()).collect(),
        root: metadata.workspace_root.as_std_path(),
        report: Report::new("deps"),
    };
    for package in &packages {
        scan.check_package(package)?;
    }
    Ok(scan.report.finish(
        &format!(
            "{} workspace crate(s) follow the ADR-0034 rules",
            packages.len()
        ),
        "The rules are ADR-0034's; change them with an amendment, then here.",
    ))
}

/// One run of `deps`: the workspace being checked, and the problems found so far.
struct DepsScan<'a> {
    members: BTreeSet<&'a str>,
    root: &'a Path,
    report: Report<Check>,
}

impl DepsScan<'_> {
    fn check_package(&mut self, package: &Package) -> Result<()> {
        let name = package.name.as_str();
        if package
            .publish
            .as_ref()
            .is_none_or(|registries| !registries.is_empty())
        {
            self.report
                .push(Check::Package.problem(name, "must set `publish = false` (ADR-0028)"));
        }
        if package.license.as_deref() != Some("MIT") {
            self.report
                .push(Check::Package.problem(name, "must use the MIT licence"));
        }

        let Some(krate) = Crate::from_name(name) else {
            self.report.push(Check::CrateTable.problem(
                name,
                "not in the ADR-0034 crate table; add it to xtask/src/deps.rs with its allowed dependencies, after amending ADR-0034",
            ));
            return Ok(());
        };

        for dep in &package.dependencies {
            self.check_dependency(krate.rule(), dep);
        }
        let uses_testkit = package
            .dependencies
            .iter()
            .any(|d| d.name == Crate::Testkit.name());
        if krate != Crate::Testkit && uses_testkit {
            self.check_testkit_use(package)?;
        }
        Ok(())
    }

    fn check_dependency(&mut self, from: &CrateRule, dep: &Dependency) {
        let edge = format!("{} -> {}", from.krate.name(), dep.name);
        let dev = dep.kind == DependencyKind::Development;

        if dep.name == Crate::Testkit.name() && dep.rename.is_some() {
            self.report.push(Check::Testkit.problem(
                edge.as_str(),
                "do not rename mm-testkit; `xtask deps` finds its uses by name",
            ));
        }

        let is_member = dep.path.is_some() && self.members.contains(dep.name.as_str());
        if !is_member {
            if !dev && let Some(why) = from.third_party.rejection(&dep.name) {
                let message = format!("{} {why} (ADR-0034)", from.krate.name());
                self.report.push(Check::ThirdParty.problem(edge, message));
            }
            return;
        }

        // A member outside the table is reported on its own; its edges have no rule to check.
        let Some(dep_crate) = Crate::from_name(&dep.name) else {
            return;
        };
        if !from.permits(dep_crate, dev) {
            let message = from.workspace_dep_rejection(dep_crate);
            self.report.push(Check::AllowedDeps.problem(edge, message));
        }
    }

    /// In a crate that depends on `mm-testkit`, no `.rs` file outside `tests/` may use it.
    fn check_testkit_use(&mut self, package: &Package) -> Result<()> {
        let dir = package
            .manifest_path
            .parent()
            .context("manifest has no parent directory")?
            .as_std_path();
        let mut files = Vec::new();
        collect_rust_files(dir, dir, &mut files)?;
        files.sort();

        for file in files {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let relative_path = file
                .strip_prefix(self.root)
                .unwrap_or(&file)
                .display()
                .to_string();
            for (index, line) in text.lines().enumerate() {
                if contains_word(code_part(line), "mm_testkit") {
                    self.report.push(Check::Testkit.problem(
                        at_line(&relative_path, index),
                        "uses mm_testkit outside tests/; oracles are used only from integration tests (ADR-0034)",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Collects `.rs` files under `dir`, skipping the package's own `tests/` and any `target/`.
fn collect_rust_files(package_dir: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            let skip = path.file_name().is_some_and(|n| n == "target")
                || path == package_dir.join("tests");
            if !skip {
                collect_rust_files(package_dir, &path, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The line up to any `//` comment (including doc comments).
fn code_part(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

fn contains_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !ident(before) && !ident(after)
    })
}

#[cfg(test)]
mod tests {
    use super::{Crate, TABLE};

    #[test]
    fn every_crate_has_exactly_one_row() {
        for krate in Crate::ALL {
            let rows = TABLE.iter().filter(|row| row.krate == krate).count();
            assert_eq!(rows, 1, "{} has {rows} rows", krate.name());
        }
        assert_eq!(TABLE.len(), Crate::ALL.len());
    }
}

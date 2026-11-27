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
    /// Why `dep` is refused, or `None` if it is allowed.
    fn refusal(self, dep: &str) -> Option<String> {
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
    name: &'static str,
    role: Role,
    may_use: Vec<&'static str>,
    third_party: ThirdParty,
}

impl CrateRule {
    /// Why this crate may not depend on the workspace crate `dep`, phrased by the rule broken.
    fn refusal_of(&self, dep: &str) -> String {
        match rule_for(dep).map(|r| r.role) {
            Some(Role::Rules) => {
                "only binaries and mm-testkit may depend on a rules crate (ADR-0034)".to_owned()
            }
            Some(Role::Host) => "only binaries may depend on mm-host (ADR-0034)".to_owned(),
            Some(Role::Testkit) => {
                "mm-testkit is used only as a dev-dependency, from integration tests (ADR-0034)"
                    .to_owned()
            }
            Some(Role::Binary) => "nothing may depend on a binary (ADR-0034)".to_owned(),
            Some(Role::Library) | None if self.may_use.is_empty() => {
                format!("{} may depend on no workspace crate (ADR-0034)", self.name)
            }
            Some(Role::Library) | None => format!(
                "{} may depend only on {} (ADR-0034)",
                self.name,
                self.may_use.join(", ")
            ),
        }
    }
}

/// ADR-0034 as amended 2026-11-27, and the crate map in architecture §8.
static TABLE: LazyLock<Vec<CrateRule>> = LazyLock::new(|| {
    use Role::{Binary, Host, Library, Rules, Testkit};

    let layered = |base: &[&'static str], more: &[&'static str]| -> Vec<&'static str> {
        base.iter().chain(more).copied().collect()
    };
    // Each layer may use everything below it.
    let below_app = [
        "mm-dice",
        "mm-core",
        "mm-load",
        "mm-import",
        "mm-engine",
        "mm-solver",
        "mm-cache",
    ];
    let below_host = layered(&below_app, &["mm-app-core"]);
    let shell = layered(&below_host, &["mm-host", "mm-rules-2024", "mm-rules-2014"]);

    let row = |name, role, may_use: &[&'static str], third_party| CrateRule {
        name,
        role,
        may_use: may_use.to_vec(),
        third_party,
    };
    vec![
        row("mm-dice", Library, &[], ThirdParty::None),
        row(
            "mm-core",
            Library,
            &[],
            ThirdParty::Only(&["blake3", "serde", "thiserror"]),
        ),
        row("mm-load", Library, &["mm-core"], ThirdParty::Any),
        row(
            "mm-import",
            Library,
            &["mm-core", "mm-load"],
            ThirdParty::Any,
        ),
        row(
            "mm-engine",
            Library,
            &["mm-core", "mm-load", "mm-dice"],
            ThirdParty::Any,
        ),
        row(
            "mm-solver",
            Library,
            &["mm-core", "mm-load", "mm-dice", "mm-engine"],
            ThirdParty::Any,
        ),
        row(
            "mm-cache",
            Library,
            &["mm-core", "mm-load", "mm-dice", "mm-engine", "mm-solver"],
            ThirdParty::Any,
        ),
        row("mm-rules-2024", Rules, &["mm-core"], ThirdParty::Any),
        row("mm-rules-2014", Rules, &["mm-core"], ThirdParty::Any),
        row("mm-app-core", Library, &below_app, ThirdParty::Any),
        row("mm-host", Host, &below_host, ThirdParty::Any),
        row(
            "mm-testkit",
            Testkit,
            &[
                "mm-dice",
                "mm-core",
                "mm-load",
                "mm-engine",
                "mm-solver",
                "mm-rules-2024",
                "mm-rules-2014",
            ],
            ThirdParty::Any,
        ),
        row("mm-gui", Binary, &shell, ThirdParty::Any),
        row("mm-worker", Binary, &shell, ThirdParty::Any),
        row("mm-cli", Binary, &shell, ThirdParty::Any),
        row("xtask", Binary, &[], ThirdParty::Any),
    ]
});

const TESTKIT: &str = "mm-testkit";

fn rule_for(name: &str) -> Option<&'static CrateRule> {
    TABLE.iter().find(|r| r.name == name)
}

pub fn run(manifest_path: &Path) -> Result<Outcome> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .with_context(|| format!("running cargo metadata on {}", manifest_path.display()))?;
    let packages = metadata.workspace_packages();
    let mut checker = Checker {
        members: packages.iter().map(|p| p.name.as_str()).collect(),
        root: metadata.workspace_root.as_std_path(),
        report: Report::new("deps"),
    };
    for package in &packages {
        checker.check_package(package)?;
    }
    Ok(checker.report.finish(
        &format!(
            "{} workspace crate(s) follow the ADR-0034 rules",
            packages.len()
        ),
        "The rules are ADR-0034's; change them with an amendment, then here.",
    ))
}

/// The workspace being checked, and what has been found so far.
struct Checker<'a> {
    members: BTreeSet<&'a str>,
    root: &'a Path,
    report: Report<Check>,
}

impl Checker<'_> {
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

        let Some(rule) = rule_for(name) else {
            self.report.push(Check::CrateTable.problem(name,
                "not in the ADR-0034 crate table; add it to xtask/src/deps.rs with its allowed dependencies, after amending ADR-0034",
            ));
            return Ok(());
        };

        for dep in &package.dependencies {
            self.check_dependency(rule, dep);
        }
        let uses_testkit = package.dependencies.iter().any(|d| d.name == TESTKIT);
        if name != TESTKIT && uses_testkit {
            self.check_testkit_use(package)?;
        }
        Ok(())
    }

    fn check_dependency(&mut self, from: &CrateRule, dep: &Dependency) {
        let edge = format!("{} -> {}", from.name, dep.name);
        let dev = dep.kind == DependencyKind::Development;

        if dep.name == TESTKIT && dep.rename.is_some() {
            self.report.push(Check::Testkit.problem(
                edge.as_str(),
                "do not rename mm-testkit; `xtask deps` finds its uses by name",
            ));
        }

        let is_member = dep.path.is_some() && self.members.contains(dep.name.as_str());
        if !is_member {
            if !dev && let Some(why) = from.third_party.refusal(&dep.name) {
                self.report.push(
                    Check::ThirdParty.problem(edge, format!("{} {why} (ADR-0034)", from.name)),
                );
            }
            return;
        }

        if from.may_use.contains(&dep.name.as_str()) || (dev && dep.name == TESTKIT) {
            return;
        }
        self.report
            .push(Check::AllowedDeps.problem(edge, from.refusal_of(&dep.name)));
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
            let shown = file
                .strip_prefix(self.root)
                .unwrap_or(&file)
                .display()
                .to_string();
            for (index, line) in text.lines().enumerate() {
                if contains_word(code_part(line), "mm_testkit") {
                    self.report.push(Check::Testkit.problem(at_line(&shown, index),
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

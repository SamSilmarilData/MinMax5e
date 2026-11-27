//! `deps`: the crate dependency rules of ADR-0034, checked against `cargo metadata`.
//!
//! This is the authority for the rules; cargo-deny bans are only a backstop. The table
//! covers every crate in the architecture's crate map, including ones not yet created,
//! so a new crate lands into rules that already exist. A workspace crate missing from
//! the table fails the check.
//!
//! Checked per crate:
//! - its normal and build dependencies on workspace crates are in its allowed set;
//! - its dev-dependencies are too, plus `mm-testkit`;
//! - its third-party normal and build dependencies, where the table limits them;
//! - `mm-testkit` is used only from integration tests (`<crate>/tests/`), never from
//!   unit-test modules, which would make a dev-dependency cycle with two copies of the
//!   engine's types;
//! - `publish = false` and the MIT licence.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, MetadataCommand, Package};

use crate::Outcome;

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

#[derive(Debug, Clone, Copy)]
enum ThirdParty {
    Any,
    None,
    Only(&'static [&'static str]),
}

#[derive(Debug)]
struct CrateRule {
    name: &'static str,
    role: Role,
    may_use: &'static [&'static str],
    third_party: ThirdParty,
}

const LIBS_BELOW_APP: &[&str] = &[
    "mm-dice",
    "mm-core",
    "mm-load",
    "mm-import",
    "mm-engine",
    "mm-solver",
    "mm-cache",
];
const SHELL_DEPS: &[&str] = &[
    "mm-dice",
    "mm-core",
    "mm-load",
    "mm-import",
    "mm-engine",
    "mm-solver",
    "mm-cache",
    "mm-app-core",
    "mm-host",
    "mm-rules-2024",
    "mm-rules-2014",
];

/// ADR-0034 as amended 2026-11-27, and the crate map in architecture §8.
const TABLE: &[CrateRule] = &[
    CrateRule {
        name: "mm-dice",
        role: Role::Library,
        may_use: &[],
        third_party: ThirdParty::None,
    },
    CrateRule {
        name: "mm-core",
        role: Role::Library,
        may_use: &[],
        third_party: ThirdParty::Only(&["blake3", "serde", "thiserror"]),
    },
    CrateRule {
        name: "mm-load",
        role: Role::Library,
        may_use: &["mm-core"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-import",
        role: Role::Library,
        may_use: &["mm-core", "mm-load"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-engine",
        role: Role::Library,
        may_use: &["mm-core", "mm-load", "mm-dice"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-solver",
        role: Role::Library,
        may_use: &["mm-core", "mm-load", "mm-dice", "mm-engine"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-cache",
        role: Role::Library,
        may_use: &["mm-core", "mm-load", "mm-dice", "mm-engine", "mm-solver"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-rules-2024",
        role: Role::Rules,
        may_use: &["mm-core"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-rules-2014",
        role: Role::Rules,
        may_use: &["mm-core"],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-app-core",
        role: Role::Library,
        may_use: LIBS_BELOW_APP,
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-host",
        role: Role::Host,
        may_use: &[
            "mm-dice",
            "mm-core",
            "mm-load",
            "mm-import",
            "mm-engine",
            "mm-solver",
            "mm-cache",
            "mm-app-core",
        ],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-testkit",
        role: Role::Testkit,
        may_use: &[
            "mm-dice",
            "mm-core",
            "mm-load",
            "mm-engine",
            "mm-solver",
            "mm-rules-2024",
            "mm-rules-2014",
        ],
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-gui",
        role: Role::Binary,
        may_use: SHELL_DEPS,
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-worker",
        role: Role::Binary,
        may_use: SHELL_DEPS,
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "mm-cli",
        role: Role::Binary,
        may_use: SHELL_DEPS,
        third_party: ThirdParty::Any,
    },
    CrateRule {
        name: "xtask",
        role: Role::Binary,
        may_use: &[],
        third_party: ThirdParty::Any,
    },
];

const TESTKIT: &str = "mm-testkit";

fn rule(name: &str) -> Option<&'static CrateRule> {
    TABLE.iter().find(|r| r.name == name)
}

pub fn run(manifest_path: &Path) -> Result<Outcome> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .no_deps()
        .exec()
        .with_context(|| format!("running cargo metadata on {}", manifest_path.display()))?;
    let root = metadata.workspace_root.as_std_path();
    let members: Vec<&Package> = metadata.workspace_packages();
    let member_names: BTreeSet<&str> = members.iter().map(|p| p.name.as_str()).collect();

    let mut problems = Vec::new();
    for package in &members {
        check_package(package, &member_names, root, &mut problems)?;
    }

    if problems.is_empty() {
        println!(
            "deps: {} workspace crate(s) follow the ADR-0034 rules",
            members.len()
        );
        return Ok(Outcome::Clean);
    }
    for problem in &problems {
        println!("{problem}");
    }
    println!(
        "deps: {} problem(s). The rules are ADR-0034's; change them with an amendment, then here.",
        problems.len()
    );
    Ok(Outcome::Violations)
}

fn check_package(
    package: &Package,
    members: &BTreeSet<&str>,
    root: &Path,
    problems: &mut Vec<String>,
) -> Result<()> {
    let name = package.name.as_str();
    if package
        .publish
        .as_ref()
        .is_none_or(|registries| !registries.is_empty())
    {
        problems.push(format!("{name}: must set `publish = false` (ADR-0028)"));
    }
    if package.license.as_deref() != Some("MIT") {
        problems.push(format!("{name}: must use the MIT licence"));
    }

    let Some(rule) = rule(name) else {
        problems.push(format!(
            "{name}: not in the ADR-0034 crate table; add it to xtask/src/deps.rs with its allowed dependencies, after amending ADR-0034"
        ));
        return Ok(());
    };

    for dep in &package.dependencies {
        let dep_name = dep.name.as_str();
        let edge = format!("{name} -> {dep_name}");
        let is_member = dep.path.is_some() && members.contains(dep_name);

        if dep_name == TESTKIT && dep.rename.is_some() {
            problems.push(format!(
                "{name} renames mm-testkit; keep its name so `xtask deps` can see where it is used"
            ));
        }

        if !is_member {
            if dep.kind != DependencyKind::Development {
                check_third_party(rule, &edge, dep_name, problems);
            }
            continue;
        }

        let dev = dep.kind == DependencyKind::Development;
        if rule.may_use.contains(&dep_name) || (dev && dep_name == TESTKIT) {
            continue;
        }
        problems.push(format!("{edge}: {}", forbidden_reason(rule, dep_name)));
    }

    let uses_testkit = package.dependencies.iter().any(|d| d.name == TESTKIT);
    if name != TESTKIT && uses_testkit {
        check_testkit_use(package, root, problems)?;
    }
    Ok(())
}

fn check_third_party(rule: &CrateRule, edge: &str, dep: &str, problems: &mut Vec<String>) {
    if allows(rule.third_party, dep) {
        return;
    }
    let why = match rule.third_party {
        ThirdParty::Only(allowed) => {
            format!(
                "may use only these third-party crates: {}",
                allowed.join(", ")
            )
        }
        ThirdParty::Any | ThirdParty::None => "may have no third-party dependencies".to_owned(),
    };
    problems.push(format!("{edge}: {} {why} (ADR-0034)", rule.name));
}

fn allows(third_party: ThirdParty, dep: &str) -> bool {
    match third_party {
        ThirdParty::Any => true,
        ThirdParty::None => false,
        ThirdParty::Only(allowed) => allowed.contains(&dep),
    }
}

/// Why `rule.name` may not depend on the workspace crate `dep`, phrased by the rule broken.
fn forbidden_reason(rule: &CrateRule, dep: &str) -> String {
    match TABLE.iter().find(|r| r.name == dep).map(|r| r.role) {
        Some(Role::Rules) => {
            "only binaries and mm-testkit may depend on a rules crate (ADR-0034)".to_owned()
        }
        Some(Role::Host) => "only binaries may depend on mm-host (ADR-0034)".to_owned(),
        Some(Role::Testkit) => {
            "mm-testkit is used only as a dev-dependency, from integration tests (ADR-0034)"
                .to_owned()
        }
        Some(Role::Binary) => "nothing may depend on a binary (ADR-0034)".to_owned(),
        Some(Role::Library) | None if rule.may_use.is_empty() => {
            format!("{} may depend on no workspace crate (ADR-0034)", rule.name)
        }
        Some(Role::Library) | None => format!(
            "{} may depend only on {} (ADR-0034)",
            rule.name,
            rule.may_use.join(", ")
        ),
    }
}

/// In a crate that depends on `mm-testkit`, no `.rs` file outside `tests/` may name `mm_testkit`.
fn check_testkit_use(package: &Package, root: &Path, problems: &mut Vec<String>) -> Result<()> {
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
        for (index, line) in text.lines().enumerate() {
            if contains_word(line, "mm_testkit") {
                let shown = file.strip_prefix(root).unwrap_or(&file);
                problems.push(format!(
                    "{}:{}: uses mm_testkit outside tests/; oracles are used only from integration tests (ADR-0034)",
                    shown.display(),
                    index + 1
                ));
            }
        }
    }
    Ok(())
}

/// Collects `.rs` files under `dir`, skipping the package's own `tests/` and any `target/`.
fn collect_rust_files(
    package_dir: &Path,
    dir: &Path,
    out: &mut Vec<std::path::PathBuf>,
) -> Result<()> {
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

fn contains_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !ident(before) && !ident(after)
    })
}

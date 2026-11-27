//! `cargo xtask deps` as a process against seeded temporary workspaces (ADR-0034).

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use std::fmt::Write as _;

use common::{MEMBER_FIELDS, TempTree, package_manifest, xtask};

/// A temporary Cargo workspace whose crates declare the given dependencies.
struct Workspace {
    tree: TempTree,
    members: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Normal,
    Dev,
    Build,
}

impl Workspace {
    fn new() -> Self {
        Self {
            tree: TempTree::new(),
            members: Vec::new(),
        }
    }

    /// Adds a crate at `crates/<name>` with path dependencies on other members
    /// and plain registry dependencies on anything else.
    fn krate(mut self, name: &str, deps: &[(&str, Kind)]) -> Self {
        let mut manifest = package_manifest(name, MEMBER_FIELDS);
        for (section, kind) in [
            ("dependencies", Kind::Normal),
            ("dev-dependencies", Kind::Dev),
            ("build-dependencies", Kind::Build),
        ] {
            writeln!(manifest, "\n[{section}]").unwrap();
            for (dep, _) in deps.iter().filter(|(_, k)| *k == kind) {
                if dep.starts_with("mm-") || *dep == "xtask" {
                    writeln!(manifest, "{dep} = {{ path = \"../{dep}\" }}").unwrap();
                } else {
                    writeln!(manifest, "{dep} = \"1\"").unwrap();
                }
            }
        }
        self.tree
            .write(&format!("crates/{name}/Cargo.toml"), manifest);
        self.tree.write(&format!("crates/{name}/src/lib.rs"), "");
        let member = format!("crates/{name}");
        if !self.members.contains(&member) {
            self.members.push(member);
        }
        self
    }

    fn manifest(self, name: &str, manifest: &str) -> Self {
        self.tree
            .write(&format!("crates/{name}/Cargo.toml"), manifest);
        self
    }

    fn file(self, rel: &str, contents: &str) -> Self {
        self.tree.write(rel, contents);
        self
    }

    fn check(self) -> common::XtaskRun {
        let members: Vec<String> = self.members.iter().map(|m| format!("\"{m}\"")).collect();
        self.tree.write(
            "Cargo.toml",
            format!(
                "[workspace]\nresolver = \"3\"\nmembers = [{}]\n",
                members.join(", ")
            ),
        );
        let manifest = self.tree.path().join("Cargo.toml");
        xtask(&["deps", "--manifest-path", manifest.to_str().unwrap()])
    }
}

use Kind::{Build, Dev, Normal};

/// The walking-skeleton crate graph, as spec #12 lays it out.
fn skeleton() -> Workspace {
    Workspace::new()
        .krate("mm-dice", &[])
        .krate(
            "mm-core",
            &[("serde", Normal), ("thiserror", Normal), ("blake3", Normal)],
        )
        .krate(
            "mm-load",
            &[("mm-core", Normal), ("ron", Normal), ("miette", Normal)],
        )
        .krate(
            "mm-engine",
            &[
                ("mm-core", Normal),
                ("mm-load", Normal),
                ("mm-dice", Normal),
                ("mm-testkit", Dev),
                ("proptest", Dev),
            ],
        )
        .krate("mm-rules-2024", &[("mm-core", Normal)])
        .krate(
            "mm-testkit",
            &[
                ("mm-core", Normal),
                ("mm-dice", Normal),
                ("mm-load", Normal),
                ("mm-engine", Normal),
                ("mm-rules-2024", Normal),
            ],
        )
        .krate(
            "mm-cli",
            &[
                ("mm-core", Normal),
                ("mm-load", Normal),
                ("mm-engine", Normal),
                ("mm-rules-2024", Normal),
                ("clap", Normal),
                ("mm-testkit", Dev),
            ],
        )
        .krate("xtask", &[("cargo_metadata", Normal)])
}

#[test]
fn the_walking_skeleton_graph_passes() {
    skeleton().check().assert_ok();
}

#[test]
fn the_engine_may_not_depend_on_a_rules_crate() {
    skeleton()
        .krate(
            "mm-engine",
            &[("mm-core", Normal), ("mm-rules-2024", Normal)],
        )
        .check()
        .assert_fails_with(&[
            "mm-engine -> mm-rules-2024",
            "only binaries and mm-testkit may depend on a rules crate",
        ]);
}

#[test]
fn rules_crates_may_not_depend_on_each_other() {
    skeleton()
        .krate(
            "mm-rules-2014",
            &[("mm-core", Normal), ("mm-rules-2024", Normal)],
        )
        .check()
        .assert_fails_with(&["mm-rules-2014 -> mm-rules-2024", "rules crate"]);
}

#[test]
fn a_rules_crate_depends_on_mm_core_only() {
    skeleton()
        .krate("mm-rules-2024", &[("mm-core", Normal), ("mm-dice", Normal)])
        .check()
        .assert_fails_with(&[
            "mm-rules-2024 -> mm-dice",
            "mm-rules-2024 may depend only on mm-core",
        ]);
}

#[test]
fn mm_core_does_not_depend_on_mm_dice() {
    skeleton()
        .krate("mm-core", &[("serde", Normal), ("mm-dice", Normal)])
        .check()
        .assert_fails_with(&[
            "mm-core -> mm-dice",
            "mm-core may depend on no workspace crate",
        ]);
}

#[test]
fn mm_dice_has_no_third_party_dependencies() {
    skeleton()
        .krate("mm-dice", &[("rustc-hash", Normal)])
        .check()
        .assert_fails_with(&[
            "mm-dice -> rustc-hash",
            "mm-dice may have no third-party dependencies",
        ]);
}

#[test]
fn mm_dice_may_have_third_party_dev_dependencies() {
    skeleton()
        .krate("mm-dice", &[("proptest", Dev), ("mm-testkit", Dev)])
        .check()
        .assert_ok();
}

#[test]
fn mm_core_third_party_dependencies_are_limited() {
    skeleton()
        .krate("mm-core", &[("serde", Normal), ("regex", Build)])
        .check()
        .assert_fails_with(&[
            "mm-core -> regex",
            "mm-core may use only these third-party crates: blake3, serde, thiserror",
        ]);
}

#[test]
fn only_binaries_may_depend_on_mm_host() {
    skeleton()
        .krate("mm-host", &[("mm-core", Normal)])
        .krate("mm-app-core", &[("mm-core", Normal), ("mm-host", Normal)])
        .check()
        .assert_fails_with(&[
            "mm-app-core -> mm-host",
            "only binaries may depend on mm-host",
        ]);
}

#[test]
fn nothing_depends_on_a_binary() {
    skeleton()
        .krate("mm-testkit", &[("mm-core", Normal), ("mm-cli", Normal)])
        .check()
        .assert_fails_with(&["mm-testkit -> mm-cli", "nothing may depend on a binary"]);
}

#[test]
fn mm_testkit_is_only_a_dev_dependency() {
    skeleton()
        .krate("mm-cli", &[("mm-engine", Normal), ("mm-testkit", Normal)])
        .check()
        .assert_fails_with(&["mm-cli -> mm-testkit", "only as a dev-dependency"]);
}

#[test]
fn a_crate_outside_the_table_fails() {
    skeleton()
        .krate("mm-extra", &[])
        .check()
        .assert_fails_with(&["mm-extra: [crate-table]", "not in the ADR-0034 crate table"]);
}

#[test]
fn crates_not_yet_created_follow_the_table_too() {
    // The solver may use the engine, but the loader may not use the solver.
    skeleton()
        .krate("mm-solver", &[("mm-engine", Normal), ("mm-core", Normal)])
        .krate("mm-load", &[("mm-core", Normal), ("mm-solver", Normal)])
        .check()
        .assert_fails_with(&["mm-load -> mm-solver"]);
}

#[test]
fn an_oracle_used_from_a_unit_test_module_fails() {
    skeleton()
        .file(
            "crates/mm-engine/src/lib.rs",
            "#[cfg(test)]\nmod tests {\n    use mm_testkit::oracle;\n}\n",
        )
        .check()
        .assert_fails_with(&[
            "crates/mm-engine/src/lib.rs:3",
            "mm_testkit",
            "only from integration tests",
        ]);
}

#[test]
fn a_comment_naming_the_testkit_is_not_a_use() {
    skeleton()
        .file(
            "crates/mm-engine/src/lib.rs",
            "//! Tested against the oracles in `mm_testkit`.\npub fn f() {} // see mm_testkit\n",
        )
        .check()
        .assert_ok();
}

#[test]
fn an_oracle_used_from_an_integration_test_passes() {
    skeleton()
        .file(
            "crates/mm-engine/tests/oracles.rs",
            "use mm_testkit::oracle;\n",
        )
        .check()
        .assert_ok();
}

#[test]
fn renaming_mm_testkit_fails() {
    skeleton()
        .manifest(
            "mm-dice",
            &package_manifest(
                "mm-dice",
                &format!("{MEMBER_FIELDS}\n[dev-dependencies]\nkit = {{ package = \"mm-testkit\", path = \"../mm-testkit\" }}\n"),
            ),
        )
        .check()
        .assert_fails_with(&["mm-dice -> mm-testkit: [testkit]", "do not rename mm-testkit"]);
}

#[test]
fn every_crate_is_unpublished_and_mit() {
    skeleton()
        .manifest(
            "mm-dice",
            &package_manifest("mm-dice", "license = \"Apache-2.0\"\n"),
        )
        .check()
        .assert_fails_with(&[
            "mm-dice: [package] must set `publish = false`",
            "mm-dice: [package] must use the MIT licence",
        ]);
}

/// Every crate in the ADR-0034 table, so any edge between them resolves.
fn full_graph() -> Workspace {
    let mut workspace = skeleton();
    for name in [
        "mm-import",
        "mm-solver",
        "mm-cache",
        "mm-rules-2014",
        "mm-app-core",
        "mm-host",
        "mm-gui",
        "mm-worker",
    ] {
        workspace = workspace.krate(name, &[]);
    }
    workspace
}

#[test]
fn each_crate_is_held_to_its_own_allowed_set() {
    // (crate, a workspace dependency it may not have, the expected explanation)
    let cases = [
        (
            "mm-load",
            "mm-engine",
            "mm-load may depend only on mm-core (ADR-0034)",
        ),
        (
            "mm-import",
            "mm-engine",
            "mm-import may depend only on mm-core, mm-load (ADR-0034)",
        ),
        (
            "mm-engine",
            "mm-solver",
            "mm-engine may depend only on mm-core, mm-load, mm-dice (ADR-0034)",
        ),
        (
            "mm-solver",
            "mm-cache",
            "mm-solver may depend only on mm-core, mm-load, mm-dice, mm-engine (ADR-0034)",
        ),
        ("mm-cache", "mm-app-core", "mm-cache may depend only on"),
        (
            "mm-app-core",
            "mm-rules-2014",
            "only binaries and mm-testkit may depend on a rules crate",
        ),
        ("mm-testkit", "mm-app-core", "mm-testkit may depend only on"),
        ("mm-host", "mm-gui", "nothing may depend on a binary"),
        ("mm-gui", "mm-worker", "nothing may depend on a binary"),
        ("mm-cli", "xtask", "nothing may depend on a binary"),
        ("xtask", "mm-core", "xtask may depend on no workspace crate"),
    ];
    for (from, to, why) in cases {
        let edge = format!("{from} -> {to}: [allowed-deps]");
        full_graph()
            .krate(from, &[(to, Normal)])
            .check()
            .assert_fails_with(&[&edge, why]);
    }
}

#[test]
fn the_full_crate_map_passes_when_every_crate_follows_its_rules() {
    full_graph()
        .krate(
            "mm-app-core",
            &[
                ("mm-solver", Normal),
                ("mm-cache", Normal),
                ("mm-import", Normal),
            ],
        )
        .krate("mm-host", &[("mm-app-core", Normal)])
        .krate("mm-gui", &[("mm-host", Normal), ("mm-rules-2014", Normal)])
        .check()
        .assert_ok();
}

#[test]
fn the_real_workspace_passes() {
    xtask(&["deps"]).assert_ok();
}

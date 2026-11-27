//! `cargo xtask glossary` as a process against seeded temporary git trees.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::{TempTree, xtask};

const GLOSSARY: &str = "# Glossary\n\n**Mastery property**:\nA weapon's property.\n_Avoid_: mastery, weapon trait\n\n**Content ID**:\nAn identity.\n_Avoid_: content key, source (clashes with book source codes)\n";

/// A Content ID of `kind`, assembled so this file does not match the guard itself.
fn content_id(ruleset: &str, kind: &str) -> String {
    format!("id: \"{ruleset}:{kind}/test-graze|tst\",\n")
}

/// A Rust enum, assembled so this file does not match the guard itself.
fn rust_enum(name: &str, body: &str) -> String {
    format!("pub {} {name} {body}", "enum")
}

fn check(tree: &TempTree) -> common::XtaskRun {
    xtask(&["glossary", "--root", tree.path().to_str().unwrap()])
}

fn tree_with_glossary() -> TempTree {
    let tree = TempTree::git();
    tree.track("GLOSSARY.md", GLOSSARY);
    tree
}

#[test]
fn kinds_named_from_the_glossary_pass() {
    let tree = tree_with_glossary();
    tree.track("fixtures/a.ron", content_id("2024", "mastery-property"));
    check(&tree).assert_ok();
}

#[test]
fn an_avoided_term_as_a_content_id_kind_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "fixtures/a.ron",
        format!("\n{}", content_id("2024", "mastery")),
    );
    check(&tree).assert_fails_with(&[
        "fixtures/a.ron:2",
        "[kind]",
        "\"mastery\"",
        "Mastery property",
    ]);
}

#[test]
fn an_avoided_term_as_a_kind_name_in_rust_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "src/kind.rs",
        "impl Kind for T {\n    const NAME: &'static str = \"weapon-trait\";\n}\n",
    );
    check(&tree).assert_fails_with(&["src/kind.rs:2", "\"weapon-trait\""]);
}

#[test]
fn a_parenthetical_note_is_not_part_of_the_term() {
    let tree = tree_with_glossary();
    tree.track("a.ron", content_id("2014", "source"));
    check(&tree).assert_fails_with(&["a.ron:1", "\"source\""]);
}

#[test]
fn research_notes_stay_as_written() {
    let tree = tree_with_glossary();
    tree.track("docs/research/09.md", content_id("2024", "mastery"));
    check(&tree).assert_ok();
}

#[test]
fn the_repository_passes_and_its_content_enums_are_read() {
    let run = xtask(&["glossary"]);
    run.assert_ok();
    // mm-core's `Record` and `Encoding` have at least these variants between them, so a
    // rename that stops the scan finding them fails here.
    let variants: usize = run
        .stdout()
        .split(" content enum variant(s)")
        .next()
        .and_then(|head| head.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap();
    assert!(variants >= 10, "{}", run.stdout());
}

#[test]
fn brackets_in_comments_and_one_line_enums_do_not_hide_variants() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        format!(
            "{}\n\n{}\n",
            rust_enum(
                "Record",
                "{\n    /// Options: a) one, b) two.\n    Class(A), // (see ADR-0008\n    Mastery(A),\n}"
            ),
            rust_enum("Encoding", "{ Plain, WeaponTrait(B) }")
        ),
    );
    check(&tree).assert_fails_with(&[
        "src/records.rs:4: [variant] the variant \"Mastery\"",
        "src/records.rs:7: [variant] the variant \"WeaponTrait\"",
    ]);
}

#[test]
fn an_avoided_term_as_a_content_enum_variant_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        format!(
            "{}\n\n{}\n",
            rust_enum(
                "Record",
                "{\n    /// A class.\n    Class(NamedRecord),\n    Mastery {\n        id: Id,\n    },\n}"
            ),
            rust_enum("Other", "{\n    WeaponTrait,\n}")
        ),
    );
    let run = check(&tree);
    run.assert_fails_with(&["src/records.rs:4", "[variant]", "\"Mastery\""]);
    assert!(
        !run.stdout().contains("WeaponTrait") && !run.stdout().contains(":5"),
        "only a content enum's own variants are checked\n{}",
        run.stdout()
    );
}

#[test]
fn a_variant_is_read_as_words() {
    let tree = tree_with_glossary();
    tree.track(
        "src/effect.rs",
        rust_enum("Encoding", "{\n    WeaponTrait(FeaturesEncoding),\n}\n"),
    );
    check(&tree).assert_fails_with(&["src/effect.rs:2", "\"WeaponTrait\""]);
}

#[test]
fn a_repository_without_a_glossary_is_an_error() {
    let tree = TempTree::git();
    tree.track("README.md", "# Hello\n");
    let run = check(&tree);
    assert_eq!(run.0.status.code(), Some(2), "{}", run.stderr());
    assert!(
        run.stderr().contains("GLOSSARY.md is not tracked"),
        "{}",
        run.stderr()
    );
}

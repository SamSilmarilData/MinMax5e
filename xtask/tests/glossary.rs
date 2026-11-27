//! `cargo xtask glossary` as a process against seeded temporary git trees.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::{TempTree, xtask};

const GLOSSARY: &str = "# Glossary\n\n**Mastery property**:\nA weapon's property.\n_Avoid_: mastery, weapon trait\n\n**Content ID**:\nAn identity.\n_Avoid_: content key, source (clashes with book source codes)\n\n**Nova DPR**:\nDamage in the opening rounds.\n_Avoid_: DPS, off-hand attack, level 20 snapshot\n";

/// A Content ID of `kind`, assembled so this file does not match the guard itself.
fn content_id(ruleset: &str, kind: &str) -> String {
    format!("id: \"{ruleset}:{kind}/test-graze|tst\",\n")
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
fn a_parenthetical_note_is_not_part_of_the_term() {
    let tree = tree_with_glossary();
    tree.track("a.ron", content_id("2014", "source"));
    check(&tree).assert_fails_with(&["a.ron:1", "\"source\""]);
}

#[test]
fn an_avoided_term_as_a_kind_name_in_rust_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "src/kind.rs",
        "impl Kind for A {\n    const NAME: &'static str = \"class\";\n}\n\nimpl crate::Kind for T {\n    const NAME: &str =\n        \"weapon-trait\";\n}\n",
    );
    check(&tree).assert_fails_with(&["src/kind.rs:7", "[kind-name] the kind \"weapon-trait\""]);
}

#[test]
fn an_avoided_term_as_a_content_enum_variant_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        "pub enum Record {\n    /// A class.\n    Class(NamedRecord),\n    Mastery {\n        id: Id,\n    },\n}\n\npub enum Other {\n    WeaponTrait,\n}\n",
    );
    let run = check(&tree);
    run.assert_fails_with(&["src/records.rs:4", "[variant]", "\"Mastery\""]);
    assert!(
        !run.stdout().contains("WeaponTrait"),
        "only a content enum's own variants are checked\n{}",
        run.stdout()
    );
}

#[test]
fn a_variant_is_read_as_words() {
    let tree = tree_with_glossary();
    tree.track(
        "src/effect.rs",
        "pub enum Encoding {\n    WeaponTrait(FeaturesEncoding),\n}\n",
    );
    check(&tree).assert_fails_with(&["src/effect.rs:2", "\"WeaponTrait\""]);
}

/// Each enum hides `Mastery` behind something a line-based scan misreads.
#[test]
fn nothing_in_a_content_enum_hides_a_later_variant() {
    let hiders = [
        "#[default] A,",
        "#[serde(alias = \"a)\")] A,",
        "#[doc = \"see http://x\"] A,",
        "A(fn(u8) -> u8),",
        "A(Box<dyn Fn() -> B>),",
        "A = 1 << 2,",
        "A, /* ) */",
        "/// Options: a) one, b) two.\n    A, // (see ADR-0008",
    ];
    for hider in hiders {
        let tree = tree_with_glossary();
        tree.track(
            "src/records.rs",
            format!("pub enum Record {{\n    {hider}\n    Mastery,\n}}\n"),
        );
        let run = check(&tree);
        assert!(
            run.stdout().contains("[variant] the variant \"Mastery\""),
            "{hider:?} hid the variant\n{}",
            run.stdout()
        );
    }
}

#[test]
fn a_one_line_content_enum_is_read() {
    let tree = tree_with_glossary();
    tree.track(
        "src/effect.rs",
        "pub enum Encoding { Plain, WeaponTrait(B) }\n",
    );
    check(&tree).assert_fails_with(&["src/effect.rs:1", "\"WeaponTrait\""]);
}

#[test]
fn a_serde_rename_is_the_name_ron_sees() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        "pub enum Record {\n    #[serde(rename = \"mastery\")]\n    MasteryProperty,\n}\n",
    );
    check(&tree).assert_fails_with(&["src/records.rs:2", "[variant] the variant \"mastery\""]);
}

#[test]
fn every_serde_name_form_is_read() {
    let attrs = [
        "#[serde(rename = \"mastery\")]",
        "#[serde(bound(serialize = \"T: X\"), rename = \"mastery\")]",
        "#[serde(rename(serialize = \"plain\", deserialize = \"mastery\"))]",
        "#[serde(alias = \"mastery\")]",
        "#[serde(default)] #[serde(skip_serializing_if = \"x\", alias = \"mastery\")]",
    ];
    for attr in attrs {
        let tree = tree_with_glossary();
        tree.track(
            "src/records.rs",
            format!("pub enum Record {{\n    {attr}\n    MasteryProperty,\n}}\n"),
        );
        let run = check(&tree);
        assert!(
            run.stdout().contains("[variant] the variant \"mastery\""),
            "{attr:?} hid the name\n{}",
            run.stdout()
        );
    }
}

#[test]
fn names_match_terms_whatever_their_case_and_separators() {
    let cases = [
        ("OffHandAttack", "\"OffHandAttack\""),
        ("Dps", "\"Dps\""),
        ("Level20Snapshot", "\"Level20Snapshot\""),
        ("#[serde(rename = \"DPS\")]\n    Damage", "\"DPS\""),
    ];
    for (variant, needle) in cases {
        let tree = tree_with_glossary();
        tree.track(
            "src/effect.rs",
            format!("pub enum Encoding {{\n    {variant},\n}}\n"),
        );
        check(&tree).assert_fails_with(&["[variant]", needle]);
    }
    let tree = tree_with_glossary();
    tree.track("fixtures/a.ron", content_id("2024", "off-hand-attack"));
    check(&tree).assert_fails_with(&["fixtures/a.ron:1", "\"off-hand-attack\""]);
}

#[test]
fn a_serde_name_inside_cfg_attr_is_read() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        "pub enum Record {\n    #[cfg_attr(all(feature = \"x\", not(test)), serde(default), serde(rename = \"mastery\"))]\n    MasteryProperty,\n}\n",
    );
    check(&tree).assert_fails_with(&["src/records.rs:2", "[variant] the variant \"mastery\""]);
}

#[test]
fn a_kind_trait_imported_under_another_name_is_read() {
    let tree = tree_with_glossary();
    tree.track(
        "src/kind.rs",
        "use crate::Kind as K;\n\nimpl K for T {\n    const NAME: &str = \"mastery\";\n}\n",
    );
    check(&tree).assert_fails_with(&["src/kind.rs:4", "[kind-name] the kind \"mastery\""]);
}

#[test]
fn raw_identifiers_are_read_as_their_names() {
    let tree = tree_with_glossary();
    tree.track(
        "src/records.rs",
        "pub enum r#Record {\n    r#Mastery,\n}\n\nimpl r#Kind for T {\n    const r#NAME: &str = \"weapon-trait\";\n}\n",
    );
    check(&tree).assert_fails_with(&[
        "src/records.rs:2: [variant] the variant \"Mastery\"",
        "src/records.rs:6: [kind-name] the kind \"weapon-trait\"",
    ]);
}

#[test]
fn only_kind_impls_give_kind_names() {
    let tree = tree_with_glossary();
    tree.track(
        "src/lens.rs",
        "impl Lens for X {\n    const NAME: &str = \"mastery\";\n}\n\nimpl Other for Y {\n    const NAME: u8 = 1;\n}\n",
    );
    check(&tree).assert_ok();
}

#[test]
fn a_name_the_guard_cannot_read_fails() {
    let tree = tree_with_glossary();
    tree.track(
        "src/kind.rs",
        "impl Kind for T {\n    const NAME: &str = OTHER;\n}\n",
    )
    .track(
        "src/records.rs",
        "pub enum Record {\n    #[serde(rename = 3)]\n    A,\n}\n",
    );
    check(&tree).assert_fails_with(&[
        "src/kind.rs:2: [parse] cannot check a Kind::NAME that is not a string literal",
        "src/records.rs:2: [parse] cannot check the serde attribute on A",
    ]);
}

#[test]
fn a_rust_file_that_does_not_parse_fails() {
    let tree = tree_with_glossary();
    tree.track("src/broken.rs", "pub enum Record {\n    A,\n");
    check(&tree).assert_fails_with(&["src/broken.rs", "[parse]"]);
}

#[test]
fn research_notes_stay_as_written() {
    let tree = tree_with_glossary();
    tree.track("docs/research/09.md", content_id("2024", "mastery"));
    check(&tree).assert_ok();
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

/// The number the clean summary line gives before `label`.
fn count_before(stdout: &str, label: &str) -> usize {
    stdout
        .split(label)
        .next()
        .and_then(|head| head.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("the summary has no count before {label:?}:\n{stdout}"))
}

#[test]
fn the_repository_passes_and_its_kinds_and_content_enums_are_read() {
    let run = xtask(&["glossary"]);
    run.assert_ok();
    // mm-core has at least these between its `Kind` impls and its `Record` and `Encoding`
    // enums, so a rename that stops the scan finding them fails here.
    let stdout = run.stdout();
    assert!(count_before(&stdout, " Kind::NAME(s)") >= 6, "{stdout}");
    assert!(
        count_before(&stdout, " content enum variant(s)") >= 10,
        "{stdout}"
    );
}

//! `mm-cli validate-packs` as a process (spec #12, seam 1).

mod common;

use std::process::{Command, Output};

use common::{stderr_of_failure, stdout_of};

/// Runs `mm-cli validate-packs --pack-dir <dir>…` from the fixtures directory, so the
/// file names in its output are relative and the snapshots are machine-independent.
fn validate_packs(dirs: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mm-cli"));
    command
        .current_dir(mm_testkit::fixtures::dir())
        .arg("validate-packs");
    for dir in dirs {
        command.args(["--pack-dir", dir]);
    }
    command.output().expect("mm-cli runs")
}

#[test]
fn the_good_fixtures_validate_with_every_in_scope_id_encoded() {
    let stdout = stdout_of(&validate_packs(&["pack"]));
    for id in [
        "2024:class/test-warrior|tst",
        "2024:species/test-folk|tst",
        "2024:background/test-drillhand|tst",
        "2024:feat/test-heavy-hitter|tst",
        "2024:mastery-property/test-graze|tst",
    ] {
        assert!(stdout.contains(&format!("encoded  {id}")), "{stdout}");
    }
    assert!(!stdout.contains("missing"), "{stdout}");
}

#[test]
fn a_record_without_an_encoding_is_reported_missing_and_fails() {
    let stderr = stderr_of_failure(&validate_packs(&["pack", "malformed/unencoded"]));
    assert!(
        stderr.contains("missing  2024:class/test-unencoded|tst"),
        "{stderr}"
    );
    assert!(
        stderr.contains("encoded  2024:class/test-warrior|tst"),
        "{stderr}"
    );
}

/// `validate-packs` on the good pack plus the malformed fixture `case`: its stderr, which
/// must name the file and quote the offending text.
fn diagnostic_for(case: &str, file: &str, offending: &str) -> String {
    let stderr = stderr_of_failure(&validate_packs(&["pack", &format!("malformed/{case}")]));
    assert!(
        stderr.contains(&format!("malformed/{case}/{file}")),
        "names the file: {stderr}"
    );
    assert!(stderr.contains(offending), "quotes the text: {stderr}");
    stderr
}

#[test]
fn an_unknown_content_id_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "unknown-content-id",
        "encodings/test-nobody.ron",
        "\"2024:class/test-nobody|tst\"",
    );
    insta::assert_snapshot!("unknown-content-id", stderr);
}

#[test]
fn a_content_id_with_the_wrong_ruleset_prefix_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "wrong-ruleset-prefix",
        "encodings/test-warrior.ron",
        "\"2014:class/test-warrior|tst\"",
    );
    assert!(stderr.contains("belongs to Ruleset `2014`"), "{stderr}");
    insta::assert_snapshot!("wrong-ruleset-prefix", stderr);
}

#[test]
fn a_misspelled_dsl_variant_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "misspelled-dsl-variant",
        "encodings/test-warrior.ron",
        "AttackHt",
    );
    insta::assert_snapshot!("misspelled-dsl-variant", stderr);
}

#[test]
fn a_feature_without_a_citation_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "missing-citation",
        "encodings/test-warrior.ron",
        "`citation`",
    );
    insta::assert_snapshot!("missing-citation", stderr);
}

#[test]
fn a_weapon_naming_an_unknown_mastery_property_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "unknown-mastery-property",
        "records/test-badblade.ron",
        "\"2024:mastery-property/test-nothing|tst\"",
    );
    insta::assert_snapshot!("unknown-mastery-property", stderr);
}

#[test]
fn an_encoding_reading_an_option_its_ruling_lacks_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "unknown-ruling-option",
        "encodings/test-ruled.ron",
        "\"sometimes\"",
    );
    assert!(
        stderr.contains("Ruling `R-TST-HEW-BONUS` has no option `sometimes`"),
        "{stderr}"
    );
    insta::assert_snapshot!("unknown-ruling-option", stderr);
}

#[test]
fn a_ruling_whose_default_is_not_an_option_is_a_diagnostic() {
    let stderr = diagnostic_for(
        "ruling-default-not-an-option",
        "rulings/R-TST-BAD-DEFAULT.ron",
        "\"third\"",
    );
    insta::assert_snapshot!("ruling-default-not-an-option", stderr);
}

//! `mm-cli evaluate` as a process (spec #12, seam 1).

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(path: &str) -> PathBuf {
    mm_testkit::fixtures::dir().join(path)
}

fn evaluate(build: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mm-cli"))
        .arg("evaluate")
        .arg(fixture(build))
        .arg("--pack-dir")
        .arg(fixture("pack"))
        .args(extra)
        .output()
        .expect("mm-cli runs")
}

fn stdout_of(output: &Output) -> String {
    assert!(
        output.status.success(),
        "mm-cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

/// The DPR and SD printed on the table row for `ac`, and whether that row is highlighted.
fn table_row(text: &str, ac: i32) -> (bool, f64, f64) {
    text.lines()
        .find_map(|line| {
            let highlighted = line.starts_with('>');
            let mut cells = line.trim_start_matches('>').split_whitespace();
            (cells.next()? == ac.to_string()).then(|| {
                let dpr = cells.next().expect("DPR cell").parse().expect("DPR number");
                let sd = cells.next().expect("SD cell").parse().expect("SD number");
                (highlighted, dpr, sd)
            })
        })
        .unwrap_or_else(|| panic!("no row for AC {ac} in:\n{text}"))
}

// By hand, for the level-1 warrior (one 2d6 + 3 attack at +5) against AC 16:
// it needs an 11, so miss 10/20, hit 9/20, crit 1/20. A hit deals 2d6 + 3 (mean 10,
// E[X²] = 35/6 + 100); a crit doubles the dice, 4d6 + 3 (mean 17, E[X²] = 35/3 + 289).
// Mean per round = 9/20 · 10 + 1/20 · 17 = 5.35.
// E[X²] per round = 9/20 · 635/6 + 1/20 · 902/3 = 751.9/12; Var = 751.9/12 − 5.35² = 34.0358…
// Nova is rounds 1–2, independent here, so the SD of the per-round average is
// √(2 · 34.0358…) / 2 = 4.1253 (to 4 places).
const WARRIOR_1_DPR_AC16: f64 = 5.35;
const WARRIOR_1_SD_AC16: f64 = 4.1253;

#[test]
fn evaluate_prints_nova_dpr_and_sd_across_ac_with_the_target_highlighted() {
    let text = stdout_of(&evaluate("builds/warrior-1.ron", &["--ac", "16"]));

    let (highlighted, dpr, sd) = table_row(&text, 16);
    assert!(
        highlighted,
        "AC 16 is the target and is highlighted:\n{text}"
    );
    assert!((dpr - WARRIOR_1_DPR_AC16).abs() < 0.0005, "DPR {dpr}");
    assert!((sd - WARRIOR_1_SD_AC16).abs() < 0.0005, "SD {sd}");

    for ac in 10..=25 {
        let (highlighted, _, _) = table_row(&text, ac);
        assert_eq!(highlighted, ac == 16, "only the target AC is highlighted");
    }
}

#[test]
fn evaluate_ron_output_is_a_machine_readable_evaluation() {
    let ron = stdout_of(&evaluate(
        "builds/warrior-1.ron",
        &["--ac", "16", "--format", "ron"],
    ));
    insta::assert_snapshot!("warrior-1", ron);
}

/// Writes `contents` to `name` under the scratch directory `dir`, and returns its path.
fn scratch_file(dir: &str, name: &str, contents: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(dir)
        .join(name);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("scratch dir");
    std::fs::write(&path, contents).expect("scratch file");
    path
}

fn fixture_text(path: &str) -> String {
    std::fs::read_to_string(fixture(path)).expect("fixture exists")
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mm-cli"))
        .args(args)
        .output()
        .expect("mm-cli runs")
}

fn stderr_of_failure(output: &Output) -> String {
    assert!(!output.status.success(), "mm-cli should have failed");
    String::from_utf8(output.stderr.clone()).expect("UTF-8 output")
}

#[test]
fn a_build_mixing_rulesets_is_rejected() {
    let build = fixture_text("builds/warrior-1.ron").replace(
        "\"2024:item/test-greatblade|tst\"",
        "\"test:item/test-greatblade|tst\"",
    );
    let build = scratch_file("mixed", "build.ron", &build);
    let pack = fixture("pack");
    let stderr = stderr_of_failure(&run(&[
        "evaluate".as_ref(),
        build.as_os_str(),
        "--ac".as_ref(),
        "16".as_ref(),
        "--pack-dir".as_ref(),
        pack.as_os_str(),
    ]));
    assert!(stderr.contains("mixes Rulesets (2024, test)"), "{stderr}");
}

#[test]
fn a_pack_id_from_another_ruleset_is_a_load_error() {
    let records = fixture_text("pack/records/test-content.ron").replace(
        "\"2024:item/test-greatblade|tst\"",
        "\"2014:item/test-greatblade|tst\"",
    );
    let records = scratch_file("cross-prefix", "pack/records/test-content.ron", &records);
    let pack = records.parent().and_then(|p| p.parent()).expect("pack dir");
    let encoding = fixture_text("pack/encodings/test-warrior.ron");
    scratch_file("cross-prefix", "pack/encodings/test-warrior.ron", &encoding);
    let build = fixture("builds/warrior-1.ron");

    let stderr = stderr_of_failure(&run(&[
        "evaluate".as_ref(),
        build.as_os_str(),
        "--ac".as_ref(),
        "16".as_ref(),
        "--pack-dir".as_ref(),
        pack.as_os_str(),
    ]));
    assert!(
        stderr.contains("2014:item/test-greatblade|tst belongs to Ruleset `2014`"),
        "{stderr}"
    );
    assert!(
        stderr.contains("test-content.ron"),
        "names the file: {stderr}"
    );
}

/// `(ShapeKey, ParamsFp)` from the text output.
fn keys(text: &str) -> (String, String) {
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name))
            .unwrap_or_else(|| panic!("no {name} in:\n{text}"))
            .trim()
            .to_owned()
    };
    (field("ShapeKey"), field("ParamsFp"))
}

#[test]
fn equal_inputs_give_equal_fingerprints_and_new_numbers_change_only_the_params() {
    let first = keys(&stdout_of(&evaluate(
        "builds/warrior-1.ron",
        &["--ac", "16"],
    )));
    let again = keys(&stdout_of(&evaluate(
        "builds/warrior-1.ron",
        &["--ac", "16"],
    )));
    assert_eq!(first, again);

    // Str 18 changes the to-hit and damage, not the structure of the turn.
    let stronger = fixture_text("builds/warrior-1.ron").replace("str: 16", "str: 18");
    let stronger = scratch_file("fingerprints", "build.ron", &stronger);
    let pack = fixture("pack");
    let stronger = keys(&stdout_of(&run(&[
        "evaluate".as_ref(),
        stronger.as_os_str(),
        "--ac".as_ref(),
        "16".as_ref(),
        "--pack-dir".as_ref(),
        pack.as_os_str(),
    ])));
    assert_eq!(stronger.0, first.0, "same ShapeKey");
    assert_ne!(stronger.1, first.1, "different ParamsFp");
}

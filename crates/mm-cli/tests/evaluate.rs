//! `mm-cli evaluate` as a process (spec #12, seam 1).

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{stderr_of_failure, stdout_of};
use mm_testkit::tolerance::assert_close;
use serde::Deserialize;

fn fixture(path: &str) -> PathBuf {
    mm_testkit::fixtures::dir().join(path)
}

fn fixture_text(path: &str) -> String {
    std::fs::read_to_string(fixture(path)).expect("fixture exists")
}

/// Runs `mm-cli evaluate <build> --ac 16 --pack-dir <pack> <extra>…`.
fn evaluate_with(build: &Path, pack: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mm-cli"))
        .arg("evaluate")
        .arg(build)
        .args(["--ac", "16", "--pack-dir"])
        .arg(pack)
        .args(extra)
        .output()
        .expect("mm-cli runs")
}

/// Evaluates the fixture Build `build` on the fixture pack against AC 16.
fn evaluate(build: &str, extra: &[&str]) -> Output {
    evaluate_with(&fixture(build), &fixture("pack"), extra)
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

/// The parts of `--format ron` output these tests read.
#[derive(Debug, Deserialize)]
struct Evaluation {
    shape_key: String,
    params_fp: String,
    nova: Mode,
}

#[derive(Debug, Deserialize)]
struct Mode {
    lanes: Vec<Lane>,
}

#[derive(Debug, Deserialize)]
struct Lane {
    ac: i32,
    dpr: f64,
    sd: f64,
}

fn evaluation(output: &Output) -> Evaluation {
    ron::from_str(&stdout_of(output)).expect("the RON output is an Evaluation")
}

// Exact fractions, so `assert_close` (spec #12's 1e-12 relative bound) applies.
// By hand, for the level-1 warrior (one 2d6 + 3 attack at +5) against AC 16:
// it needs an 11, so miss 10/20, hit 9/20, crit 1/20. A hit deals 2d6 + 3 (mean 10,
// E[X²] = 35/6 + 100 = 635/6); a crit doubles the dice, 4d6 + 3 (mean 17,
// E[X²] = 35/3 + 289 = 902/3).
// Mean per round = 9/20 · 10 + 1/20 · 17 = 5.35.
// E[X²] per round = 9/20 · 635/6 + 1/20 · 902/3 = 751.9/12, so Var = 751.9/12 − 5.35².
// Nova is rounds 1–2, independent here, so the SD of the per-round average is
// √(2 · Var) / 2 ≈ 4.12528.
const WARRIOR_1_DPR_AC16: f64 = 5.35;
const WARRIOR_1_VAR_PER_ROUND_AC16: f64 = 751.9 / 12.0 - 5.35 * 5.35;

#[test]
fn evaluate_matches_the_hand_derived_nova_dpr_and_sd_at_ac_16() {
    let evaluation = evaluation(&evaluate("builds/warrior-1.ron", &["--format", "ron"]));
    let lane = evaluation
        .nova
        .lanes
        .iter()
        .find(|lane| lane.ac == 16)
        .expect("an AC 16 lane");
    assert_close(lane.dpr, WARRIOR_1_DPR_AC16);
    assert_close(lane.sd, (2.0 * WARRIOR_1_VAR_PER_ROUND_AC16).sqrt() / 2.0);
    let acs: Vec<i32> = evaluation.nova.lanes.iter().map(|lane| lane.ac).collect();
    assert_eq!(acs, (10..=25).collect::<Vec<_>>(), "one lane per AC 10–25");
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

#[test]
fn evaluate_prints_nova_dpr_and_sd_across_ac_with_the_target_highlighted() {
    let text = stdout_of(&evaluate("builds/warrior-1.ron", &[]));

    let (highlighted, dpr, sd) = table_row(&text, 16);
    assert!(
        highlighted,
        "AC 16 is the target and is highlighted:\n{text}"
    );
    // The table prints three decimals: 5.35 and 4.12528 to the nearest 0.001.
    assert_eq!((dpr, sd), (5.350, 4.125));

    for ac in 10..=25 {
        let (highlighted, _, _) = table_row(&text, ac);
        assert_eq!(highlighted, ac == 16, "only the target AC is highlighted");
    }
}

#[test]
fn evaluate_ron_output_is_a_machine_readable_evaluation() {
    let ron = stdout_of(&evaluate("builds/warrior-1.ron", &["--format", "ron"]));
    insta::assert_snapshot!("warrior-1", ron);
}

#[test]
fn a_build_mixing_rulesets_is_rejected() {
    let build = fixture_text("builds/warrior-1.ron").replace(
        "\"2024:item/test-greatblade|tst\"",
        "\"test:item/test-greatblade|tst\"",
    );
    let build = scratch_file("mixed", "build.ron", &build);
    let stderr = stderr_of_failure(&evaluate_with(&build, &fixture("pack"), &[]));
    assert!(stderr.contains("mixes Rulesets (2024, test)"), "{stderr}");
}

#[test]
fn a_pack_id_from_another_ruleset_is_a_load_error() {
    let records = fixture_text("pack/records/test-content.ron").replace(
        "\"2024:item/test-greatblade|tst\"",
        "\"2014:item/test-greatblade|tst\"",
    );
    let records = scratch_file("cross-prefix", "pack/records/test-content.ron", &records);
    let pack = records.parent().and_then(Path::parent).expect("pack dir");
    let encoding = fixture_text("pack/encodings/test-warrior.ron");
    scratch_file("cross-prefix", "pack/encodings/test-warrior.ron", &encoding);

    let stderr = stderr_of_failure(&evaluate_with(&fixture("builds/warrior-1.ron"), pack, &[]));
    assert!(
        stderr.contains("2014:item/test-greatblade|tst belongs to Ruleset `2014`"),
        "{stderr}"
    );
    assert!(
        stderr.contains("test-content.ron"),
        "names the file: {stderr}"
    );
}

#[test]
fn equal_inputs_give_equal_fingerprints_and_new_numbers_change_only_the_params() {
    let keys = |output: &Output| {
        let evaluation = evaluation(output);
        (evaluation.shape_key, evaluation.params_fp)
    };
    let first = keys(&evaluate("builds/warrior-1.ron", &["--format", "ron"]));
    let again = keys(&evaluate("builds/warrior-1.ron", &["--format", "ron"]));
    assert_eq!(first, again);

    // Str 18 changes the to-hit and damage, not the structure of the turn.
    let stronger = fixture_text("builds/warrior-1.ron").replace("str: 16", "str: 18");
    let stronger = scratch_file("fingerprints", "build.ron", &stronger);
    let stronger = keys(&evaluate_with(
        &stronger,
        &fixture("pack"),
        &["--format", "ron"],
    ));
    assert_eq!(stronger.0, first.0, "same ShapeKey");
    assert_ne!(stronger.1, first.1, "different ParamsFp");
}

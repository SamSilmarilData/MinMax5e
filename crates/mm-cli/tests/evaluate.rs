//! `mm-cli evaluate` as a process (spec #12, seam 1).

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{stderr_of_failure, stdout_of};
use mm_testkit::tolerance::{assert_close, assert_rounds_to};
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
    minimum: f64,
    band: Band,
    kill: Option<Kill>,
}

#[derive(Debug, Deserialize)]
struct Kill {
    time_to_kill: Vec<f64>,
    kill_probability: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct Band {
    p10: f64,
    p50: f64,
    p90: f64,
    floor: f64,
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

/// The target's hit points in the snapshots: within reach of every fixture Build over the
/// Nova horizon, so their Kill odds are not all 0.
const SNAPSHOT_HP: &str = "30";

#[test]
fn evaluate_ron_output_is_snapshotted_for_every_fixture_build() {
    let mut builds: Vec<PathBuf> = std::fs::read_dir(fixture("builds"))
        .expect("the builds directory")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    builds.sort();
    for build in builds {
        let name = build.file_stem().expect("a file name").to_string_lossy();
        let ron = stdout_of(&evaluate_with(
            &build,
            &fixture("pack"),
            &["--hp", SNAPSHOT_HP, "--format", "ron"],
        ));
        // No result is negative, so a −0.0 is a sign error, such as an empty `f64` `sum`.
        assert!(
            !ron.contains("-0.0"),
            "{name} prints a negative zero:\n{ron}"
        );
        insta::assert_snapshot!(name.into_owned(), ron);
    }
}

/// The AC 16 lane of `build` evaluated over Nova round 1 alone: a single round's DPR
/// and SD (GLOSSARY.md, **SD**).
fn round_1_at_ac_16(build: &str) -> Lane {
    ac_16_lane(build, &["--nova-rounds", "1"])
}

/// The AC 16 lane of `build` evaluated with the flags `extra`.
fn ac_16_lane(build: &str, extra: &[&str]) -> Lane {
    let args: Vec<&str> = extra.iter().copied().chain(["--format", "ron"]).collect();
    evaluation(&evaluate(build, &args))
        .nova
        .lanes
        .into_iter()
        .find(|lane| lane.ac == 16)
        .expect("an AC 16 lane")
}

// Research 03 §7.2's figures are rounded, so each is checked to within half a unit of its
// last stated digit.

#[test]
fn the_level_5_warrior_prints_research_03_mean_and_sd_for_nova_round_1() {
    let text = stdout_of(&evaluate("builds/warrior-5.ron", &["--nova-rounds", "1"]));
    assert!(text.starts_with("Nova DPR, round 1,"), "{text}");
    let (highlighted, dpr, sd) = table_row(&text, 16);
    assert!(highlighted, "{text}");
    // The table prints three decimals. The crit-gated Bonus Action attack is its own
    // Attack source, so the feat's +PB on Attack-action hits does not reach it: 22.897,
    // not the 23.07 of the alternative reading (research 03 §7.2, "Hew counted with +PB").
    assert_rounds_to("mean", dpr, 22.897, 0.001);
    assert_rounds_to("SD", sd, 10.26, 0.01);
    // Then the minimum and the Band (p10, p50, p90, Floor), to one decimal.
    let row = text
        .lines()
        .find(|line| line.starts_with('>'))
        .expect("the target row");
    let band: Vec<&str> = row.split_whitespace().skip(4).collect();
    assert_eq!(band, ["8.0", "8.0", "20.0", "34.0", "8.0"], "{text}");
}

#[test]
fn with_hp_the_text_output_prints_kill_odds_by_round() {
    let text = stdout_of(&evaluate("builds/warrior-1.ron", &["--hp", "6"]));
    // The hand count above: 39/80 in round 1, 59/80 by round 2.
    assert!(text.contains("Against 6 HP at AC 16, by round:"), "{text}");
    let rows: Vec<Vec<&str>> = text
        .lines()
        .skip_while(|line| !line.contains("Time-to-kill"))
        .skip(1)
        .take(2)
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(
        rows,
        [["1", "48.75%", "48.75%"], ["2", "25.00%", "73.75%"]],
        "{text}"
    );
    assert!(text.contains("survives the horizon: 26.25%"), "{text}");
}

#[test]
fn the_level_5_warrior_reproduces_research_03_reliability_band_for_nova_round_1() {
    assert_research_03_band(&round_1_at_ac_16("builds/warrior-5.ron"));
}

/// Research 03 §7.2's round: minimum 8, p10 8, median 20, p90 34. Both attacks graze (8)
/// with probability 0.40² = 0.16, more than a tenth, so the worst tenth of rounds is all
/// 8s: the Floor is 8 too.
fn assert_research_03_band(lane: &Lane) {
    assert_close(lane.minimum, 8.0);
    let band = &lane.band;
    assert_close(band.p10, 8.0);
    assert_close(band.p50, 20.0);
    assert_close(band.p90, 34.0);
    assert_close(band.floor, 8.0);
}

#[test]
fn over_two_rounds_the_band_is_still_that_of_a_round() {
    // The warrior's rounds are alike and spend nothing, so a round picked from rounds 1–2
    // has research 03 §7.2's round distribution: the Floor is the worst 10% of rounds,
    // not of two-round totals.
    assert_research_03_band(&ac_16_lane("builds/warrior-5.ron", &["--nova-rounds", "2"]));
}

/// The AC 16 lane's Kill odds for `build` over the default Nova horizon (rounds 1–2)
/// against `hp`.
fn kill_at_ac_16(build: &str, hp: &str) -> Kill {
    ac_16_lane(build, &["--hp", hp])
        .kill
        .expect("Kill odds, with --hp given")
}

// By hand, for the level-1 warrior (one 2d6 + 3 attack: miss 1/2, hit 9/20, crit 1/20 at
// AC 16) against a 6 HP target over rounds 1–2. A hit deals at least 5 and fells it
// unless the 2d6 is 2 (1/36); a crit deals at least 7.
// Round 1: 9/20 · 35/36 + 1/20 = 39/80.
// It survives both rounds only on totals below 6: two misses (1/4), or a miss and a hit
// for exactly 5, in either order (2 · 1/2 · 9/20 · 1/36 = 1/80); two hits deal at least
// 10. So by round 2: 1 − 21/80 = 59/80, and the first kill is in round 2 with 20/80.
#[test]
fn time_to_kill_and_kill_probability_match_a_hand_count() {
    let kill = kill_at_ac_16("builds/warrior-1.ron", "6");
    assert_eq!(kill.kill_probability.len(), 2, "one entry per round");
    assert_close(kill.kill_probability[0], 39.0 / 80.0);
    assert_close(kill.kill_probability[1], 59.0 / 80.0);
    assert_close(kill.time_to_kill[0], 39.0 / 80.0);
    assert_close(kill.time_to_kill[1], 20.0 / 80.0);
}

#[test]
fn without_hp_there_are_no_kill_odds() {
    let evaluation = evaluation(&evaluate("builds/warrior-1.ron", &["--format", "ron"]));
    assert!(evaluation.nova.lanes.iter().all(|lane| lane.kill.is_none()));
}

#[test]
fn the_warrior_variants_reproduce_research_03_variant_table() {
    for (build, mean, sd) in [
        ("builds/warrior-5-no-hew.ron", 22.00, 8.64),
        ("builds/warrior-5-no-graze.ron", 19.54, 12.49),
        ("builds/warrior-5-no-style.ron", 21.53, 9.57),
    ] {
        let lane = round_1_at_ac_16(build);
        assert_rounds_to(&format!("{build} mean"), lane.dpr, mean, 0.01);
        assert_rounds_to(&format!("{build} SD"), lane.sd, sd, 0.01);
    }
}

#[test]
fn a_build_mixing_rulesets_is_rejected() {
    let build = fixture_text("builds/warrior-1.ron").replace(
        "\"2024:item/test-plainblade|tst\"",
        "\"test:item/test-plainblade|tst\"",
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

#[test]
fn a_rider_the_engine_cannot_run_yet_is_a_compile_error_naming_its_feature() {
    // A once-per-turn limit needs resource tracking, which arrives with #22.
    let feat = fixture_text("pack/encodings/test-heavy-striker.ron").replace(
        "effect: ProficiencyBonusDamage,",
        "limit: Some(OncePerTurn),\n                    effect: ProficiencyBonusDamage,",
    );
    let pack = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unsupported/pack");
    let _ = std::fs::remove_dir_all(&pack);
    copy_dir(&fixture("pack"), &pack);
    std::fs::write(pack.join("encodings/test-heavy-striker.ron"), feat).expect("write");

    let stderr = stderr_of_failure(&evaluate_with(
        &fixture("builds/warrior-5-no-hew.ron"),
        &pack,
        &[],
    ));
    assert!(
        stderr.contains("Test Heavy Striker has a rider the engine cannot run yet"),
        "{stderr}"
    );
}

/// Copies the directory tree `from` to `to`.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create dir");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let path = entry.expect("entry").path();
        let target = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copy");
        }
    }
}

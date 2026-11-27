//! d20 attack outcomes against the hand-derived table in research 03 §1.2.
//!
//! Each probability is one integer count divided once, so it is the correctly rounded
//! value of the exact fraction and equals the decimal literal bit for bit.

use mm_dice::{AttackRoll, OutcomeOdds, RollMode};

fn odds(bonus: i32, ac: i32, crit_on: i32, mode: RollMode) -> OutcomeOdds {
    AttackRoll {
        bonus,
        ac,
        crit_on,
        mode,
    }
    .odds()
}

fn assert_odds(got: OutcomeOdds, miss: f64, hit: f64, crit: f64) {
    assert_eq!(
        [got.miss.to_bits(), got.hit.to_bits(), got.crit.to_bits()],
        [miss.to_bits(), hit.to_bits(), crit.to_bits()],
        "got {got:?}, expected miss {miss}, hit {hit}, crit {crit}"
    );
}

#[test]
fn plus_seven_against_ac_16_matches_research_03_table() {
    assert_odds(odds(7, 16, 20, RollMode::Normal), 0.40, 0.55, 0.05);
    assert_odds(odds(7, 16, 20, RollMode::Advantage), 0.16, 0.7425, 0.0975);
    assert_odds(
        odds(7, 16, 20, RollMode::Disadvantage),
        0.64,
        0.3575,
        0.0025,
    );
}

#[test]
fn widened_crit_range_moves_odds_from_hit_to_crit() {
    // Research 03 §1.2, last column: crit on 19–20.
    assert_odds(odds(7, 16, 19, RollMode::Normal), 0.40, 0.50, 0.10);
    assert_odds(odds(7, 16, 19, RollMode::Advantage), 0.16, 0.65, 0.19);
    assert_odds(odds(7, 16, 19, RollMode::Disadvantage), 0.64, 0.35, 0.01);
}

#[test]
fn natural_one_always_misses_and_natural_twenty_always_crits() {
    // AC far below the bonus: only a natural 1 misses.
    assert_odds(odds(20, 5, 20, RollMode::Normal), 0.05, 0.90, 0.05);
    // AC far above the bonus: only a natural 20 lands, as a crit.
    assert_odds(odds(0, 40, 20, RollMode::Normal), 0.95, 0.0, 0.05);
}

#[test]
fn crit_range_hits_even_when_the_roll_falls_short_of_ac() {
    // Research 03 Open Q1, read as "a crit always hits": 19 + 0 < 30, yet 19 crits.
    assert_odds(odds(0, 30, 19, RollMode::Normal), 0.90, 0.0, 0.10);
}

#[test]
fn one_more_to_hit_equals_one_less_ac_bit_for_bit() {
    for mode in [
        RollMode::Normal,
        RollMode::Advantage,
        RollMode::Disadvantage,
    ] {
        for bonus in -5..=20 {
            for ac in 5..=35 {
                for crit_on in [18, 19, 20] {
                    let a = odds(bonus + 1, ac, crit_on, mode);
                    let b = odds(bonus, ac - 1, crit_on, mode);
                    assert_odds(a, b.miss, b.hit, b.crit);
                }
            }
        }
    }
}

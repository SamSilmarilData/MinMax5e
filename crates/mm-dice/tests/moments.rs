//! Moments (E and E²) without the full distribution.

use mm_dice::{AttackRoll, Moments, Pmf, RollMode};

/// The bound the spec sets for float results against exact arithmetic (spec #12).
const RELATIVE: f64 = 1e-12;

fn assert_close(got: f64, expected: f64) {
    assert!(
        (got - expected).abs() <= RELATIVE * expected.abs(),
        "got {got}, expected {expected}"
    );
}

#[test]
fn moments_of_a_die_are_its_mean_and_second_moment() {
    let m = Moments::of(&Pmf::die(6));
    // E = 7/2, E² = 91/6.
    assert_close(m.mean(), 3.5);
    assert_close(m.second(), 91.0 / 6.0);
    assert_close(m.variance(), 35.0 / 12.0);
}

#[test]
fn independent_add_matches_convolution() {
    // 2d6 + 3: E = 10, Var = 35/6, so E² = 100 + 35/6.
    let d6 = Moments::of(&Pmf::die(6));
    let m = d6.add_independent(d6).add_independent(Moments::point(3));
    assert_close(m.mean(), 10.0);
    assert_close(m.second(), 100.0 + 35.0 / 6.0);
}

#[test]
fn two_independent_attacks_match_research_03_no_hew_row() {
    // Research 03 §7.2, "No Hew": mean 22.00, SD 8.64, from moments alone. The published
    // figures are rounded to 0.01, so each is checked to within 0.005.
    let odds = AttackRoll {
        bonus: 7,
        ac: 16,
        crit_on: 20,
        mode: RollMode::Normal,
    }
    .odds();
    let style_d6 = Moments::of(&Pmf::die(6).floor_at(3));
    let dice = |n: usize| (0..n).fold(Moments::point(0), |acc, _| acc.add_independent(style_d6));
    let hit = dice(2).add_independent(Moments::point(7));
    let crit = dice(4).add_independent(Moments::point(7));
    let graze = Moments::point(4);
    let attack = Moments::mix(&[(odds.hit, hit), (odds.crit, crit), (odds.miss, graze)]);
    let round = attack.add_independent(attack);
    assert!((round.mean() - 22.00).abs() <= 0.005, "{round:?}");
    assert!((round.variance().sqrt() - 8.64).abs() <= 0.005, "{round:?}");
}

//! `mm-dice` against exact `BigRational` arithmetic (ADR-0023, spec #12): generated dice and
//! attacks give the same distribution, statistics and moments to within 1e-12 relative.

use mm_dice::{AttackRoll, Moments, Pmf, RollMode};
use mm_testkit::rational::{ExactPmf, Ratio, exact_odds, within_relative};
use proptest::prelude::*;

/// The bound the spec sets (spec #12, ticket #14).
const RELATIVE: f64 = 1e-12;

/// The quantiles the Reliability Band reads, and a few more.
const QUANTILES: [f64; 5] = [0.10, 0.25, 0.50, 0.75, 0.90];

#[derive(Debug, Clone)]
struct AttackRecipe {
    /// `(count, sides)` groups of damage dice, doubled on a crit.
    dice: Vec<(u32, u32)>,
    /// Faces below this count as this, as with a style that counts 1s and 2s as 3.
    low_face: Option<i32>,
    flat: i32,
    graze: i32,
    roll: AttackRoll,
}

#[derive(Debug, Clone)]
struct TurnRecipe {
    attacks: Vec<AttackRecipe>,
    floor: Option<i32>,
    cap: Option<i32>,
}

fn roll_mode() -> impl Strategy<Value = RollMode> {
    prop_oneof![
        Just(RollMode::Normal),
        Just(RollMode::Advantage),
        Just(RollMode::Disadvantage),
    ]
}

fn attack() -> impl Strategy<Value = AttackRecipe> {
    (
        prop::collection::vec(
            (1..=2_u32, prop::sample::select(vec![4_u32, 6, 8, 10, 12])),
            1..=2,
        ),
        prop::option::of(2..=3_i32),
        0..=6_i32,
        0..=5_i32,
        (-2..=14_i32, 8..=28_i32, 18..=20_i32, roll_mode()),
    )
        .prop_map(
            |(dice, low_face, flat, graze, (bonus, ac, crit_on, mode))| AttackRecipe {
                dice,
                low_face,
                flat,
                graze,
                roll: AttackRoll {
                    bonus,
                    ac,
                    crit_on,
                    mode,
                },
            },
        )
}

fn turn() -> impl Strategy<Value = TurnRecipe> {
    (
        prop::collection::vec(attack(), 1..=3),
        prop::option::of(0..=12_i32),
        prop::option::of(5..=60_i32),
    )
        .prop_map(|(attacks, floor, cap)| TurnRecipe {
            attacks,
            floor,
            cap,
        })
}

/// The recipe built with `mm-dice`: the turn's distribution and its moments before any
/// floor or cap.
fn float_turn(recipe: &TurnRecipe) -> (Pmf, Moments) {
    let mut pmf = Pmf::point(0);
    let mut moments = Moments::point(0);
    for a in &recipe.attacks {
        let dice = |times: u32| {
            a.dice.iter().fold(Pmf::point(0), |acc, &(count, sides)| {
                let die = match a.low_face {
                    Some(low) => Pmf::die(sides).floor_at(low),
                    None => Pmf::die(sides),
                };
                acc.convolve(&die.sum_of(count * times))
            })
        };
        let hit = dice(1).shift(a.flat);
        let crit = dice(2).shift(a.flat);
        let miss = Pmf::point(a.graze);
        let odds = a.roll.odds();
        let attack = Pmf::mix(&[(odds.miss, &miss), (odds.hit, &hit), (odds.crit, &crit)]);
        let attack_moments = Moments::mix(&[
            (odds.miss, Moments::of(&miss)),
            (odds.hit, Moments::of(&hit)),
            (odds.crit, Moments::of(&crit)),
        ]);
        pmf = pmf.convolve(&attack);
        moments = moments.add_independent(attack_moments);
    }
    if let Some(floor) = recipe.floor {
        pmf = pmf.floor_at(floor);
    }
    if let Some(cap) = recipe.cap {
        pmf = pmf.cap_at(cap);
    }
    (pmf, moments)
}

/// The same recipe in exact arithmetic: the turn's distribution, and the distribution
/// before any floor or cap.
fn exact_turn(recipe: &TurnRecipe) -> (ExactPmf, ExactPmf) {
    let mut pmf = ExactPmf::point(0);
    for a in &recipe.attacks {
        let dice = |times: u32| {
            a.dice
                .iter()
                .fold(ExactPmf::point(0), |acc, &(count, sides)| {
                    let die = match a.low_face {
                        Some(low) => ExactPmf::die(sides).floor_at(low),
                        None => ExactPmf::die(sides),
                    };
                    acc.convolve(&die.sum_of(count * times))
                })
        };
        let hit = dice(1).shift(a.flat);
        let crit = dice(2).shift(a.flat);
        let miss = ExactPmf::point(a.graze);
        let [p_miss, p_hit, p_crit] = exact_odds(a.roll);
        let attack = ExactPmf::mix(&[(p_miss, &miss), (p_hit, &hit), (p_crit, &crit)]);
        pmf = pmf.convolve(&attack);
    }
    let unbounded = pmf.clone();
    if let Some(floor) = recipe.floor {
        pmf = pmf.floor_at(floor);
    }
    if let Some(cap) = recipe.cap {
        pmf = pmf.cap_at(cap);
    }
    (pmf, unbounded)
}

fn check(what: &str, got: f64, exact: &Ratio) -> Result<(), TestCaseError> {
    prop_assert!(
        within_relative(got, exact, RELATIVE),
        "{what}: got {got}, exact {}",
        exact
    );
    Ok(())
}

proptest! {
    #[test]
    fn d20_odds_match_exact_counts(
        bonus in -10..=25_i32,
        ac in 0..=40_i32,
        crit_on in 15..=21_i32,
        mode in roll_mode(),
    ) {
        let roll = AttackRoll { bonus, ac, crit_on, mode };
        let odds = roll.odds();
        let [miss, hit, crit] = exact_odds(roll);
        check("miss", odds.miss, &miss)?;
        check("hit", odds.hit, &hit)?;
        check("crit", odds.crit, &crit)?;
    }

    #[test]
    fn distributions_and_moments_match_exact_arithmetic(recipe in turn()) {
        let (pmf, moments) = float_turn(&recipe);
        let (exact, unbounded) = exact_turn(&recipe);

        prop_assert_eq!(pmf.support(), exact.support());
        let (lo, hi) = exact.support();
        for x in lo..=hi {
            check(&format!("P({x})"), pmf.prob(x), &exact.prob(x))?;
            check(&format!("P(>= {x})"), pmf.tail(x), &exact.tail(x))?;
        }
        check("mass", pmf.mass(), &exact.mass())?;
        check("mean", pmf.mean(), &exact.mean())?;
        check("variance", pmf.variance(), &exact.variance())?;
        for q in QUANTILES {
            // A float cumulative within rounding of q may land either side of it; the
            // exact answer is only comparable away from that edge.
            if !exact.cumulative_near(q, RELATIVE) {
                prop_assert_eq!(pmf.quantile(q), exact.quantile(q), "quantile {}", q);
            }
        }

        check("moments mean", moments.mean(), &unbounded.mean())?;
        check("moments E[X²]", moments.second(), &unbounded.second())?;
    }
}

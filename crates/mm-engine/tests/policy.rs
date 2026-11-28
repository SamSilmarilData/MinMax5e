//! The forward pass follows the Policy the backward pass recorded (spec #12, seam 2: the
//! `mm-engine` facade on a raw Kit Shape and Kit Params).

use mm_core::AssumptionSet;
use mm_dice::Pmf;
use mm_engine::{AttackSource, CombatKit, KitParams, KitShape, RowParams, RowShape};
use mm_testkit::tolerance::assert_close;

/// A row that always hits at AC 16 except on a natural 1, dealing `hit` on a hit or crit.
fn row(hit: &Pmf) -> RowParams {
    RowParams {
        to_hit: 100,
        crit_on: 20,
        miss: Pmf::point(0),
        hit: hit.clone(),
        crit: hit.clone(),
    }
}

/// One Attack-action attack, chosen from rows with these damages.
fn kit(rows: &[&Pmf]) -> CombatKit {
    CombatKit {
        shape: KitShape {
            attacks_per_action: 1,
            rows: rows
                .iter()
                .map(|_| RowShape {
                    source: AttackSource::AttackAction,
                    crit_offers: None,
                })
                .collect(),
        },
        params: KitParams {
            rows: rows.iter().map(|&hit| row(hit)).collect(),
        },
    }
}

/// The round-1 p90 at AC 16.
fn p90(kit: &CombatKit) -> f64 {
    let assumptions = AssumptionSet {
        nova_rounds: 1,
        ..AssumptionSet::against(16)
    };
    let evaluation = mm_engine::evaluate(kit, &assumptions);
    let lane = evaluation
        .nova
        .lanes
        .iter()
        .find(|lane| lane.ac == 16)
        .expect("an AC 16 lane");
    lane.band.p90
}

#[test]
fn a_tie_in_mean_goes_to_the_lowest_option_in_the_forward_pass_too() {
    // Both rows deal 10 on average when they connect (19 times in 20): a steady 10, or 5
    // and 15 equally often. The means tie exactly, so the Policy takes the first row, and
    // the round's p90 is that row's: 10 for the steady row, 15 for the swingy one.
    let steady = Pmf::point(10);
    let swingy = Pmf::mix(&[(0.5, &Pmf::point(5)), (0.5, &Pmf::point(15))]);
    assert_close(p90(&kit(&[&steady, &swingy])), 10.0);
    assert_close(p90(&kit(&[&swingy, &steady])), 15.0);
}

#[test]
fn the_forward_pass_takes_the_higher_mean_whatever_its_place() {
    // A steady 10 against a steady 12: the 12 is taken from either place.
    let ten = Pmf::point(10);
    let twelve = Pmf::point(12);
    assert_close(p90(&kit(&[&ten, &twelve])), 12.0);
    assert_close(p90(&kit(&[&twelve, &ten])), 12.0);
}

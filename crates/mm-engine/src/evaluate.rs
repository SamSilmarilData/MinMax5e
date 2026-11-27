//! Evaluating a Combat Kit under an Assumption Set.

use mm_core::{AssumptionSet, Fp};
use serde::Serialize;

use crate::interp;
use crate::kit::CombatKit;

/// The result of evaluating one kit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evaluation {
    pub shape_key: Fp,
    pub params_fp: Fp,
    /// The AC the Assumption Set targets; every lane is still reported.
    pub target_ac: i32,
    pub nova: ModeResult,
}

/// One mode's results at every AC lane.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModeResult {
    /// The horizon: rounds 1 to `rounds`.
    pub rounds: u32,
    /// One per AC from 10 to 25, in increasing AC.
    pub lanes: Vec<LaneResult>,
}

/// Damage per round at one target AC.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LaneResult {
    pub ac: i32,
    /// Expected damage per round: E[total over the horizon] / rounds.
    pub dpr: f64,
    /// The standard deviation of damage per round over the horizon: SD(total) / rounds.
    pub sd: f64,
}

/// Evaluates `kit` in Nova mode: rounds 1 to `assumptions.nova_rounds`, full resources.
///
/// # Panics
/// If `nova_rounds` is 0.
pub fn evaluate(kit: &CombatKit, assumptions: &AssumptionSet) -> Evaluation {
    let rounds = assumptions.nova_rounds;
    assert!(rounds > 0, "the Nova horizon has at least one round");
    let totals = interp::total_moments(&kit.shape, &kit.params, rounds);
    let per_round = f64::from(rounds);
    Evaluation {
        shape_key: kit.shape.key(),
        params_fp: kit.params.fp(),
        target_ac: assumptions.encounter.target_ac,
        nova: ModeResult {
            rounds,
            lanes: totals
                .iter()
                .map(|(ac, total)| {
                    // E[X²] − E[X]² can round a little below zero when the spread is nil.
                    let variance = total.variance();
                    let variance = if variance > 0.0 { variance } else { 0.0 };
                    LaneResult {
                        ac,
                        dpr: total.mean() / per_round,
                        sd: variance.sqrt() / per_round,
                    }
                })
                .collect(),
        },
    }
}

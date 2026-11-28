//! Evaluating a Combat Kit under an Assumption Set.

use mm_core::{AssumptionSet, Fp, RulingSet};
use mm_dice::Pmf;
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
    /// The hit points Time-to-kill and Kill probability are measured against.
    pub target_hp: u32,
    /// The Rulings the kit read, each with the option it had (ADR-0006, ADR-0052).
    pub rulings: RulingSet,
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaneResult {
    pub ac: i32,
    /// Expected damage per round: E[total over the horizon] / rounds.
    pub dpr: f64,
    /// The standard deviation of damage per round over the horizon: SD(total) / rounds.
    pub sd: f64,
    /// The least a round of the horizon can deal: the true minimum, which the Floor is not.
    pub minimum: f64,
    /// The spread of damage per round over the horizon.
    pub band: ReliabilityBand,
    /// Time-to-kill and Kill probability against the target's hit points.
    pub kill: KillOdds,
}

/// The Reliability Band (GLOSSARY.md) of damage per round. Each figure is that of one
/// round of the horizon, every round equally likely, so the Floor is the mean of the worst
/// 10% of rounds. Over one round it is that round's.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ReliabilityBand {
    /// The 10th percentile: the smallest total at least a tenth of rounds stay at or below.
    pub p10: f64,
    /// The median.
    pub p50: f64,
    /// The 90th percentile.
    pub p90: f64,
    /// The Floor: the mean of the worst 10% of rounds.
    pub floor: f64,
}

/// How soon the target falls, from the distribution of cumulative damage after each round
/// of the horizon. Entry `k − 1` is for round `k`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KillOdds {
    /// The Time-to-kill distribution: the probability that cumulative damage first
    /// reaches the target's hit points in round `k`. What is left of 1 is the chance it
    /// survives the horizon.
    pub time_to_kill: Vec<f64>,
    /// The Kill probability within `k` rounds: the probability that cumulative damage has
    /// reached the target's hit points by the end of round `k`.
    pub kill_probability: Vec<f64>,
}

/// The share of rounds the Floor averages over, and the Band's percentiles.
const FLOOR_SHARE: f64 = 0.1;
const P10: f64 = 0.1;
const P50: f64 = 0.5;
const P90: f64 = 0.9;

/// Evaluates `kit` in Nova mode: rounds 1 to `assumptions.nova_rounds`, full resources.
pub fn evaluate(kit: &CombatKit, assumptions: &AssumptionSet) -> Evaluation {
    let rounds = assumptions.nova_rounds.get();
    let target_hp = assumptions.encounter_profile.target_hp.get();
    let lanes = interp::totals(&kit.shape, &kit.params, rounds);
    let per_round = f64::from(rounds);
    Evaluation {
        shape_key: kit.shape.key(),
        params_fp: kit.params.fp(),
        target_ac: assumptions.encounter_profile.target_ac,
        target_hp,
        rulings: kit.rulings_read(),
        nova: ModeResult {
            rounds,
            lanes: lanes
                .iter()
                .map(|(ac, lane)| {
                    let total = lane.moments;
                    let one_round = &lane.one_round;
                    // E[X²] − E[X]² can round a little below zero when the spread is nil.
                    let variance = total.variance();
                    let variance = if variance > 0.0 { variance } else { 0.0 };
                    LaneResult {
                        ac,
                        dpr: total.mean() / per_round,
                        sd: variance.sqrt() / per_round,
                        minimum: f64::from(one_round.support().0),
                        band: ReliabilityBand {
                            p10: f64::from(one_round.quantile(P10)),
                            p50: f64::from(one_round.quantile(P50)),
                            p90: f64::from(one_round.quantile(P90)),
                            floor: one_round.mean_of_lowest(FLOOR_SHARE),
                        },
                        kill: kill_odds(&lane.by_round, target_hp),
                    }
                })
                .collect(),
        },
    }
}

/// Time-to-kill and Kill probability against `hp`, from the cumulative distributions.
fn kill_odds(by_round: &[Pmf], hp: u32) -> KillOdds {
    // A target with more hit points than an i32 holds is never reached.
    let hp = i32::try_from(hp).unwrap_or(i32::MAX);
    let kill_probability: Vec<f64> = by_round.iter().map(|total| total.tail(hp)).collect();
    let time_to_kill = kill_probability
        .iter()
        .scan(0.0, |before, &by_now| {
            let first_now = by_now - *before;
            *before = by_now;
            Some(first_now)
        })
        .collect();
    KillOdds {
        time_to_kill,
        kill_probability,
    }
}

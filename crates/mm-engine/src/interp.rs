//! The interpreter: a hash-map DP over turn states, holding values as 16 AC lanes
//! (architecture §4.1, §8). It stays as the differential reference once the Tape arrives
//! (ADR-0056).
//!
//! The backward pass computes, for each state, the moments of the damage still to come
//! under the mean-optimal Policy (ADR-0017): at a decision it keeps, per lane, the option
//! with the highest mean, ties going to the lowest option index. The moment fold is ported
//! from `fold` in the tape spike (`spikes/tape-spike/src/main.rs`).

use mm_dice::{AcLanes, AttackRoll, Moments, Pmf, RollMode};
use rustc_hash::FxHashMap;

use crate::kit::{Damage, KitParams, KitShape};
use crate::turn::{self, Outcome, Step, TurnState};

/// The moments of total damage over `rounds` rounds, one per AC lane.
pub(crate) fn total_moments(shape: &KitShape, params: &KitParams, rounds: u32) -> AcLanes<Moments> {
    let rows = params
        .rows
        .iter()
        .map(|row| RowTerms {
            hit: moments(&row.hit),
            crit: moments(&row.crit),
            odds: AcLanes::from_fn(|ac| {
                AttackRoll {
                    bonus: row.to_hit,
                    ac,
                    crit_on: row.crit_on,
                    mode: RollMode::Normal,
                }
                .odds()
            }),
        })
        .collect();
    let mut dp = Dp {
        shape,
        rows,
        rounds,
        memo: FxHashMap::default(),
    };
    dp.value(0, TurnState::start())
}

struct RowTerms {
    hit: Moments,
    crit: Moments,
    odds: AcLanes<mm_dice::OutcomeOdds>,
}

struct Dp<'a> {
    shape: &'a KitShape,
    rows: Vec<RowTerms>,
    rounds: u32,
    memo: FxHashMap<(u32, TurnState), AcLanes<Moments>>,
}

impl Dp<'_> {
    /// The moments of the damage from `state` in round `round` (from 0) to the horizon.
    fn value(&mut self, round: u32, state: TurnState) -> AcLanes<Moments> {
        if let Some(&value) = self.memo.get(&(round, state)) {
            return value;
        }
        let value = if state.ended() {
            if round + 1 < self.rounds {
                self.value(round + 1, TurnState::start())
            } else {
                AcLanes::from_fn(|_| Moments::point(0))
            }
        } else if let Some(row) = state.pending_roll() {
            self.chance(round, state, usize::from(row))
        } else {
            self.decide(round, state)
        };
        self.memo.insert((round, state), value);
        value
    }

    /// A pending roll: each outcome's damage plus what follows, weighted by its odds.
    fn chance(&mut self, round: u32, state: TurnState, row: usize) -> AcLanes<Moments> {
        let after = Outcome::ALL.map(|outcome| {
            let next = turn::apply(self.shape, state, Step::Roll(outcome))
                .expect("a pending roll accepts every outcome");
            self.value(round, next)
        });
        let terms = &self.rows[row];
        let dealt = [Moments::point(0), terms.hit, terms.crit];
        AcLanes::from_fn(|ac| {
            let odds = terms.odds[ac];
            let weights = [odds.miss, odds.hit, odds.crit];
            let branches: [(f64, Moments); 3] =
                std::array::from_fn(|i| (weights[i], dealt[i].add_independent(after[i][ac])));
            Moments::mix(&branches)
        })
    }

    /// A decision: per lane, the legal step with the highest mean; ties go to the first.
    fn decide(&mut self, round: u32, state: TurnState) -> AcLanes<Moments> {
        let options: Vec<AcLanes<Moments>> = turn::legal_steps(self.shape, state)
            .into_iter()
            .map(|step| {
                let next = turn::apply(self.shape, state, step).expect("legal_steps are legal");
                self.value(round, next)
            })
            .collect();
        AcLanes::from_fn(|ac| {
            options
                .iter()
                .map(|option| option[ac])
                .reduce(|best, m| if m.mean() > best.mean() { m } else { best })
                .expect("ending the turn is always legal")
        })
    }
}

/// The moments of one outcome's damage.
fn moments(damage: &Damage) -> Moments {
    damage
        .dice
        .iter()
        .fold(Pmf::point(damage.flat), |total, dice| {
            total.convolve(&Pmf::die(dice.sides).sum_of(dice.count))
        })
        .moments()
}

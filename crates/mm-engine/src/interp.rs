//! The interpreter: a hash-map DP over turn states, holding values as 16 AC lanes
//! (architecture §4.1, §8). It stays as the differential reference once the Tape arrives
//! (ADR-0056).
//!
//! The backward pass computes, for each state, the moments of the damage still to come
//! under the mean-optimal Policy (ADR-0017): at a decision it keeps, per lane, the option
//! with the highest mean, ties going to the lowest option index, and records that choice.
//! The moment fold is ported from `fold` in the tape spike (`spikes/tape-spike/src/main.rs`).
//!
//! The forward pass then follows the recorded Policy from the start of round 1, carrying
//! each reachable state's share of the Damage Distribution of the damage dealt so far
//! (research 03 §3.1). It gives the exact distribution of the total after every round,
//! and of each round's own damage.

use mm_dice::{AcLanes, AttackRoll, Moments, OutcomeOdds, Pmf, RollMode};
use rustc_hash::FxHashMap;

use crate::kit::{KitParams, KitShape, RowParams};
use crate::turn::{self, Outcome, Step, TurnState};

/// What one evaluation of a kit over a horizon gives at one AC lane.
pub(crate) struct LaneTotals {
    /// The moments of total damage over the horizon, from the backward pass.
    pub moments: Moments,
    /// The Damage Distribution of the total over rounds 1 to `k`, for each `k` from 1 to
    /// the horizon, from the forward pass under the same Policy.
    pub by_round: Vec<Pmf>,
    /// The Damage Distribution of one round of the horizon, each equally likely: the
    /// mixture of every round's own damage.
    pub a_round: Pmf,
}

/// Evaluates the kit over `rounds` rounds: the backward pass, then the forward pass.
pub(crate) fn totals(shape: &KitShape, params: &KitParams, rounds: u32) -> AcLanes<LaneTotals> {
    let odds = params
        .rows
        .iter()
        .map(|row| {
            AcLanes::from_fn(|ac| {
                AttackRoll {
                    bonus: row.to_hit,
                    ac,
                    crit_on: row.crit_on,
                    mode: RollMode::Normal,
                }
                .odds()
            })
        })
        .collect();
    let mut dp = Dp {
        shape,
        params,
        moments: params
            .rows
            .iter()
            .map(|row| dealt(row).map(Pmf::moments))
            .collect(),
        odds,
        rounds,
        memo: FxHashMap::default(),
        policy: FxHashMap::default(),
    };
    let moments = dp.value(0, TurnState::start());
    AcLanes::from_fn(|ac| {
        let (by_round, a_round) = dp.forward(ac);
        LaneTotals {
            moments: moments[ac],
            by_round,
            a_round,
        }
    })
}

/// A row's damage by outcome, in `Outcome::ALL` order.
fn dealt(row: &RowParams) -> [&Pmf; 3] {
    [&row.miss, &row.hit, &row.crit]
}

/// An outcome's odds, in `Outcome::ALL` order.
fn weights(odds: OutcomeOdds) -> [f64; 3] {
    [odds.miss, odds.hit, odds.crit]
}

struct Dp<'a> {
    shape: &'a KitShape,
    params: &'a KitParams,
    /// Per row, the moments of each outcome's damage, in `Outcome::ALL` order.
    moments: Vec<[Moments; 3]>,
    /// Per row, the odds of each outcome at each lane.
    odds: Vec<AcLanes<OutcomeOdds>>,
    rounds: u32,
    memo: FxHashMap<(u32, TurnState), AcLanes<Moments>>,
    /// At each decision, per lane, the index into `legal_steps` the backward pass chose.
    /// Only looked up, never iterated, so its layout cannot reach a result.
    policy: FxHashMap<(u32, TurnState), AcLanes<usize>>,
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
        let dealt = self.moments[row];
        AcLanes::from_fn(|ac| {
            let weights = weights(self.odds[row][ac]);
            let branches: [(f64, Moments); 3] =
                std::array::from_fn(|i| (weights[i], dealt[i].add_independent(after[i][ac])));
            Moments::mix(&branches)
        })
    }

    /// A decision: per lane, the legal step with the highest mean; ties go to the first.
    /// The choice is recorded for the forward pass.
    fn decide(&mut self, round: u32, state: TurnState) -> AcLanes<Moments> {
        let options: Vec<AcLanes<Moments>> = turn::legal_steps(self.shape, state)
            .into_iter()
            .map(|step| {
                let next = turn::apply(self.shape, state, step).expect("legal_steps are legal");
                self.value(round, next)
            })
            .collect();
        assert!(!options.is_empty(), "ending the turn is always legal");
        let choice = AcLanes::from_fn(|ac| {
            (1..options.len()).fold(0, |best, i| {
                if options[i][ac].mean() > options[best][ac].mean() {
                    i
                } else {
                    best
                }
            })
        });
        self.policy.insert((round, state), choice);
        AcLanes::from_fn(|ac| options[choice[ac]][ac])
    }

    /// The forward pass at lane `ac`: the distribution of the total after each round, and
    /// that of one round picked evenly from the horizon.
    fn forward(&self, ac: i32) -> (Vec<Pmf>, Pmf) {
        let mut by_round = Vec::new();
        let mut each_round = Vec::new();
        let mut starts = Frontier::default();
        starts.add(TurnState::start(), Pmf::point(0));
        for round in 0..self.rounds {
            // The round's own damage: the same start states, each holding only its mass.
            let mut fresh = Frontier::default();
            for (state, so_far) in &starts.entries {
                fresh.add(*state, scaled(so_far.mass(), &Pmf::point(0)));
            }
            each_round.push(self.play_round(round, ac, fresh).total());
            let ended = self.play_round(round, ac, starts);
            by_round.push(ended.total());
            // As in `value`: every ended turn leads to the start of the next round.
            starts = Frontier::default();
            for (_, so_far) in ended.entries {
                starts.add(TurnState::start(), so_far);
            }
        }
        let share = 1.0 / f64::from(self.rounds);
        let rounds: Vec<(f64, &Pmf)> = each_round.iter().map(|d| (share, d)).collect();
        (by_round, Pmf::mix(&rounds))
    }

    /// Plays round `round` at lane `ac` under the Policy from `starts`, and returns the
    /// ended turns, each with its share of the damage so far.
    fn play_round(&self, round: u32, ac: i32, starts: Frontier) -> Frontier {
        let mut ended = Frontier::default();
        let mut frontier = starts;
        while !frontier.is_empty() {
            let mut next = Frontier::default();
            for (state, so_far) in frontier.entries {
                if state.ended() {
                    ended.add(state, so_far);
                } else if let Some(row) = state.pending_roll() {
                    let row = usize::from(row);
                    let weights = weights(self.odds[row][ac]);
                    let dealt = dealt(&self.params.rows[row]);
                    for (i, outcome) in Outcome::ALL.into_iter().enumerate() {
                        if weights[i] == 0.0 {
                            continue;
                        }
                        let after = turn::apply(self.shape, state, Step::Roll(outcome))
                            .expect("a pending roll accepts every outcome");
                        next.add(after, scaled(weights[i], &so_far.convolve(dealt[i])));
                    }
                } else {
                    let choice = self.policy[&(round, state)][ac];
                    let step = turn::legal_steps(self.shape, state)[choice];
                    let after =
                        turn::apply(self.shape, state, step).expect("legal_steps are legal");
                    next.add(after, so_far);
                }
            }
            frontier = next;
        }
        ended
    }
}

/// `dist` with every mass multiplied by `weight`: a mix of one branch.
fn scaled(weight: f64, dist: &Pmf) -> Pmf {
    Pmf::mix(&[(weight, dist)])
}

/// The reachable states of one step of the forward pass, each with its share of the
/// distribution of damage so far. Entries stay in the order they were first reached, so
/// the sums run in a fixed order on every target (ADR-0050); the index is only looked up.
#[derive(Default)]
struct Frontier {
    entries: Vec<(TurnState, Pmf)>,
    index: FxHashMap<TurnState, usize>,
}

impl Frontier {
    /// Adds `share` to what `state` already holds: a mix with weight 1 adds partial
    /// distributions.
    fn add(&mut self, state: TurnState, share: Pmf) {
        if let Some(&i) = self.index.get(&state) {
            let held = &mut self.entries[i].1;
            let merged = Pmf::mix(&[(1.0, &*held), (1.0, &share)]);
            *held = merged;
        } else {
            self.index.insert(state, self.entries.len());
            self.entries.push((state, share));
        }
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry's share added together.
    fn total(&self) -> Pmf {
        let shares: Vec<(f64, &Pmf)> = self.entries.iter().map(|(_, d)| (1.0, d)).collect();
        Pmf::mix(&shares)
    }
}

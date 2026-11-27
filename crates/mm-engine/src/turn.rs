//! The turn state machine: the single authority on what is legal in a turn (ADR-0012).
//!
//! [`apply`] decides legality; [`legal_steps`] lists the candidate steps `apply` accepts, so
//! the two cannot disagree. The interpreter only follows `legal_steps`, and the oracles in
//! `mm-testkit` replay through `apply`.
//!
//! A turn is a sequence of steps. Taking an attack leaves its roll pending, and the next
//! step is the roll's outcome, a chance step with odds from the Kit Params.

use crate::kit::{AttackSource, KitShape};

/// Where a turn stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurnState {
    action_used: bool,
    /// Attacks still to make from an Attack action already taken.
    attacks_left: u8,
    /// The attack row whose roll is pending.
    pending: Option<u8>,
    ended: bool,
}

/// One step of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// Make an attack with this row of the Kit Shape.
    Attack {
        row: u8,
    },
    /// The pending attack roll comes up with this outcome.
    Roll(Outcome),
    EndTurn,
}

/// The outcome of an attack roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Miss,
    Hit,
    Crit,
}

impl Outcome {
    pub const ALL: [Outcome; 3] = [Outcome::Miss, Outcome::Hit, Outcome::Crit];
}

/// Why a step is illegal in a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Illegal {
    #[error("the turn has ended")]
    TurnOver,
    #[error("an attack roll is pending; its outcome comes first")]
    RollPending,
    #[error("no attack roll is pending")]
    NoRollPending,
    #[error("the Kit Shape has no attack row {0}")]
    NoSuchRow(u8),
    #[error("the Action is already used and its attacks are spent")]
    AttackActionSpent,
}

impl TurnState {
    /// The start of a turn: nothing used yet.
    pub fn start() -> TurnState {
        TurnState {
            action_used: false,
            attacks_left: 0,
            pending: None,
            ended: false,
        }
    }

    /// The attack row whose roll is pending, if any: the state is a chance node.
    pub fn pending_roll(self) -> Option<u8> {
        self.pending
    }

    pub fn ended(self) -> bool {
        self.ended
    }
}

/// The steps `apply` accepts from `state`, in a fixed order: attacks by row, then rolls
/// (miss, hit, crit), then ending the turn.
///
/// # Panics
/// If the shape has more than 256 attack rows.
pub fn legal_steps(shape: &KitShape, state: TurnState) -> Vec<Step> {
    let rows = u8::try_from(shape.rows.len()).expect("a Kit Shape has under 256 attack rows");
    (0..rows)
        .map(|row| Step::Attack { row })
        .chain(Outcome::ALL.map(Step::Roll))
        .chain([Step::EndTurn])
        .filter(|&step| apply(shape, state, step).is_ok())
        .collect()
}

/// The state after taking `step` in `state`.
///
/// # Errors
/// With the reason, if `step` is illegal in `state`.
pub fn apply(shape: &KitShape, state: TurnState, step: Step) -> Result<TurnState, Illegal> {
    if state.ended {
        return Err(Illegal::TurnOver);
    }
    let mut next = state;
    match step {
        Step::Attack { row } => {
            if state.pending.is_some() {
                return Err(Illegal::RollPending);
            }
            let shape_row = shape
                .rows
                .get(usize::from(row))
                .ok_or(Illegal::NoSuchRow(row))?;
            match shape_row.source {
                AttackSource::AttackAction => {
                    if state.attacks_left > 0 {
                        next.attacks_left -= 1;
                    } else if !state.action_used {
                        next.action_used = true;
                        next.attacks_left = shape.attacks_per_action.saturating_sub(1);
                    } else {
                        return Err(Illegal::AttackActionSpent);
                    }
                }
            }
            next.pending = Some(row);
        }
        Step::Roll(_) => {
            if state.pending.is_none() {
                return Err(Illegal::NoRollPending);
            }
            next.pending = None;
        }
        Step::EndTurn => {
            if state.pending.is_some() {
                return Err(Illegal::RollPending);
            }
            next.ended = true;
        }
    }
    Ok(next)
}

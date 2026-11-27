//! The engine: compile a Build into a Combat Kit, then evaluate it exactly over AC lanes
//! (architecture §8, build-order step 3).
//!
//! The public seam:
//! - [`compile`]: a [`Pack`](mm_load::Pack) and a [`Build`](mm_core::Build) give a
//!   [`CombatKit`], split into a [`KitShape`] (with its `ShapeKey`, [`KitShape::key`]) and
//!   [`KitParams`] (with its `ParamsFp`, [`KitParams::fp`]), or a [`CompileError`].
//! - [`evaluate`]: a kit (or a raw shape and params, put together as a `CombatKit`) and an
//!   [`AssumptionSet`](mm_core::AssumptionSet) give an [`Evaluation`].
//! - [`turn`]: the turn state machine, `legal_steps` and `apply`, and [`turn::Illegal`]: the
//!   single legality authority (ADR-0012), public so the oracles share it.
//!
//! The interpreter that does the evaluating is hidden. There is no Tape or
//! canonicalisation yet (step 4).

mod evaluate;
mod interp;
mod kit;
pub mod turn;

pub use evaluate::{Evaluation, LaneResult, ModeResult, evaluate};
pub use kit::{
    AttackSource, CombatKit, CompileError, Damage, KitParams, KitShape, RowParams, RowShape,
    compile,
};

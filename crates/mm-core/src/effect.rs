//! The closed Effect DSL (ADR-0010) that encodings are written in, loaded from RON into
//! these enums (ADR-0035). Effects attach to fixed resolution stages, never to priority
//! numbers (ADR-0011).
//!
//! Every consumer matches these exhaustively, so adding a variant breaks compilation until
//! each consumer handles it. The variants grow with the fixtures that need them
//! (architecture §11.1); there is no hook escape hatch yet (ADR-0036).

use serde::{Deserialize, Serialize};

use crate::records::WeaponProperty;
use crate::{Citation, ContentId};

/// One encoding file: our mechanics for one record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Encoding {
    Class(ClassEncoding),
}

/// What a class grants at each level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassEncoding {
    /// The class record this encodes.
    pub class: ContentId,
    /// Where these mechanics come from (ADR-0007).
    pub citation: Citation,
    pub features: Vec<LevelFeature>,
}

/// A feature gained at a class level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelFeature {
    /// The class level that grants it, from 1.
    pub level: u8,
    pub name: String,
    pub riders: Vec<Rider>,
}

/// An effect that applies at a stage of resolving an attack, when its predicate holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rider {
    pub on: Stage,
    #[serde(default)]
    pub when: Option<Pred>,
    #[serde(default)]
    pub limit: Option<Limit>,
    /// Whether applying it is the player's choice: a Policy decision (ADR-0014).
    #[serde(default)]
    pub optional: bool,
    pub effect: Op,
}

/// The fixed resolution stages of an attack (ADR-0011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    AttackHit,
    AttackCrit,
    AttackMiss,
}

/// A test over the facts of an attack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pred {
    Fact(Fact),
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
}

/// A fact about an attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fact {
    WeaponProperty(WeaponProperty),
}

/// How often a rider may apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Limit {
    OncePerTurn,
}

/// What a rider does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    /// Flat damage added to the attack's damage.
    FlatDamage(i32),
}

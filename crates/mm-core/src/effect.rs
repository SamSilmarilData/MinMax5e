//! The closed Effect DSL (ADR-0010) that encodings are written in, loaded from RON into
//! these enums (ADR-0035). Effects attach to fixed resolution stages, never to priority
//! numbers (ADR-0011).
//!
//! Every consumer matches these exhaustively, so adding a variant breaks compilation until
//! each consumer handles it. The variants grow with the fixtures that need them
//! (architecture §11.1); there is no hook escape hatch yet (ADR-0036).

use serde::{Deserialize, Serialize};

use crate::records::WeaponProperty;
use crate::{Citation, ContentId, RulingId, RulingOptionId};

/// One encoding file: our mechanics for one record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Encoding {
    Class(ClassEncoding),
    Species(FeaturesEncoding),
    Background(FeaturesEncoding),
    /// A feat, Origin feats and fighting styles included.
    Feat(FeaturesEncoding),
    /// A weapon's mastery property, which Weapon Mastery unlocks.
    MasteryProperty(FeaturesEncoding),
}

/// What a class grants at each level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassEncoding {
    /// The class record this encodes.
    pub class: ContentId,
    pub features: Vec<LevelFeature>,
}

/// A feature gained at a class level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelFeature {
    /// The class level that grants it, from 1.
    pub level: u8,
    pub name: String,
    /// Where this feature's mechanics come from (ADR-0007).
    pub citation: Citation,
    pub riders: Vec<Rider>,
    #[serde(default)]
    pub grants: Vec<Grant>,
}

/// What a record without levels grants: its features, all at once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeaturesEncoding {
    /// The record this encodes; its kind matches the encoding's variant.
    pub id: ContentId,
    pub features: Vec<Feature>,
}

/// A feature of a record without levels: a species trait, a feat, a mastery property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub name: String,
    /// Where this feature's mechanics come from (ADR-0007).
    pub citation: Citation,
    pub riders: Vec<Rider>,
    #[serde(default)]
    pub grants: Vec<Grant>,
}

/// A standing change to how the character attacks, rather than an effect at a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Grant {
    /// The Attack action makes this many attacks; the highest grant wins.
    ExtraAttack(u8),
    /// The mastery property of each weapon in the Loadout applies to its attacks.
    WeaponMastery,
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
    /// The attack hits, crit or not: a crit is a hit.
    AttackHit,
    /// The attack crits.
    AttackCrit,
    AttackMiss,
}

/// A test over the facts of an attack, and over the active Rulings.
///
/// `All` and `Any` test their parts in order and stop once the answer is known, so a
/// Ruling placed after a part that settles it is not read (ADR-0052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pred {
    Fact(Fact),
    /// The active Rulings choose this option of this Ruling (ADR-0006). The pack checks
    /// both exist when it loads.
    Ruling(RulingId, RulingOptionId),
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
}

/// A fact about an attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fact {
    /// The attack's weapon has this property.
    WeaponProperty(WeaponProperty),
    /// The attack is made as part of the Attack action.
    AttackAction,
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
    /// Damage equal to the character's proficiency bonus.
    ProficiencyBonusDamage,
    /// Damage equal to the attack's ability modifier.
    AbilityModifierDamage,
    /// Each weapon damage die that rolls lower than this counts as this.
    MinimumDieRoll(u32),
    /// One attack with the same weapon as a Bonus Action, taken immediately after this
    /// stage or not at all.
    BonusActionAttack,
}

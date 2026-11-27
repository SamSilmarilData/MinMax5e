//! What an edition supplies to the edition-agnostic engine (ADR-0008).

use std::fmt::Debug;

use crate::Sources;

/// One edition's rules: its Content ID prefix, its economy and its own encodings. The
/// engine and loader are generic over it, and an edition's crate implements it
/// (`mm-rules-2024`; `TestRules` in the testkit).
pub trait Ruleset: Debug + Send + Sync + 'static {
    /// The Content ID prefix every item of this Ruleset carries, e.g. `2024`.
    const PREFIX: &'static str;

    /// The spendable resources this edition declares (spell slots and the like).
    fn resources() -> Vec<ResourceDecl>;

    /// The edition's own encodings, embedded at build time (ADR-0035).
    fn encodings() -> Sources;
}

/// A kind of spendable resource and the rules for spending it (architecture §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDecl {
    /// The resource's name, unique within the Ruleset, e.g. `spell-slot`.
    pub name: &'static str,
    /// How many units may be spent in one turn (1 for 2024 spell slots: the one-slot rule).
    pub spend_cap_per_turn: u8,
    /// Whether units are ordered by level, as spell slots are, so which unit is spent matters.
    pub ordered: bool,
    /// When spent units come back.
    pub refresh: Refresh,
}

/// The rest that restores a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    ShortRest,
    LongRest,
}

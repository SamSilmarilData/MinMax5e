//! What a Build is evaluated against: the Assumption Set, and the Encounter Baseline that
//! supplies its default target (ADR-0024).
//!
//! # The Encounter Baseline
//!
//! The default Target Dummy has a CR equal to the character's level. Only its AC
//! and HP are modelled yet; the per-ability save curves and Magic Resistance that ADR-0024
//! also gives wait for saves (spec #12, "Out of Scope").
//!
//! - **AC** is Tom Dunn's baseline table, about 13 + CR/3
//!   (<https://tomedunn.github.io/the-finished-book/monsters/baseline-monster-stats>).
//!   Our own least-squares fit to the 330 SRD 5.2.1 stat blocks with a CR gives
//!   AC ≈ 12.9 + 0.37·CR, which agrees with it within about one point at every level
//!   (research 03 §5.2–5.3).
//! - **HP** is ≈ 10 + 17·CR, our least-squares fit to the same SRD 5.2.1 stat blocks
//!   (research 03 §5.2: 9.9 + 17.0·CR).
//!
//! Both fits are derived from material in the System Reference Document 5.2.1
//! ("SRD 5.2.1") by Wizards of the Coast LLC, available at
//! <https://www.dndbeyond.com/srd>. The SRD 5.2.1 is licensed under the Creative Commons
//! Attribution 4.0 International License, available at
//! <https://creativecommons.org/licenses/by/4.0/legalcode>.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// The assumptions behind an evaluation (GLOSSARY.md, **Assumption Set**). The active
/// Rulings, which the glossary also counts in, are given beside it (#21).
///
/// As a file (`mm-cli evaluate --assumptions`), it is RON with every field given:
///
/// ```ron
/// (
///     encounter_profile: (target_ac: 16, target_hp: 60),
///     adventuring_day: (encounters: 4, rounds_per_encounter: 3, short_rests_after: [2]),
///     nova_rounds: 2,
/// )
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionSet {
    pub encounter_profile: EncounterProfile,
    pub adventuring_day: AdventuringDayModel,
    /// The Nova horizon: rounds 1 to `nova_rounds`, with full resources.
    pub nova_rounds: NonZeroU32,
}

/// The target. Every evaluation covers ACs 10–25; `target_ac` is the one reported first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterProfile {
    pub target_ac: i32,
    /// The target's hit points, which Time-to-kill and Kill probability are measured
    /// against.
    pub target_hp: NonZeroU32,
}

/// The encounters, their rounds and the rests between them that budget resources for
/// Sustained DPR (GLOSSARY.md, **Adventuring Day Model**). Sustained mode reads it (#23).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AdventuringDayFields")]
pub struct AdventuringDayModel {
    encounters: NonZeroU32,
    rounds_per_encounter: NonZeroU32,
    short_rests_after: Vec<NonZeroU32>,
}

/// [`AdventuringDayModel`] as written, before its rests are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdventuringDayFields {
    encounters: NonZeroU32,
    rounds_per_encounter: NonZeroU32,
    short_rests_after: Vec<NonZeroU32>,
}

/// Why assumptions cannot be used as given.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssumptionError {
    #[error(
        "the short rests must come after distinct encounters, in order, before the last \
         of the {encounters}; got after {short_rests_after:?}"
    )]
    RestsOutOfOrder {
        encounters: u32,
        short_rests_after: Vec<u32>,
    },
    #[error("the Encounter Baseline covers character levels 1 to 20, not {level}")]
    BeyondBaseline { level: usize },
}

/// The Nova horizon by default: rounds 1–2 (GLOSSARY.md, **Nova DPR**).
pub const DEFAULT_NOVA_ROUNDS: NonZeroU32 = NonZeroU32::new(2).expect("2 is not zero");

/// Dunn's baseline AC at CR 1 to 20, entry `CR − 1`.
const BASELINE_AC: [i32; 20] = [
    14, 14, 15, 15, 15, 16, 16, 16, 17, 17, 17, 17, 18, 18, 18, 19, 19, 19, 19, 20,
];

impl AssumptionSet {
    /// The default assumptions against `encounter_profile`: the default Adventuring Day
    /// Model and the Nova horizon of rounds 1–2.
    pub fn against(encounter_profile: EncounterProfile) -> AssumptionSet {
        AssumptionSet {
            encounter_profile,
            adventuring_day: AdventuringDayModel::default(),
            nova_rounds: DEFAULT_NOVA_ROUNDS,
        }
    }

    /// The default assumptions for a character of `level`: against the Encounter
    /// Baseline at CR = `level`.
    ///
    /// # Errors
    /// If `level` is outside 1–20, where the baseline has no target.
    pub fn baseline(level: usize) -> Result<AssumptionSet, AssumptionError> {
        Ok(AssumptionSet::against(EncounterProfile::baseline(level)?))
    }
}

impl EncounterProfile {
    /// The Encounter Baseline's target at CR = `level` (the module docs give its source).
    ///
    /// # Errors
    /// If `level` is outside 1–20.
    pub fn baseline(level: usize) -> Result<EncounterProfile, AssumptionError> {
        let beyond = || AssumptionError::BeyondBaseline { level };
        let target_ac = *level
            .checked_sub(1)
            .and_then(|index| BASELINE_AC.get(index))
            .ok_or_else(beyond)?;
        let cr = u32::try_from(level).map_err(|_| beyond())?;
        let target_hp = NonZeroU32::new(10 + 17 * cr).ok_or_else(beyond)?;
        Ok(EncounterProfile {
            target_ac,
            target_hp,
        })
    }
}

impl AdventuringDayModel {
    /// An Adventuring Day Model of `encounters` encounters of `rounds_per_encounter` rounds each, with a
    /// short rest after each encounter in `short_rests_after` (counted from 1).
    ///
    /// # Errors
    /// Unless the rests follow distinct encounters, in increasing order, each before the
    /// last encounter.
    pub fn new(
        encounters: NonZeroU32,
        rounds_per_encounter: NonZeroU32,
        short_rests_after: Vec<NonZeroU32>,
    ) -> Result<AdventuringDayModel, AssumptionError> {
        let in_order = short_rests_after.is_sorted_by(|a, b| a < b)
            && short_rests_after
                .last()
                .is_none_or(|&last| last < encounters);
        if !in_order {
            return Err(AssumptionError::RestsOutOfOrder {
                encounters: encounters.get(),
                short_rests_after: short_rests_after.iter().map(|n| n.get()).collect(),
            });
        }
        Ok(AdventuringDayModel {
            encounters,
            rounds_per_encounter,
            short_rests_after,
        })
    }

    pub fn encounters(&self) -> NonZeroU32 {
        self.encounters
    }

    pub fn rounds_per_encounter(&self) -> NonZeroU32 {
        self.rounds_per_encounter
    }

    /// The encounters, counted from 1, that a short rest follows, in increasing order.
    pub fn short_rests_after(&self) -> &[NonZeroU32] {
        &self.short_rests_after
    }
}

/// The default Adventuring Day Model (spec #12): 4 encounters of 3 rounds, with a short
/// rest after encounter 2.
impl Default for AdventuringDayModel {
    fn default() -> AdventuringDayModel {
        let n = |n| NonZeroU32::new(n).expect("not zero");
        AdventuringDayModel::new(n(4), n(3), vec![n(2)]).expect("the default rests are in order")
    }
}

impl TryFrom<AdventuringDayFields> for AdventuringDayModel {
    type Error = AssumptionError;

    fn try_from(fields: AdventuringDayFields) -> Result<AdventuringDayModel, AssumptionError> {
        AdventuringDayModel::new(
            fields.encounters,
            fields.rounds_per_encounter,
            fields.short_rests_after,
        )
    }
}

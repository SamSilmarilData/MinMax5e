//! A Build: the character choices that are evaluated.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ContentId;

/// A character, by the Content IDs of its choices.
///
/// Subclasses and spells join with the fixtures that need them. Whether the choices are
/// legal together (a feat for each feat a level grants, say) is not checked yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    pub species: ContentId,
    pub background: ContentId,
    /// The Origin feat the background grants.
    pub origin_feat: ContentId,
    /// Final ability scores, every increase included.
    pub abilities: AbilityScores,
    /// The class taken at each character level, in order: entry 0 is level 1.
    pub levels: Vec<ContentId>,
    /// Every feat taken after the Origin feat, fighting styles included.
    pub feats: Vec<ContentId>,
    pub loadout: Loadout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityScores {
    pub str: i32,
    pub dex: i32,
    pub con: i32,
    pub int: i32,
    pub wis: i32,
    pub cha: i32,
}

/// What the character carries into combat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loadout {
    pub main_hand: ContentId,
}

/// Why a Build cannot be evaluated as written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    #[error("the Build names no class levels")]
    NoLevels,
    #[error(
        "the Build mixes Rulesets ({}); every Content ID must share one prefix (ADR-0008)",
        .0.join(", ")
    )]
    MixedRulesets(Vec<String>),
}

impl Build {
    /// The character level: the number of class levels.
    pub fn level(&self) -> usize {
        self.levels.len()
    }

    /// The level in its class that character level `index + 1` grants: how many of the
    /// levels up to and including it are in the same class.
    ///
    /// # Panics
    /// If `index` is not below [`Build::level`].
    pub fn class_level(&self, index: usize) -> usize {
        let class = &self.levels[index];
        self.levels[..=index]
            .iter()
            .filter(|id| *id == class)
            .count()
    }

    /// Every Content ID the Build names.
    pub fn content_ids(&self) -> impl Iterator<Item = &ContentId> {
        [&self.species, &self.background, &self.origin_feat]
            .into_iter()
            .chain(&self.levels)
            .chain(&self.feats)
            .chain([&self.loadout.main_hand])
    }

    /// The Ruleset prefix all its Content IDs share: the Ruleset it is evaluated under.
    ///
    /// # Errors
    /// If it names no levels, or its Content IDs carry more than one prefix.
    pub fn ruleset(&self) -> Result<&str, BuildError> {
        if self.levels.is_empty() {
            return Err(BuildError::NoLevels);
        }
        let prefixes: BTreeSet<&str> = self.content_ids().map(ContentId::ruleset).collect();
        match prefixes.len() {
            1 => Ok(self.levels[0].ruleset()),
            _ => Err(BuildError::MixedRulesets(
                prefixes.into_iter().map(str::to_owned).collect(),
            )),
        }
    }
}

impl AbilityScores {
    /// The modifier for a score: ⌊(score − 10) / 2⌋.
    pub fn modifier(score: i32) -> i32 {
        (score - 10).div_euclid(2)
    }
}

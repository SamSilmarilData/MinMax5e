//! Compiling a Build into a Combat Kit, split into Kit Shape and Kit Params (ADR-0047).

use mm_core::records::DiceTerm;
use mm_core::{AbilityScores, Build, BuildError, ContentId, Fp, FpHasher, Ruleset};
use mm_load::Pack;

/// A Build compiled for evaluation: its structure and its numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatKit {
    pub shape: KitShape,
    pub params: KitParams,
}

/// What decides which turn states exist and which steps are legal (architecture §2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitShape {
    /// Attacks one Attack action grants (at least 1).
    pub attacks_per_action: u8,
    /// The ways the character can attack, one row each.
    pub rows: Vec<RowShape>,
}

/// One attack row's structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowShape {
    pub source: AttackSource,
}

/// What lets an attack be made (ADR-0012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackSource {
    AttackAction,
}

/// The numbers evaluated over a Kit Shape: one entry per row of the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitParams {
    pub rows: Vec<RowParams>,
}

/// One attack row's numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowParams {
    pub to_hit: i32,
    /// The lowest natural roll that crits.
    pub crit_on: i32,
    pub hit: Damage,
    pub crit: Damage,
}

/// Damage dealt by one outcome: dice plus a flat amount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    pub dice: Vec<DiceTerm>,
    pub flat: i32,
}

/// Why a Build does not compile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error("the Build is for Ruleset `{build}`, but the pack is `{pack}`")]
    WrongRuleset { build: String, pack: &'static str },
    #[error("{0} is not a class in the pack")]
    UnknownClass(ContentId),
    #[error("{0} is not a weapon in the pack")]
    UnknownWeapon(ContentId),
    #[error("{0} has no encoding, so its mechanics are unknown (ADR-0002)")]
    Unencoded(ContentId),
    #[error("{class} level {level} grants {feature}, whose riders the engine cannot run yet")]
    UnsupportedRiders {
        class: ContentId,
        level: u8,
        feature: String,
    },
}

/// Compiles `build` against `pack`.
///
/// # Errors
/// If the Build is not for `R`, names content the pack lacks or has not encoded, or gains
/// a feature the engine cannot run yet.
pub fn compile<R: Ruleset>(pack: &Pack<R>, build: &Build) -> Result<CombatKit, CompileError> {
    let ruleset = build.ruleset()?;
    if ruleset != R::PREFIX {
        return Err(CompileError::WrongRuleset {
            build: ruleset.to_owned(),
            pack: R::PREFIX,
        });
    }

    // Each class's encoding, checked for features gained by this Build's level in it.
    for (index, class_id) in build.levels.iter().enumerate() {
        let class = pack
            .class(class_id)
            .ok_or_else(|| CompileError::UnknownClass(class_id.clone()))?;
        let encoding = pack[class]
            .encoding
            .as_ref()
            .ok_or_else(|| CompileError::Unencoded(class_id.clone()))?;
        let class_level = build.levels[..=index]
            .iter()
            .filter(|id| *id == class_id)
            .count();
        for feature in &encoding.features {
            if usize::from(feature.level) == class_level && !feature.riders.is_empty() {
                return Err(CompileError::UnsupportedRiders {
                    class: class_id.clone(),
                    level: feature.level,
                    feature: feature.name.clone(),
                });
            }
        }
    }

    let weapon_id = &build.loadout.main_hand;
    let weapon = &pack[pack
        .weapon(weapon_id)
        .ok_or_else(|| CompileError::UnknownWeapon(weapon_id.clone()))?];

    // `build.ruleset()` has checked there is at least one level.
    let proficiency = i32::try_from(2 + (build.levels.len() - 1) / 4).unwrap_or(i32::MAX);
    let ability = AbilityScores::modifier(build.abilities.str);
    // A crit doubles the attack's dice, not its flat bonus.
    let crit_dice = DiceTerm {
        count: weapon.damage.count * 2,
        ..weapon.damage
    };

    Ok(CombatKit {
        shape: KitShape {
            attacks_per_action: 1,
            rows: vec![RowShape {
                source: AttackSource::AttackAction,
            }],
        },
        params: KitParams {
            rows: vec![RowParams {
                to_hit: ability + proficiency,
                crit_on: 20,
                hit: Damage {
                    dice: vec![weapon.damage],
                    flat: ability,
                },
                crit: Damage {
                    dice: vec![crit_dice],
                    flat: ability,
                },
            }],
        },
    })
}

impl KitShape {
    /// The `ShapeKey`: a fingerprint of everything in the shape. Equal shapes give equal keys.
    pub fn key(&self) -> Fp {
        let mut h = FpHasher::new("mm-engine/ShapeKey/1");
        h.u64(u64::from(self.attacks_per_action))
            .u64(self.rows.len() as u64);
        for row in &self.rows {
            h.str(match row.source {
                AttackSource::AttackAction => "attack-action",
            });
        }
        h.finish()
    }
}

impl KitParams {
    /// The `ParamsFp`: a fingerprint of every number in the params. Equal params give
    /// equal fingerprints.
    pub fn fp(&self) -> Fp {
        let mut h = FpHasher::new("mm-engine/ParamsFp/1");
        h.u64(self.rows.len() as u64);
        for row in &self.rows {
            h.i64(row.to_hit.into()).i64(row.crit_on.into());
            for damage in [&row.hit, &row.crit] {
                h.u64(damage.dice.len() as u64);
                for dice in &damage.dice {
                    h.u64(dice.count.into()).u64(dice.sides.into());
                }
                h.i64(damage.flat.into());
            }
        }
        h.finish()
    }
}

//! Compiling a Build into a Combat Kit, split into Kit Shape and Kit Params (ADR-0047).

use mm_core::effect::{Fact, Grant, Op, Pred, Rider, Stage};
use mm_core::kind::{self, Kind};
use mm_core::records::WeaponRecord;
use mm_core::{AbilityScores, Build, BuildError, ContentId, Fp, FpHasher, Ruleset};
use mm_dice::{Moments, Pmf};
use mm_load::{FeaturesKind, Pack};

/// A Build compiled for evaluation: its structure and its numbers.
#[derive(Debug, Clone, PartialEq)]
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
    /// The row a crit with this row offers immediately after it, if any.
    pub crit_offers: Option<u8>,
}

/// What lets an attack be made (ADR-0012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackSource {
    /// An attack made as part of the Attack action.
    AttackAction,
    /// A Bonus Action attack a crit offers, taken immediately after it or not at all.
    BonusActionAfterCrit,
}

/// The numbers evaluated over a Kit Shape: one entry per row of the shape.
#[derive(Debug, Clone, PartialEq)]
pub struct KitParams {
    pub rows: Vec<RowParams>,
}

/// One attack row's numbers (ADR-0047): the roll, and the moments of each outcome's damage.
#[derive(Debug, Clone, PartialEq)]
pub struct RowParams {
    pub to_hit: i32,
    /// The lowest natural roll that crits.
    pub crit_on: i32,
    /// The damage of a miss, such as Graze damage.
    pub miss: Moments,
    /// The damage of a hit that is not a crit.
    pub hit: Moments,
    /// The damage of a crit.
    pub crit: Moments,
}

/// Why a Build does not compile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error("the Build is for Ruleset `{build}`, but the pack is `{pack}`")]
    WrongRuleset { build: String, pack: &'static str },
    #[error("{id} is not a {kind} in the pack")]
    NotInPack { kind: &'static str, id: ContentId },
    #[error("{0} has no encoding, so its mechanics are unknown (ADR-0002)")]
    Unencoded(ContentId),
    #[error("{feature} has a rider the engine cannot run yet: {reason}")]
    UnsupportedRider {
        feature: String,
        reason: &'static str,
    },
}

/// Compiles `build` against `pack`.
///
/// # Errors
/// If the Build is not for `R`, names content the pack lacks or has not encoded, or gains
/// a rider the engine cannot run yet.
pub fn compile<R: Ruleset>(pack: &Pack<R>, build: &Build) -> Result<CombatKit, CompileError> {
    let ruleset = build.ruleset()?;
    if ruleset != R::PREFIX {
        return Err(CompileError::WrongRuleset {
            build: ruleset.to_owned(),
            pack: R::PREFIX,
        });
    }

    let mut gained = Gained::default();
    for (index, class_id) in build.levels.iter().enumerate() {
        let class = pack
            .class(class_id)
            .ok_or_else(|| not_in_pack::<kind::Class>(class_id))?;
        let encoding = pack[class]
            .encoding
            .as_ref()
            .ok_or_else(|| CompileError::Unencoded(class_id.clone()))?;
        let class_level = build.class_level(index);
        for feature in &encoding.features {
            if usize::from(feature.level) == class_level {
                gained.add(&feature.name, &feature.riders, &feature.grants)?;
            }
        }
    }
    gained.with_features::<R, kind::Species>(pack, &build.species)?;
    gained.with_features::<R, kind::Background>(pack, &build.background)?;
    gained.with_features::<R, kind::Feat>(pack, &build.origin_feat)?;
    for feat in &build.feats {
        gained.with_features::<R, kind::Feat>(pack, feat)?;
    }

    let weapon_id = &build.loadout.main_hand;
    let weapon = &pack[pack
        .weapon(weapon_id)
        .ok_or_else(|| not_in_pack::<kind::Item>(weapon_id))?];
    // With Weapon Mastery, the weapon's mastery property joins its attacks. The Loadout
    // holds one weapon, so what the property grants is the Build's.
    if let Some(property) = weapon
        .mastery_property
        .as_ref()
        .filter(|_| gained.weapon_mastery)
    {
        gained.with_features::<R, kind::MasteryProperty>(pack, property)?;
    }

    // `build.ruleset()` has checked there is at least one level.
    let proficiency = i32::try_from(2 + (build.level() - 1) / 4).unwrap_or(i32::MAX);
    let ability = AbilityScores::modifier(build.abilities.str);
    let attack = |source| Attack {
        weapon,
        source,
        proficiency,
        ability,
    };

    // Row 0 is the Attack action. A crit-gated Bonus Action attack, if any rider offers
    // one, is row `BONUS_ROW`, with the same weapon.
    let offers_bonus = gained
        .riders
        .iter()
        .any(|rider| rider.effect == Op::BonusActionAttack);
    let attacks = if offers_bonus {
        vec![
            attack(AttackSource::AttackAction),
            attack(AttackSource::BonusActionAfterCrit),
        ]
    } else {
        vec![attack(AttackSource::AttackAction)]
    };

    let mut shape_rows = Vec::new();
    let mut param_rows = Vec::new();
    for attack in &attacks {
        let applying: Vec<&Rider> = gained
            .riders
            .iter()
            .filter(|rider| rider.when.as_ref().is_none_or(|pred| attack.holds(pred)))
            .collect();
        let crit_offers = applying
            .iter()
            .any(|rider| rider.effect == Op::BonusActionAttack)
            .then_some(BONUS_ROW);
        shape_rows.push(RowShape {
            source: attack.source,
            crit_offers,
        });
        param_rows.push(attack.params(&applying));
    }

    Ok(CombatKit {
        shape: KitShape {
            attacks_per_action: gained.attacks_per_action,
            rows: shape_rows,
        },
        params: KitParams { rows: param_rows },
    })
}

/// The row of the crit-gated Bonus Action attack, when the kit has one: the second, after
/// the Attack action's.
const BONUS_ROW: u8 = 1;

fn not_in_pack<K: Kind>(id: &ContentId) -> CompileError {
    CompileError::NotInPack {
        kind: K::NAME,
        id: id.clone(),
    }
}

/// Everything the Build's features grant, gathered before the rows are built.
#[derive(Debug)]
struct Gained {
    riders: Vec<Rider>,
    attacks_per_action: u8,
    weapon_mastery: bool,
}

impl Default for Gained {
    fn default() -> Self {
        Gained {
            riders: Vec::new(),
            attacks_per_action: 1,
            weapon_mastery: false,
        }
    }
}

impl Gained {
    /// Adds the features of the species, background, feat or mastery property `id`.
    fn with_features<R: Ruleset, K: FeaturesKind>(
        &mut self,
        pack: &Pack<R>,
        id: &ContentId,
    ) -> Result<(), CompileError> {
        let found = pack
            .with_features::<K>(id)
            .ok_or_else(|| not_in_pack::<K>(id))?;
        let encoding = pack[found]
            .encoding
            .as_ref()
            .ok_or_else(|| CompileError::Unencoded(id.clone()))?;
        for feature in &encoding.features {
            self.add(&feature.name, &feature.riders, &feature.grants)?;
        }
        Ok(())
    }

    /// Adds one feature, checking that the engine can run its riders.
    fn add(
        &mut self,
        feature: &str,
        riders: &[Rider],
        grants: &[Grant],
    ) -> Result<(), CompileError> {
        for rider in riders {
            supported(rider).map_err(|reason| CompileError::UnsupportedRider {
                feature: feature.to_owned(),
                reason,
            })?;
            self.riders.push(rider.clone());
        }
        for grant in grants {
            match *grant {
                Grant::ExtraAttack(attacks) => {
                    self.attacks_per_action = self.attacks_per_action.max(attacks);
                }
                Grant::WeaponMastery => self.weapon_mastery = true,
            }
        }
        Ok(())
    }
}

/// Whether the engine can run `rider`, and if not, why.
fn supported(rider: &Rider) -> Result<(), &'static str> {
    if rider.limit.is_some() {
        return Err("limits need resource tracking in the turn state (#22)");
    }
    match rider.effect {
        Op::FlatDamage(_) | Op::ProficiencyBonusDamage | Op::AbilityModifierDamage => {
            if rider.optional {
                Err("optional damage needs Decide points (#22)")
            } else {
                Ok(())
            }
        }
        Op::MinimumDieRoll(_) => match rider.on {
            Stage::AttackHit => Ok(()),
            Stage::AttackCrit | Stage::AttackMiss => {
                Err("a minimum die roll runs on AttackHit only, which covers crits")
            }
        },
        Op::BonusActionAttack => match (rider.on, rider.optional) {
            (Stage::AttackCrit, true) => Ok(()),
            (Stage::AttackCrit, false) => {
                Err("an offered attack is the player's choice, so it is optional")
            }
            (Stage::AttackHit | Stage::AttackMiss, _) => {
                Err("only a crit can offer a Bonus Action attack yet")
            }
        },
    }
}

/// One attack row before its numbers are worked out.
#[derive(Debug, Clone, Copy)]
struct Attack<'a> {
    weapon: &'a WeaponRecord,
    source: AttackSource,
    proficiency: i32,
    ability: i32,
}

impl Attack<'_> {
    /// Whether `pred` holds for this attack.
    fn holds(&self, pred: &Pred) -> bool {
        match pred {
            Pred::Fact(Fact::WeaponProperty(property)) => self.weapon.properties.contains(property),
            Pred::Fact(Fact::AttackAction) => self.source == AttackSource::AttackAction,
            Pred::All(preds) => preds.iter().all(|p| self.holds(p)),
            Pred::Any(preds) => preds.iter().any(|p| self.holds(p)),
            Pred::Not(pred) => !self.holds(pred),
        }
    }

    /// The row's numbers, with `riders` (those whose predicates hold) applied.
    fn params(&self, riders: &[&Rider]) -> RowParams {
        // The weapon's dice plus the ability modifier on a hit; a crit doubles the dice,
        // not the flat damage.
        let mut flat = FlatDamage {
            miss: 0,
            hit: self.ability,
            crit: self.ability,
        };
        let mut minimum_roll = 1;
        for rider in riders {
            let amount = match rider.effect {
                Op::FlatDamage(amount) => amount,
                Op::ProficiencyBonusDamage => self.proficiency,
                Op::AbilityModifierDamage => self.ability,
                Op::MinimumDieRoll(roll) => {
                    minimum_roll = minimum_roll.max(roll);
                    continue;
                }
                Op::BonusActionAttack => continue,
            };
            match rider.on {
                Stage::AttackMiss => flat.miss += amount,
                Stage::AttackHit => {
                    flat.hit += amount;
                    flat.crit += amount;
                }
                Stage::AttackCrit => flat.crit += amount,
            }
        }
        let die = Pmf::die(self.weapon.damage.sides)
            .at_least(i32::try_from(minimum_roll).unwrap_or(i32::MAX));
        let count = self.weapon.damage.count;
        RowParams {
            to_hit: self.ability + self.proficiency,
            crit_on: 20,
            miss: Moments::point(flat.miss),
            hit: die.sum_of(count).shift(flat.hit).moments(),
            crit: die.sum_of(count * 2).shift(flat.crit).moments(),
        }
    }
}

/// The flat damage each attack outcome adds to its dice.
#[derive(Debug, Clone, Copy)]
struct FlatDamage {
    miss: i32,
    hit: i32,
    crit: i32,
}

impl KitShape {
    /// The `ShapeKey`: a fingerprint of everything in the shape. Equal shapes give equal keys.
    pub fn key(&self) -> Fp {
        let mut h = FpHasher::new("mm-engine/ShapeKey/2");
        h.u64(u64::from(self.attacks_per_action))
            .u64(self.rows.len() as u64);
        for row in &self.rows {
            h.str(match row.source {
                AttackSource::AttackAction => "attack-action",
                AttackSource::BonusActionAfterCrit => "bonus-action-after-crit",
            });
            // 0 for no offer, else the offered row plus one.
            h.u64(row.crit_offers.map_or(0, |offered| u64::from(offered) + 1));
        }
        h.finish()
    }
}

impl KitParams {
    /// The `ParamsFp`: a fingerprint of every number in the params, floats by their bits.
    /// Equal params give equal fingerprints, so damage terms with equal moments share one.
    pub fn fp(&self) -> Fp {
        let mut h = FpHasher::new("mm-engine/ParamsFp/2");
        h.u64(self.rows.len() as u64);
        for row in &self.rows {
            h.i64(row.to_hit.into()).i64(row.crit_on.into());
            for moments in [row.miss, row.hit, row.crit] {
                h.u64(moments.mean().to_bits())
                    .u64(moments.second().to_bits());
            }
        }
        h.finish()
    }
}

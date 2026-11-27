//! Resolved records: what a piece of content *is*, in the shape `mm-import` will produce
//! from the corpus (architecture §1). What it *does* is its encoding ([`effect`](crate::effect)).
//!
//! The fixtures write records by hand; nothing here is rules text.

use serde::{Deserialize, Serialize};

use crate::ContentId;

/// One record in a records file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Record {
    Class(ClassRecord),
    Weapon(WeaponRecord),
}

impl Record {
    pub fn id(&self) -> &ContentId {
        match self {
            Record::Class(r) => &r.id,
            Record::Weapon(r) => &r.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassRecord {
    pub id: ContentId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponRecord {
    pub id: ContentId,
    pub name: String,
    /// The weapon's damage dice; the wielder's ability modifier is added on a hit.
    pub damage: DiceTerm,
    pub properties: Vec<WeaponProperty>,
}

/// `count` dice of `sides` sides, e.g. 2d6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiceTerm {
    pub count: u32,
    pub sides: u32,
}

/// A weapon property that changes how a weapon is used. Melee Strength weapons only, for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponProperty {
    Heavy,
    TwoHanded,
}

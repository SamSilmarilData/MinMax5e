//! The shared vocabulary every other crate speaks (architecture §8). It knows no edition:
//! a [`Ruleset`] supplies the edition's facts.
//!
//! The public seam:
//! - [`Ruleset`], [`ResourceDecl`] and [`ActionEconomy`]: an edition's prefix and economy.
//! - [`ContentId`] (`<ruleset>:<kind>/<slug>|<source>`, ADR-0008) and the typed
//!   [`Id<R, K>`](Id) handles a Rules Pack hands out, with the kinds in [`kind`].
//! - [`records`]: resolved records, the shape `mm-import` will produce.
//! - [`effect`]: the closed Effect DSL (ADR-0010, ADR-0011) that encodings are written in.
//! - [`Build`] and [`AssumptionSet`]: what is evaluated, and against what, with the
//!   Encounter Baseline default target ([`EncounterProfile::baseline`], ADR-0024).
//! - [`Citation`] and [`SourceTier`]: where an encoded feature comes from, and how far it is
//!   trusted (ADR-0007).
//! - [`Ruling`] and [`RulingSet`]: a pack's declared Rulings and a user's chosen options
//!   (ADR-0006).
//! - [`Fp`]: the 128-bit fingerprint (ADR-0052).
//! - [`Sources`]: named RON texts, the input to loading a pack.
//!
//! Dependencies are serde, thiserror and blake3 only, and not `mm-dice` (ADR-0034).

pub mod assumptions;
pub mod build;
mod citation;
pub mod effect;
mod fp;
mod id;
pub mod kind;
pub mod records;
mod ruleset;
mod ruling;
mod sources;

pub use assumptions::{
    AdventuringDayModel, AssumptionError, AssumptionSet, DEFAULT_NOVA_ROUNDS, EncounterProfile,
};
pub use build::{AbilityScores, Build, BuildError, Loadout};
pub use citation::{Citation, SourceTier};
pub use fp::{Fp, FpHasher};
pub use id::{ContentId, ContentIdError, Id};
pub use ruleset::{ActionEconomy, Refresh, ResourceDecl, Ruleset};
pub use ruling::{
    Ruling, RulingError, RulingId, RulingIdError, RulingOption, RulingOptionId,
    RulingOptionIdError, RulingSet,
};
pub use sources::{Source, SourceRole, Sources};

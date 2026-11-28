//! What a Build is evaluated against.

/// The assumptions behind an evaluation. The Encounter Baseline default and the Adventuring
/// Day Model join with #20 and #23.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumptionSet {
    pub encounter_profile: EncounterProfile,
    /// The Nova horizon: rounds 1 to `nova_rounds`, with full resources.
    pub nova_rounds: u32,
}

/// The target. Every evaluation covers ACs 10–25; `target_ac` is the one reported first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterProfile {
    pub target_ac: i32,
    /// The target's hit points, which Time-to-kill and Kill probability are measured
    /// against. `None` leaves them out; the Encounter Baseline default arrives with #20.
    pub target_hp: Option<u32>,
}

impl AssumptionSet {
    /// The default assumptions against a target of `target_ac`, with no hit points given:
    /// Nova over rounds 1–2.
    pub fn against(target_ac: i32) -> AssumptionSet {
        AssumptionSet {
            encounter_profile: EncounterProfile {
                target_ac,
                target_hp: None,
            },
            nova_rounds: 2,
        }
    }
}

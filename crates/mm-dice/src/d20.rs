//! d20 attack rolls: the odds of each Attack outcome.

/// How many d20s are rolled for an attack, and which is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RollMode {
    Normal,
    /// Two d20s, keep the higher.
    Advantage,
    /// Two d20s, keep the lower.
    Disadvantage,
}

/// One attack roll against a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttackRoll {
    /// The total attack bonus added to the d20.
    pub bonus: i32,
    /// The target's Armor Class.
    pub ac: i32,
    /// The lowest natural roll that is a Critical Hit (20 normally, 19 with a widened range).
    pub crit_on: i32,
    pub mode: RollMode,
}

/// The probability of each Attack outcome. `hit` excludes crits; the three sum to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutcomeOdds {
    pub miss: f64,
    pub hit: f64,
    pub crit: f64,
}

impl AttackRoll {
    /// The odds of a miss, a non-crit hit and a crit.
    ///
    /// A natural 1 always misses and a natural 20 always crits. A natural roll in the crit
    /// range always hits, even if it falls short of the AC (research 03, Open Q1). The odds
    /// depend only on `ac - bonus`, so one more to-hit equals one less AC bit for bit.
    ///
    /// Each probability is an exact integer count of the equally likely dice faces, divided
    /// once, so it is the correctly rounded value of the exact fraction. Ported from
    /// `outcome` in the tape spike (`spikes/tape-spike/src/main.rs`), with the hit count
    /// also taken in integers rather than as `1 - miss - crit`.
    pub fn odds(self) -> OutcomeOdds {
        // The lowest natural roll that crits, and the lowest that hits.
        let crit_face = self.crit_on.clamp(2, 20);
        let hit_face = (self.ac - self.bonus).clamp(2, 20).min(crit_face);
        // Faces below `face` on one d20, and faces at or above it.
        let below = |face: i32| face - 1;
        let at_or_above = |face: i32| 21 - face;
        let (miss, crit, total) = match self.mode {
            RollMode::Normal => (below(hit_face), at_or_above(crit_face), 20),
            // Both dice must miss to miss; either die in the crit range crits.
            RollMode::Advantage => (
                below(hit_face) * below(hit_face),
                400 - below(crit_face) * below(crit_face),
                400,
            ),
            // Either die missing misses; both dice must be in the crit range to crit.
            RollMode::Disadvantage => (
                400 - at_or_above(hit_face) * at_or_above(hit_face),
                at_or_above(crit_face) * at_or_above(crit_face),
                400,
            ),
        };
        let hit = total - miss - crit;
        let total = f64::from(total);
        OutcomeOdds {
            miss: f64::from(miss) / total,
            hit: f64::from(hit) / total,
            crit: f64::from(crit) / total,
        }
    }
}

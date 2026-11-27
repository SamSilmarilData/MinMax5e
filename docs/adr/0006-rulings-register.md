---
status: proposed
---

# Ambiguous rules become named, versioned Rulings, and every result records which Rulings it used

Every ambiguous or disputed mechanic becomes a **Ruling** with:

- a stable id (e.g. `R-NICK-WEAPON`);
- its options and a default;
- citations;
- the reasoning behind the default.

No interpretation is hard-coded in feature logic; feature logic reads the active Ruling set. Every DPR result, build ranking, and combat script records the version of the Ruling set it was computed under. Rulings that are disputed at real tables, such as mid-Attack weapon swap chains or Nick without wielding both weapons, show to the user as toggles with a conservative default, and the dashboard can show how much DPR flipping one would change.

At least ten mechanics that affect DPR have no official answer. Examples: which weapon must carry Nick; Nick with Action Surge; whether Hunter's Mark and Hex dice double on a crit; whether the Two-Weapon Fighting style applies to the Dual Wielder attack. Sage Advice also changes over time. Hard-coded choices would make old rankings impossible to reproduce, and the engine must take Rulings as input from the start, because adding that later is expensive.

## Considered Options

- One hard-coded "RAW as we read it" interpretation. Rejected: opaque, and it loses the trust of tables that rule differently.
- The most permissive reading. Rejected: it inflates numbers, which is the "white-room" criticism the product exists to avoid.
- Free-form house rules per user. Rejected: results stop being comparable and caching breaks.

Research: [02](../research/02-rules-mechanics-2024.md) §8, [06](../research/06-market-users-and-ux.md)

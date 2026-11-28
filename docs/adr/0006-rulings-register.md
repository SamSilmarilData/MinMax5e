---
status: accepted
---

# Ambiguous rules become named, versioned Rulings, and every result records which Rulings it used

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-28 (#21):* a pack declares each Ruling in a RON file under its `rulings/` directory: the id, the question, the options (each a stable id and its reading), the default, the reasoning and the citations. A Ruling is versioned by its option ids: an option's meaning never changes once results record it, and a changed reading is a new option. So a result records each Ruling it read with its option id, in place of a version of the whole Ruling set (ADR-0052). Encodings read a Ruling with the Effect DSL predicate `Ruling(id, option)`, and a user's choices are a Ruling Set (`mm-cli evaluate --rulings`).

> *Amended 2026-11-27 (ADR-0052):* fingerprints include only the Rulings that a kit's compile or trace actually read. So "how much DPR flipping this Ruling changes" re-scores only the pool entries that depend on that Ruling, not the whole Solve. Each disputed Ruling also gets a test scenario asserting the DPR delta between its options (ADR-0056).

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

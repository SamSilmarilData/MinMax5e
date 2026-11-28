---
status: accepted
---

# The default target is a monster of CR equal to character level, with stats fitted to SRD 5.2.1

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-28 (what the baseline ships, #20):* Only AC and HP are modelled so far; the save curves and Magic Resistance wait until saves are modelled (spec #12, "Out of Scope"). AC is Dunn's tabulated value at CR 1–20, and the baseline covers character levels 1–20. Rounded as he tabulates it, the table lies within about 1.0 AC of our SRD 5.2.1 fit, not 0.5 (research errata, 03 §5.3). Dunn fitted his table to the 2014 books, so the CC-BY reasoning above covers HP (our SRD 5.2.1 fit) and our check on AC, but not the AC values themselves. Those ship as his published figures, credited in `crates/mm-core/src/assumptions.rs`.

When the user hasn't overridden the Encounter Profile, the target dummy has these stats:

- **CR:** equal to the character's level.
- **AC:** from Dunn's baseline table, about 13 + CR/3.
- **Saves:** a separate curve for each ability, fitted to SRD 5.2.1 monster stat blocks.
- **Magic Resistance:** present at its observed rate among SRD monsters.
- **HP:** about 10 + 17·CR.

Three independent analyses (Hughes, Dunn, Yaarel) and our own fit to all 330 SRD 5.2.1 stat blocks with a CR (AC ≈ 12.9 + 0.37·CR) agree within about 0.5 AC. Because the source data is CC-BY, the baseline can be shipped and reproduced. Separate save curves expose a spread of roughly 20 percentage points in failure rate between abilities.

## Considered Options

- The 2014 DMG "Monster Statistics by CR" table. Rejected: AC is too flat at CR 1–3 and 17–30, HP is far too high, and it has no save bonuses.
- A fixed 65% hit rate. Rejected: it hides the AC trade-off the product exists to show.
- A single average save bonus. Rejected: it hides which saving throws are weak.

Research: [03](../research/03-probability-and-damage-math.md)

---
status: accepted
---

# Each edition is a separate Rules Pack over a shared core, and every content ID starts with its ruleset

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

The spec requires "hard boundaries" against mixing editions. We enforce them at the data and package level, not in solver logic:

- `rules-2024` and `rules-2014` are separate Rules Packs (separate crates; see ADR-0034) loaded into a shared engine core that contains no edition-specific code. Each pack implements its edition's action economy, crit, and stacking rules. Foundry dnd5e's approach of scattering `isLegacy` checks through shared code is the pattern we are avoiding.
- Content IDs take the form `<ruleset>:<kind>/<slug>|<source>`, derived from the 5e.tools key (e.g. `2024:feat/great-weapon-master|xphb`). Subclasses and features use composite paths. Hand-made content uses source `mm5e`. Loading a pack fails if it references an ID from another pack.
- Every record also stores `upstream_key` (`Name|SOURCE`) and `upstream_hash` (ADR-0031).
- *Amended 2026-11-27:* the original form had no `|source` and kept the 5e.tools key only as an alias. With 5e.tools as the corpus (ADR-0027), deriving IDs from its key removes the alias table. Keeping the source avoids 25 slug collisions, and the ruleset prefix still lets one upstream key carry two encodings (e.g. Spirit Shroud under 2014 rules and as legacy-compatible 2024).

## Considered Options

- A single entity carrying an `edition` attribute (the 5e.tools model). Rejected: it allows the generated copies that attach 2014 subclasses to 2024 classes, which is exactly the mixing the spec forbids.
- Bare 5e.tools `Name|SOURCE` keys as the identity (proposed in research 04). Rejected: one key can need encodings in both rulesets, and book source codes don't map 1:1 to rulesets.
- Separate engines per edition. Rejected: the two engines would drift apart.

Research: [08](../research/08-5etools-ingestion-personal-use.md) §5, [01](../research/01-data-source-and-licensing.md), [04](../research/04-rules-engine-architecture.md), [07](../research/07-tech-stack-and-architecture.md)

---
status: accepted
---

# Hand-author mechanics instead of parsing rules prose

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

The spec says features are "converted from natural language text into executable mathematical triggers". We will not parse prose, at runtime or at build time. Executable behaviour (triggers, conditions, effects, action costs, limits) is hand-authored in our own Effect DSL (ADR-0010) and keyed by content ID. Drafts written with an LLM are allowed but must always be reviewed. Descriptive text and metadata are a separate layer that the engine never reads, so the legally sensitive text can be swapped or left out without touching the engine.

Feature text is untemplated prose, and no available source (5e.tools, Open5e, dnd5eapi, Foundry) encodes mechanics. Even MTG Arena's dedicated parser, working on far more regular card text, leaves about 20% of cards to humans. A wrongly parsed trigger silently corrupts solver rankings.

CI fails if any in-scope content ID has no encoding (a coverage report).

> *Amended 2026-11-28 (where the coverage gate runs, #17):* CI cannot see the in-scope content IDs. Records come from the corpus, which stays in the local cache and which CI never fetches (ADR-0028). So the coverage report is a gate in `mm-cli validate-packs`, which fails on any in-scope record without an encoding. It runs locally against the cache once `mm-import` produces records (build-order step 6), or in an opt-in, manual-trigger job within ADR-0028's limits. In CI, `validate-packs` checks only that the embedded encodings load, and the synthetic fixtures' tests exercise the coverage report itself.

**Amended 2026-11-27 (5e.tools as the local corpus, ADR-0027).** These structured fields are imported deterministically:

- class chassis, class tables (Sneak Attack dice, Rage damage, Martial Arts die), and feature, feat and optional-feature progressions;
- feat prerequisites and ability increases, and backgrounds;
- weapon damage, properties and mastery;
- spell metadata (casting time, concentration, saving throw, damage-scaling dice).

Spell damage taken from `{@damage}` tags is reviewed before use. Encodings reference these upstream numbers instead of restating them, and carry the upstream record hash so changes upstream are caught (ADR-0031). Feature *behaviour* is still prose and still hand-authored: about 110–150 encodings for the MVP, down from 150–200 ([research 08 §4](../research/08-5etools-ingestion-personal-use.md)).

## Considered Options

- NLP or regex parsing of prose. Rejected: fragile and unverifiable.
- Unreviewed LLM extraction. Rejected for the same reasons.
- Adopt Foundry Active Effects as the rule model. Rejected: tied to Foundry's actor model, and only partly covers what we need.

Research: [01](../research/01-data-source-and-licensing.md), [04](../research/04-rules-engine-architecture.md)

---
status: proposed
---

# 2014-era content enters the 2024 ruleset only through an explicit "legacy-compatible" whitelist

A ruleset is defined by its rules **chassis**: core rules, glossary, and action economy. Not by publication date. 2014-era items that were never reprinted for 2024 (e.g. Spirit Shroud from *Tasha's*, Booming Blade, older subclasses) may carry an opt-in `legacy-compatible-2024` flag. The 2024 ruleset loads 2024 core content plus these whitelisted items, interpreted under 2024 rules (one spell slot per turn, the 2024 Attack action). Any item that was reprinted for 2024 is always replaced by its 2024 version.

This resolves a contradiction in the spec. It demands strict isolation, yet its MVP names Spirit Shroud and Booming Blade, which are 2014-era content. Official 2024 guidance also lets older content that wasn't reprinted run under 2024 rules.

*Amended 2026-11-27:* a 2014 record that has `reprintedAs` set is never eligible for the whitelist. The importer may *propose* candidates, but a human approves every entry. 5e.tools' synthetic "2014 subclass on a 2024 class" copies are never the route in; they are dropped at import (ADR-0030).

## Considered Options

- Strictly 2024 books and SRD only. Rejected: it drops damage riders the spec names.
- Allow every 2014 supplement in 2024 mode. Rejected: it brings in items that were reprinted and changed, which is exactly the "pollution" the spec forbids.
- A per-build user toggle only. Rejected: results become incomparable.

Research: [02](../research/02-rules-mechanics-2024.md)

---
status: proposed
---

# A source allowlist decides the ruleset; 5e.tools' synthetic cross-edition copies are dropped

A committed `rulesets.toml` maps each allowed source code (`XPHB`, `XDMG`, `PHB`, `TCE`, …) to a ruleset, following 5e.tools' own rule that anything published before XPHB belongs to 2014. A source that isn't listed is ignored, so new books never enter silently. Where a record has an `edition` field it must agree with the mapping, or the import fails. The importer drops:

- subclasses whose `_copy.classSource` differs from their `classSource`, which are 5e.tools' generated "2014 subclass on a 2024 class" copies;
- `reskinOf` classes.

A record with `reprintedAs` is never eligible for ADR-0009's legacy-compatible whitelist.

The `edition` field can't be the key on its own: it is missing on every feat, spell, and optional feature. Where it is present, it agreed with the date rule on all 512 records. The synthetic copies are exactly the cross-edition mixing the spec forbids, and 5e.tools itself warns that their feature levels may be wrong.

## Considered Options

- `edition` alone. Rejected: too sparse.
- An automatic date rule without an allowlist. Rejected: new books, including licensed crossover books, would enter silently.
- Keeping the synthetic copies with a warning. Rejected: they are wrong by construction.

Research: [08](../research/08-5etools-ingestion-personal-use.md) §5

---
status: superseded by ADR-0027
---

# Do not use the 5e.tools corpus as a data source

> **Superseded 2026-11-27 by [ADR-0027](0027-5etools-local-corpus.md). The project became a personal tool (ADR-0026), so the shipping and hosting concerns below became repo-hygiene rules (ADR-0028) rather than a ban.**

The spec names "the complete 5e.tools corpus" as its mechanical data source. We will not bundle, host, build from, or fetch 5e.tools data at runtime. Shipped content comes only from SRD 5.2.1 (and SRD 5.1 for the 2014 sandbox) under CC-BY-4.0, plus mechanics encodings we write ourselves.

Three reasons. The corpus is mostly verbatim non-SRD text from Wizards of the Coast, and GitHub took down 5etools mirrors in August 2024 after a DMCA notice from Wizards. Feature behaviour in the corpus is prose, not mechanics: only 31 of the 325 feature records for the six MVP classes carry any dice tag, so using it saves little engineering. And the Fan Content Policy excludes "game mechanics", is revocable at will, and forbids charging.

This decision **supersedes the spec's metadata line** "Mechanical Data Source: Complete 5e.tools Corpus".

## Considered Options

- Ingest 5e.tools at build time and ship derived data. Rejected: legal exposure, and the data is tainted at its source.
- Fetch from 5e.tools at runtime. Rejected for the same reasons.
- Rely on the Fan Content Policy alone. Rejected: it excludes mechanics and can be revoked.

## Consequences

SRD 5.2.1 lacks most of what the MVP needs: Great Weapon Master, Sharpshooter, Dual Wielder, Polearm Master, Crossbow Expert, all but one subclass per class, and most smite spells. Whether we may ship **our own encodings** of that non-SRD content (name, mechanical model, and an original paraphrase, with no copied text) is an open legal question, and the MVP's value depends on the answer. See ADR-0003 and ADR-0004.

Research: [01](../research/01-data-source-and-licensing.md), [06](../research/06-market-users-and-ux.md)

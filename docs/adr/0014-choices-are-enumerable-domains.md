---
status: accepted
---

# Choices are enumerable domains; per-turn decisions belong to the Policy, not the Build

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

A **Choice** has a typed domain query, a count, and a scope: `build`, `perLongRest` (e.g. Weapon Mastery picks), or `perTurn`. The solver assigns build-scope and long-rest Choices. Per-turn decisions (whether to smite, which weapon to attack with) are made by the Policy during evaluation and are never stored as Build attributes.

The solver needs to know how large each domain is in order to enumerate and prune. Designs that prompt the user and store the answer, like PF2e's `ChoiceSet`, hide the domain.

Research: [04](../research/04-rules-engine-architecture.md)

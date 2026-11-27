---
status: superseded by ADR-0034
---

# A pnpm monorepo, with each edition's Rules Pack as its own package

> **Superseded 2026-11-27 by [ADR-0034](0034-cargo-workspace.md). The package boundaries survive as crates.**

Packages: `dice` (Distribution Kernel), `rules-schema`, `engine`, `solver`, `rules-2024`, `rules-2014`, `worker`, `apps/web`, and `apps/cli` (batch and golden tests). The engine is generic over a ruleset type and never imports a pack. Packs are injected. pnpm's strict per-package dependency linking plus lint rules enforce that boundary, which is how ADR-0008's isolation is enforced in the build. The same boundary lets user-imported packs plug in.

## Considered Options

- One rules package with edition flags. Rejected: risk of mixing editions.
- Separate repositories. Rejected: copies of the shared engine drift apart.

Research: [07](../research/07-tech-stack-and-architecture.md)

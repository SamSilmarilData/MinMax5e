---
status: superseded by ADR-0028
---

# User-imported content stays on the user's device

> **Superseded 2026-11-27 by [ADR-0028](0028-no-upstream-bytes-in-public-repo.md). There is no separate user and no server. The rule is generalised to: all upstream-derived data stays in a local cache.**

Any rules data a user imports (for example, content from books they own) is parsed and stored only on their device. It is tagged as a user import and is never uploaded, shared, or included in published builds or server-side features. This is the only way to support the full catalog without MinMax5e itself distributing content it has no licence for. It is also one reason all compute runs on the client (ADR-0019).

## Considered Options

- A server-side library of user uploads. Rejected: it would make us a distributor.
- No import at all. Rejected: it locks the product to SRD-only content forever.

Research: [01](../research/01-data-source-and-licensing.md), [07](../research/07-tech-stack-and-architecture.md)

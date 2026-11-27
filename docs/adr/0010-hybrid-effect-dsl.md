---
status: proposed
---

# Effects are written in a small, closed data format, with named code hooks for anything it can't express

Content behaviour is data built from a fixed set of building blocks: stages (ADR-0011), predicates, operations, limits, and costs. Predicates are tests over tags such as `weapon:property:finesse`, in the style of PF2e. Where the building blocks aren't enough, the data refers to a Rust hook by id (ADR-0036). Each hook **declares** its stage, costs, and output kinds, and must be a pure function of (TurnState, event).

The solver must be able to inspect costs and triggers without running code, for enumeration, pruning, and legality checks. Fully declarative systems grow fragile ad-hoc formula languages (PF2e's documented pain points). Fully coded systems are opaque and expensive to review (XMage, Forge). Foundry PF2e, Forge/XMage, and dnd5e with midi-qol all converge on this split.

*Amended 2026-11-27 for the Rust engine:* encodings are RON data loaded into closed Rust enums (ADR-0035), and hooks are `fn` pointers with static declarations (ADR-0036). There is still no embedded scripting.

## Considered Options

- Pure DSL. Rejected: it grows an ad-hoc formula language.
- Code-as-rules. Rejected: opaque, and the solver can't inspect it.
- Arbitrary user-script hooks, like midi-qol macros. Rejected: impure, untestable, and unsafe for imported packs.

Research: [04](../research/04-rules-engine-architecture.md)

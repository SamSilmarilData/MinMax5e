---
status: proposed
---

# Hooks are pure `fn` pointers with static declarations, registered per ruleset crate

A hook is `HookDecl<R> { id, stage, costs, outputs, run: fn(&TurnView<R>, &Event, &HookArgs) -> EffectList }`, declared in its ruleset crate and resolved to an index when the pack loads. Predicates compile to bitmask tests in the Combat Kit. Plain `fn` pointers are stateless and `Send + Sync`, and the solver can inspect their declarations without running them. Dispatch stays out of the hot loop.

## Considered Options

- `Box<dyn Hook>`. Rejected: it invites hidden state.
- `enum_dispatch`. Rejected: it needs a closed list of types across crates.
- Scripted hooks. Rejected (ADR-0035).

Research: [09](../research/09-rust-engine-core.md) §3.3

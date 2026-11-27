---
status: proposed
---

# Build search is separate from rotation, and each build's rotation is solved exactly

The outer build search never chooses actions. Each build compiles to a **Combat Kit**: its attacks, riders, resources, and choices. The rotation is solved per kit by exact finite-horizon backward induction over rounds and resources, with decisions that depend on the outcome of rolls (e.g. smite after a hit or crit). Results are memoised by a hash of the kit, so the many builds that compile to the same kit are evaluated once.

In 2024 several decisions are made after the roll (Divine Smite is cast "immediately after" a hit), so the hand-written priority lists that SimulationCraft uses leave damage on the table.

*Amended 2026-11-27:* the kit hash is a compact canonical `KitKey`, memoised in a process-wide concurrent map shared by the rayon workers and persisted in the disk cache for each pack version (ADR-0038). Memoising kits matters more for speed than the choice of language.

## Considered Options

- Hand-written priority lists per class. Rejected: suboptimal, and labour-intensive for every class.
- A joint search over builds and actions. Rejected: the search space explodes.
- Monte Carlo rotation evaluation. Rejected: rankings become noisy at close margins.

Research: [05](../research/05-solver-and-optimization.md)

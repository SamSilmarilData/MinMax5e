---
status: proposed
---

# Parallel solving is deterministic

Rankings must not flicker between runs or between thread counts (ADR-0023). rayon's floating-point `sum`/`reduce` is documented as non-deterministic, so:

- parallelise only *across candidates*, never inside a single evaluation's float accumulation;
- rank top-K by a total order: quantised score, then a canonical build key;
- never sum floats in parallel;
- prune strictly (`<`, not `≤`), so ties never depend on scheduling;
- do d20 probability maths in exact integers;
- a snapshot test asserts that 1-thread and N-thread solves are identical.

## Considered Options

- Single-threaded solving. Rejected: it gives up about 6×.
- Accepting non-determinism. Rejected: rankings would flicker between runs.
- Exact rationals at runtime. Rejected: too slow (ADR-0023).

Research: [09](../research/09-rust-engine-core.md) §5

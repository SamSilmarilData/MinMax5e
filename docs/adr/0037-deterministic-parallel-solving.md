---
status: proposed
---

# Parallel solving is deterministic

> *Amended 2026-11-27 (ADR-0050):*
>
> - **Two levels.** Determinism is split into *outcome* determinism (the `Done` result: guaranteed across executors, thread counts **and targets**) and *trajectory* determinism (snapshots, progress, cancel points: not promised).
> - **Measured:** identical bits at 1–10 threads, and one result hash across native, `wasm32` and `wasm32`+`simd128` ([research 13 §4](../research/13-tape-spike.md)).
> - **Banned in scored code:** `relaxed-simd`, `mul_add`, transcendentals, and `f64::max`/`min` where NaN is possible.
> - **Budgets and CI performance gates** count work, not time.

> *Amended 2026-11-27:* there are now two executors for the same chunk plan: rayon natively, and the Web Worker pool in the browser (ADR-0045). Both merge per-chunk results in chunk order, so rankings are identical across executors and worker counts. The snapshot test asserts native 1 thread = native N threads = web 1 worker = web N workers (the web case under `wasm-bindgen-test` in a headless browser).

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

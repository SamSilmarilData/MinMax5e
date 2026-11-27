---
status: proposed
---

# Solves are cancelled cooperatively with a generation token, and results stream as immutable snapshots

Every new search increments an `AtomicU64` generation. Solver chunks of about 5–10 ms check whether their generation is stale and stop if so. Workers send `SolveEvent { gen, … }` messages carrying `Arc` snapshots over a `crossbeam-channel`, and wake the UI with `ctx.request_repaint()`. The UI discards events from stale generations.

Inputs are classified as either *lookup* (target AC: re-score the in-memory pool, ADR-0020) or *re-search* (constraints). Only re-search inputs dispatch a new solve, debounced by about 100 ms. `mm-solver` takes the token through its `Control` parameter, so the GUI, the CLI, and any fallback GUI share the same cancellation path.

Without the `parallel` feature (a wasm32 build, ADR-0042), the same chunk loop runs on one thread and yields to the UI between chunks. The generation check is unchanged.

rayon has no preemptive cancellation, and killing threads isn't possible safely in Rust. This approach is deterministic and keeps locks off the UI path.

## Considered Options

- Killing threads. Rejected: no safe API exists.
- tokio as the core runtime. Rejected: there's no async I/O to justify it.
- Shared mutable result state behind a mutex. Rejected: contention on the UI thread.

Research: [10](../research/10-rust-ui-and-app-shell.md), [09](../research/09-rust-engine-core.md) §4.3

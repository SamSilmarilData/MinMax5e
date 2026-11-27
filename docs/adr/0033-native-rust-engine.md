---
status: proposed
---

# The engine and solver are native Rust, parallelised with rayon

Everything is Rust (edition 2024, with the toolchain pinned in `rust-toolchain.toml`): the Distribution Kernel, core types, engine, solver, Rules Packs, importer, CLI, and GUI. The solver runs in parallel across candidates using rayon.

**The honest gain** ([research 09 §1](../research/09-rust-engine-core.md), [estimate] from Benchmarks Game per-core ratios, not yet measured):

- about **10–30×** faster than single-threaded TypeScript (≈20× central);
- only about **2–5×** faster than research 07's TypeScript Web Worker pool;
- about 1.5× faster than Rust compiled to WASM in a browser.

That moves each class of search in research 05 roughly one tier toward "live" (ADR-0020). The bigger wins remain the two-tier evaluation and kit memoisation, not the language.

Rust's other advantages here:

- real shared-memory threads;
- predictable performance with no JIT warm-up;
- compact packed state keys, which make memo tables about 10× smaller;
- compiler-checked exhaustive `match` over rule enums.

The cost is compile time. It is contained by keeping encodings as data loaded at runtime (ADR-0035), so editing rules doesn't trigger a recompile.

rayon is a default-on `parallel` feature, not a hard dependency, so the engine still compiles to wasm32 (ADR-0042).

Supersedes ADR-0021. The `mm-dice` crate keeps the narrow Distribution Kernel interface, with SIMD (the `wide` crate) only if profiling demands it.

## Considered Options

- TypeScript with a worker pool. Rejected: slower per core, about 15× more memo memory, and it needs a browser.
- Rust→WASM in a browser. Rejected: no browser is needed (ADR-0032).
- GPU compute. Not evaluated; the branchy dynamic programming is likely a poor fit [unverified].

Research: [09](../research/09-rust-engine-core.md)

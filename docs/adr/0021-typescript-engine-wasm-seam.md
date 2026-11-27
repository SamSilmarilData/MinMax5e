---
status: superseded by ADR-0033
---

# The engine is written in TypeScript, with the numeric kernel behind an interface Rust/WASM can replace

> **Superseded 2026-11-27 by [ADR-0033](0033-native-rust-engine.md). The Distribution Kernel seam survives as the `mm-dice` crate boundary.**

The engine, solver, and Rules Packs are written in strict TypeScript. The probability maths sits behind a narrow, typed-array **Distribution Kernel** API (`packages/dice`) that a Rust→WASM build can implement later if benchmarks demand it.

Benefits: types shared with the UI, the fastest iteration on rules encodings, and exhaustive discriminated unions for modelling rules. WASM's speed edge depends on the workload, and calls across the JS↔WASM boundary penalise rich objects, so the data that crosses it is kept to flat arrays. The rustwasm GitHub organisation was sunset in July 2025, and wasm-pack is slated for archiving, which adds toolchain risk to a full Rust core.

## Considered Options

- A full Rust core. Rejected: slower iteration, glue code to share types, toolchain risk.
- Python or Pyodide. Rejected: bundle size and search speed.
- Go. Rejected: binary size.
- Kotlin/Wasm. Rejected: still in Beta.

Research: [07](../research/07-tech-stack-and-architecture.md)

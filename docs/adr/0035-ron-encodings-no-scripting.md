---
status: proposed
---

# Encodings are RON files loaded at runtime into closed Rust enums; no macro DSL and no embedded scripting

Hand-authored encodings live in `data/rules-*/**/*.ron` and deserialize with serde into the closed Effect DSL enums of ADR-0010. They are embedded with `include_str!` in release builds and can be overridden with `--pack-dir` during development. That is safe under ADR-0028 because they are our own text. Load errors come back as `miette` diagnostics, and a `cargo test` loads every pack. Research 09 §3.5 encodes Vex, Sneak Attack, and 2024 Divine Smite entirely in RON, with no hooks.

Keeping encodings as data means rules can be edited and re-run without recompiling, packs imported at runtime use the same types, and the solver can inspect everything.

## Considered Options

- A `macro_rules!` or proc-macro DSL. Rejected: compile times, and no way to load at runtime.
- JSON as the authoring format. Rejected: no comments, and enums are clumsy.
- TOML. Rejected: poor for nested enums.
- rhai or mlua scripted hooks. Rejected: opaque to the solver, slow, and they reopen the purity concern in ADR-0010.

Research: [09](../research/09-rust-engine-core.md) §3

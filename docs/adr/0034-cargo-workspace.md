---
status: accepted
---

# A Cargo workspace with UI-agnostic core crates and thin GUI and CLI shells

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-28 (retro after #18):*
>
> - **`cargo xtask glossary`** joins the repo guards and CI. It fails when a Content ID kind, in a Content ID or a Rust `Kind::NAME`, is a term on one of GLOSSARY.md's _Avoid_ lists; research notes are not scanned, since they stay as written. #18's review spent two rounds on the kind `mastery`, which the glossary avoids, so the mechanical part of that check moves out of review.

> *Amended 2026-11-28 (retro after #16):*
>
> - **`adr-check` also checks amendment order.** Every ADR in the working tree, new or old, lists its amendment blocks newest first; a block dated after the one above it fails as `adr-amendment-order`. Every ADR already did this; now it is enforced.
> - **Every fixture Build is in the cross-target hash.** An ordinary xtask test, which needs no wasmtime and so runs on every CI leg, fails when `crates/mm-testkit/fixtures/builds/` and `xtask/xhash.toml` disagree.

> *Amended 2026-11-28 (#16):* **`xtask xhash` has joined CI,** as steps in the macOS leg of the test job rather than a job of its own: they install wasmtime, run `cargo xtask xhash`, and run its `#[ignore]`d process tests (`cargo test -p xtask --test xhash -- --ignored`). This completes the CI list in the walking-skeleton amendment below, except for the web-build checks (step 8). What is hashed is recorded in ADR-0050's amendment of the same date.

> *Amended 2026-11-27 (environment retro after #13):* **`cargo xtask adr-check`** joins the repo guards and CI. An ADR accepted at the base commit may gain dated amendment blocks and change its `status:` line; any other edit, or deleting it, fails. It enforces CLAUDE.md's rule that decisions change by amendment or a superseding ADR, never by a silent edit.

> *Amended 2026-11-27 (workspace foundation, #13):*
>
> - **`mm-core`'s third-party dependencies are limited** to serde, thiserror and blake3 (spec #12), and `mm-dice` has none. `xtask deps` enforces both for normal and build dependencies; dev-dependencies are free.
> - **`mm-testkit` may not be renamed** in a manifest, so `xtask deps` can find its uses by name. That scan ignores `//` comments. **It flags any use outside `<crate>/tests/`,** including `benches/`, `examples/` and `build.rs`, not only unit-test modules: integration tests are the one place oracles belong.
> - **cargo-deny bans do apply to workspace path crates,** dev-dependencies and `wrappers` included (checked with cargo-deny 0.22, answering research 09 §7 question 4). `deny.toml` therefore bans the rules crates and `mm-host` outside their allowed users, as the backstop to `xtask deps`. This settles the "unverified" notes in the body above and in the walking-skeleton amendment.
> - **`xtask xhash` joins CI with its own ticket (#16).** Until the engine exists there is no evaluation to hash, so CI from commit one runs every other check in the walking-skeleton amendment below.
> - **The `trunk build --release` and `dist/` checks (ADR-0043 amendment) join CI with the web build,** at build-order step 8. Until `mm-gui` and `web/` exist there is nothing to build or check.
> - **`mm-testkit` may depend on** `mm-dice`, `mm-core`, `mm-load`, `mm-engine`, `mm-solver` and both rules crates, the crates its oracles and fixtures exercise. A step that needs more (for example `mm-import` for the synthetic mini-corpus at step 6) adds it here and in `xtask deps` with an amendment.
> - **The wasm32 check skips `xtask` as well as `mm-cli`.** `xtask` is a native developer tool that spawns `git` and `cargo`, and never ships. Until the first library crate exists, CI also checks a canary crate that compiles only when `.cargo/config.toml`'s `+simd128` reaches both wasm targets.
> - **`xtask deps` is tested against seeded temporary workspaces,** with real `cargo metadata` output, rather than saved metadata files. That exercises the same code path, and it avoids tracked metadata JSON, which the leak scan would flag.
> - **`serde_json` joins `[workspace.dependencies]`,** for the leak scan's check of the JSON fixture marker. **`deny.toml` also denies wildcard versions and unknown registries or git sources,** so every dependency comes from crates.io at a pinned version.
> - **The workspace lists its members explicitly** until the first crate lands under `crates/`, because Cargo rejects a `crates/*` glob that matches nothing. `[workspace.lints.cargo]` allows `unused_workspace_dependencies` until every workspace dependency has a user.
> - **2024 encodings live in `data/rules-2024/`,** as in the layout above; the directory starts empty with a README.

> *Amended 2026-11-27 (walking-skeleton lock):*
>
> - **Ports are owned by their lowest consumer.** `Storage` lives in `mm-cache`, `Fetcher` in `mm-import`, and `Clock` in `mm-app-core`. `mm-host` implements all three. Defining them all in `mm-app-core` made a cycle, because `mm-cache` and `mm-import` sit below it.
> - **`Inline` lives in `mm-solver`** (`run_inline`): it has no threads, so ADR-0050 allows it, and solver tests need no higher crate. `Rayon` stays in `mm-app-core` behind the `parallel` feature, and `WorkerPool` stays in `mm-host`.
> - **`Pack<R>` lives in `mm-load`,** so it can be built only by `load_pack`, and `mm-engine` depends on `mm-load`. `mm-core` does not depend on `mm-dice`: the DSL holds dice as data. `Sources` (named texts) lives in `mm-core`, so a rules crate can return its embedded encodings without depending on the loader.
> - **`mm-engine` exposes `pub mod turn`** (`TurnState`, `legal_steps`, `apply`, `Illegal`) beside `compile`/`evaluate`/`validate`, so the oracles and `validate` share the ADR-0012 authority.
> - **`mm-import` emits resolved records** (our own serde types in `mm-core`), and `mm-load` turns records plus RON encodings into a `Pack<R>`.
> - **Crates are added when a build-order step needs them,** not as empty stubs. Day one has `mm-dice`, `mm-core`, `mm-load`, `mm-engine`, `mm-rules-2024`, `mm-testkit`, `mm-cli` and `xtask`.
> - **`mm-testkit` defines a `TestRules` ruleset** with a different economy, so engine genericity (ADR-0008) is tested before `mm-rules-2014` exists.
> - **Oracles are used only from integration tests** (`crates/*/tests/`), never from unit-test modules, which avoids a dev-dependency cycle with two copies of engine types. `xtask deps` enforces this.
> - **`xtask deps` is the authority** for the dependency rules, checked against `cargo metadata`. cargo-deny bans are a backstop, since it is unverified whether they apply to path crates.
> - **Conventions:** toolchain pinned to 1.100.0 with targets `wasm32-unknown-unknown` and `wasm32-wasip1`; edition 2024, `rust-version = "1.100"`, resolver 3; `[workspace.dependencies]` and `[workspace.lints]` as in research 09 §6. ADR-0050's float bans are `clippy.toml` `disallowed-methods`; `+simd128` is set for wasm in `.cargo/config.toml`; `xtask` fails on `relaxed-simd`.
> - **CI from commit one:** fmt; clippy `-D warnings`; tests; the wasm32 check; `xtask deps`; `xtask leak-scan`; `cargo deny`; and `xtask xhash`, the cross-target result hash (native vs `wasm32-wasip1` under wasmtime). `cargo-mutants` and wall-clock benchmarks stay out of per-commit CI.

> *Amended 2026-11-27 (ADR-0047–0052):*
>
> - **New crate `mm-host`:** native and web adapters for `Storage`, `Fetcher` and `Clock`, plus the WorkerPool executor. Only binaries depend on it.
> - **New `xtask/`:** the dependency-graph check, the `dist/` check, the leak scan and the cross-target result hash.
> - **New `spikes/`:** standalone measurement code, not workspace members.
> - **`mm-core`** gains `ResourceDecl` and `Fp`.
> - **`mm-engine`** gains the `trace`, `reduce`, `algebra` and `script` modules.
> - **`mm-solver`** gains `factor`, `join`, `plan` and `job`.
> - **`mm-app-core`** owns the protocol (ADR-0051) and the `Inline` and `Rayon` executors.

> *Amended 2026-11-27 (ADR-0043, ADR-0045):*
>
> - `mm-gui` builds for both native and `wasm32`.
> - New crate **`mm-worker`**: the browser's solver worker binary. It is a thin shell over `mm-app-core`, `mm-solver`, and `mm-import`, with no UI dependencies, built by Trunk with `data-type="worker"`.
> - New folder **`web/`**: `index.html`, the manifest, icons, the service worker, `_headers`, the invite-gate Worker script (JavaScript, ADR-0044), and `wrangler.jsonc`.
> - The rule that every crate except `mm-gui`'s native backend and `mm-cli` builds for wasm32 is unchanged. CI now also runs a full `trunk build --release` and the `dist/` content check (ADR-0028).

```
crates/
  mm-dice        Distribution Kernel (no deps)
  mm-core        IDs, Ruleset trait, Effect DSL enums, HookDecl, Build/Choice/Encounter/Ruling
  mm-load        packs + RON encodings → Pack<R>, coverage report
  mm-import      library: fetch + verify corpus, resolve → compiled packs in the local cache (ADR-0029, ADR-0041)
  mm-engine      compile → CombatKit, TurnState, legal_steps/apply, evaluator, rotation DP
  mm-solver      enumeration, dominance, B&B, level-graph DP, Screen→Exact, top-K/MMR
  mm-cache       on-disk result cache (ADR-0038)
  mm-rules-2024  Rules2024: impl Ruleset + hooks   (depends on mm-core ONLY)
  mm-rules-2014  Rules2014: impl Ruleset + hooks   (depends on mm-core ONLY)
  mm-app-core    Session, cancellation, view models (no UI deps)
  mm-gui         egui shell (ADR-0039)
  mm-cli         solve | fetch | import | validate-packs | golden | bench
  mm-testkit     Monte Carlo + BigRational oracles, proptest strategies, synthetic fixtures
data/rules-2024/**.ron, data/rules-2014/**.ron   hand-authored encodings (our own words only)
```

The engine is generic over `R: Ruleset`, with typed `Id<R, K>` IDs, so a cross-edition reference is a compile error in a hook or a load error in data. Dependency rules:

- the engine, solver, loader, and cache never depend on an `mm-rules-*` crate;
- the two rules crates never depend on each other;
- only the binaries and `mm-testkit` may depend on rules crates.

These rules are enforced three ways: the crate graph itself, `cargo-deny` bans with `wrappers` (whether those apply to workspace path crates is unverified), and a test that checks `cargo metadata`. The GUI and CLI are thin shells over `mm-app-core`, so golden and batch runs use exactly the GUI's code path, and swapping GUI frameworks touches only `mm-gui`. The CLI ships from day one; a ratatui TUI is optional and post-MVP.

Every crate except `mm-gui`'s native backend and `mm-cli` must build for `wasm32-unknown-unknown`, and CI checks it. Filesystem and network access live only behind the `Storage` and `Fetcher` traits in `mm-app-core` (ADR-0042).

Supersedes ADR-0022 and implements ADR-0008's isolation.

## Considered Options

- `dyn Ruleset`. Rejected: loses the typed IDs.
- Edition flags inside one crate. Rejected: this is Foundry dnd5e's `isLegacy` anti-pattern.
- One big crate. Rejected: slow incremental builds, and UI types leak into the engine.
- Separate repositories. Rejected: the shared engine would drift.

Research: [09](../research/09-rust-engine-core.md) §4, [10](../research/10-rust-ui-and-app-shell.md)

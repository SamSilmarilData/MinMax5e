<div align="center">

# MinMax5e

**A build solver for 5th-edition characters. It searches the space of legal builds and ranks them by exact, probability-grounded damage, not white-room guesses.**

![status: walking skeleton](https://img.shields.io/badge/status-walking%20skeleton-orange)
![language: Rust](https://img.shields.io/badge/language-Rust-b7410e?logo=rust)
![UI: egui](https://img.shields.io/badge/UI-egui-4b8bbe)
![runs in: browser + native](https://img.shields.io/badge/runs%20in-browser%20%2B%20native-0a7bbb)
![rules: 2024 first](https://img.shields.io/badge/rules-2024%20first-6f42c1)
[![license: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

</div>

---

Character builders record the choices you make. Optimization guides rate options under fixed assumptions. **MinMax5e solves instead.** You set the constraints (level, classes, multiclassing, target AC, how many rounds a fight lasts) and it searches every legal combination of species, background, feats, class levels, and weapons. It returns the strongest builds, with the exact turn-by-turn play that produces their numbers.

## What it will do

- **Solve, not template.** A real search over legal builds, with level orderings, feat picks, and Weapon Mastery loadouts, to surface combinations a hand-written guide would miss.
- **Exact probability.** Damage is computed as exact distributions, not averages or Monte Carlo noise: hit and crit chances, advantage, Graze floors, once-per-turn riders, and resource spending across rounds.
- **Reliability you can read.** Time-to-kill and kill chance sit alongside mean DPR, so a steady 45 DPR build and a swingy 45 DPR build look as different as they play.
- **2024 rules first.** Weapon Mastery chains (Vex into Nick), Divine Smite as a Bonus Action spell, the one-slot-per-turn rule, and per-attack weapon swaps are all enforced by a turn state machine. The 2014 rules run in a separate, isolated ruleset.
- **Assumptions out in the open.** Disputed rules are named, versioned rulings you can toggle, and every result shows which ones it used.

### The dashboard (planned)

- **Top 3 builds**, chosen to be meaningfully different rather than near-copies of the winner.
- **AC sensitivity curve** from AC 10 to 25, marking where one build overtakes another.
- **Reliability band**: p10–p90 damage, the floor, and the chance to kill within *N* rounds.
- **Damage source breakdown**: weapon, mastery, feat, and class or spell riders.
- **Combat script**: setup, round 1 opener, and the sustained loop, with every Action, Bonus Action, and weapon draw spelled out.

## Status

**Building the walking skeleton:** build-order steps 1–3 of [`docs/architecture.md`](docs/architecture.md) §11, specified in [#12](https://github.com/SamSilmarilData/MinMax5e/issues/12). There is no solver or UI yet. What works today:

- the Cargo workspace, its CI and its repo guards ([#13](https://github.com/SamSilmarilData/MinMax5e/issues/13));
- `mm-dice`, exact damage distributions checked against a `BigRational` oracle ([#14](https://github.com/SamSilmarilData/MinMax5e/issues/14));
- `mm-cli evaluate`, which gives exact Nova DPR and its SD at every AC from 10 to 25 for the simplest synthetic Build ([#15](https://github.com/SamSilmarilData/MinMax5e/issues/15));
- the cross-target result hash, which shows that evaluation gives the same bits natively and in WebAssembly ([#16](https://github.com/SamSilmarilData/MinMax5e/issues/16)).

Try it on the synthetic fixtures:

```sh
cargo run -p mm-cli -- evaluate crates/mm-testkit/fixtures/builds/warrior-1.ron \
  --ac 16 --pack-dir crates/mm-testkit/fixtures/pack
```

Next come the rest of the skeleton's tickets (#17–#27): diagnostics, the full level-5 warrior, distributions, Assumption Sets, Rulings, resources, Sustained mode, Turn Plans and the independent oracles. Then the Tape (step 4) and the solver (step 5).

The design is written up:

| | |
|---|---|
| [`docs/architecture.md`](docs/architecture.md) | The map: the pipeline, the crates, and the build order |
| [`docs/adr/`](docs/adr/) | 57 Architecture Decision Records: 26 accepted, 23 proposed (each to be accepted at the build-order step that needs it), and 8 superseded or deprecated, kept for history |
| [`docs/research/`](docs/research/README.md) | Fifteen research notes: rules mechanics, probability maths, solver design, rules-engine prior art, the Rust stack, data ingestion and web delivery, plus three with measurements (the tape spike, prior art, and the problems ahead) |
| [`GLOSSARY.md`](GLOSSARY.md) | Project vocabulary |

### Working on the code

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.100.0, with the `wasm32-unknown-unknown` and `wasm32-wasip1` targets); rustup installs it on first use.

Enable the pre-commit hook once per clone. It runs the leak scan before every commit:

```sh
git config core.hooksPath .githooks
```

The repo guards, which CI also runs on every push:

| Command | Checks |
|---|---|
| `cargo xtask deps` | The crate dependency rules of [ADR-0034](docs/adr/0034-cargo-workspace.md), against `cargo metadata` |
| `cargo xtask leak-scan` | No upstream rules data in tracked files ([ADR-0028](docs/adr/0028-no-upstream-bytes-in-public-repo.md)). Exemptions, each with a reason, live in `xtask/leak-allow.toml` |
| `cargo xtask adr-check` | Accepted ADRs change only by dated amendment, compared with `main` |
| `cargo xtask xhash` | Evaluation gives the same bits natively and in WebAssembly ([ADR-0050](docs/adr/0050-solve-as-deterministic-job-plan.md)). Needs [wasmtime](https://wasmtime.dev) on PATH; CI runs it on macOS. The hashed fixture Builds are listed in `xtask/xhash.toml`, which says how to add one |
| `cargo deny check` | Advisories, licences and banned crates (needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)) |

`cargo test --workspace` runs the tests. The `xhash` process tests build two targets and need wasmtime, so they are ignored by default; run them with `cargo test -p xtask --test xhash -- --ignored`.

Snapshot tests use [insta](https://insta.rs). After a deliberate change in output, `INSTA_UPDATE=always cargo test` rewrites the snapshots in place. Review them with `git diff`; `cargo-insta` isn't needed.

### Crates

| Crate | Role |
|---|---|
| `mm-dice` | The distribution kernel: exact damage distributions, moments, d20 outcomes and AC lanes |
| `mm-core` | The shared vocabulary: Rulesets, Content IDs, the Effect DSL, records, Builds and fingerprints |
| `mm-load` | Loads records and RON encodings into a Rules Pack for one Ruleset |
| `mm-engine` | Compiles a Build into a Combat Kit and evaluates it exactly over AC lanes |
| `mm-rules-2024` | The 2024 Ruleset: its economy and its embedded encodings |
| `mm-cli` | The command-line shell |
| `mm-testkit` | Test oracles and synthetic fixtures, used only from integration tests |
| `xtask` | The repo guards and the cross-target hash |

The rest of the crate map (`mm-solver`, `mm-import`, `mm-app-core`, `mm-gui`, `mm-worker` and others) arrives with the build-order step that needs it.

### Planned architecture

```
mm-gui (egui) ─┐                         ┌─ mm-rules-2024
mm-cli ────────┴─► mm-app-core ─► mm-solver ─► mm-engine ─► mm-core ◄─┤
                     │                          │                     └─ mm-rules-2014
                     └─► mm-import ─► local cache          mm-dice (exact distributions)
```

One Rust codebase, two ways to run it:

- **In the browser:** the egui app compiled to WebAssembly, with the solver on a pool of Web Workers. Friends open a personal invite link; there's nothing to install.
- **Native:** the same app with the solver on all cores (rayon).

The UI is egui with egui_plot charts. Everything is computed on your own device; there's no server-side solving.

## Rules data

This repository, and the web app once it is deployed, contain **no rules content**. The tests use invented, synthetic fixtures. On first launch, the app will download a pinned version of the community 5e.tools data from its public mirror and build its rule packs on your device. Mechanics are encoded by hand, in our own words. Quoted rules text comes only from the System Reference Documents, under CC-BY-4.0.

## License

[MIT](LICENSE) © 2026 Samyak Meshram

MinMax5e is unofficial fan software, compatible with fifth edition. It is not affiliated with, endorsed by, or sponsored by Wizards of the Coast. Dungeons & Dragons and D&D are trademarks of Wizards of the Coast LLC. Portions of the research notes include material from the System Reference Documents 5.1 and 5.2.1 by Wizards of the Coast LLC, licensed under [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/legalcode). See the [attribution](docs/research/README.md#attribution-and-quoting-policy).

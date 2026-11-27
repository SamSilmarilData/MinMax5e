<div align="center">

# MinMax5e

**A build solver for 5th-edition characters. It searches the space of legal builds and ranks them by exact, probability-grounded damage, not white-room guesses.**

![status: design phase](https://img.shields.io/badge/status-design%20phase-orange)
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

**Design phase.** There is no code yet. The research and the architecture decisions are written up:

| | |
|---|---|
| [`docs/research/`](docs/research/README.md) | Twelve research notes: rules mechanics, probability maths, solver design, rules-engine prior art, the Rust stack, data ingestion, web delivery, and open access |
| [`docs/adr/`](docs/adr/) | 46 Architecture Decision Records (38 proposed; superseded and deprecated ones kept for history) |
| [`GLOSSARY.md`](GLOSSARY.md) | Project vocabulary |

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

This repository, and the deployed web app, contain **no rules content**. On first launch, the app downloads a pinned version of the community 5e.tools data from its public mirror and builds its rule packs on your device. Mechanics are encoded by hand, in our own words. Quoted rules text comes only from the System Reference Documents, under CC-BY-4.0.

## License

[MIT](LICENSE) © 2026 Samyak Meshram

MinMax5e is unofficial fan software, compatible with fifth edition. It is not affiliated with, endorsed by, or sponsored by Wizards of the Coast. Dungeons & Dragons and D&D are trademarks of Wizards of the Coast LLC. Portions of the research notes include material from the System Reference Documents 5.1 and 5.2.1 by Wizards of the Coast LLC, licensed under [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/legalcode). See the [attribution](docs/research/README.md#attribution-and-quoting-policy).

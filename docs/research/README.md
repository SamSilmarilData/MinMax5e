# Research

Research into the topics in the [project spec](../spec.md), done November 2026. Each note cites primary sources, marks what it could not verify, and ends with candidate ADRs and glossary terms. The candidates were merged into the proposed ADRs in [`docs/adr/`](../adr/) and the root [`GLOSSARY.md`](../../GLOSSARY.md).

| # | Note | Headline finding | ADRs |
|---|---|---|---|
| 01 | [Data source & licensing](01-data-source-and-licensing.md) | The 5e.tools corpus is legally risky (WotC DMCA, Aug 2024) and its feature text is prose, not mechanics. SRD 5.2.1 (CC-BY-4.0) is the only licensed 2024 source, and it lacks GWM, Sharpshooter, Dual Wielder and all but one subclass per class. | 0001–0003, 0008 |
| 02 | [2024 rules mechanics](02-rules-mechanics-2024.md) | Exact text of the masteries, swap rules and smites; a 32-claim spec fact-check; 28 edge cases with ruling options. | 0006, 0007, 0009, 0012 |
| 03 | [Probability & damage math](03-probability-and-damage-math.md) | Exact damage distributions are cheap (≈0.04 ms/round) and beat Monte Carlo. Worked GWM Fighter example. Monster AC and HP by CR fitted from SRD 5.2.1. | 0013, 0017, 0023–0025 |
| 04 | [Rules-engine architecture](04-rules-engine-architecture.md) | Survey of Foundry PF2e/dnd5e, Avrae, MTG engines and others. A hybrid data/code encoding, fixed resolution stages, and one turn state machine for both generation and validation. | 0002, 0008, 0010–0014 |
| 05 | [Solver & optimization](05-solver-and-optimization.md) | About 10^35 raw builds, about 5×10^7 after canonicalising for single-class level 8. Build search is separate from per-build rotation; dynamic programming over the level graph; MMR for top-3 diversity. | 0015–0018, 0020 |
| 06 | [Market, users & UX](06-market-users-and-ux.md) | No 2024 build *solver* exists; users want visible, editable assumptions. Chart recommendations per deliverable. Fan Content Policy and trademark limits. | 0004, 0005, 0006 |
| 07 | [Tech stack & architecture](07-tech-stack-and-architecture.md) | Static site with compute in Web Workers, a TypeScript engine with a seam for a WASM kernel, a pnpm monorepo, two-tier Screen/Refine evaluation, and a spike list. *Largely superseded by 09–10.* | 0019–0022 |

### Revision of 2026-11-27: personal use, 5e.tools data, full Rust

The owner decided MinMax5e is a personal tool (ADR-0026), sourced from 5e.tools (ADR-0027) and written fully in Rust (ADR-0033).

| # | Note | Headline finding | ADRs |
|---|---|---|---|
| 08 | [5e.tools ingestion for personal use](08-5etools-ingestion-personal-use.md) | All data for both editions is in `5etools-src/data` at a weekly tagged commit; the MVP needs about 13 MB. Every takedown notice targeted *publicly hosted* copies, so the public repo must hold no upstream data in any form. Structured fields cut hand-written encodings to about 110–150. `edition` is too sparse to decide the ruleset, so a source allowlist does. | 0026–0031 |
| 09 | [Rust engine core](09-rust-engine-core.md) | Native Rust with rayon is about 20× faster than single-threaded TypeScript, but only about 2–5× faster than a worker pool [estimate]. That moves most searches to live. RON encodings with closed enums, `fn` hooks, a deterministic parallel solver, a postcard cache; bincode is unmaintained. | 0033–0038 |
| 10 | [Rust UI & app shell](10-rust-ui-and-app-shell.md) | Native egui/eframe with egui_plot covers every required chart; Dioxus desktop is the fallback. A rayon pool, crossbeam snapshots, and generation-token cancellation. | 0032, 0034, 0039, 0040 |
| 11 | [Web-first delivery](11-web-first-delivery.md) | Friends open a personal invite link. The same egui app, built for WASM (eframe 0.36 fixed mobile input), is served from Cloudflare Workers on a free `workers.dev` URL. The solver runs on a stable-Rust Web Worker pool. Rules data is 1.26 MB brotli from jsDelivr, pinned by commit SHA, cached in OPFS, and rebuilt after eviction. Includes a verification log of every claim. | 0043–0045 |
| 12 | [Open access for anyone](12-open-access-for-anyone.md) | Any device works: the browser floor is Safari 15.2, Chrome 102, and Firefox 111, with SIMD and baseline modules. In-app browsers isolate cookies, so the gate also needs a typed invite code. "Anyone, no legal issues" needs a split: a public **Open edition** (SRD 5.2.1 plus our own-words mechanics, no corpus fetch) and an invite-only **Extended edition** (5e.tools corpus). | 0044 (typed invite codes), 0046 |

### Measurements, 2026-11-27

| # | Note | Headline finding | ADRs |
|---|---|---|---|
| 13 | [Tape spike](13-tape-spike.md) | First measured numbers (Rust 1.100, M5). Results:<ul><li>A traced tape of the rotation MDP is 3.4–6.2× faster than hash-map DP and bit-identical.</li><li>Clamping resources to what can still be spent is exact and cuts states up to 21×, so a 12-round adventuring day costs 0.34 ms per kit.</li><li>Lagrangian day pricing is slower than exact and keeps a 1–3% gap.</li><li>Results are bit-identical across 1–10 threads and native/WASM.</li><li>WASM costs 1.0–1.5× native per core.</li></ul>Code in `spikes/tape-spike/`. | proposed in [`architecture.md`](../architecture.md) §10 |
| 14 | [Prior art and refinements](14-prior-art-and-refinements.md) | How others solve each hard problem: Darwiche's arithmetic circuits, BDD unique tables, MDP minimisation, salsa, skyline, any-k, VIPER, Rapier/Gurobi determinism, and existing 2024 tools (all evaluate; none search). Measured findings:<ul><li>BDD-style reduction removes another 1.8–2.4× of nodes, bit-identical.</li><li>One occupancy pass gives exact attribution, exact sensitivities, and a bit-exact "+1 to-hit = one AC lane".</li><li>Simple smite rules lose 2.3–35% against optimal play.</li></ul> | architecture.md rev 3, §10 |
| 15 | [Problems ahead and methods](15-problems-ahead-and-methods.md) | The problems real content will raise, and the method the literature supports for each:<ul><li>oracle-free correctness: metamorphic and mutation testing (6 exact relations, about 57,000 checks, 0 violations, measured);</li><li>three-class curves: beam-stack search with a certified gap;</li><li>assumption-sensitive rankings: a minimax-regret lens;</li><li>trustworthy explanations: contrastive (Miller);</li><li>playable rotations: size-limited rule policies (OMDT);</li><li>noisy benchmarks: deterministic work counters.</li></ul> | 0047–0057 |

## Where the research contradicts the spec

1. **Data source.** The spec's 5e.tools corpus is now used for personal use (ADR-0027), but its prose still can't be parsed into triggers (ADR-0002), and none of it may be committed to the public repo (ADR-0028).
3. **Rules facts:**
   - "Vex Dagger" is wrong: the Dagger's mastery is Nick, and the light Vex weapons are the Shortsword and Handaxe.
   - Spirit Shroud (Tasha's) and Booming Blade are 2014-era content, which clashes with strict ruleset isolation (ADR-0009).
   - Hex is a Warlock spell, reachable in the MVP roster only through a feat or a multiclass.
   - The 2024 Monk has no Weapon Mastery.
   - The "advantage loop" description of Vex, the "guaranteed floor" description of Graze, and "free-action smite on crits" all overstate the rules.

   The full list is in [02 §7](02-rules-mechanics-2024.md).
4. **Variance in ranking.** The solver optimises expected value; variance re-ranks finalists and is always shown, but doesn't steer the search (ADR-0017). "Reliability" is headlined as time-to-kill and kill probability rather than SD (ADR-0025).

## Errata

Notes are left as written; corrections found later are listed here. A note's table may sample its source: when code needs every value, read the cited source, and test against the note's rows.

- **03 §7.2, AC sensitivity, AC 25** (2026-11-27): the base round's mean is 12.646 (12.6), not 12.7. By hand, +7 needs an 18 (miss 0.85, hit 0.10, crit 0.05): 2·6.05 + 0.0975·5.6 = 12.646. The note's own "Graze worth 7.1 at AC 25" agrees. Found by `crates/mm-dice/tests/golden.rs` (#14).
- **03 §5.3, default AC against the SRD fit** (2026-11-28): Dunn's table, the default AC, lies within about ±1.0 of our SRD 5.2.1 fit (12.90 + 0.374·CR) over levels 1–20, not ±0.6. By the note's own table, level 3 is 15 against 14.0; at CR 19 Dunn gives 19 against 20.0. The default stays Dunn's table, as ADR-0024 says. Found while encoding the Encounter Baseline in `crates/mm-core/src/assumptions.rs` (#20).
- **09 §5, determinism rule 8** (2026-11-28): the reason is incomplete. Hash-map iteration order differs between native and WebAssembly even on one thread with a fixed insertion order, because FxHash hashes in `usize`, which is 64 bits natively and 32 bits on wasm32. Scored code keeps anything it iterates in a `Vec` (a map may index into it), and `clippy.toml` bans `HashMap`'s iterating methods. Found while writing the forward pass's `Frontier` (#19).

## Blocking open questions

1. **Repo visibility.** The GitHub repo is public. Making it private removes most of the remaining takedown risk (ADR-0028). Either way, the audit for verbatim non-SRD rules text in `docs/research/` was done on 2026-11-27: non-SRD quotes were paraphrased and SRD quotes attributed (see below). (08 §8) *(The original legal question about shipping non-SRD content only matters if the project ever goes public; see ADR-0026.)*
2. **Scope of "2024 only".** Is it PHB 2024/SRD only, or PHB plus older content that wasn't reprinted (ADR-0009)? And which subclasses are in the MVP? (02)
3. **Adventuring Day Model defaults.** The 2024 DMG dropped "6–8 encounters per day", so Sustained DPR needs an explicit default for short rests and fights per rest. (02, 03, 05)
4. **Performance spikes.** No Rust toolchain was installed during research, so every speed figure is an estimate. Run 09's spike S1 (PMF kernel and Screen throughput) and S2 (rotation DP cost per kit), and 10's S1/S2 (egui chart fidelity and frame rate while a solve streams), before building on ADR-0020. For the web build, run 11's W1–W4 (WASM size, worker throughput, boot time, and the invite gate on real devices) before building on ADR-0043–0045. (09, 10, 11)

Each note has its own full open-questions list.

## Attribution and quoting policy

Rules text quoted in these notes comes only from the SRDs, under CC-BY-4.0:

> This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.

> This work includes material taken from the System Reference Document 5.1 ("SRD 5.1") by Wizards of the Coast LLC, available at https://dnd.wizards.com/resources/systems-reference-document. The SRD 5.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.

Non-SRD material (2024 PHB feats such as Great Weapon Master and Dual Wielder, 2014 PHB feats, and supplement content) is **paraphrased, never quoted** (ADR-0028). Quotes in notes 02 and 04 that cite a 5e.tools file for a rule that is also in the SRD (e.g. the one-slot rule, Vex, Nick, Advantage) quote the SRD wording.

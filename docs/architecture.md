# MinMax5e architecture

Status: proposal, revision 4 (2026-11-27). This note is the map. The decisions themselves live in **ADR-0047 to ADR-0057** (proposed), and in dated amendments to ADR-0006, 0015, 0017, 0018, 0020, 0034, 0037, 0038 and 0045. The evidence is in research [13](research/13-tape-spike.md) (spike measurements), [14](research/14-prior-art-and-refinements.md) (prior art) and [15](research/15-problems-ahead-and-methods.md) (problems ahead and the methods the literature supports). The code is in `spikes/tape-spike/`. Nothing here overrides an ADR. Where the note and an ADR disagree, the ADR wins.

Conventions follow `docs/research/`: **[measured]** cites research 13–15, **[design]** is a proposal, **[estimate]** is arithmetic a spike must confirm. Domain terms are used as `GLOSSARY.md` defines them; the engine terms (Kit Shape, Tape, Occupancy, …) were added to it in this revision.

**Changes in revision 4:**

- Decisions moved into ADRs 0047–0057, with amendments to existing ADRs; §10 maps them.
- **Problems ahead** (§12), each with the method research 15 supports:
  - metamorphic and mutation testing [measured: 6 exact relations, ~57k checks, 0 violations];
  - beam-stack search with a certified gap for three-class curves;
  - a minimax-regret **Robust** lens;
  - **contrastive** #1-vs-#2 explanations;
  - exhaustively enumerated Script rules (OMDT's guarantee within the grammar);
  - deterministic work counters as CI performance gates.

**Changes in revision 3:**

- **BDD-style tape reduction** added: 1.8–2.4× fewer nodes on top of canonicalisation, bit-identical [measured].
- **Occupancy algebra** added (Darwiche's "evaluate up, differentiate down"). It gives exact attribution, exact stat-weight sensitivities, expected resource use, and exact evaluation of any fixed policy [measured], so Screen no longer carries per-source sums.
- **The Combat Script is a fitted rule plus its exact DPR cost**, because simple rules lose 2.3–35% [measured].
- **Determinism hardened:** ban relaxed SIMD and `mul_add`; outcome vs trajectory determinism made explicit (as in Gurobi).
- **Ruling-aware fingerprints** (salsa-style dependency recording), so "what does flipping this Ruling change" is cheap.
- **Skyline presort,** an **any-k fallback**, and a note on exact k = 3 diversity.

Revision 2 had already added resource canonicalisation, demoted kit batching, ruled out Lagrangian pricing, fixed multi-root horizons, and confirmed cross-target bit identity.

---

## TL;DR

The ADRs fix the stack: Rust everywhere, egui for native and web, RON encodings, exact maths, deterministic parallel solving, and invite-gated web delivery. This note fixes **how the hot path runs**, which decides whether "live" is achievable. Four moves, each with its evidence:

1. **Trace each Kit Shape's rotation MDP once into a flat Tape, then read it in several algebras** (ADR-0047). A Combat Kit splits into a *Kit Shape* (structure) and *Kit Params* (numbers). The `legal_steps`/`apply` state machine (ADR-0012) runs once per shape and records a post-order DAG with liveness-based value-slot reuse. Screen moments, exact PMFs, attribution and the test oracles all read that tape. **[measured] 3.4–6.2× faster than per-kit hash-map DP, bit-identical, with 2–16% of nodes live at once. Policy-aware E and SD agree with an independent Monte Carlo within 2σ and 0.2%.**
2. **Clamp resource state to what can still be spent** (ADR-0048). With R turns left and a per-turn spending cap, only the top R slots can matter. The tracer canonicalises at turn boundaries, so states that differ only in unspendable resources merge. **[measured] Exact (bit-identical), up to 21× fewer nodes. The 12-round default adventuring day drops from 4.2 ms to 0.34 ms per kit.** This defuses the risk research 09 ranked largest (spike S2).
3. **Push projections down the enumeration** (ADR-0049). Build enumeration is treated as a relational join of factors (species, origin, abilities, class skeleton, feats, loadout). Each factor is projected onto its *kit contribution*, deduplicated and skyline-pruned **before** the next join. Equal kits are merged when they are constructed, so the web build (no shared memory, ADR-0045) needs no shared memo. It also lets the UI say which choices are DPR-neutral.
4. **The Solve is data** (ADR-0050). `mm-solver` emits a deterministic plan of serialisable Jobs plus an ordered merge. Three interchangeable executors run it: inline, rayon, and the Web Worker pool. The B&B incumbent is a monotone hint. **[measured] Bit-identical at 1–10 threads and across native, WASM and WASM+SIMD128 (one result hash). 4.3–5.5× on the M5's 10 cores. WASM costs 1.0–1.5× native per core.**

Three consequences:

- **Search narrow, pool wide:** only survivors are scored over the full encounter grid, so almost every slider is a lookup.
- **One downward pass explains a result.** The occupancy pass gives exact attribution, exact "what is +1 worth" sensitivities (SimulationCraft gets the same from noisy re-simulation), expected resource use, and the exact cost of any simplified play rule. [measured]
- **The Combat Script tells the truth about simplification.** It shows a short rule together with how much DPR that rule gives up against optimal play. [measured: "smite crits only" loses 17–35% in the synthetic model]

Every move has prior art: arithmetic circuits (Darwiche), ROBDD unique tables (Bryant), MDP minimisation (Givan, Dean & Greig), salsa's red-green caching, skyline queries, VIPER policy extraction, and Rapier's deterministic WASM. [Research 14](research/14-prior-art-and-refinements.md) maps each one to the design.

---

## 1. The pipeline is a compiler

Each stage is a pure function from a fingerprinted input to a fingerprinted output. Caches sit at the stage boundaries.

```
 upstream.lock.toml ─┐
 5e.tools corpus  ───┼─ mm-import ─► Pack<R>              frozen arena, dense Id<R,K>, predicates → bitmasks
 data/rules-*/*.ron ─┘                 │  PackFp = H(lock, encodings, importer version)
                                       ▼
 Query (constraints + Assumption Set) ─ mm-solver::factor ─► Factor tables   projected, deduped, skyline-pruned
                                       │
                         mm-solver::join ─► Kit lattice     unique (ShapeKey, Params) + back-pointers to Builds
                                       │
                         mm-engine::trace ─► Tape            once per ShapeKey: canonicalise → trace → reduce → slots
                                       │
            ┌──────────────────────────┼──────────────────────────────────┐
            ▼                          ▼                                  ▼
   Screen (Moments algebra)    Explain (finalists only)           Oracles (testkit only)
   reduced Tape × Params       Policy → Occupancy: attribution,   Rational, Monte Carlo,
   × 16 AC lanes → E, E²       sensitivities, Script rule + cost; plain (uncanonicalised,
                               Pmf: distributions, TTK            unreduced) tape
            │                          │
            └────────► Pool ─► top-K per grid point ─► Signature collapse ─► MMR ─► view models ─► UI
```

**Fingerprints record what was read, not everything that was available** [design, after salsa]. While compiling a kit and tracing its shape, the engine records which Rulings it actually consulted. `ShapeKey` and `ParamsFp` hash only those Rulings, which is salsa's dependency recording ([research 14](research/14-prior-art-and-refinements.md)). Two consequences:

- Flipping a Ruling invalidates only the kits that read it. ADR-0006's "show how much DPR flipping this Ruling would change" re-evaluates only the pool entries that depend on it.
- If recompiling yields the same fingerprint, everything downstream is reused (salsa's *backdating*). Pack-derived inputs are high-durability; Query inputs are low-durability.

A hand-rolled fingerprint chain is enough. The `salsa` crate isn't needed, because the pipeline is a fixed DAG of stages rather than an open query graph.

The fingerprint is one 128-bit type, `Fp` (blake3, truncated) [design]. These keys are all `Fp`s or tuples of them: `PackFp`, `RulingsFp`, `ShapeKey`, `KitKey = (ShapeKey, ParamsFp)`, `QueryFp`, and the disk-cache key (ADR-0038). The evaluator is bit-identical across native and WASM [measured], so **the same fingerprints give the same numbers on the owner's Mac and in a friend's browser**. A Share Link (a serialised `Query`) therefore reproduces exact results on the recipient's own data.

---

## 2. Move 1: Kit Shape, Kit Params, and the Tape

### 2.1 The split [design]

| Goes into **Kit Shape** (changes the state graph) | Goes into **Kit Params** (numbers only) |
|---|---|
| attack sources and counts (Attack action, Extra Attack, Light, Nick, Hew, Bonus Action features) | to-hit bonus per attack row, crit threshold |
| for each attack row, which Riders can apply, with their limits, costs and `optional` flag | damage-term moments per outcome: E and E² on hit, on crit (dice doubled), Graze value |
| resource kinds, their maxima, and their **spend caps** (§3) | save DCs; resistance variants of each damage term |
| weapons in hand, legal swaps, mastery property per weapon | enhancement (+1/+2/+3), flat fighting-style bonuses |
| concentration options, whether pre-combat setup is allowed, the set of horizons | advantage baseline from the Encounter Profile |
| the active Ruling set (whatever changes legality or branching) | |

Rule of thumb: if changing it can change *which states exist or which choices are legal*, it belongs to the shape. Otherwise it is a param.

### 2.2 Tracing [design, measured]

`trace(&KitShape) -> Tape` runs the existing state machine (the TurnState packed into a `u64`, `legal_steps`, `apply`, outcome branching, state merging) over rounds × resources, and records the result:

```rust
/// The rotation MDP of one Kit Shape, recorded once. Nodes are in post-order: children before parents.
pub struct Tape {
    pub nodes: Box<[Node]>,
    pub edges: Box<[Edge]>,
    pub terms: Box<[TermRef]>,   // damage terms read from Params, each tagged with its attribution Source
    pub slot_of: Box<[u32]>,     // liveness-based value-slot assignment (register allocation)
    pub live_slots: u32,         // peak live values: 2–16% of nodes [measured]
    pub roots: Box<[(u8, NodeIx)]>, // one root per horizon N (§2.3)
}
pub enum Node {
    Chance { row: RowIx, adv: AdvSlot, out: [EdgeIx; 3] }, // miss / hit / crit; probabilities come from the lane
    Decide { opts: EdgeRange },                            // optional Rider, swap order, slot level: argmax per lane
    Save   { row: RowIx, out: [EdgeIx; 2] },               // Topple and the like
    Leaf,
}
pub struct Edge { pub to: NodeIx, pub terms: TermRange }
```

- **There is still one legality authority (ADR-0012).** The tape records `legal_steps`/`apply` and does not reimplement them. In the spike, the baseline and the tracer call the same `step()` function [measured].
- **Hooks run only while tracing (ADR-0036),** once per shape. Hook purity is what makes this valid: hooks see `(TurnState, Event)`, never AC or damage.
- **Liveness slot reuse** keeps the working set small. The largest spike tape (74k nodes) needed only 2,245 live slots, so values for 16 lanes fit in L2 [measured]. This also bounds WASM memory.
- **Reduction after tracing** (Bryant's ROBDD rules, applied through a unique table in post-order). Nodes with the same kind and the same (terms, reduced child) edges merge, and pass-through `Decide` nodes are bypassed. Canonicalisation merges states that are equal *by dominance*; reduction merges states that are *structurally identical*. The two stack: **1.8–2.4× fewer nodes after canonicalisation, bit-identical, 0.01–10 ms per shape** [measured, research 14 §A]. Pipeline: `canonicalise (during trace) → reduce → allocate slots`.
- **Two tapes per shape.** The *Screen tape* is canonical and reduced, and carries no labels. The *Script tape* is canonical, not reduced, and labelled with the state predicates the Combat Script may mention. Only finalists need a Script tape.

### 2.3 Every horizon, via multi-root tapes [design]

A value at round r means "damage over the rounds remaining until the horizon". Canonicalisation (§3) depends on the turns remaining, so a single-root tape for horizon N doesn't contain the fresh-start state for a shorter horizon N′. The fix: trace **one root per supported horizon**. That root is the fresh, full-resource state with N′ turns left, and the sub-DAGs below it are shared. The rounds slider is then a lookup across roots. The extra cost is the nodes unique to each fresh start, and it is paid once per shape.

### 2.4 Algebras [design]

A tape is read through an algebra. Each algebra is one small module, and none of them knows about rules:

| Algebra | Pass | Value per slot | Used for |
|---|---|---|---|
| `Moments` | backward | `(E, E²)` × 16 lanes; argmax at `Decide` by E, ties to the lowest option index | Screen tier (ADR-0013, ADR-0017, ADR-0020) |
| `PolicyRecord` | backward | the argmax index per `Decide` node per lane | input to Occupancy and Pmf |
| `Occupancy` | forward, any fixed policy | reach probability per node; expected uses per damage term and per resource unit | attribution, sensitivities, expected resource use, exact value of *any* fixed policy (§2.7) |
| `Pmf` (mm-dice) | forward, policy fixed | unnormalised damage PMF | round/combat distributions, Reliability Band, TTK (cumulative mass ≥ HP per round layer) |
| `Rational` (testkit) | both | BigRational | golden oracle (ADR-0023) |
| `Sample` (testkit) | forward walk over the *state machine* | one trajectory | Monte Carlo oracle; deliberately does not read the tape |

The moment update for independent edge damage D and remaining damage R is `E[(D+R)²] = E[D²] + 2·E[D]·E[R] + E[R²]`. Monte Carlo confirms it under the recorded policy: |z(E)| ≤ 1.67 and SD within 0.17% [measured].

### 2.5 What it costs [measured]

Per kit, all 16 ACs, single thread, canonical tapes, best of runs on the M5:

| Synthetic shape | Nodes | Per kit |
|---|---|---|
| 2 attacks, no resources, 3 rounds | 49 | ≈3–4 µs |
| slots 4/3, 3 rounds | 78 | ≈2–4 µs |
| slots 4/3/3/3/2, 3 rounds | 130 | ≈5 µs |
| slots 4/3/3/3/2, 10 rounds | 3,584 | ≈120 µs |
| slots 4/3/3/3/2, 12 rounds (default adventuring day) | 9,586 | ≈340 µs |

Tracing costs 0.01–1 ms per canonical shape, so **tapes are cached per `ShapeKey`** (in memory, and on disk/OPFS per `PackFp`). Batching several kits of one shape into one lane block adds only 1.2–1.4× [measured]. It is an optional optimisation, not part of the design's spine.

### 2.6 Explaining a result: one downward pass [design, measured]

A Tape is an arithmetic circuit with max nodes, which is Darwiche's compiled form for Bayesian networks: values flow up and derivatives flow down, each in time linear in the circuit. Under the recorded policy, the `Occupancy` pass computes the reach probability of every node and the expected number of uses of every damage term and resource unit. From that one pass:

- **Attribution:** Σ over terms of (uses × term mean, grouped by Source) = E. [measured: relative error ≤ 7.2e-16] Enablers that add no damage (Vex's advantage, Topple's prone, Archery's to-hit) are reported as a **counterfactual delta**, by re-solving with the effect off. They are shown separately and labelled with the method, never mixed into one bar.
- **Sensitivities ("stat weights"):** dE/d(term mean) = expected uses of that term, by the envelope theorem whenever the policy is locally stable. [measured: equals the finite difference to 4 decimals] SimulationCraft's scale factors need extra noisy simulations to estimate the same thing. **+1 to-hit is exactly the neighbouring AC lane** [measured: bit-exact], so "what is +1 to-hit worth" is free. Together these answer "why did this Build win" and "what would one more feat or ASI buy" (Path of Building's node-power idea, research 05 §7).
- **Expected resource use:** for example, "you'll spend 0.2 / 2.2 / 3.0 / 3.0 / 2.0 slots of level 1–5 per day".
- **Exact value of any fixed policy:** this is what prices the Combat Script (§2.7).

Screen therefore carries only (E, E²). Explanation runs for finalists only, at about the cost of one more pass each.

### 2.7 The Combat Script is a fitted rule plus its cost [design, measured]

The optimal policy is a large decision table, and nobody can play it at a table. Simple rules can be expensive [measured, research 14 §C, synthetic 12-round smite model]:

| Rule | DPR lost vs optimal |
|---|---|
| Smite crits only, highest slot | 17.5–35.5% |
| Smite every hit, highest slot | 1.2–8.2% |
| Crit: highest slot; hit: lowest slot | 2.6–5.7% |

So the Script is a search over a small **rule grammar**: outcome (hit/crit), round band, resource predicates and slot level. The engine looks for the *simplest* rule within a tolerance (default ≤ 1% loss), and ADR-0055 records the decision. The grammar is small and pricing a rule exactly takes one Occupancy pass (about 0.2 ms), so rules up to a size limit are **enumerated exhaustively**. That gives OMDT's guarantee (Vos & Verwer, IJCAI 2023: the best policy of bounded size) *within the grammar*, without a MILP. VIPER-style imitation can't make that guarantee. Disagreements with the optimal policy are shown weighted by occupancy × regret. The Script shows the rule, the optimal play, and the loss in plain numbers ("this rule gives up 1.8 DPR"). Agreement rates are not used: a rule can disagree with the optimal choice most of the time and lose almost nothing, because many decisions are near-indifferent [measured].

Rule predicates must be **canonical-invariant**. Canonical states clamp resource counts, so "more slots than turns left" never holds there [measured pitfall]. Predicates such as "at least one slot per remaining turn" or "has a slot of level ≥ k" are safe.

Prior tools spend resources on fixed schedules. DPR Lab, for example, budgets smites "for 12 rounds per day" and assumes Vex gives "advantage on 50% of attacks" ([research 14](research/14-prior-art-and-refinements.md)). Showing the price of that kind of simplification is the product's core claim, made measurable.

---

## 3. Move 2: horizon-bounded resource canonicalisation

### 3.1 The rule [design, measured]

A resource's state only needs to distinguish values that can still change the future. Every resource kind in a Ruleset's economy declares:

```rust
pub struct ResourceDecl {
    pub id: ResourceId,
    pub spend_cap_per_turn: u8,   // 2024 spell slots: 1 (the one-slot rule); Action Surge: 1; ...
    pub ordered: Option<Order>,   // slot levels: a higher unit is never worse for any use (upcasting)
    pub refresh: Refresh,         // Short / Long / PerEncounter
}
```

At every turn boundary the tracer clamps each resource to `cap × turns_left`. For ordered resources it keeps the **highest** units (top R slot levels). States that differ only in units that can no longer be spent then become one node.

| Shape (slots, rounds) | Nodes, plain → canonical | Per kit, plain → canonical |
|---|---|---|
| 4/3/3/3/2, 3 | 1,268 → 130 (9.8×) | 51 → 5 µs |
| 4/3/3/3/2, 10 | 74,173 → 3,584 (20.7×) | 2,552 → 122 µs |
| 4/3/3/3/2, 12 | 118,911 → 9,586 (12.4×) | 4,206 → 339 µs |
| 4/3/3/3/2, 20 | 310,370 → 143,941 (2.2×) | 11,137 → 5,281 µs |

Every result is bit-identical to the plain tape [measured, research 13 §2]. It helps least when the horizon exceeds the slot total (20 rounds with 15 slots). That case stays at a few ms per kit and is handled as a background or cached partition (ADR-0020).

### 3.2 Why this is safe [design]

Soundness rests on the two declared properties. The spend cap bounds how much can still be spent, and the ordering makes the top units dominate. Neither is proved for real content, so:

- **CI differential test:** for every shape in the golden set, and for proptest-generated shapes, the canonical tape equals the plain tape bit for bit.
- A resource whose declaration is wrong fails that test, and the fix is to correct its `ResourceDecl`, not the tracer.
- 2014 rules allow different per-turn spending (a Bonus-Action spell plus a cantrip). That is just a different `spend_cap_per_turn` in `Rules2014`'s economy, so the engine stays edition-agnostic (ADR-0008).

### 3.3 Rejected: Lagrangian day pricing

Research 05 §3.3 suggested pricing resources with shadow prices λ for the adventuring day. Measured on the same shapes [research 13 §2a], the dual is a valid bound (zero violations), but:

- reaching a tight bound costs 193–481 tape evaluations, **more than the exact canonical MDP**;
- at 10–12 rounds it keeps an irreducible 1–3% median (6–11% max) duality gap.

The Adventuring Day Model is therefore solved **exactly**: one canonical MDP over all its rounds, with refresh points at rests. λ stays available only as an optional valid upper bound.

---

## 4. Lanes and "search narrow, pool wide"

### 4.1 Lanes [design, measured]

A **lane** is one kit at one target AC. Every evaluation runs 16 lanes (AC 10–25) through the tape at once. Lanes are independent evaluations with no cross-lane sums, and each lane follows the same operation order as a scalar run. So scalar, auto-vectorised, WASM and WASM+SIMD128 builds all produce the same bits [measured, one result hash across targets]. The rule "parallelise only across candidates" (ADR-0037) is kept, just at the instruction level. SIMD128 is worth about 10% in WASM [measured]: enable it, but don't hand-write intrinsics.

### 4.2 Search narrow, pool wide [design]

| Phase | Lanes evaluated | What |
|---|---|---|
| **Search** (B&B over the kit lattice) | 16 ACs × the user's current advantage/resistance; the user's horizon root | kits that survive their bound |
| **Pool fill** | 16 ACs × advantage {dis, none, adv} × resistance {off, on}; every horizon root | survivors only: the top-N at any search lane |

The pool then holds exact values over the whole ADR-0020 grid, so **AC, rounds, advantage and resistance are lookups** with no message round trip. The interactive worker re-scores only for off-grid inputs: custom HP, save bonuses, Magic Resistance, or a non-default Adventuring Day Model.

For the pool to be exact across the grid, the B&B prune test compares a kit's bound with the incumbent **at every grid point where the kit could still enter the top N**. Bounds are tape-free (§5.3), so computing them over the grid costs little.

**Lenses re-rank the pool and never steer the rotation** (ADR-0053, ADR-0017):

- **Expected** (the default);
- **Reliability** (TTK and Floor, ADR-0025);
- **Robust:** minimax regret over a user-chosen range, e.g. AC 14–20. That is the build whose worst shortfall against the best build *at each grid point* is smallest (Kouvelis & Yu).

The Robust lens answers the "white-room" objection that a ranking holds only at one AC. Over the whole build space it would be a hard problem; over a finite pool on a finite grid it is a re-rank. Diversity (ADR-0018) runs after the chosen lens.

---

## 5. Move 3: enumeration as a projected join

### 5.1 Factors [design]

At one level, a Build is a tuple of factors. Each factor gets a **projection** onto what the kit can observe. Values with equal projections form one **factor class**, and only the class representative takes part in the join:

| Factor | Projection (kit-observable) | Typical collapse |
|---|---|---|
| Species (with lineage) | DPR-relevant Riders and traits | ~40 → ~6 classes (research 05 §1.4) |
| Background + bonus split + Origin feat | ability increases on relevant abilities, plus the feat's kit rows | 112 → tens |
| Base ability array (point buy) | (attack score, secondary score, parity, multiclass-13 flags) | 12,282 → 1–3 on the Pareto front |
| Class skeleton (levels, subclasses, starting class) | chassis rows, resource decls and maxima, feat-slot count | 3,711 end states at L20 with ≤3 classes |
| Feat/ASI multiset | kit rows + final ability profile | K≈15 relevant per slot |
| Loadout + mastery picks | weapon rows after the weapon-dominance collapse | 38 weapons → ~15–20 |

### 5.2 The join plan [design]

```
skeleton ⋈ feats            → project to (chassis rows, feat rows, score deltas)  → dedupe → skyline
         ⋈ abilities+origin → project to (attack mod, secondary mod, …)          → dedupe → skyline
         ⋈ loadout          → project to KitShape + Params                        → dedupe → Kit lattice
         ⋈ species          → project (mostly identity)                           → dedupe
```

- **Projection is pushed down.** Intermediate sizes track the number of *distinct kits*, not the number of Builds. Equal kits are merged when they are constructed, so the process-wide `DashMap` from ADR-0015's amendment is no longer load-bearing. That matters because Web Workers share no memory (ADR-0045), so a per-worker memo would recompute shared kits once per worker.
- **The skyline is the dominance pruning from research 05 §2.2,** computed with Börzsönyi et al.'s skyline operator in its presorted form. Sort by a monotone score, then make one window pass, which costs O(n·w) for window size w. Parity is a column, so a 16 is not dominated by a 17 when a half-feat is still possible.
- **Fallback if a lattice is too big to materialise** (three-class curves). Any-k ranked enumeration (Tziavelis et al.) shows how to produce join answers lazily in rank order. DPR doesn't decompose over factors, so the rank here is the *upper bound*: successors are generated on demand in bound order, and nothing below the incumbent is ever materialised.
- **Legality is a filter inside the join.** Prerequisites and multiclass 13s are checked where their columns first meet.
- **Back-pointers.** Each lattice entry keeps compact references to its factor classes. For shown Builds the solver rebuilds one canonical Build (lowest Content IDs) and lists the other class members as **DPR-neutral alternatives**, e.g. "any of these 5 species give identical numbers".

### 5.3 Bounds stay tape-free [design]

The tape is cheap (≈2–340 µs per kit), but B&B needs bounds that are cheaper still. Bounds therefore come from research 05 §2.5's relaxations, computed at the factor level with no tape:

- kit relaxation (no action-economy conflicts; every Rider at its best case);
- accuracy relaxation;
- the fractional-knapsack resource relaxation.

Tightening bounds is the main open tuning knob once real content exists (spike S2′).

### 5.4 Curves (ADR-0016)

The level-DAG DP reuses the factor tables per level. A DP state is a lattice entry plus the path-relevant columns (starting class, feat set). Per-level values come from that level's pool. V\*\_L frontiers are cached by fingerprint (ADR-0038, ADR-0052).

### 5.5 Partitions too big to exhaust (ADR-0057)

Unconstrained curves and all three-class queries (≈10¹³ states) run as **beam-stack search** over the level DAG (Zhou & Hansen, ICAPS 2005). It is a beam per level, plus a stack that records what each beam pruned so the search can backtrack. The search is anytime and, given the budget, **complete**.

- **Certificate:** the gap is (max bound over unexplored nodes − incumbent) / incumbent, the running bound of anytime heuristic search. `Done` reports "proven optimal" or "within x%", and a beam result is never shown as if it were optimal.
- **Budgets count nodes,** not seconds (ADR-0050).
- **Beam stacks are cached** (ADR-0038), so a later query resumes rather than restarts.
- **Prune ratio is logged** per partition. It decides empirically which partitions can be promoted to live.

---

## 6. Move 4: the Solve as data

### 6.1 Jobs, plan, merge [design]

```rust
// mm-solver: no threads, no channels, no clocks.
pub fn plan(pack: &Pack<R>, q: &Query) -> JobPlan;                          // deterministic: (bound desc, key)
pub fn run_job(ctx: &SolveCtx<R>, job: &Job, hint: &Incumbent) -> JobOut;  // pure
pub fn merge(acc: &mut Merge, out: JobOut, chunk_ix: u32);                  // called in chunk order
pub fn snapshot(acc: &Merge) -> SolveSnapshot;                              // best so far, at any moment

pub enum Job {
    Join     { stage: Stage, range: Range<u32> },
    Screen   { shape: ShapeKey, kits: Range<u32> },   // shape-affine: a worker reuses its cached tape
    PoolFill { shape: ShapeKey, kits: Range<u32> },
    Exact    { kit: KitKey, ac: u8 },
}
```

- **Shape affinity.** Jobs for one shape go to the same executor slot first, so each tape is traced once per worker. Affinity changes speed only, because results merge in chunk order.
- **The incumbent is a hint.** A stale hint prunes less but never prunes wrongly (pruning is strict, `bound < hint`). On the web the hint travels by message, which costs work, not correctness.
- **Job size is 10–30 ms** (ADR-0040's web amendment). Use per-thread scratch for tape values (`map_init`): the spike's per-call allocation is what limited small-kit scaling to 4.3× [measured].

### 6.2 Executors [measured where stated]

| Executor | Where | Notes |
|---|---|---|
| `Inline` | tests, `mm-cli --threads 1`, oracles | the determinism reference |
| `Rayon` | native (owner) | `cores − 1` threads; 4.3–5.5× at 10 threads on the M5 [measured]; `AtomicU64` incumbent |
| `WorkerPool` | web (friends) | coordinator + N compute + interactive workers; postcard over transferable `ArrayBuffer`s; one compiled `WebAssembly.Module` (ADR-0045); 1.0–1.5× native per core [measured, Cranelift proxy] |

ADR-0037's snapshot test becomes `Inline` = `Rayon(N)` = `WorkerPool(N)`, plus a **cross-target result hash**: the same fixed synthetic job set must hash to the same value on native and `wasm32` (research 13 §4 does exactly this).

**What is deterministic, and what is not** [design, after Gurobi and Rapier]:

- **Outcome determinism (guaranteed, and here cross-machine):** a `Done` result is identical for any executor, thread count and target. This goes further than Gurobi, which guarantees repeatability only on the same machine. It holds because scored values use only IEEE-754 basic operations.
- **Trajectory determinism (not guaranteed):** the sequence of `BestSoFar` snapshots, progress counts, and what a cancelled solve had reached all depend on scheduling. Tests compare `Done` results only, and the UI never presents an intermediate snapshot as final.
- **Budgets are counted in work, not seconds.** A "stop after X" limit, such as a curve beam's budget, counts evaluated kits, as Gurobi's `WorkLimit` does. A wall-clock limit would make the result depend on the machine.
- **Performance gates count work too.** CI fails on regressions in deterministic counters (tape nodes × lanes, kits screened, prune ratio), which reproduce exactly. Wall-clock benchmarks are advisory and reported with confidence intervals, not best-of-N. Georges et al. (OOPSLA 2007) showed best-of-N comparisons reaching wrong conclusions, and the M5's mixed cores varied up to 1.8× between runs [measured].
- **Banned in scored code (CI-enforced):**
  - the WASM `relaxed-simd` target feature: its FMA may round once or twice depending on the host, so it is deterministic only *locally*;
  - `f64::mul_add`;
  - transcendental functions (`powf`, `exp`, `ln`), as in Rapier's guidance;
  - `f64::max`/`min` on values that might be NaN (use explicit comparisons, as the spike does);
  - fast-math flags.

  `+simd128` is allowed: it is deterministic [measured].

### 6.3 Expected Solve times [estimate from measured per-kit costs]

| Query | Distinct kits (unknown, S2′) | Native, 9 threads | Web, M5 laptop, 7 workers | Web, iPhone, 3 workers |
|---|---|---|---|---|
| Single class, L8 snapshot, short fight | 10⁵–10⁶ × ≈3 µs | 0.05–0.6 s | 0.1–1 s | 0.3–3 s |
| Paladin-like, L20, adventuring day, B&B keeps 10⁴ | 10⁴ × ≈340 µs | ≈0.7 s | ≈1.5 s | ≈4–6 s |

Both rows are within ADR-0020's "live, with progress" class. The unknown is the kit count per partition, which needs real content.

---

## 7. Runtime topology

### 7.1 One protocol, two transports [design]

The UI always talks to the engine through **messages**, even natively. The protocol is defined once in `mm-app-core::proto`:

```rust
pub enum ToEngine   { Submit { gen: u64, query: Query }, Rescore { gen: u64, enc: Encounter },
                      Import { source: CorpusSource }, Cancel { gen: u64 } }
pub enum FromEngine { Progress { gen: u64, done: u32, of: u32 }, BestSoFar { gen: u64, snap: Arc<SolveSnapshot> },
                      Done { gen: u64, cert: Optimality }, ImportProgress(..), Error { gen: u64, err: EngineError } }
```

- **Native transport:** `crossbeam-channel` carrying the enums directly (no serialisation), plus `ctx.request_repaint()` (ADR-0040).
- **Web transport:** postcard over `postMessage`.
- **Effect:** `mm-gui` view code has no `cfg(target_arch)`, and a scripted message sequence can replay against both transports in CI.
- **`Optimality`** reports "proven optimal" or "best found, gap ≤ x%" (research 05 §6), and the UI always shows which.

### 7.2 Threads and workers

```
 native (owner)                                   web (friends)
 ──────────────                                   ─────────────
 UI thread       egui, lookups into Pool          main thread      egui, lookups into Pool
 coordinator     plan / merge / snapshot          coordinator W    plan / merge / snapshot; import → OPFS
 rayon pool      run_job × (cores − 1)            compute W × N    run_job (tape cache per worker)
 interactive T   off-grid Rescore                 interactive W    off-grid Rescore, holds Pool copy
```

The UI thread never runs solver work on either target (ADR-0043). Grid sliders read the `Arc<Pool>` from the last snapshot, so they cost zero messages.

### 7.3 Ports and adapters

`mm-app-core` defines `Storage`, `Fetcher` and `Clock`. `mm-host` implements them for `native` (`directories` + `std::fs`, a blocking HTTP client, `Instant`) and `web` (OPFS via `FileSystemSyncAccessHandle`, `fetch` with jsDelivr first, `performance.now()`), keeping `cfg` out of the core. The executor is picked by the shell, not by a port.

---

## 8. Crate map

This is ADR-0034's workspace with module boundaries filled in. New items are marked ★.

```
crates/
  mm-dice         Pmf, Moments, d20 exact-integer tables, Scratch                       (no deps)
  mm-core         Ruleset, Id<R,K>, Effect DSL, HookDecl, ResourceDecl ★, Build/Choice/
                  Encounter/Ruling, Fp ★, Provenance (ADR-0046)
  mm-load         RON/JSON → Pack<R>; predicate → bitmask; coverage; miette diagnostics
  mm-import       fetch + verify + resolve corpus → packs (library; native and worker)
  mm-engine       compile: Build → CombatKit → (KitShape, KitParams) ★
                  turn:    TurnState, legal_steps, apply, validate → Illegal(reason)
                  trace ★: KitShape → Tape (canonicalisation, multi-root, liveness slots)
                  reduce ★: unique-table reduction + liveness slot allocation
                  algebra ★: moments | policy | occupancy | pmf
                  explain: occupancy → attribution, sensitivities, resource use;
                           script ★ (rule-grammar search, priced by occupancy); crossover AC
  mm-solver       factor ★ | join ★ | bound | plan ★ | job ★ | merge | pool | diversity | curve
  mm-cache        Fp-keyed postcard store over the Storage port; Tape ★ and Pool entries
  mm-rules-2024   Rules2024: impl Ruleset (economy incl. ResourceDecls) + hooks   (mm-core only)
  mm-rules-2014   Rules2014: impl Ruleset + hooks                                  (mm-core only)
  mm-app-core     Session, proto ★, executors ★ (Inline, Rayon), ports, view models
  mm-host ★       native/ and web/ adapters for Storage, Fetcher, Clock; WorkerPool executor
  mm-gui          egui shell, native + wasm32
  mm-worker       web worker binary: coordinator | compute | interactive roles
  mm-cli          solve | evaluate | explain ★ | fetch | import | validate-packs | golden | bench
  mm-testkit      synthetic mini-corpus ★, Rational + Sample algebras, proptest strategies
xtask/ ★          dep-graph check (ADR-0034 via cargo metadata), dist/ content check, leak scan,
                  cross-target result hash (native vs wasm32-wasip1 under wasmtime)
spikes/ ★         standalone measurement code (tape-spike), not workspace members
data/rules-2024/**.ron, data/rules-2014/**.ron
web/              index.html, manifest, service worker, _headers, gate Worker (JS), wrangler.jsonc
```

The dependency rules are unchanged from ADR-0034. `mm-host` sits beside the shells, and only binaries depend on it.

---

## 9. Correctness architecture (ADR-0056)

Most answers have no hand-computable expected value: nobody can compute a 12-round Paladin rotation by hand. That is the *test oracle problem*, and three techniques address it:

1. **Differential testing:** every fast path is checked against an independent slow path (table below).
2. **Metamorphic relations** (Segura et al., *IEEE TSE* 2016): relations between outputs that need no expected value. This domain has exact ones:
   - **option monotonicity:** adding an optional Rider, a resource unit, a round or an attack source never lowers optimal E;
   - **parameter monotonicity:** E is non-increasing in AC, and non-decreasing in to-hit and in damage.

   [measured, research 15 §1: 6 relations, ~57,000 checks, 0 violations] Each Rider and resource kind declares which relations apply to it, because mandatory drawbacks can legitimately lower E.
3. **Mutation testing** (`cargo-mutants` on `mm-dice` and `mm-engine` before each encoding milestone) measures whether 1 and 2 actually check behaviour. A surviving mutant in scoring code is a missing test.

Differential checks; the spike has already run each one marked with a result:

| Check | Fast path | Reference | Spike result |
|---|---|---|---|
| Tape vs interpretation | `Tape × Moments` | hash-map DP over the same `step()` | bit-identical |
| Canonicalisation | canonical tape | plain tape | bit-identical |
| Moments vs reality | `Tape × Moments` | Monte Carlo over the state machine | \|z\| ≤ 1.67, SD ≤ 0.17% |
| Threads | `Rayon(N)` | `Inline` | bit-identical, N = 1…10 |
| Targets | `wasm32` (± simd128) | native aarch64 | one result hash |
| Reduction | reduced tape | unreduced tape | bit-identical |
| Attribution | Σ occupancy × term mean | E from Moments | ≤ 7.2e-16 relative |
| Sensitivities | occupancy | finite difference | equal to 4 decimals |
| To-hit / AC duality | E(to-hit + 1, AC) | E(to-hit, AC − 1) | bit-exact |
| Metamorphic relations | any evaluation | the relation itself | 0 violations in ~57k checks |
| Script rule cost | Occupancy under the rule | brute-force rule evaluation | to do |
| Exact arithmetic | `Tape × Pmf`/`Moments` | `Tape × Rational` | to do (step 4) |
| Legality | every tape path | `apply` replay | to do (step 4) |

- **Synthetic mini-corpus.** CI never fetches the corpus (ADR-0028), so `mm-testkit` ships a small corpus in 5e.tools *shape* with invented content (`Test Blade|TST`, `"_fixture": "synthetic"`). The whole pipeline (import → pack → solve → snapshot) runs in CI on it.
- **Proptest strategies** generate random Kit Shapes, ResourceDecls and Params for every row of the table above.
- **Golden scenarios** (insta RON) are hand-checked worked examples on synthetic weapons. Real-corpus goldens run only locally (ADR-0028).
- **Rulings matrix.** Each disputed Ruling (ADR-0006) gets a scenario that asserts the DPR delta between its options.
- **Benchmarks** compare ratios (tape vs reference, N threads vs 1) with a pinned thread count. Per-kit times on the M5 varied up to about 1.8× between runs because of P-core vs E-core scheduling [measured].

---

## 10. Decisions recorded

**New ADRs (proposed):**

| ADR | Decision | Section |
|---|---|---|
| [0047](adr/0047-kit-shapes-and-traced-tapes.md) | Kit Shape / Kit Params; traced, reduced Tapes evaluated over AC lanes | §2 |
| [0048](adr/0048-horizon-bounded-resource-canonicalisation.md) | Resource state clamped to what can still be spent; the adventuring day solved exactly | §3 |
| [0049](adr/0049-enumeration-as-projected-join.md) | Enumeration as a projected join with skyline pruning and DPR-neutral alternatives | §5 |
| [0050](adr/0050-solve-as-deterministic-job-plan.md) | The Solve as a job plan; executors; outcome vs trajectory determinism; work-based budgets and gates | §6 |
| [0051](adr/0051-one-engine-protocol.md) | One message protocol between UI and engine on both targets | §7.1 |
| [0052](adr/0052-fingerprints-record-what-was-read.md) | Fingerprints of what was read (only the Rulings consulted) | §1 |
| [0053](adr/0053-pool-covers-encounter-grid.md) | The pool covers the encounter grid; Expected/Reliability/Robust lenses | §4 |
| [0054](adr/0054-explanations-from-occupancy.md) | Explanations from one occupancy pass, contrastive (#1 vs #2) | §2.6 |
| [0055](adr/0055-combat-script-is-simplest-rule-within-tolerance.md) | The Combat Script is the simplest rule within a tolerance, priced exactly | §2.7 |
| [0056](adr/0056-differential-metamorphic-mutation-testing.md) | Differential, metamorphic and mutation testing | §9 |
| [0057](adr/0057-certified-anytime-search-for-large-partitions.md) | Beam-stack search with a certified optimality gap for large partitions | §5.5 |

**Amended (dated blocks at the top of each):**

- **0006:** cheap Ruling-flip deltas.
- **0015:** shape tapes; dedupe by construction; DashMap optional.
- **0017:** Robust lens; the Script is a priced simplification.
- **0018:** exact k = 3 is cheap; diversity runs after the lens.
- **0020:** grid-wide pool; exact day model; λ-pricing rejected.
- **0034:** new crates and modules.
- **0037:** outcome vs trajectory determinism, and the bans.
- **0038:** fingerprint keys; cached tapes and beam stacks.
- **0045:** measured WASM cost.

**Unchanged and reinforced:**

- **0012:** the tape is a recording; enforced by a replay test.
- **0013:** `Decide` nodes are the outcome-dependent choices.
- **0036:** hooks run only while tracing.

Research 05 §3.3 is contradicted by measurement. That is recorded in the ADR-0020 amendment, and the research note is left unedited.

## 11. Build order (walking skeleton first)

Each step ends with something runnable from `mm-cli`.

1. **`mm-dice` + d20 tables + Rational oracle.** Reproduce research 03 §7.2 on a synthetic greatsword.
2. **`mm-core` (incl. `ResourceDecl`, `Fp`) + `mm-load` + synthetic mini-corpus.** `validate-packs` passes in CI.
3. **Engine interpreter.** `mm-cli evaluate build.ron` prints DPR, the Reliability Band and a Combat Script. *This already meets spec §5.4 for a hand-entered Build.*
4. **Trace (canonical, multi-root) + Moments/Pmf algebras,** with every check in §9 as a test. The spike is the template.
5. **Factor tables + projected join + bounds + Screen + top-K + MMR** on `Inline`. `mm-cli solve` answers "L8, Sustained, no multiclass, AC 17". **Run S2′ here:** distinct kits and shapes per partition, and how tight the bounds are.
6. **`mm-import` on the real corpus** (local only), plus the staleness report (ADR-0031).
7. **`mm-app-core` Session + protocol + Rayon + native egui** (research 10 spikes S1–S3).
8. **Web:** `mm-worker`, `mm-host::web`, WorkerPool, Trunk, the gate and the deploy (spikes W1–W4, with W2 now a browser check of research 13 §4).
9. **Curves, disk cache, and 2–3-class beams with optimality certificates.**

## 12. Problems ahead, and the method for each

[Research 15](research/15-problems-ahead-and-methods.md) lists the problems real content and real users will raise. For each, it gives the method with the best support in the literature:

| Problem | Method | Prior art | Recorded in |
|---|---|---|---|
| Wrong encodings with no hand-computable oracle | Metamorphic relations + differential paths + mutation testing | Segura et al. 2016; `cargo-mutants` | ADR-0056 |
| Three-class curves too big to exhaust | Beam-stack search with a running certified gap | Zhou & Hansen 2005; Hansen & Zhou (anytime search) | ADR-0057 |
| Rankings that hold at only one AC | A minimax-regret Robust lens over the pool's grid | Kouvelis & Yu 1997 | ADR-0053 |
| "Why this build?" | Contrastive #1-vs-#2 differences; Crossover AC; DPR-neutral alternatives | Miller 2019 | ADR-0054 |
| An unplayable optimal policy | The simplest rule within tolerance, enumerated and priced exactly | OMDT (Vos & Verwer 2023); VIPER | ADR-0055 |
| Noisy performance numbers | Deterministic work counters gate CI; confidence intervals for timing | Georges et al. 2007 | ADR-0050 |
| Results that differ across devices | IEEE basic operations only; banned operations; outcome vs trajectory | Rapier; WASM relaxed-SIMD notes; Gurobi | ADR-0050 |
| Rules *misread* (an internally consistent but wrong encoding) | Citations and source tiers, Rulings, one golden per encoding, cross-checks against independent calculators | — (needs people) | ADR-0006, 0007, 0056 |

### Risks

| Risk | Status | Fallback |
|---|---|---|
| Rotation MDP too costly per kit (S2) | **Largely retired** [measured]: canonical tapes cost 2–340 µs per kit up to a 12-round day | long horizons beyond the slot total (≈5 ms per kit at 20 rounds) become background/cached partitions |
| A resource violates the canonicalisation assumptions | caught by the canonical == plain test | fix its `ResourceDecl`, or mark it non-canonicalisable |
| Too many distinct kits per partition | unknown until S2′; the prune ratio is logged per partition | tighter bounds; beam-stack search with a certified gap (ADR-0057) |
| Bounds too loose to prune | unknown until S2′ | a fixed-λ priced tape is a valid, cheap bound [measured valid] |
| Browser per-core cost worse than Cranelift's | W2 | the ADR-0045 wasm-threads upgrade path is unchanged |
| A projection misses an interaction | the differential test: projected join vs brute force on small synthetic packs | widen the projection key; correctness never depends on how much the join collapses |

## 13. Glossary

These terms were added to `GLOSSARY.md` ("Evaluation engine") in revision 4, together with **Optimality gap**. Summary:

- **Kit Shape:** the structure of a Combat Kit that determines its rotation state graph (attack sources, Rider slots, resource declarations, swaps, Rulings), without its numbers. _Avoid_: template, archetype.
- **Kit Params:** the numeric part of a Combat Kit (to-hit, damage-term moments, DCs) that is evaluated over a fixed Kit Shape. _Avoid_: stats.
- **Tape:** the recorded rotation state graph of one Kit Shape, evaluated in different algebras. _Avoid_: trace (the verb that makes it), bytecode.
- **Lane:** one independent evaluation inside a tape pass: one kit at one target AC.
- **Occupancy:** the probability of reaching each state, and the expected number of uses of each damage term and resource, under a fixed policy. _Avoid_: visit counts, heat map.
- **Sensitivity:** the exact change in expected damage per unit change of one Kit Param, read from Occupancy. _Avoid_: stat weight, scale factor (SimulationCraft's noisy estimates).
- **Script rule:** the simplest play rule within the Combat Script tolerance of the optimal policy, shown with its DPR cost. _Avoid_: priority list, APL.
- **Spend cap:** the most units of a resource that can be spent in one turn. Together with the turns remaining, it bounds which resource states are distinguishable.
- **Factor class:** the set of values of one Build factor that project to the same kit contribution. Its members are DPR-neutral alternatives. _Avoid_: equivalence group.

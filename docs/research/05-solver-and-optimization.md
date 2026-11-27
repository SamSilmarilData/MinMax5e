# 05 — Solver and optimization layer

> Research note for MinMax5e. Scope: how to search the build space, how to pick the in-combat rotation, and how to define the objective. Written 2026-11-26.
>
> **Citation convention.** Every claim has a URL. Claims marked **(unverified)** were not confirmed against a primary source in this session. URLs marked **(nf)** were cited from memory and not fetched in this session. Numbers marked **(computed)** come from the scripts described in the appendix.

---

## Question

How should MinMax5e (1) find the best build (species, background/origin feat, ability scores, class/subclass level order, feats, weapons, fighting style) under user constraints, and (2) find the best in-combat rotation for each build? It has to do both fast enough that the dashboard updates live as the sliders move (AC 10–25, level 1–20, rounds, Sustained/Nova). It also has to return a top 3 whose members are actually different builds. What objective functions, Sustained, Nova, Smooth Leveling Curve and risk-aware, should the solver optimise?

## TL;DR

- **The raw space is about 10^35 builds (computed)**, but most of it is symmetry or dominated. If we canonicalise (peak objective depends only on the end state, not the level order), project ability scores onto the abilities that matter, and keep only DPR-relevant feats and weapon archetypes, the MVP comes down to **~10^7 candidates for a single-class level-8 snapshot** and **~10^9–10^10 for single-class level-20 curves**. That is fine offline and too big to enumerate on every slider move. Three-class multiclassing (~10^13 states) is not exhaustively enumerable even offline. It needs bounds or beam search.
- **Split the problem in two.** The *outer* search chooses the build. The *inner* search chooses the rotation, a finite-horizon MDP solved exactly by backward induction over (round, resources, buffs) for each build and encounter. The 2024 Divine Smite is cast "immediately after hitting" ([dnd-wiki mirror of SRD 5.2](https://new.dnd-wiki.org/wiki/Divine_Smite_(5e24))), so smite-on-crit is a real decision made after the dice land, and a fixed priority list (SimulationCraft style) is suboptimal.
- **MVP algorithm:** exhaustive enumeration over canonicalised, dominance-pruned builds. Run it **offline** to produce a *candidate pool* per partition (level × class-combo × mode). At runtime, re-score the pool exactly in a **Web Worker** for every slider change, with branch-and-bound as the fallback for parameter combinations that were not precomputed. **Full scope:** dynamic programming over levels on a state DAG (exact for max-min, weighted-sum and trough-penalty curve objectives), with beam pruning and admissible upper bounds for 3-class multiclassing. Add MAP-Elites as an exploration and diversity tool.
- **CP-SAT/MILP is a poor fit for the objective.** CP-SAT "works over the integers" ([OR-Tools docs](https://developers.google.com/optimization/cp/cp_solver)), and the probabilistic DPR objective is a nonlinear simulation. CP-SAT is still useful for *legality*: prerequisites, multiclass 13s and enumerating feasible skeletons.
- **Objectives:** Nova = expected damage in rounds 1–2 with full resources. Sustained = expected damage per round over a configurable adventuring day. **The 2024 DMG no longer prescribes encounters per day** ([EN World](https://www.enworld.org/threads/dungeons-dragons-has-done-away-with-the-adventuring-day.707237/)), so the day model has to be an explicit, user-visible assumption. Smooth curve = **leximin of normalised level scores** by default, with a weighted-mean-minus-trough-penalty variant. All of these decompose over a level DAG, so DP stays exact.
- **Top-3 diversity:** group candidates by a *build signature* (class split, subclass, weapon archetype, key feats), take the best of each group, then apply MMR ([Carbonell & Goldstein 1998](https://www.cs.cmu.edu/~jgc/publication/MMR_DiversityBased_Reranking_SIGIR_1998.pdf)) with a minimum-distance threshold. Show near-duplicates as "sidegrades", as Raidbots does ([Icy Veins](https://icy-veins.com/wow/how-to-use-the-top-gear-feature-of-raidbots)).

---

## 1. Search-space sizing

### 1.1 Inputs and their cardinalities

| Dimension | Count used | Basis |
|---|---|---|
| Point-buy arrays (27 points, scores 8–15, cost 8=0 … 13=5, 14=7, 15=9) | **12,282** ordered arrays spending exactly 27; 191,587 spending ≤27; **65** distinct sorted multisets (computed) | Cost table: [Roll20 compendium](https://roll20.net/compendium/dnd5e/Rules:Step%203%20-%20Ability%20Scores). Whether the 2024 SRD includes the table is unverified ([rpgbot](https://rpgbot.net/dnd5/how-to-play/choosing-ability-scores/)) |
| Background ability bonus | Background lists 3 abilities; +2/+1 (6 ordered ways) or +1/+1/+1 (1) = **7** per background; cap 20 | [SRD 5.2.1 Character Origins (Fantasy Grounds)](https://www.fantasygrounds.com/library/5e/dd-srd-5-2-1/character-origins) |
| Backgrounds / origin feats | SRD has 4 backgrounds; PHB 2024 has **16** backgrounds with **10** origin feats **(unverified count)**. Each background fixes its origin feat ([SRD 5.2.1](https://www.fantasygrounds.com/library/5e/dd-srd-5-2-1/character-origins)). A "custom background" (any 3 abilities × any origin feat = 20 × 10) is in the 2024 DMG **(unverified)** | |
| Species | SRD 5.2.1 lists 9 ([Fantasy Grounds](https://www.fantasygrounds.com/library/5e/dd-srd-5-2-1/character-origins)); PHB adds Aasimar = 10 **(unverified)**. With lineage/ancestry sub-choices and Human's extra origin feat, about **40** variants **(estimate)** | |
| Classes × subclasses (MVP) | 6 classes × 4 PHB subclasses = **24** **(unverified: 4 per class)** | |
| Multiclass prerequisites | 13 in the primary ability of both the old and the new class | [D&D Beyond basic rules](https://www.dndbeyond.com/sources/basic-rules/customization-options) (2014 text; 2024 wording unverified) |
| ASI/feat slots | All classes at 4, 8, 12, 16 + Epic Boon at 19; Fighter also at 6 and 14; Rogue also at 10 **(unverified, PHB 2024)** | |
| Options per slot | ASI: +2 to one (6) or +1/+1 (15) = 21; general feats ≈ 40 **(unverified)**, many of them half-feats with 2–3 ability choices → **~100** raw options per slot; **K ≈ 15** DPR-relevant options after pruning **(estimate)** | |
| Weapons | 38 in the PHB weapon table **(unverified)**; loadouts of ≤3 weapons = 38 + C(38,2) + C(38,3) = **9,177** (computed) | |
| Fighting style | ~10 Fighting Style feats **(unverified)**; ~5 DPR-relevant | |

### 1.2 Class level orderings and the symmetry gap

Counts from `dp.py`/`size.py` (appendix), for 6 classes:

| Level | Ordered level sequences, ≤2 classes | ≤3 classes | Distinct *end states* (class-level splits), ≤2 | ≤3 |
|---|---|---|---|---|
| 5 | 456 | 3,456 | — | — |
| 8 | 3,816 | 1.20 × 10^5 | 111 | 531 |
| 11 | 3.07 × 10^4 | 3.45 × 10^6 | — | — |
| 20 | 1.57 × 10^7 | **6.97 × 10^10** | 291 | **3,711** |

At level 20 with ≤3 classes, the order of levels inflates the count by about **10^7** over the end states. For the *peak* objective, only the end state plus the **starting class** (proficiencies and saving throws come from the first class; multiclassing gives fewer proficiencies) affects the level-N build. Everything else in the ordering is pure symmetry. For the *curve* objective, the ordering matters, but it can be handled as a path through a DAG of end states (§2.4), so it never has to be enumerated.

### 1.3 Raw product (naive, level 20, ≤3 classes)

40 species × 112 (16 backgrounds × 7 bonus splits) × 12,282 arrays × 6.97×10^10 sequences × 64 subclass combos × 100^5 feat picks × 9,177 loadouts × 10 styles ≈ **2.3 × 10^35** (computed). This is an upper bound with heavy overcounting. It shows only that naive enumeration is out of the question.

### 1.4 After canonicalisation and dominance (what the solver actually sees)

**Ability scores.** Over all 20 possible ability triples, point buy plus background bonus gives 96,572 distinct final arrays, of which 37,291 are Pareto-maximal in all six dimensions (computed). Six-dimensional dominance alone does not prune much. **Projection** does: DPR for an MVP martial depends on the attack ability and, for a few features, one secondary ability (WIS for Monk/Ranger save DCs, CHA for Paladin). Projected onto (STR, CHA) or (DEX, WIS), the Pareto front is **1–2 arrays** (computed): STR 17 / CHA 16 is reachable at the same time. Projected onto five abilities as *modifiers*, it is 185 arrays. So each build carries an **ability profile** = (attack score incl. parity for half-feats, secondary score, multiclass-13 flags). That is a handful of values instead of 12,282 × 7.

**MVP single-class, level-8 snapshot** (the spec's validation example, §5.4):

- Species DPR-equivalence classes ≈ 6. Origin feat options ≈ 15. Ability profiles ≈ 3.
- Class/subclass × ASI multisets with K = 15: Fighter 4 × C(17,3) = 2,720; other five classes 4 × C(16,2) = 480 each → **5,120 skeletons** (computed, `dp.py`).
- × fighting style (Fighter/Paladin/Ranger only) × ~10 weapon archetypes → **~5 × 10^7 candidates** (computed by hand from the above).

**MVP single-class, levels 1–20 (curve):** DP states summed over levels = **2.6 × 10^6** per origin × weapon archetype (K = 15; 6.6 × 10^8 with K = 40). With ~225 origins × ~50 archetype/style combos, that is about **10^10** state evaluations. Fine offline, impossible live.

**Multiclass, levels 1–20 (curve):** ≤2 classes **2.95 × 10^8**, ≤3 classes **6.0 × 10^9** DP states *per origin* (K = 15) (computed). Times ~10^4 origin × archetype combos gives ~10^13–10^14. **Exhaustive enumeration is infeasible even offline.** Bounds or beams are required.

**Evaluation cost.** A toy closed-form expected-DPR evaluator (k attacks, crit range, advantage, Graze floor, once-per-turn rider) costs **8.3 µs per build for all 16 ACs in CPython** (computed, `bench.py`). Compiled Rust/WASM should be 10–100× faster **(unverified estimate)**. A real evaluator with the rotation MDP (§3.3) will be 10^2–10^3× more expensive than the toy. Budget **~10–100 µs per (build, encounter) compiled** as the planning figure.

---

## 2. Decomposition and pruning

### 2.1 Outer/inner split (build vs rotation)

`score(build, encounter) = max over policies π of E[damage | build, π, encounter]`. The inner max is an MDP (§3.3) that depends only on the build's *combat kit*: attack bonus, damage dice, riders, resources, actions and masteries. Many builds share a kit. For example, two builds that differ only in a non-combat origin feat or a CON bump do not change DPR. **Memoise the inner solve on a canonical kit hash.** This is the single largest practical saving and the main reason to separate "build" from "kit" in the domain model.

### 2.2 Dominance pruning

The objective is monotone non-decreasing in most inputs, so componentwise dominance is sound:

- **Ability profile:** profile A dominates B if every relevant score is ≥ and the multiclass-13 flags are a superset. Caveat: an odd score has option value for a later half-feat, so **parity** must be part of the profile. 16 is not dominated by 17 when the plan is a half-feat at level 4.
- **Weapons:** within the same (mastery property, handedness, damage type, properties such as Light/Heavy/Finesse/Reach/Thrown), only the largest die survives. This collapses 38 weapons to about 15–20 **(estimate)**.
- **Feats:** drop feats with no modelled DPR effect in the current mode. They are kept only as tie-breakers.
- **Species:** collapse species whose modelled DPR contribution is identical into equivalence classes.

Dominance has to be recomputed when the *mode* changes. A feat that is dominated for single-target DPR may not be once AoE or tank modes exist.

### 2.3 Symmetry reduction

- **Peak objective:** canonical key = (starting class, multiset of {class: level, subclass}, multiset of feat/ASI picks, ability profile, loadout). Level order and ASI timing are irrelevant at the snapshot. This turns 6.97 × 10^10 sequences into 3,711 splits × ≤3 starting classes.
- **State merging:** two pick histories that produce the same (class levels, subclasses, *feat set*, *effective* ability scores after the cap at 20) are the same state. Scores are capped at 20, so many ASI multisets collapse.

### 2.4 DP over levels: memoising sub-builds

Define a state *s_L* = (class-level vector, subclasses, feat set, ability profile, origin, loadout archetype). The score *q_L(s_L)* depends only on the state, not on the path to it. The progression is then a **path through a layered DAG**: from *s_L*, add one level in one of ≤3 classes, and pick a feat or ASI if that class level grants a slot. Every Smooth-Curve objective in §4.3 decomposes along this path:

- Additive (weighted sum, trough penalty): longest-path DP, `V(s_L) = q̃_L(s_L) + max_{s_{L+1}} V(s_{L+1})`.
- Bottleneck (max-min): `V(s_L) = min(q_L(s_L), max_{s_{L+1}} V(s_{L+1}))`. This is a widest-path recurrence and is exact on a DAG.
- Leximin: run the bottleneck DP, fix the optimum, then repeat with that level excluded (or sort-vector comparison per state, which costs more memory).

This replaces the 10^7× ordering blow-up with work linear in the number of states and edges.

### 2.5 Upper bounds for branch-and-bound

An admissible bound must never underestimate the best completion:

1. **Kit relaxation:** drop action-economy conflicts (Bonus Action, concentration, one-slot-per-turn) and sum every rider's best-case contribution. The result is ≥ any legal rotation.
2. **Accuracy relaxation:** use the maximum attack ability (20) the remaining ASI slots could reach, and the maximum weapon enhancement allowed.
3. **Resource relaxation (Nova/Sustained):** assume every slot is spent at its best damage-per-slot ratio, i.e. a fractional knapsack.
4. **Curve DP:** for partial paths, bound the remaining levels by the per-level ceiling *V\*_L* from the peak solves (§4.3). That gives `bound = min(current_min, 1.0)` for max-min.

Bounds are cheap (no MDP), so B&B evaluates the inner MDP only for nodes that survive.

---

## 3. Algorithm survey

### 3.1 Comparison table

| Approach | Exact? | Fits a nonlinear, simulation-based objective? | Handles curve objective? | Live (<100 ms)? | Notes / reference |
|---|---|---|---|---|---|
| Exhaustive enumeration + dominance/symmetry | Yes | Yes (black box) | Only via DP | Only over a precomputed pool | Baseline; trivially parallel |
| Branch-and-bound | Yes, given admissible bounds | Yes, if bounds can be derived from the kit | Yes (bound remaining levels) | Often, for constrained queries | [Land & Doig 1960](https://doi.org/10.2307/1910129) (nf, unverified) |
| Beam search | No | Yes | Yes (beam per level) | Yes | Width-B frontier; [AIMA](https://aima.cs.berkeley.edu/) (nf) |
| DP over levels (DAG) | Yes, for decomposable objectives | Yes (per-state black box) | **Natively** | Offline; online with beams | Bellman; [Sutton & Barto, ch. 4](http://incompleteideas.net/book/the-book-2nd.html) (nf) |
| CP-SAT / MILP | Yes for the model | **Poorly**: integer-only; products need auxiliary variables (`AddMultiplicationEquality`) and the objective must be tabulated | Awkwardly | Rarely | "CP-SAT solver works over the integers" ([OR-Tools](https://developers.google.com/optimization/cp/cp_solver)); [API ref](https://developers.google.com/optimization/reference/python/sat/python/cp_model) |
| Genetic algorithm | No | Yes | Yes | No (needs many generations) | [Holland 1975](https://mitpress.mit.edu/9780262581110/) (nf). PoE tree optimizer ships GA as "experimental" ([poe-optimizer](https://github.com/alecrivet/poe-optimizer)) |
| Simulated annealing | No | Yes | Yes | No | [Kirkpatrick et al. 1983, Science 220:671](https://pubmed.ncbi.nlm.nih.gov/17813860/) |
| Greedy marginal improvement | No | Yes | Partly | Yes | Default in [poe-optimizer](https://github.com/alecrivet/poe-optimizer); fails on feat synergies (GWM + Heavy weapon) |
| MAP-Elites (quality-diversity) | No | Yes | Yes | No | Illuminates best-per-niche; [Mouret & Clune 2015](https://arxiv.org/abs/1504.04909) |
| MDP backward induction (rotation) | Yes | Yes | n/a (inner loop) | Yes per build | [Sutton & Barto](http://incompleteideas.net/book/the-book-2nd.html) (nf) |
| Expectimax / MCTS (rotation) | Expectimax exact; MCTS approximate | Yes | n/a | Expectimax yes; MCTS budgeted | [Browne et al. 2012 MCTS survey](https://researchr.org/publication/BrownePWLCRTPSC12) |
| Priority lists (APL) | No (hand-authored) | n/a | n/a | Yes | SimC scans "starting with the first action … until an available action is found" ([SimC wiki](https://github.com/simulationcraft/simc/wiki/ActionLists)) |

### 3.2 Why not CP-SAT/MILP for the core objective

Expected DPR is a sum over attacks of p_hit(AC, bonus, adv) × E[dmg] plus crit, Graze, and once-per-turn terms like Sneak Attack's 1 − (1 − p)^k. That is polynomial or exponential in decision-dependent quantities. CP-SAT requires integer models and scaling of non-integer terms ([docs](https://developers.google.com/optimization/cp/cp_solver)). Products need auxiliary bounded variables ([API reference](https://developers.google.com/optimization/reference/python/sat/python/cp_model); integer-only behaviour also noted in [or-tools-discuss](https://groups.google.com/g/or-tools-discuss/c/IUrUodzW5Fo)). The only clean encoding is to **precompute a table of DPR per discrete kit** and use element/table constraints. That is enumeration again, with solver overhead added. CP-SAT can also enumerate all feasible solutions (`enumerate_all_solutions`, [docs](https://developers.google.com/optimization/cp/cp_solver)), which makes it a candidate *legality engine* for generating canonical skeletons. A hand-written generator is simpler at MVP scale.

### 3.3 Rotation as an MDP (inner optimisation)

- **State:** (round r ≤ N, remaining resources: spell slots by level, Channel Divinity, Action Surge, Rage, Focus/Ki, Superiority dice, Heroic Inspiration; active concentration spell and its target status, e.g. Hunter's Mark placed; weapon in hand; pending Vex advantage; once-per-turn flags).
- **Action:** a legal *turn plan*: Action + Bonus Action + item interaction + attack ordering + weapon swaps. The turn-plan compiler enforces the spec's guarantees: one spell slot per turn, Bonus Action collisions, Nick/Vex sequencing.
- **Chance nodes:** each attack's outcome (miss/hit/crit). Decisions after an outcome include casting Divine Smite "immediately after hitting" ([SRD 5.2 mirror](https://new.dnd-wiki.org/wiki/Divine_Smite_(5e24)); [D&D Beyond Paladin article](https://www.dndbeyond.com/posts/1749-2024-paladin-vs-2014-paladin-whats-new)), Stunning Strike, and Battle Master manoeuvres.
- **Solve:** finite-horizon backward induction maximising expected damage. A Paladin 20 has slot state 5×4×4×4×3 = 960 × concentration/buff flags × N rounds, so about **10^4–10^5 states**, small enough to solve exactly in about a millisecond compiled **(estimate)**.
- **Across an adventuring day** (E encounters, short rests every k encounters): an exact DP over (encounter index, resources) is the same size times E. A cheaper and near-exact alternative is **Lagrangian resource pricing**: put a shadow price λ on each resource, solve each encounter independently for damage − λ·cost, and bisect λ until spending meets the budget ([Fisher 1981, Management Science](https://doi.org/10.1287/mnsc.27.1.1) (nf, unverified)).
- **Output:** the optimal policy is a decision tree. For the dashboard's "Turn-by-Turn Combat Script" it is compressed into Pre-Combat / Round 1 / Rounds 2+ plus "if crit → smite" annotations.

Monte Carlo (SimC's method, which repeats fights until a target error is reached per [Wowhead](https://cn.wowhead.com/news=284548) **(secondary)**) is **not needed for means**. 5e per-round damage distributions are small discrete convolutions, computable exactly. Monte Carlo stays useful as a test oracle.

### 3.4 Recommendation

- **MVP:** canonicalised exhaustive enumeration, run offline per partition (level × class-combo × mode × default encounter grid). It produces a *candidate pool*: the union of top-N over the grid of AC 10–25, rounds 1–10, advantage states and resistance toggles. The runtime re-scores the pool exactly in a Web Worker. **Branch-and-bound** with kit-relaxation bounds handles queries outside the precomputed grid. The rotation is an exact MDP per kit, memoised.
- **Full scope:** level-DAG **DP with beam pruning** (keep the top-B states per class-level vector by optimistic bound) for 3-class multiclass curves. B&B for snapshots. **MAP-Elites** as an offline "discovery" mode that fills a grid of niches (class split × weapon archetype), which also feeds top-K diversity. Revisit CP-SAT only if constraints become complex enough (full casters, spell preparation lists) that hand-written legality generation becomes a maintenance burden.

---

## 4. Objective-function design

Notation: *b* = build, *e* = encounter parameters (AC, save bonuses, rounds N, advantage state, resistances), *D_r* = damage dealt in round r (a random variable), π = rotation policy.

### 4.1 Nova

`Nova(b, e) = max_π E[D_1 + D_2]`, with all resources full and no value placed on leftover resources. "Pre-combat setup" (e.g. casting Hunter's Mark before initiative) is an explicit toggle, because it changes Round 1 action economy. Reported per round (÷2) so it is comparable with Sustained. The horizon "Rounds 1–2" is fixed by the spec (§3.1). Expose it as N_nova ∈ {1, 2, 3}.

### 4.2 Sustained over an adventuring day

`Sustained(b, e, day) = (1 / Σ_j N_j) · max_π E[ Σ_{j=1..E} Σ_{r=1..N_j} D_{j,r} ]`, subject to resources recovering only at the day's short and long rests.

- **The day model is an assumption, not a rule.** The 2014 DMG recommended 6–8 medium/hard encounters per day. The 2024 DMG drops that recommendation and discusses encounter pace and short-rest balancing instead ([EN World, Oct 2024](https://www.enworld.org/threads/dungeons-dragons-has-done-away-with-the-adventuring-day.707237/), secondary; primary text unverified). 2024 encounter difficulty uses Low/Moderate/High XP budgets per character ([D&D Beyond](https://www.dndbeyond.com/posts/1901-creating-combat-encounters-using-the-new-dungeon)), with old Medium→Low, Hard→Moderate, Deadly→High, and higher budgets at high levels ([Blog of Holding](https://www.blogofholding.com/?p=8402)).
- **Proposed default** (configurable, shown on the dashboard): E = 4 encounters, N = 3 rounds each, 1 short rest after encounter 2. Presets: "Light" (E = 2), "2014 Classic" (E = 6, 2 short rests). These numbers are **design choices, not rules text**.
- **Resource-light shortcut** (spec §3.1 wording): `Sustained_0(b, e) = max_π E[D_r]` with no daily resources spent. Show it as the floor line.
- Because Sustained depends on the day model, the cache key must include it (§6).

### 4.3 Smooth Leveling Curve

Let *V_L(b)* = the chosen per-level objective (Sustained or Nova) at level L for the build's level-L state. Let *V\*_L* = the best V_L over all builds satisfying the same constraints (from 20 peak solves). Normalised score: `q_L(b) = V_L(b) / V*_L ∈ (0, 1]`. Levels in a user range [L_lo, L_hi] with weights w_L (default: uniform; "campaign" preset weights tiers 1–2 more heavily, since tables play there most often **(unverified claim about play patterns; treat as a preset, not a fact)**).

| Variant | Formula | Behaviour | DP-decomposable? |
|---|---|---|---|
| A. Max-min | `J = min_L q_L` | No weak levels; ignores everything except the worst level. Ties are common because levels 1–2 are similar across builds | Yes (bottleneck) |
| A'. **Leximin** (default) | Sort q_L ascending, compare lexicographically | Fixes A's ties. Prefers delaying a dip to a level where the build is still near the frontier | Yes (repeated bottleneck) |
| B. Weighted mean | `J = Σ w_L q_L / Σ w_L` | A deep trough can be bought off with a high peak. This is the behaviour the spec wants to avoid | Yes (additive) |
| C. Mean − trough penalty | `J = Σ w_L q_L − λ Σ w_L · max(0, τ − q_L)^2` (τ = 0.85, λ = 4 default) | Smooth trade-off: dips below τ are penalised quadratically. τ is intuitive ("never below 85% of the best possible build at that level") | Yes (additive) |
| D. Power mean | `J = (Σ w_L q_L^p / Σ w_L)^{1/p}`, p < 0 | One knob: p = 1 is B, p → −∞ is A | Yes (monotone transform of an additive sum) |
| E. Reference-curve penalty | `J = −Σ w_L max(0, R_L − V_L)` against a user-chosen reference build R | "Never worse than a straight Fighter" | Yes |

Recommended behaviour: default **C** (it gives an explainable "minimum percentage" knob), with **A'** as the strict mode. Caveats:

1. *V\*_L* changes with constraints (locks, encounter AC), so q_L must be recomputed per query. This is one more reason for precomputed per-level frontiers.
2. At levels 1–2, *V\*_L* is small and noisy. Floor the denominator, or start at L_lo = 3, where 2024 subclasses arrive.
3. A path-dependent penalty (e.g. "respec cost") would break DP. Avoid it.

### 4.4 Risk lenses (optional)

The solver computes exact per-round damage pmfs (discrete convolution), so variance and tail metrics are cheap:

- **Mean–variance:** `J = μ − κσ` (κ ≥ 0 user slider).
- **CVaR_α:** the expected damage in the worst α fraction of rounds. Minimising CVaR (for losses) is tractable ([Rockafellar & Uryasev 2000](https://sites.math.washington.edu/~rtr/papers/rtr179-CVaR1.pdf); [CAS abstract](https://casact.org/abstract/optimization-conditional-value-risk)). For damage we *maximise* the lower-tail mean.
- **Floor probability:** P(D_r ≥ x), e.g. "chance to deal at least 20 this round". Graze shows up here.

Risk-sensitive objectives are **not time-consistent inside the MDP** (CVaR of a sum ≠ sum of CVaRs), so the MVP should **optimise the rotation for expectation and apply risk lenses as a re-ranking of the candidate pool**. Label this clearly in the UI.

---

## 5. Top-K diversity

Problem: the raw #2 and #3 are usually #1 with a different background or a +1 shuffle. The spec wants runner-ups that "expose concrete mathematical trade-offs" (§5.4).

1. **Build signature** (equivalence key): (class split rounded to dip/major, subclasses, weapon archetype {GWF, sword-and-board, Nick dual-wield, ranged, polearm, unarmed}, DPR-relevant feat set). Keep only the best build per signature. Collapsed variants appear under each card as **sidegrades**, like Raidbots' grouping of "statistically close" results ([Icy Veins](https://icy-veins.com/wow/how-to-use-the-top-gear-feature-of-raidbots), secondary).
2. **MMR re-rank** among the signature winners: `next = argmax_b [ λ·score(b)/score(#1) − (1−λ)·max_{s∈S} sim(b, s) ]` ([Carbonell & Goldstein 1998](https://www.cs.cmu.edu/~jgc/publication/MMR_DiversityBased_Reranking_SIGIR_1998.pdf)), λ ≈ 0.7. `sim` is a weighted Jaccard/Hamming over signature fields, plus **behavioural** similarity: cosine similarity of the AC-sensitivity curves. That makes "different shape across AC" count as different.
3. **Quality floor:** reject any candidate below 90% of #1's score (configurable), so diversity never shows junk.
4. Literature for stronger guarantees: MAX-DIVERSE-kSET in CP ([Hebrard et al. AAAI 2005](https://cgi.cse.unsw.edu.au/~tw/hhowaaai05.pdf)); diverse high-quality solutions to constraint optimisation ([Petit & Trapp, IJCAI 2015](https://ijcai.org/proceedings/2015/43)); quality-diversity archives ([MAP-Elites](https://arxiv.org/abs/1504.04909)). The MAP-Elites niche grid can double as the signature grid.

---

## 6. Interactive performance plan

**Targets:** ~0.1 s feels instantaneous and 1 s keeps the user's flow uninterrupted ([Nielsen, NN/g](https://www.nngroup.com/articles/response-times-3-important-limits/)). **Budget: ≤100 ms for slider feedback** (AC, rounds, advantage), **≤1 s for constraint changes** (class locks, level, mode), with progress shown beyond that.

**Layered compute:**

1. **Offline (build-time or server cron):** for each partition (level 1–20 × class-combo [≤3 of 6 = 41 combos] × mode × day preset), enumerate or B&B and store a **candidate pool** of ≤ ~500 builds. The pool is the union of the top-N at every grid point (AC 10–25 × rounds 1–10 × advantage {dis, none, adv} × resistance {off, on}). The AC slider is integer and finite, so the grid is exact for those sliders. Storage is roughly 20 × 41 × 2 × 3 × 500 ≈ 2.5 × 10^6 build references. These are compact canonical encodings of tens of bytes, so the pool is **tens of MB, split by level and fetched lazily** **(estimate)**.
2. **Client (Web Worker + WASM):** on a slider change, re-score the current partition's pool exactly at the new parameters. That is ≤500 builds × ≤~100 µs ≈ **≤50 ms**, run off the UI thread ([MDN Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API) (nf); [WebAssembly](https://webassembly.org/) (nf)). Then apply diversity selection. Class whitelists are answered by **merging** top-K lists across the allowed class-combo partitions. Species locks are answered by filtering, which needs a pool per species when a lock is active, or a larger pool.
3. **Fallback (rare parameters such as custom resistances, Magic Resistance, odd AC offsets):** run B&B in the worker, seeded with the pool's best as the incumbent bound. It returns progressive results: show the pool's re-scored answer immediately and replace it when B&B finishes.

**Incremental re-solving:**

- AC changes only p_hit. Precompute each kit's DPR *vector over all 16 ACs in one pass*, so AC slider moves are a column lookup.
- Changing rounds N changes the MDP horizon. Backward induction already computes values for all horizons ≤ N, so store the value table per N.
- Level changes swap partitions. Prefetch L ± 1.
- **Cache key:** (ruleset version, data-corpus hash, kit hash, encounter tuple, day model, mode). Include the corpus hash so that rules-data updates invalidate the caches.

**Is precomputing the full MVP solution table feasible?**

- Single-class, all levels: ~10^10 state evaluations at 10–100 µs compiled is about 1–10 CPU-days. With memoisation of kits (many states share a kit) and dominance, expect hours on a multi-core machine **(estimate)**. **Feasible.**
- ≤2 classes: ~10^11–10^12. Feasible only with B&B or beams, on a cloud batch job.
- ≤3 classes: ~10^13–10^14. **Not feasible exhaustively.** Precompute with beam-DP and admissible bounds, and record an *optimality gap certificate* per partition (best found vs best bound) so the UI can say "proven optimal" or "best found (gap ≤ x%)".

**Server vs client:** the MVP should run entirely client-side (pool + worker). That means no server cost per slider event, offline use, and privacy. A server or batch tier is needed only for the offline precompute and for fallback-heavy custom encounters at full scope.

---

## 7. Prior art

| Tool | What it does | How it searches | What transfers |
|---|---|---|---|
| SimulationCraft (WoW) | Combat simulator | Hand-authored Action Priority Lists scanned top-down ([SimC wiki](https://github.com/simulationcraft/simc/wiki/ActionLists)). Repeated simulation to a target error ([Wowhead](https://cn.wowhead.com/news=284548), secondary) | Text-defined rotation DSL; separation of rotation from gear; "stat scaling" plots ([StatsScaling](https://ddprojects.chickenkiller.com/simulationcraft/simc/wiki/StatsScaling), mirror) ≈ our AC-sensitivity curves |
| Raidbots Top Gear / Droptimizer | Cloud front end to SimC | "Every combination of them will be simulated". Statistically close results grouped as sidegrades ([Icy Veins](https://icy-veins.com/wow/how-to-use-the-top-gear-feature-of-raidbots), [Maxroll](https://maxroll.gg/wow/resources/simulationcraft-and-raidbots-guide)) | Brute force over a *user-chosen* candidate set; sidegrade grouping for top-K |
| Path of Building Community (PoE) | Offline build planner ([GitHub](https://github.com/PathOfBuildingCommunity/PathOfBuilding)) | Incremental calculator; "node power" heat map ranks nodes by per-point power ([pathofbuilding.community](https://pathofbuilding.community)) | Marginal-value heat maps ("what would +1 feat/ASI here give?") as an explanation UI |
| poe-optimizer | Passive-tree optimiser on PoB's engine | Greedy marginal improvement (default) and experimental GA; ~0.2 s per candidate in parallel LuaJIT workers ([GitHub](https://github.com/alecrivet/poe-optimizer)) | Greedy plus incremental re-evaluation is fast but myopic. It confirms that evaluator speed is the bottleneck |
| wolfwood/dndamage | 5e expected-DPR calculator in Rust, GPL-3.0 ([GitHub](https://github.com/wolfwood/dndamage)) | Compares user-specified abilities; no build search found | Rust evaluator, a candidate for a WASM core (licence matters) |
| AClaybrook/DAndDSimulator | 5e/BG3 damage simulator: vectorised NumPy Monte Carlo plus an analytic module ([GitHub](https://github.com/AClaybrook/DAndDSimulator)) | "Simulate them all" over user-chosen feats; leaderboard of example builds | Analytic-plus-Monte-Carlo cross-check pattern |
| RPGBot DPR calculator | Web DPR calculator ([rpgbot](https://rpgbot.net/dnd5/tools/dpr-calculator)) | Evaluates entered builds; method not documented | Baseline for UX comparison |
| Pathbuilder 2e (PF2e) | Character builder | Manual planner; no search found **(unverified)** | — |

**Finding:** no open-source 5e tool was found that *searches* across builds. Existing tools evaluate builds the user supplies ([DAndDSimulator](https://github.com/AClaybrook/DAndDSimulator), [dndamage](https://github.com/wolfwood/dndamage), [rpgbot](https://rpgbot.net/dnd5/tools/dpr-calculator)). Community "optimisers" for multiclassing are hand-curated guides ([GiantITP thread](https://forums.giantitp.com/showthread.php?p=23349924)). The absence claim is **unverified**: it rests on a limited search.

---

## 8. Open questions

1. Exact 2024 PHB counts (backgrounds, general feats, fighting-style feats, weapon table, mastery counts per class) and whether the custom-background rule is in scope. These change the sizing by a constant factor. Pull them from the 5e.tools corpus.
2. The default adventuring-day model. Needs a product decision, given that the 2024 DMG is silent on encounters per day ([EN World](https://www.enworld.org/threads/dungeons-dragons-has-done-away-with-the-adventuring-day.707237/)).
3. Should Sustained in "resource-light" mode forbid *all* daily resources, or allow short-rest resources (Action Surge, Channel Divinity)?
4. Is pre-combat buffing (Hunter's Mark before initiative) allowed by default in Nova?
5. How "different" must #2 and #3 be? Pick values for the signature fields and the MMR λ and quality floor, ideally by user testing.
6. Should the Smooth Curve weights reflect campaign time-at-level? Needs data or a product preset.
7. Licence compatibility if a GPL-3.0 evaluator (dndamage) were reused.
8. How large is the real evaluator's per-kit cost? Benchmark early. The §6 plan depends on ≤100 µs per (kit, encounter).

---

## Candidate ADRs

**ADR-A: Separate build search from rotation solving, with rotation as an exact per-kit MDP**
- *Decision:* the outer optimiser never chooses actions. Each build compiles to a *combat kit*, and the rotation is solved by finite-horizon backward induction with outcome-dependent decisions (e.g. smite after a hit or crit). Results are memoised by kit hash.
- *Rationale:* 2024 rules make several decisions post-roll (Divine Smite "immediately after hitting"), so fixed priority lists leave damage on the table. Memoising on kits collapses many builds.
- *Rejected:* SimC-style hand-written priority lists (cheaper, but suboptimal and labour-intensive per class); joint search over build and actions (the space explodes); Monte Carlo rotation evaluation (noisy rankings at close margins).

**ADR-B: Precomputed candidate pools plus exact client-side re-scoring, instead of live full search**
- *Decision:* offline enumeration or B&B produces per-partition pools covering a discrete encounter grid. The browser re-scores pools in a Web Worker/WASM, and B&B is the fallback.
- *Rationale:* live exhaustive search is about 10^7–10^13 evaluations, far beyond 100 ms. Pools give exact answers on the slider grid, with no per-interaction server cost.
- *Rejected:* server-side live solving (latency, cost); a fully materialised answer table (combinatorial in filters); heuristics-only live search (no optimality claim).

**ADR-C: Progression is modelled as a level-DAG and solved by DP; Smooth Curve objectives must be DP-decomposable**
- *Decision:* level orderings are paths through canonical states. Curve objectives are restricted to forms that decompose along paths: bottleneck/leximin, additive, and power mean.
- *Rationale:* this removes the ~10^7× ordering symmetry and keeps exactness. It constrains future objective design, which is why it is worth recording.
- *Rejected:* enumerating level sequences; GA/SA over sequences (no optimality guarantee); path-dependent penalties (break DP).

**ADR-D: Expected value is the optimisation target; risk measures re-rank but do not drive the rotation**
- *Decision:* the MDP maximises expectation. Variance, CVaR and floor probability are computed exactly and offered as re-ranking lenses.
- *Rationale:* CVaR and mean–variance are not time-consistent inside a multi-round MDP. Expectation keeps the inner solve exact and cacheable.
- *Rejected:* risk-sensitive MDPs (complex, slower, hard to explain); Monte Carlo risk estimates.

**ADR-E: Top-K = best-per-signature plus MMR with a quality floor**
- *Decision:* collapse candidates by build signature, re-rank with MMR using structural and AC-curve similarity, and show collapsed variants as sidegrades.
- *Rationale:* without this, the spec's "runner-up builds that expose trade-offs" fails in practice. The signature definition becomes a domain concept that the UI and cache both depend on.
- *Rejected:* raw top-3 (trivial variants); clustering of the full pool (unstable, opaque); exact MAX-DIVERSE-kSET (overkill for k = 3).

(CP-SAT was considered as the core optimiser and rejected. That decision is easy to revisit, so it is not listed as an ADR.)

## Glossary candidates

| Term | Definition | Avoid |
|---|---|---|
| **Build** | A complete set of character choices (species, background, origin feat, ability array, class levels and order, subclasses, feats, loadout, fighting style) evaluated by the solver. | "character", "sheet" |
| **Build state** | The canonical snapshot of a build at one level, independent of the order in which choices were made. | "sub-build", "partial build" |
| **Combat kit** | The combat-relevant projection of a build state (attack bonuses, damage, riders, resources, actions, masteries) that the rotation solver consumes. | "stat block", "loadout" (loadout is weapons only) |
| **Loadout** | The weapons (and shield/armor) carried and their mastery assignments. | "gear set", "kit" |
| **Rotation** | The optimal policy mapping combat state to a turn plan for a given kit and encounter. | "combo", "priority list", "APL" |
| **Turn plan** | One legal combination of Action, Bonus Action, object interactions and attack ordering for a single turn. | "turn", "sequence" |
| **Encounter profile** | The target and fight parameters: AC, save bonuses, rounds, advantage state, resistances. | "dummy", "scenario" |
| **Adventuring day model** | The assumed number of encounters, rounds per encounter and rest schedule used for Sustained scoring. | "day", "rest frequency" |
| **Sustained DPR** | Expected damage per round averaged over an adventuring day model, spending resources optimally within it. | "average DPR", "steady DPR" |
| **Nova DPR** | Expected damage per round over rounds 1–2 with all resources available. | "burst", "alpha strike" |
| **Level score** | A build's objective value at one level, normalised by the best achievable value at that level under the same constraints. | "relative power", "rating" |
| **Smooth Leveling Curve** | The objective over level scores across a level range that penalises troughs (default: mean minus trough penalty; strict: leximin). | "progression score", "curve mode" |
| **Build signature** | The coarse key (class split, subclasses, weapon archetype, key feats) used to decide whether two builds are meaningfully different. | "archetype", "family" |
| **Sidegrade** | A build with the same signature as a shown build and a score within tolerance of it, collapsed under that build's card. | "variant", "alt" |
| **Candidate pool** | The precomputed set of builds per partition that is guaranteed to contain the top-K at every encounter grid point. | "cache", "shortlist" |
| **Partition** | The slice of the search space defined by level, class combination, mode and day model, for which a pool is computed. | "bucket", "shard" |
| **Ability profile** | The projection of final ability scores onto the scores and parities that affect the objective (plus multiclass thresholds). | "stat array" (that is the raw six scores) |

---

## Appendix: computation scripts (scratch, not in repo)

- `/private/tmp/claude-502/mm5e/size.py`: enumerates point-buy arrays, applies all background bonus splits, computes Pareto fronts and projections, and counts class-level sequences (surjection formula) and end-state splits.
- `/private/tmp/claude-502/mm5e/dp.py`: counts level-DAG states: Σ over class sets ≤ maxc and level splits of (starting-class choices × 4^(classes ≥ L3) × C(K+s−1, s) feat multisets).
- `/private/tmp/claude-502/mm5e/bench.py`: toy closed-form expected-DPR evaluator timed over 10^5 builds × 16 ACs in CPython.

# 14: Prior art for the architecture's hard problems, plus three more measurements

Status: research note, 2026-11-27. For each hard problem in [`architecture.md`](../architecture.md), this note asks how others solve it, then measures what transfers. The measurements extend [research 13](13-tape-spike.md): same machine and spike, with new modes `reduce` and `occ`. **[measured]** marks a spike result, and **[unverified]** marks a claim taken from a search summary without reading the primary text.

## TL;DR

| Problem | Closest prior art | What transfers | Evidence |
|---|---|---|---|
| Compile once, query many | Darwiche's arithmetic circuits: a Bayesian network compiled once into a circuit that is "evaluated and differentiated in time and space linear in the circuit size" ([Darwiche, UAI 2000](https://arxiv.org/pdf/1301.3847); [Chavira et al.](https://web3.arxiv.org/pdf/1207.1372)) | A Tape is an arithmetic circuit with max nodes. An upward pass gives values; a **downward (occupancy) pass gives all derivatives at once** | [measured] exact attribution (error ≤ 7e-16), exact sensitivities = finite differences, cost of one pass |
| Shrinking the state graph | ROBDD reduction: merge isomorphic subgraphs and drop redundant tests, enforced by a unique table ([Brace, Rudell & Bryant](https://khoury.northeastern.edu/~pete/courses/Decision-Procedures/2007-Fall/readings/Brace_Rudell_Bryant_Efficient_Implementation_BDD_Package.pdf); [BDD](https://en.wikipedia.com/wiki/OBDD)). MDP bisimulation minimisation ([Givan, Dean & Greig 2003](https://www.cse.iitm.ac.in/~ravi/papers/TR01-43.pdf)); used in the Storm model checker ([Storm](https://arxiv.org/pdf/2002.07080v2)) | Hash-cons the tape after tracing. Canonicalisation (research 13) goes *beyond* bisimulation: it merges states that are not structurally equivalent, using declared resource dominance | [measured] 1.8–2.4× fewer nodes on top of canonicalisation, bit-identical |
| Pruning choices | MacQueen-style action elimination: drop an action once its upper bound falls below another's lower bound ([Even-Dar et al.](https://www.cs.tau.ac.il/~mansour/papers/06jmlr.pdf), [ICML 2003](https://www.aaai.org/Papers/ICML/2003/ICML03-024.pdf)) | Not needed per kit yet (the tape is cheap). It is the principle behind resource canonicalisation, and it can be applied to `Decide` nodes at trace time when one option dominates for *all* params | — |
| Stat weights / "what is +1 worth" | SimulationCraft *scale factors*: add a delta to one stat, re-simulate, divide the change in DPS by the delta ([SimC wiki](https://ddprojects.chickenkiller.com/simulationcraft/simc/wiki/StatsScaling)); needs extra simulations and carries noise | **Exact** sensitivities from one occupancy pass. **+1 to-hit is exactly a one-lane AC shift**, so the 16 AC lanes already contain it | [measured] bit-exact |
| Explaining the rotation | VIPER extracts small decision trees from a policy, weighting states by Q-value loss ([Bastani et al., NeurIPS 2018](https://arxiv.org/pdf/1805.08328)) | The Combat Script is a *fitted rule plus its exact cost*, weighted by occupancy × regret | [measured] simple rules lose 2.3–35% |
| Competing 2024 tools | DPR Lab ([dndprovinggrounds.com](https://dndprovinggrounds.com/dpr-calculator)), dnd5e-dpr-calc ([oefi](https://github.com/oefi/dnd5e-dpr-calc)), Fernflower ([fernflowerpress](https://fernflowerpress.com/pages/dpr-calculator)) | All evaluate builds; none searches. DPR Lab budgets resources on fixed schedules (smite "budgeted for 12 rounds per day", Vex "advantage on 50% of attacks") and shows averages only, against a fixed AC schedule | Confirms research 06: the solver and the optimal rotation are the differentiators |
| Cross-platform float determinism | Rapier: a deterministic WASM build "cross-platform deterministic across browsers"; avoid `Math.sin`-style transcendentals; a deterministic feature that excludes some SIMD ([Rapier](https://rapier.rs/docs/user_guides/javascript/determinism), [Rust](https://www.rapier.rs/docs/user_guides/rust/determinism)). WASM relaxed SIMD is *locally* deterministic only; its FMA may round once or twice ([relaxed-simd discussion](https://www.github.com/WebAssembly/design/issues/1401)) | Ban the `relaxed-simd` target feature and `mul_add`; no transcendentals in scored values | [measured, research 13] one result hash on native, WASM and SIMD128 |
| Deterministic parallel search | Gurobi: results repeat across thread counts *on the same machine*, and it offers a deterministic `WorkLimit` because time limits are not deterministic ([Gurobi](https://support.gurobi.com/hc/en-us/articles/360031636051), [work](https://gurobi.com/documentation/11.0/refman/work.html)) | Separate **outcome determinism** (the final answer: guaranteed, and here cross-machine) from **trajectory determinism** (best-so-far snapshots and cancel points: not guaranteed). Budgets in work units, not seconds | — |
| Incremental caching | Salsa (rust-analyzer): red-green re-validation, **backdating** (an unchanged result keeps its old revision), **durability** levels ([salsa book](https://docs.rs/crate/salsa/0.26.1/source/book/src/overview.md), [rust-analyzer](https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html)) | Fingerprints include only the Rulings a shape actually *read* while compiling and tracing. Flipping a Ruling re-evaluates only the kits that depend on it. That makes ADR-0006's "DPR change if this Ruling flips" cheap | — |
| Per-stage dominance pruning | Skyline operator ([Börzsönyi, Kossmann & Stocker, ICDE 2001](https://www.cs.ucr.edu/~ravi/CS236Papers/skyline-operator.pdf)); presorting by a monotone score before block-nested-loops ([survey](https://arxiv.org/pdf/1704.01788)) | Presort each factor table by a monotone score, then a window scan: O(n·w) with a small window | — |
| Top-k over a join without materialising it | Any-k ranked enumeration: "time to the kth answer O(n + k log k)" for ranking functions over a selective dioid ([Tziavelis et al.](https://arxiv.org/abs/2205.05649)) | DPR is not decomposable over factors, so any-k doesn't apply directly. Its *lazy, bound-ordered* successor generation is the fallback if a multiclass lattice gets too big to materialise | — |
| Diverse top-3 | MMR is the standard baseline; k-DPPs model whole-set diversity ([Chen et al., NeurIPS 2018](https://papers.NeurIPS.cc/paper_files/paper/2018/file/dbbf603ff0e99629dda5d75b6f75f966-Paper.pdf); [survey](https://arxiv.org/pdf/2212.14464)) | At k = 3, **exact** subset selection over the top ~50 signature winners is C(50,3) = 19,600 subsets, which is trivial. ADR-0018 rejected exact selection as "overkill", but it costs nothing. The open question is the objective, not the algorithm | — |
| Exact dice maths | Icepool: dynamic programming that iterates over die *outcomes* "vertically" instead of over dice ([Liu, AIIDE 2022](https://ojs.aaai.org/index.php/AIIDE/article/view/21971)) | Relevant only to future dice-pool mechanics (keep-highest pools, Elven Accuracy is k = 3). Our d20 closed forms already cover the MVP (research 03) | — |

## New measurements

### A. BDD-style reduction after canonicalisation [measured]

After tracing, a post-order pass applies Bryant's two rules through a unique table. Nodes with the same kind and the same (terms, reduced child) edges merge, and single-option `Decide` nodes with no terms are bypassed.

| Canonical shape | Nodes → reduced | Eval per kit (16 ACs) | Bit-identical |
|---|---|---|---|
| 2 atk, no slots, 3 rds | 49 → 27 (1.8×) | 3.3 → 2.6 µs | yes |
| slots 4/3, 3 rds | 78 → 42 (1.9×) | 5.0 → 3.6 µs | yes |
| 4/3/3/3/2, 10 rds | 3,584 → 1,861 (1.9×) | 232 → 91 µs | yes |
| 4/3/3/3/2, 12 rds | 9,586 → 4,789 (2.0×) | 367 → 227 µs | yes |
| 4/3/3/3/2, 12 rds, no Hew/Vex | 6,517 → 2,682 (2.4×) | 235 → 137 µs | yes |
| 4/3/3/3/2, 20 rds | 143,941 → 77,724 (1.9×) | 5,056 → 3,708 µs | yes |

Reduction costs 0.01–10 ms per shape, paid once and then cached. The two techniques stack: canonicalisation merges states that are *value-equivalent by dominance*, and reduction merges states that are *structurally identical*. Reduction loses per-node state labels, so the tape used for Combat Scripts is not reduced (§C).

### B. The occupancy (downward) pass [measured]

A forward pass under the recorded argmax policy computes the reach probability of every node, and the expected number of times each damage term is applied. The test shape is 4/3/3/3/2 with 12 rounds, 9,586 nodes, three kits:

- **Attribution is exact.** Σ (term uses × term mean) equals E with a maximum relative error of **4.5e-16 to 7.2e-16**. Linearity under the fixed policy holds to machine precision.
- **Sensitivities are exact.** The occupancy of the weapon-hit term (10.54–17.84 expected hits) **equals the finite-difference derivative** dE/d(weapon-hit mean) to four decimals. By the envelope theorem, the derivative of the max equals the derivative at the argmax whenever the policy is locally stable.
- **+1 to-hit equals a one-lane AC shift, bit-exact,** for every lane, because outcome probabilities depend only on AC minus to-hit.
- **Expected resource use falls out:** for example, expected slots used per level at AC 17 were [0.18, 2.20, 2.98, 2.96, 1.97] for L1–L5.
- **Cost:** about one backward pass (the forward pass took 164–795 µs and the backward 166–706 µs; run-to-run noise on P vs E cores is large).

### C. Combat Script fidelity: what simple rules cost [measured]

Any fixed policy is evaluated *exactly* by the same forward pass. Human-readable smite rules on the 12-round shape, for three kits:

| Rule | Loss at AC 13 / 17 / 21 | Agreement with optimal (reach-weighted) |
|---|---|---|
| Smite crits only, highest slot | **17.5–35.5%** | 47–60% |
| Smite every hit, highest slot | 1.2–8.2% | 18–37% |
| Crit: highest; hit: lowest | 2.6–5.7% | 23–37% |

- **Low agreement doesn't mean big loss.** Many decisions are near-indifferent, so a rule can disagree with the optimal choice most of the time and still lose little. The Combat Script should therefore be judged by its **value loss**, not by how often it matches.
- **"Smite crits only" is expensive** in this model. That is exactly the kind of overstated folk wisdom research 02 §7 flagged in the spec.
- **Labels must express canonical-invariant predicates.** In canonical states the slot count is clamped to the turns left, so a test like "slots > turns left" can never fire. Its correct form is "≥", and with 15 slots and at most one per turn over 12 turns it is always true. Script rules are therefore phrased in predicates that canonicalisation preserves, such as "at least one slot per remaining turn" or "has a level ≥ k". Finalists can instead be traced uncanonicalised, since they are few.

## Implications for the architecture

1. Add **reduction** to the trace pipeline (canonicalise → trace → reduce → allocate value slots). The Screen tape is reduced; the Script tape is canonical and labelled.
2. Add an **`Occupancy` algebra**. It replaces carrying `E_by_source` through the Screen pass, so Screen carries only (E, E²). It powers attribution, sensitivities ("what +1 is worth"), expected resource use, and exact evaluation of any fixed policy. It runs for finalists only.
3. **The Combat Script is a fitted rule plus its cost.** Search a small rule grammar (VIPER-style, weighting states by occupancy × regret) for the simplest rule within a tolerance (default ≤ 1% loss). Show the optimal play and that rule, with the exact loss in DPR.
4. **Determinism rules:** ban `relaxed-simd` and `mul_add`; allow no transcendentals in scored values; compare with explicit `<`/`>` (never `f64::max` on possible NaN). Only `Done` results are claimed to be deterministic. Budgets are counted in work units.
5. **Ruling-aware fingerprints:** record the Rulings read per shape, salsa-style. Flipping a Ruling re-evaluates only the dependent kits.
6. **Skyline with presort** in each join stage. **Exact k = 3 selection** stays optional next to MMR (an ADR-0018 note, not a reversal).

## Not done

- No primary-text reading of Givan, Dean & Greig, MacQueen, or the any-k paper beyond abstracts and search summaries **[unverified details]**.
- The rule-grammar search (implication 3) isn't implemented. Section C hand-picked the rules it tests.
- DPR Lab's page was truncated; its results table and changelog were not read.

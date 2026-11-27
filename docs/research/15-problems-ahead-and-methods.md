# 15: Problems ahead, and what the research says to do about them

Status: research note, 2026-11-27. Research 13 and 14 settled *how the hot path runs*. This note lists the problems still ahead once real content and real users arrive, and for each one the best-supported method in the literature. One spike result is new (§1, `cargo run --release -- meta`). Claims marked **[unverified]** come from abstracts or search summaries, not full texts.

## Summary

| # | Problem | Method the research supports | Decision |
|---|---|---|---|
| 1 | Encodings can be wrong, and nobody can hand-compute the right answer for a 12-round Paladin | **Metamorphic testing**, plus differential testing, plus **mutation testing** to measure how strong the suite is | ADR-0056 |
| 2 | Three-class curves (≈10¹³ states) can't be searched exhaustively | **Beam-stack search** (complete, anytime), with an **optimality-gap certificate** of incumbent / best open bound | ADR-0057 |
| 3 | How well bounds prune is unknown until real content exists | Layered bounds and lazy, bound-ordered generation; measure the prune ratio per partition | ADR-0049, ADR-0057 |
| 4 | The winner depends on assumptions (AC, rounds, advantage) | A **minimax-regret** lens over the pool's grid (robust discrete optimisation) | ADR-0053 |
| 5 | Users ask "why this build and not that one?" | **Contrastive explanations**: explain against the alternative | ADR-0054 |
| 6 | The optimal policy can't be played at a table | **Size-limited decision-tree policies** are often near-optimal; search the smallest rule within a tolerance, priced exactly | ADR-0055 |
| 7 | Performance regressions hide in noisy timings (P/E cores) | Confidence intervals, not best-of-N; **deterministic work counters** as the CI gate | ADR-0050, ADR-0056 |
| 8 | Results must be identical everywhere | IEEE basic operations only; outcome vs trajectory determinism | ADR-0050 (research 13, 14) |

## 1. Correctness without an oracle

**The problem.** There will be 110–150 hand-written encodings (ADR-0002), some drafted by an LLM. A wrong rider silently corrupts rankings. Golden tests need someone to compute the right answer by hand, which stops being practical beyond small cases. That is the *test oracle problem*.

**What the research says.** Metamorphic testing replaces "is this output correct?" with relations that must hold *between* outputs, such as "if X increases, Y doesn't decrease". The standard survey is Segura, Fraser, Sánchez & Ruiz-Cortés, "A Survey on Metamorphic Testing", *IEEE TSE* 42(9), 2016 ([PDF](https://eprints.whiterose.ac.uk/110335/1/segura16-tse.pdf)). It frames the technique as the answer when comparing against an expected output is impractical. Mutation testing measures whether a suite actually checks behaviour: insert a bug and see whether any test fails. In Rust, `cargo-mutants` does this. Its README puts it well: coverage says "what code is reached", while mutants show whether tests "check behavior" ([cargo-mutants](https://WWW.GITHUB.COM/sourcefrog/cargo-mutants); Thoughtworks Radar, "Trial", April 2026, [radar](https://www.thoughtworks.com/en-cn/radar/tools/cargo-mutants)).

**Our domain has unusually strong metamorphic relations.** Two exact facts generate them:

- **Option monotonicity:** adding an *optional* choice (a Rider with `optional: true`, an extra resource unit, an extra round, an extra attack source) can never lower the optimal expected value. The old policy is still available.
- **Parameter monotonicity:** E is non-increasing in target AC, and non-decreasing in to-hit and in the damage of any term.

**[measured]** Six relations over 200 random kits × 3 shapes, about 57,000 checks in total, found **0 violations**: AC+1, +1 slot of any level, +1 round, more smite damage, enabling the Hew option, +1 to-hit. The existing exact identities from research 14 join them: "+1 to-hit = one AC lane", canonical = plain, reduced = unreduced.

**Where they stop holding.** Relations must be stated per content kind, because some content can legitimately lower E. A mandatory drawback, or a resistance on the *target*, is not optional for the player. The relation catalogue is part of each `ResourceDecl` and Rider kind, not a blanket rule.

**Method:**
- every proptest-generated kit runs the relation catalogue;
- every hand-written encoding gets one golden scenario plus the relations;
- `cargo-mutants` runs on `mm-engine` and `mm-dice` before each encoding milestone;
- a surviving mutant in scoring code is a missing test, by definition.

## 2. Three-class curves: complete, anytime, certified

**The problem.** Research 05 §1.4 counts about 6×10⁹ DP states per origin for three-class curves, or 10¹³–10¹⁴ in total, which can't be enumerated even natively. Plain beam search is fast but gives no guarantee.

**What the research says:**

- **Beam-stack search** (Zhou & Hansen, ICAPS 2005, [PDF](https://cdn.aaai.org/ICAPS/2005/ICAPS05-010.pdf)) adds a "beam stack" so that beam search can backtrack systematically. It finds a good solution fast, then keeps improving "until it converges on an optimal one". It is **anytime and complete**. Completeness can be expensive in the worst case ([Lemons et al., 2022](https://arxiv.org/pdf/2204.02929)), which is why the budget matters.
- **A running certificate.** For any best-first or anytime search with an admissible bound, the incumbent's suboptimality is bounded by incumbent / max-bound-over-open-nodes. This is Hansen & Zhou's anytime heuristic search ([JAIR](https://arxiv.org/pdf/1110.2737)), stated for minimisation as G / min f over OPEN. We maximise, so the gap is (max open bound − incumbent) / incumbent.

**Fit.** The level-DAG DP (ADR-0016) is layered by level, which is exactly the setting beam search was built for. Beam-stack search over the layered DAG gives the UI a monotonically shrinking "gap ≤ x%" and, given enough budget, "proven optimal". Budgets are counted in **work units** (kits evaluated), not seconds, so the certificate is deterministic (ADR-0050).

## 3. Bound quality

**The problem.** Every speed estimate (architecture §6.3) assumes bounds prune well. That can only be measured with real content.

**Method.**

- Report **prune ratio = kits evaluated / kits in the lattice** per partition, as a deterministic metric logged by `mm-cli bench`.
- Bounds stack from cheapest to most expensive: factor-level relaxations (research 05 §2.5), then a fixed-λ priced tape (valid per research 13 §2a, and cheap), then the exact tape.
- If the lattice is too large to materialise, generate successors lazily in bound order (any-k style, research 14).

There is no single paper to adopt here. This is standard branch-and-bound practice, and what this note contributes is the measurement plan.

## 4. Rankings that depend on assumptions

**The problem.** The #1 build at AC 17 may be #4 at AC 14. Showing one number invites over-confidence, the "white-room" criticism the product exists to answer (ADR-0024, research 06).

**What the research says.** Robust discrete optimisation's **minimax-regret** criterion (Kouvelis & Yu, 1997; summarised in [Aissi, Bazgan & Vanderpooten](https://optimization-online.org/wp-content/uploads/2007/08/1741.pdf) and [lecture notes](https://www.math.tugraz.at/~lendl/2020-ro/slides/03-regret.pdf)) prefers the option whose worst-case shortfall against the best choice *in each scenario* is smallest. Plain worst-case (max-min) "might have terrible regret". Finding the robust optimum over a whole combinatorial space is usually NP-hard **[unverified detail]**, but **re-ranking a finite pool over a finite scenario grid is trivial**. Our pool already holds exact values at every grid point (ADR-0053).

**Method.** A *Robust* lens: regret(b) = max over scenarios s in the user's range of (best(s) − value(b, s)) / best(s). The user picks the range, for example AC 14–20. It is a re-rank, consistent with ADR-0017 (risk measures re-rank; they never steer the rotation).

## 5. Explaining "why this build"

**The problem.** A ranked list with numbers doesn't build trust.

**What the research says.** Miller's review of the social-science literature on explanation (*Artificial Intelligence* 267, 2019, [arXiv](https://arxiv.org/pdf/1706.07269v3.pdf)) finds that people explain an event *relative to an alternative that did not happen*. Explanations are contrastive and selective, and people do not want complete causal chains.

**Method.** The primary explanation is **#1 against #2**, not #1 alone:

- the difference in attribution by Source;
- the difference in sensitivities ("#2 gains more from +1 to-hit");
- the Crossover AC, where the ordering flips.

All three come from data we already compute (the Occupancy pass and the pool; ADR-0054). Keep it selective: the top 2–3 differences, not a full table.

## 6. A playable rotation

**The problem.** The optimal policy is a table over thousands of states. Research 14 measured that simple folk rules lose 2.3–35%.

**What the research says:**

- **VIPER** (Bastani et al., NeurIPS 2018) imitates a policy with a decision tree, but "cannot prove optimality even if it identifies the optimal solution".
- **OMDT** (Vos & Verwer, IJCAI 2023, [arXiv](https://arxiv.org/pdf/2301.13185)) finds the *optimal* size-limited decision-tree policy for an MDP with a MILP. It reports that "decision trees of 7 decision nodes are enough to perform close to unrestricted optimal policies in 8 out of 13 environments", and that depth-3 trees are often close to optimal.

**Fit.** Our grammar is small (outcome, round band, resource predicate, slot-level choice), and pricing any rule is one exact Occupancy pass (about 0.2 ms). So rules up to a small size can be **enumerated exhaustively and priced exactly**, with no MILP. The result is provably the simplest rule *within the grammar* that meets the tolerance. That is OMDT's guarantee, obtained by brute force because the problem is small. OMDT is the fallback if the grammar grows (ADR-0055).

## 7. Performance regressions on noisy hardware

**The problem.** Research 13 saw run-to-run variation of up to about 1.8× on the M5's mixed cores.

**What the research says.** Georges, Buytaert & Eeckhout ("Statistically Rigorous Java Performance Evaluation", OOPSLA 2007, [PDF](https://dri.es/files/oopsla07-georges.pdf)) show that best-of-N comparisons can reach wrong conclusions where confidence intervals don't. Their example: "best of 30" suggested one collector was faster, but the 95% confidence intervals overlapped.

**Method.** Use two layers:

- **CI gates on deterministic work counters:** tape nodes × lanes evaluated, kits screened, prune ratio. These are exactly reproducible, so any change is real.
- **Wall-clock benchmarks are advisory,** reported with confidence intervals (divan or criterion) and run locally with a pinned thread count.

## 8. Determinism

Covered in research 13 §4 and research 14: IEEE basic operations only; no `relaxed-simd`, `mul_add` or transcendentals; and outcome determinism separated from trajectory determinism. Recorded in ADR-0050.

## Risks not addressed by research

- **Rules-reading errors** (misreading what a feature does) are caught by none of the above if the encoding is internally consistent. The only defences are the citation and source tier per encoding (ADR-0007), the Rulings register (ADR-0006), and comparing against independent calculators on shared cases. The tools listed in research 14 evaluate the same builds, so their numbers are useful cross-checks, though their simplifying assumptions differ.
- **Model fidelity** (a single target, no enemy turns) is a stated scope limit (ADR-0012, ADR-0024), not a defect.

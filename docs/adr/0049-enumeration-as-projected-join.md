---
status: proposed
---

# Build enumeration is a projected join: each factor is reduced to its kit contribution before it is combined

At one level, a Build is a tuple of factors: species, background with bonus split and Origin feat, base ability array, class skeleton, feat/ASI multiset, and Loadout. Each factor is **projected** onto what the Combat Kit can observe. Values with equal projections form a **factor class**, and only one representative of each class takes part in the join.

The solver joins factors in stages. After each join it projects, deduplicates, and applies a **skyline** (Pareto dominance within each class of the other columns). It uses Börzsönyi et al.'s operator in its presorted, window-scan form. Legality (prerequisites, multiclass 13s) is a filter applied where its columns first meet.

The output is the **kit lattice**: unique `(ShapeKey, Params)` entries, each with back-pointers to its factor classes.

- **Intermediate sizes follow distinct kits, not Builds.** Equal kits are merged when they are built, not looked up in a memo afterwards. A process-wide shared memo (ADR-0015's amendment) is therefore no longer needed. That matters because Web Workers share no memory (ADR-0045).
- **Back-pointers let the UI name DPR-neutral alternatives.** For each shown Build the solver rebuilds one canonical Build (lowest Content IDs) and lists the other members of each factor class as equivalents, e.g. "any of these 5 species give identical numbers".
- **Parity is a column.** An odd score keeps its option value for a later half-feat, so a 16 is not dominated by a 17 (research 05 §2.2).
- **Fallback for lattices too large to materialise** (three-class curves, ADR-0057): successors are generated lazily in upper-bound order, after any-k ranked enumeration (Tziavelis et al.). DPR doesn't decompose over factors, so the rank is the bound, not the score.

Correctness never depends on how much the join collapses. A differential test compares the projected join against unprojected brute force on small synthetic packs. A projection that misses an interaction fails that test, and the fix is to widen the projection key.

## Considered Options

- **Enumerate Builds, then dedupe kits through a memo** (DashMap). Rejected: the intermediate counts are the Build counts, and on the web each worker would hold its own memo.
- **CP-SAT / MILP over the whole space.** Rejected as in research 05 §3.2: the objective is a nonlinear simulation.
- **Genetic or annealing search.** Rejected: no optimality guarantee (ADR-0016).

Research: [05](../research/05-solver-and-optimization.md), [14](../research/14-prior-art-and-refinements.md)

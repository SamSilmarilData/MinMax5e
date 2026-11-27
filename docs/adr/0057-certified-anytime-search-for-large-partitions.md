---
status: proposed
---

# Partitions too large to search exhaustively use complete anytime search, and always report a certified optimality gap

Single-class and constrained two-class queries run exhaustive branch-and-bound (ADR-0020). Unconstrained curves and every three-class query cannot: about 10¹³ DP states (research 05 §1.4). For those, the level-DAG DP (ADR-0016) runs as **beam-stack search** (Zhou & Hansen, ICAPS 2005):

- a beam per level finds a good solution quickly;
- a *beam stack* records what each beam pruned, so the search can backtrack systematically and keep improving;
- given enough budget, the search is **complete** and converges on the optimum.

**Certificate.** With admissible bounds (research 05 §2.5), the gap at any moment is:

> gap = (max bound over unexplored nodes − incumbent) / incumbent

This is the running bound of anytime heuristic search (Hansen & Zhou). `Done` carries an `Optimality` value (ADR-0051), and the UI shows either "proven optimal" or "best found, within x%". It never shows a beam result as if it were optimal.

- **Budgets are counted in work** (nodes expanded, kits evaluated), not seconds, so a budgeted result and its certificate are deterministic (ADR-0050).
- **Heavy partitions run in the background,** and their results go to the disk cache (ADR-0038), keyed by fingerprint (ADR-0052). A later query resumes from the cached beam stack instead of starting again.
- **Prune ratio is logged.** Kits evaluated ÷ kits in the partition is a deterministic metric. It decides empirically which partitions can move to the live class as bounds improve.

## Considered Options

- **Plain beam search.** Rejected: no guarantee and no certificate.
- **Weighted A*.** Rejected for this space: the bound is a fixed factor w chosen before the search, while beam-stack search tightens towards the optimum as the budget allows.
- **Genetic algorithms or MAP-Elites as the solver.** Rejected (ADR-0016). MAP-Elites remains a candidate *discovery* tool for diversity (research 05).
- **Limited discrepancy beam search (BULB).** Considered equivalent in guarantees. Beam-stack search is chosen because the level DAG is layered.

Research: [15](../research/15-problems-ahead-and-methods.md) §2, [05](../research/05-solver-and-optimization.md) §2.4, §6

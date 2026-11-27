---
status: proposed
---

# Two-tier evaluation: live search where it fits, with an in-memory pool and a local disk cache

> *Amended 2026-11-27 (web delivery, ADR-0043/0045):*
>
> - **Slider re-scores** go to the **interactive worker**, which holds the in-memory pool. They are a message round trip within one frame, and never queue behind a running solve.
> - **Device-dependent times:** the latency table below is for the native M5. In a browser, expect roughly 1.5–2× those times on a laptop, and more on older phones [estimate, spike W2]. The UI estimates the cost before a heavy solve and labels it a background job when it is. It always shows the best result found so far, which branch-and-bound guarantees.
> - **Share links:** the Assumption Set, constraints, and optionally Builds as Content IDs are encoded in the URL fragment. A friend's browser re-scores a shared result from its own corpus in under a second, with no heavy search. Links carry IDs and numbers, never rules text.

> **Amended 2026-11-27** for the native Rust app (ADR-0032, ADR-0033). The original version had offline-computed pools shipped as build artifacts of a static site. That site no longer exists. Native multithreaded speed makes live search the primary path for most queries, and pools are now local.

Exact damage distributions for every candidate are not interactive in any language. Evaluation therefore has two tiers:

1. **Screen tier:** rank candidates by policy-aware exact moments `(P, E, E²)` for every target AC from 10 to 25 at once. These are cheap and exact, and they let variance-aware re-ranking work without full distributions.
2. **Exact tier:** full damage distributions, and the risk metrics in ADR-0017, are computed only for the top-K finalists.

Where candidates come from depends on the query. These are [estimates] from [research 09 §1.6](../research/09-rust-engine-core.md), pending spike S1:

| Query | Path |
|---|---|
| Single class, snapshot at one level (≈5×10⁷ candidates) | **Live** branch-and-bound, ≈0.1–0.8 s |
| Up to 2 classes, snapshot, with a class or subclass locked | **Live** with streamed progress, ≈5–50 s |
| Single class, Smooth Curve, origin and weapon style locked | **Live** dynamic programming, ≈0.4 s |
| Unconstrained curves; any 3-class curve | Background job, then the **disk cache** (ADR-0038); beam search with an optimality-gap bound for 3 classes |

Once a constraint has changed, slider moves (AC, rounds, advantage, resistance) **re-score the in-memory pool**: the union of the top-N candidates from the last solve across the encounter grid. That keeps slider feedback within the 100 ms budget. The AC slider is a lookup into the per-AC top-K.

## Considered Options

- Exact distributions for every candidate. Rejected: not interactive.
- Monte Carlo everywhere. Rejected: noisy, non-deterministic rankings.
- A fully materialised answer table. Rejected: it grows combinatorially with the filters.
- Shipped precomputed pools (the original decision). Superseded: there is no shipped site, and live search is now fast enough for most queries.

Research: [05](../research/05-solver-and-optimization.md), [07](../research/07-tech-stack-and-architecture.md), [09](../research/09-rust-engine-core.md) §1.6, [10](../research/10-rust-ui-and-app-shell.md)

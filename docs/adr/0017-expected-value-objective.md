---
status: accepted
---

# Expected value is what the solver optimizes; risk measures only re-rank

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-27 (ADR-0053, ADR-0055):*
>
> - **New lens:** a **Robust** (minimax-regret) lens joins the risk lenses. Like them, it only re-ranks the pool.
> - **Combat Script:** the one mean-optimal Policy still drives every chart. The Combat Script shows a *simplified rule* together with its exact DPR cost against that Policy, so a simplification is never presented as the policy itself.

The rotation solve (ADR-0015) and the build search maximize expected damage. Variance, standard deviation, CVaR, and floor probabilities are computed exactly for the finalists. They can re-rank the shortlist and are always displayed, but they don't steer the rotation.

This may surprise readers, given how much weight the spec puts on variance and reliability (§4). CVaR and mean-variance criteria are not time-consistent inside a multi-round decision process, while expectation keeps the inner solve exact and cacheable.

One Policy per build, the DP mean-optimal one, drives every chart, and distributions are reported under it. Fixed heuristics are measurably worse: smiting only on crits loses 5–19 damage per fight.

## Considered Options

- Risk-sensitive decision processes. Rejected: complex, slower, and hard to explain to users.
- A different optimal policy for each metric. Rejected: the charts would describe different ways of playing.

Research: [05](../research/05-solver-and-optimization.md), [03](../research/03-probability-and-damage-math.md)

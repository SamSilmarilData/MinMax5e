---
status: accepted
---

# Each Kit Shape's rotation is traced once into a reduced Tape and evaluated over AC lanes

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-28 (#19):* **Kit Params carry each damage term as its Damage Distribution, not its moments.** The `Pmf` algebra needs the distributions, and moments cannot give them back. The `Moments` algebra reads each term's E and E² from its distribution, so its results are unchanged. `ParamsFp` fingerprints every term by its support and the mass at each total in it, so two terms with equal moments but different spreads no longer share a fingerprint.

A Combat Kit (ADR-0015) is split into a **Kit Shape**, everything that changes which states exist or which choices are legal, and **Kit Params**, the numbers: to-hit, damage-term moments, DCs.

For each shape, the turn state machine (ADR-0012) runs **once** over rounds × resources and records a **Tape**:

- a post-order DAG of `Chance`, `Decide`, `Save` and `Leaf` nodes;
- edges carry damage terms that are read from the Params;
- one root per supported horizon, so every round count is available.

After tracing:

1. **Reduce** the tape. A unique table merges structurally identical nodes and bypasses pass-through `Decide` nodes, which are Bryant's two ROBDD rules.
2. **Allocate value slots** by liveness, so only the live frontier is held in memory.

Every evaluation reads the tape through an **algebra**:

- `Moments` (backward, policy-aware E and E²) for the Screen tier;
- `PolicyRecord`, `Occupancy` (ADR-0054) and `Pmf` for finalists;
- `Rational` and `Sample` (Monte Carlo over the state machine, not the tape) for tests.

Each pass evaluates one kit over **16 lanes** (AC 10–25). Lanes are independent and never sum across each other.

Measured on synthetic kits ([research 13](../research/13-tape-spike.md), [14](../research/14-prior-art-and-refinements.md)):

- **3.4–6.2× faster than per-kit hash-map DP**, bit-identical;
- reduction removes a further **1.8–2.4×** of nodes, bit-identical;
- **2–16% of nodes are live** at once;
- policy-aware E and SD agree with an independent Monte Carlo (|z| ≤ 1.67, SD within 0.17%).

Hooks (ADR-0036) run only while tracing, once per shape. Batching several kits per pass added only 1.2–1.4×, so it is optional. Tapes are cached per `ShapeKey`.

This keeps ADR-0012's single legality authority: the tape is a *recording* of `legal_steps`/`apply`, and a replay test enforces that. It makes the per-kit memo of ADR-0015 a per-shape cache. A Tape is an arithmetic circuit with max nodes, the compiled form Darwiche uses for Bayesian networks: evaluate up, differentiate down.

## Considered Options

- **Per-kit hash-map DP** (the ADR-0015 baseline). Rejected: 3.4–6.2× slower, and per-kit hashing dominates.
- **Batching many kits per shape as SIMD lanes as the main mechanism.** Demoted: measured at only 1.2–1.4×.
- **Symbolic parametric solving** (closed-form rational functions of the params, as in parametric model checking). Rejected: the expression size explodes with `Decide` (max) nodes, and numeric evaluation is already cheap.
- **Labels kept on the reduced tape.** Rejected: reduction merges nodes from different states. A separate unreduced, labelled *Script tape* serves finalists (ADR-0055).

Amends ADR-0015 (memoisation). Research: [13](../research/13-tape-spike.md), [14](../research/14-prior-art-and-refinements.md), [09](../research/09-rust-engine-core.md)

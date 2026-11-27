---
status: accepted
---

# Correctness rests on differential, metamorphic and mutation testing, because most answers have no hand-computable oracle

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

Golden tests need a hand-computed expected value, which isn't practical for a 12-round adventuring day or a multiclass rotation. This is the *test oracle problem*. Three techniques cover it.

**1. Differential testing:** every fast path is checked against an independent slow path.

| Fast path | Reference |
|---|---|
| Tape × Moments | hash-map DP over the same `step()` |
| canonical tape | unclamped tape (ADR-0048) |
| reduced tape | unreduced tape |
| Occupancy attribution | E from Moments |
| sensitivities | finite differences |
| Moments | Monte Carlo over the *state machine* |
| floats | BigRational (ADR-0023) |
| projected join | brute force on small synthetic packs (ADR-0049) |
| Rayon / WorkerPool | Inline |
| wasm32 | native (ADR-0050) |
| every tape path | `apply` replay (ADR-0012) |

The spike has already passed the first eight rows that it can run [measured, research 13/14].

**2. Metamorphic relations,** as surveyed by Segura et al. (*IEEE TSE*, 2016). These are relations *between* outputs, so they need no expected value. The domain has exact ones:

- **Option monotonicity:** adding an optional Rider, a resource unit, a round or an attack source never lowers optimal E.
- **Parameter monotonicity:** E is non-increasing in target AC, and non-decreasing in to-hit and in any term's damage.
- **Identities:** E(to-hit + 1, AC) = E(to-hit, AC − 1) bit for bit, and canonical = clamped-off.

[measured, research 15 §1: six relations, about 57,000 checks, 0 violations]

Each Rider and resource kind declares which relations apply to it. Mandatory drawbacks and target-side effects can legitimately lower E. Proptest strategies run the whole relation catalogue on generated kits.

**3. Mutation testing** measures whether the first two actually check behaviour. `cargo-mutants` runs on `mm-dice` and `mm-engine` before each encoding milestone. A mutant that survives in scoring code is a missing test.

**What still needs people:** reading the rules correctly. That is guarded by citations and source tiers (ADR-0007), the Rulings register (ADR-0006), one golden scenario per encoding, and cross-checks against independent DPR calculators on shared cases, keeping in mind that their assumptions differ (research 14).

All of this runs on the synthetic mini-corpus in CI, which never fetches upstream data (ADR-0028).

## Considered Options

- **Goldens only.** Rejected: they can't be computed by hand at realistic sizes.
- **Line coverage as the adequacy metric.** Rejected: it shows what runs, not what is checked.
- **Monte Carlo as the main oracle.** Rejected: noisy. It is kept as one differential reference (ADR-0023).

Research: [15](../research/15-problems-ahead-and-methods.md) §1, [13](../research/13-tape-spike.md), [14](../research/14-prior-art-and-refinements.md), [04](../research/04-rules-engine-architecture.md) §2.8

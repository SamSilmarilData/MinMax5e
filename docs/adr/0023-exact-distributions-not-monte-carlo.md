---
status: accepted
---

# Exact damage distributions are the source of truth; Monte Carlo is only a test oracle

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

Every probability, percentile, and curve the product displays comes from exact float64 damage distributions propagated through the turn state machine (ADR-0012 and ADR-0013). Damage spreads over few enough values that this is cheap: about 0.04 ms per round for a level-5 Great Weapon Master Fighter, against about 13 ms for Monte Carlo to reach ±0.03 DPR. It is also deterministic, so solver rankings and slider updates never jitter, and rare outcomes such as P(kill in round 1) are exact.

## Considered Options

- Monte Carlo everywhere. Rejected: noisy, about 10⁴ samples per comparison point, and rankings flicker.
- FFT convolution. Rejected: no speed-up at these sizes, and it produced tiny negative probabilities.
- Exact rational arithmetic at runtime. Rejected: too slow. Kept for tests.

*Amended 2026-11-27:* distributions are stored as `Box<[f64]>` in `mm-dice`. `num-rational` BigRational serves as the exact oracle for golden tests. Determinism across threads is covered by ADR-0037.

Monte Carlo remains a test oracle, and a fallback for situations exact calculation can't handle (several targets each with their own HP, or enemy decisions that matter).

Research: [03](../research/03-probability-and-damage-math.md)

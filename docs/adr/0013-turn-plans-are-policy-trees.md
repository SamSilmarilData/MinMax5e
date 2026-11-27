---
status: proposed
---

# Turn plans are decision trees evaluated over hit, miss, and crit branches, not lists of independent attacks

A Turn Plan's later steps may depend on how earlier attacks turned out: smite only after a hit or crit, Vex granting advantage on the next attack, Sneak Attack once per turn. The evaluator branches on each attack's miss, hit, or crit outcome, merges branches that reach the same TurnState, and convolves damage distributions along each branch. Sustained DPR is a Markov chain over the state at each round boundary.

Treating attacks as independent and summing them gives wrong **means**, not just wrong variances. This qualifies research 07's shortcut that "variance is additive": the cheap screening tier (ADR-0020) must still use the policy-aware mean.

## Considered Options

- Summing independent attacks. Rejected: wrong results.
- Monte Carlo as the main evaluator. Rejected: noisy and non-deterministic. Kept as a test oracle.

Research: [04](../research/04-rules-engine-architecture.md), [03](../research/03-probability-and-damage-math.md)

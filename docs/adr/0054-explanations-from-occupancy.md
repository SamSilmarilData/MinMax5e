---
status: proposed
---

# Explanations come from one occupancy pass, and they are contrastive

For each finalist, one forward **Occupancy** pass over the tape, under the recorded optimal policy, gives the reach probability of every state and the expected uses of every damage term and resource unit. From that pass:

- **Damage Source Attribution (the stated method):** expected uses × term mean, grouped by Source. This is exact by linearity of expectation under the fixed policy [measured: relative error ≤ 7.2e-16].
  - **Enablers** that add no damage of their own (Vex's advantage, Topple's prone, Archery's to-hit) are reported as a separate **counterfactual delta**: re-solve with the effect off, with the rotation re-optimised. They are never mixed into the additive bar.
- **Sensitivities:** dE/d(term mean) = expected uses of that term [measured: equals the finite difference to 4 decimals]. **+1 to-hit is exactly the neighbouring AC lane** [measured: bit-exact]. SimulationCraft estimates these "scale factors" with extra, noisy simulations; here they are exact and free.
- **Expected resource use**, for example slots per level per day.
- **The exact value of any fixed policy**, which prices the Combat Script (ADR-0055).

**Explanations are contrastive.** People explain relative to an alternative that didn't happen (Miller, *AI* 267, 2019). The primary explanation is therefore **#1 against #2**, and it is selective: only the 2–3 largest differences in attribution and sensitivity, plus the Crossover AC where they swap. It is not a full table of #1 alone. DPR-neutral alternatives (ADR-0049) answer "does my species choice matter?".

Screen carries only (E, E²); explanation runs for finalists, at about the cost of one extra pass each [measured].

## Considered Options

- **Carrying per-source sums through the Screen pass.** Rejected: it multiplies Screen work by the number of Sources, for every kit, when only finalists need it.
- **Shapley-value attribution.** Rejected: exponential in the number of Sources, and harder to explain than "expected uses × damage".
- **Finite-difference sensitivities.** Rejected: N extra solves, and policy switches add noise.
- **Non-contrastive single-build explanations.** Rejected (Miller 2019).

Research: [14](../research/14-prior-art-and-refinements.md), [15](../research/15-problems-ahead-and-methods.md) §5

---
status: proposed
---

# Level progression is a graph solved by dynamic programming, so Smooth Curve objectives must decompose level by level

Level orderings are paths through canonical states in a level graph: class levels, ASI and feat choices made so far, and so on. Up to three classes at level 20 allow about 7×10^10 level orderings but only about 3,711 end states. Modelling paths removes that ordering redundancy and keeps the solve exact.

The cost: every Smooth Leveling Curve objective must decompose along a path (bottleneck/leximin, additive, or power mean). An objective that depends on the whole path breaks the dynamic programming. That limit on future objective design is why this is recorded.

## Considered Options

- Enumerating level orderings. Rejected: roughly 10^7 times redundant.
- Genetic algorithms or simulated annealing over orderings. Rejected: no optimality guarantee.

Research: [05](../research/05-solver-and-optimization.md)

---
status: accepted
---

# Reliability is headlined as time-to-kill and kill probability, with a low-end floor beside it

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

The dashboard's main reliability figures are **time-to-kill** and **P(kill within K rounds)** against the baseline HP, with the floor (p10, or CVaR₁₀, the mean of the worst 10% of rounds) shown beside them. Standard deviation and coefficient of variation are secondary.

These are in game units and account for overkill. They correctly reward variance for nova play and penalise it for sustained play. In the spec's Build A vs Build B example, both builds average 45 DPR:

- Build A (85% hit rate, Graze) kills a 135 HP target in 3.47 rounds on average, against 3.95 for Build B.
- Build A one-shots a 60 HP target only 1.5% of the time, against 34% for Build B.

SD can't express either point.

## Considered Options

- SD alone. Rejected: symmetric, so it penalises upside as much as downside.
- CV as the headline. Rejected: fine for sorting but hard to interpret.
- A bespoke weighted "reliability score". Rejected: opaque.

Research: [03](../research/03-probability-and-damage-math.md), [06](../research/06-market-users-and-ux.md) (chart forms)

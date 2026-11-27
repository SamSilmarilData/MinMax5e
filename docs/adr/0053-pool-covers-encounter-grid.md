---
status: proposed
---

# The Candidate pool holds exact values over the whole encounter grid; sliders and lenses read it, they don't re-search

**Search narrow, pool wide.**

- **Search:** branch-and-bound evaluates kits only at the user's current advantage, resistance and horizon, across all 16 AC lanes.
- **Pool fill:** survivors (the top-N at any search lane) are evaluated over the full grid: 16 ACs × advantage {dis, none, adv} × resistance {off, on} × every horizon root (ADR-0047).

The prune test compares a kit's bound with the incumbent at every grid point where the kit could still enter the top N. Bounds are tape-free, so computing them over the grid is cheap. The pool is therefore exact across the grid.

**Consequences:**

- **Sliders are lookups.** AC, rounds, advantage and resistance cost zero messages. Only off-grid inputs (custom HP, save bonuses, Magic Resistance, a non-default Adventuring Day Model) go to the interactive worker.
- **Lenses re-rank the pool and never steer the rotation** (ADR-0017):
  - **Expected** (the default);
  - **Reliability** (TTK and Floor, ADR-0025);
  - **Robust:** minimax regret over a user-chosen range, for example AC 14–20. That is the build whose worst shortfall against the best build *at each grid point* is smallest (Kouvelis & Yu's criterion from robust discrete optimisation).

  Finding a robust optimum over the whole build space is hard in general, but re-ranking a finite pool over a finite grid is trivial.
- **Diversity** (ADR-0018) runs on the lens's ranking.

## Considered Options

- **Re-scoring the pool for rounds, advantage and resistance** (ADR-0020 as amended). Superseded: pool fill makes those inputs lookups.
- **Max-min (worst case) as the robust lens.** Rejected: it ignores how well other builds do in the same scenario, so it can prefer a build that is everywhere mediocre. Regret fixes that.
- **Searching directly for the robust optimum.** Rejected: it doesn't decompose, and the re-rank answers the question users actually ask.

Amends ADR-0020. Research: [15](../research/15-problems-ahead-and-methods.md) §4, [05](../research/05-solver-and-optimization.md) §6

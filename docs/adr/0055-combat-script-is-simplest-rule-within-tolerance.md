---
status: proposed
---

# The Combat Script shows the simplest play rule within a tolerance of optimal, with its exact DPR cost

The optimal policy (ADR-0015, ADR-0017) is a table over thousands of states, and nobody can play it at a table. Simplifying it has a real price. In the synthetic 12-round smite model [measured, research 14 §C], "smite crits only" loses **17–35%** and the best hand-picked rule still loses 2.3–4.5%.

The Combat Script is therefore built from a small **rule grammar**:

- the attack outcome (hit or crit);
- a round band (opener, sustained, last turns);
- resource predicates (for example "at least one slot per remaining turn", "has a slot of level ≥ k");
- an action (which option, which slot level).

The engine **enumerates rules up to a size limit**. It prices each one *exactly* with an Occupancy pass under that rule (ADR-0054), at about 0.2 ms each, and selects the **smallest rule whose loss is within the tolerance** (default 1% of DPR). The Script shows:

- the rule, as the Combat Script's Round 1 Opener and Sustained Loop;
- its cost against optimal play, in DPR ("this rule gives up 1.8 DPR");
- where it differs from optimal play, weighted by occupancy × regret.

Agreement rate is never shown as a quality measure. Many decisions are near-indifferent, so rules matching 18% of optimal decisions lost only 1–8% [measured].

**Rule predicates must survive canonicalisation** (ADR-0048). The Script tape is canonical, so predicates must be ones canonicalisation preserves. "More slots than turns left" can never be true there; "at least one per turn left" can. Finalists may also be traced without canonicalisation, since there are few of them.

**Prior art:**

- **VIPER** (Bastani et al., 2018) imitates a policy with a decision tree, but can't prove its tree is the best of its size.
- **OMDT** (Vos & Verwer, IJCAI 2023) finds the *optimal* size-limited tree policy with a MILP, and reports trees of 7 decision nodes performing close to optimal in 8 of 13 environments.

Our grammar is small and exact pricing is cheap, so exhaustive enumeration gives OMDT's guarantee (the simplest rule *within the grammar*) without a solver. OMDT is the fallback if the grammar grows.

## Considered Options

- **Show the optimal policy as a full decision tree.** Rejected: unreadable.
- **Hand-written priority lists per class** (SimulationCraft APLs, research 05). Rejected: they cost labour and their loss is never stated.
- **VIPER-style imitation.** Rejected: there is no guarantee, and exact pricing makes it unnecessary.
- **Rank by agreement with the optimal policy.** Rejected: it tracks the cost of a rule poorly [measured].

Research: [14](../research/14-prior-art-and-refinements.md) §C, [15](../research/15-problems-ahead-and-methods.md) §6

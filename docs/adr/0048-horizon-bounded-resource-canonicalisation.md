---
status: accepted
---

# Resource state is clamped to what can still be spent before the horizon

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

Every resource kind in a Ruleset's economy declares a `ResourceDecl`:

- a **spend cap per turn** (2024 spell slots: 1, from the one-slot rule; Action Surge: 1; …);
- whether its units are **ordered**: a higher unit is never worse for any use, as with upcasting slot levels;
- its **refresh** point (short rest, long rest, or per encounter).

At every turn boundary the tracer clamps each resource to `cap × turns_left`, keeping the **highest** units of an ordered resource. States that differ only in units that can never be spent then merge into one node.

Measured on synthetic Paladin-like kits ([research 13 §2](../research/13-tape-spike.md)): **exact** (bit-identical to the unclamped tape) with up to **21× fewer nodes**. The 12-round default adventuring day drops from 4.2 ms to 0.34 ms per kit, which retires spike S2 (research 09's largest risk). It helps least when the horizon exceeds the resource total (20 rounds with 15 slots, about 5 ms per kit). Those partitions are background or cached jobs (ADR-0020).

This is dominance-based state merging. It goes beyond bisimulation minimisation (Givan, Dean & Greig 2003), which merges only states that are behaviourally equivalent, and is related to MacQueen-style action elimination. Soundness rests on the two declared properties and is **not proved** for real content. So a differential test asserts canonical = unclamped, bit for bit, for every golden shape and for proptest-generated `ResourceDecl`s. A wrong declaration fails that test, and the fix is to correct the declaration.

Different per-turn spending in 2014 is just a different cap in `Rules2014`, so the engine stays edition-agnostic (ADR-0008).

The Adventuring Day Model is solved **exactly**, as one canonical MDP across its rounds with refresh points at rests.

## Considered Options

- **Lagrangian λ-pricing of resources** (research 05 §3.3). Rejected after measurement: the bound is valid, but it takes 193–481 tape evaluations to tighten, which is slower than the exact MDP, and it keeps a 1–3% duality gap at 10–12 rounds. It survives only as an optional cheap upper bound for branch-and-bound.
- **Automatic detection of dominated resource units** without declarations. Rejected for now: inferring "never worse for any use" is a proof obligation the tracer can't discharge cheaply.
- **No reduction; accept large MDPs.** Rejected: 4–7 ms per kit makes adventuring-day solves too slow to be live.

Research: [13](../research/13-tape-spike.md), [14](../research/14-prior-art-and-refinements.md), [05](../research/05-solver-and-optimization.md)

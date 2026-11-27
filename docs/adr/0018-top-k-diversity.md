---
status: proposed
---

# The top 3 builds are the best of each Build Signature, re-ranked for diversity

> *Amended 2026-11-27 (note):*
>
> - **Cost of exact selection:** exact k = 3 subset selection over ~50 signature winners is only C(50,3) = 19,600 subsets. "Overkill" below is therefore a judgement about the objective, not about cost. MMR stays the default; exact selection over an explicit objective may be offered.
> - **Order of steps:** diversity runs on whichever lens the user chose (Expected, Reliability or Robust; ADR-0053).

Candidates are collapsed by **Build Signature** (the structural identity of a build, e.g. class split, core feats, and weapon style), keeping only the best of each. The survivors are re-ranked with Maximal Marginal Relevance (Carbonell & Goldstein 1998) using two similarities: structural, and the shape of each build's AC sensitivity curve. A quality floor keeps weak builds out. Collapsed near-duplicates appear as "sidegrades" under each card.

Without this, the spec's "runner-up builds that expose trade-offs" would be trivial variants of the winner, such as the same build with one skill changed. The UI and the cache both depend on the signature definition.

## Considered Options

- Raw top 3. Rejected: trivial variants.
- Clustering the whole candidate pool. Rejected: unstable and opaque.
- Exact max-diversity subset selection. Rejected: overkill for k = 3.

Research: [05](../research/05-solver-and-optimization.md)

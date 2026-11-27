---
status: proposed
---

# The top 3 builds are the best of each Build Signature, re-ranked for diversity

Candidates are collapsed by **Build Signature** (the structural identity of a build, e.g. class split, core feats, and weapon style), keeping only the best of each. The survivors are re-ranked with Maximal Marginal Relevance (Carbonell & Goldstein 1998) using two similarities: structural, and the shape of each build's AC sensitivity curve. A quality floor keeps weak builds out. Collapsed near-duplicates appear as "sidegrades" under each card.

Without this, the spec's "runner-up builds that expose trade-offs" would be trivial variants of the winner, such as the same build with one skill changed. The UI and the cache both depend on the signature definition.

## Considered Options

- Raw top 3. Rejected: trivial variants.
- Clustering the whole candidate pool. Rejected: unstable and opaque.
- Exact max-diversity subset selection. Rejected: overkill for k = 3.

Research: [05](../research/05-solver-and-optimization.md)

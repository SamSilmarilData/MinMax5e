---
status: proposed
---

# Cache keys are fingerprints of what each stage actually read, including only the Rulings it consulted

Every pipeline stage is a pure function from fingerprinted inputs to a fingerprinted output:

- corpus + encodings → `PackFp`;
- Build → kit → (`ShapeKey`, `ParamsFp`);
- shape → Tape;
- Query → `QueryFp`.

The fingerprint is one 128-bit type, `Fp` (blake3, truncated), which is stable across native and wasm.

While compiling a kit and tracing its shape, the engine **records which Rulings it consulted**, and those Rulings alone enter `ShapeKey` and `ParamsFp`. This is salsa's dependency recording (rust-analyzer). Two consequences:

- **Flipping a Ruling re-evaluates only the kits that read it.** ADR-0006 requires that the dashboard can show "how much DPR flipping this Ruling changes". That becomes a re-score of the affected pool entries, not a new Solve.
- **Backdating:** if recompiling gives the same fingerprint, everything downstream is reused.

Pack-derived inputs change rarely and Query inputs change constantly. That durability split decides which caches are checked first.

Because the evaluator is bit-identical across targets (ADR-0050), equal fingerprints mean equal numbers on any device. A Share Link (a serialised Query, ADR-0020) reproduces exact results on the recipient's own data.

## Considered Options

- **Hashing the whole Ruling set into every key.** Rejected: any toggle invalidates everything.
- **Adopting the `salsa` crate.** Rejected: our pipeline is a fixed DAG of a few stages, not an open query graph, and a hand-rolled fingerprint chain is enough.
- **Time-based cache invalidation.** Rejected: non-deterministic.

Strengthens ADR-0006 and ADR-0038. Research: [14](../research/14-prior-art-and-refinements.md)

---
status: proposed
---

# A local disk cache in postcard, keyed by engine version, pack hash, and query partition; bincode is banned

`mm-cache` stores candidate pools and per-level frontiers for expensive partitions (ADR-0020), plus kit memo tables, encoded with postcard, whose wire format has been stable since 1.0. Including the engine version and pack hash in the key makes format stability across releases unnecessary. rkyv may be added later if memory-mapped pools are needed. `bincode` is banned in `deny.toml`: its development has ceased (RUSTSEC-2025-0141), its repository was archived on 2025-08-15, and 3.0.0 is a release that only produces a compile error.

## Considered Options

- bincode. Rejected: unmaintained.
- serde_json for the cache. Rejected: about 2.5× larger and about 9× slower to serialise.
- No cache. Rejected: unconstrained curve partitions take minutes even natively.

Research: [09](../research/09-rust-engine-core.md) §2, [08](../research/08-5etools-ingestion-personal-use.md) §6.4

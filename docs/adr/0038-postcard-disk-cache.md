---
status: proposed
---

# A local disk cache in postcard, keyed by engine version, pack hash, and query partition; bincode is banned

> *Amended 2026-11-27 (ADR-0052):* cache keys are fingerprints of what each stage *read*. That includes only the Rulings a kit consulted, so a Ruling toggle invalidates only the entries that depend on it. Tapes (per `ShapeKey`) and resumable beam stacks (ADR-0057) are cached alongside pools.

> *Amended 2026-11-27:* in the browser, `Storage` (ADR-0042, now ADR-0043) is backed by the **Origin Private File System**: `navigator.storage.getDirectory()` and `FileSystemSyncAccessHandle` in workers, available in Safari 15.2+, Chrome 102+, and Firefox 111+ (MDN). Browser storage can be evicted. WebKit evicts by origin under storage pressure or after ITP's 7 days without interaction, and that includes Cache API and service-worker data. So every cache entry, and the packs, must be rebuildable, and a miss is never an error. The app calls `navigator.storage.persist()`. WebKit grants it by heuristics, such as the app being opened from the Home Screen ([WebKit](https://webkit.org/blog/14403/updates-to-storage-policy/)).

`mm-cache` stores candidate pools and per-level frontiers for expensive partitions (ADR-0020), plus kit memo tables, encoded with postcard, whose wire format has been stable since 1.0. Including the engine version and pack hash in the key makes format stability across releases unnecessary. rkyv may be added later if memory-mapped pools are needed. `bincode` is banned in `deny.toml`: its development has ceased (RUSTSEC-2025-0141), its repository was archived on 2025-08-15, and 3.0.0 is a release that only produces a compile error.

## Considered Options

- bincode. Rejected: unmaintained.
- serde_json for the cache. Rejected: about 2.5× larger and about 9× slower to serialise.
- No cache. Rejected: unconstrained curve partitions take minutes even natively.

Research: [09](../research/09-rust-engine-core.md) §2, [08](../research/08-5etools-ingestion-personal-use.md) §6.4

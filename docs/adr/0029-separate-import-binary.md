---
status: proposed
---

# Importing is a separate step that writes compiled packs to the local cache; no `build.rs`

> *Amended 2026-11-27:* `mm-import` is a **library crate** called by the app (ADR-0041) and by `mm-cli import`, not just a developer binary. That way a friend's install can build its own packs with no developer tooling.

The `mm-import` library:

1. resolves `_copy`, `_mod`, and `_versions` in the raw JSON;
2. assigns each record to a ruleset (ADR-0030);
3. derives the structured fields (ADR-0002);
4. joins in the hand-authored encodings;
5. writes a coverage report;
6. writes `packs/<lockhash>/<ruleset>.pack` (postcard, with a versioned header) to the cache.

The engine loads packs at runtime and refuses stale ones.

This keeps `cargo build` and CI independent of the ignored data, makes it structurally impossible to embed upstream data in a binary (ADR-0028), surfaces data errors at import time, and gives fast startup.

## Considered Options

- `build.rs` code generation or `include_bytes!` of the corpus. Rejected: builds come to depend on ignored data, and the data ends up embedded in binaries.
- Parsing raw JSON on every launch. Rejected: slow, and errors surface late.

Research: [08](../research/08-5etools-ingestion-personal-use.md) §6–7

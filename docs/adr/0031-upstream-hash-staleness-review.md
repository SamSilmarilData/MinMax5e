---
status: proposed
---

# Encodings record a hash of their upstream record, and a version bump triggers a staleness review

Each hand-authored encoding stores the sha256 of its upstream record, in canonical resolved form. When the corpus pin is bumped (ADR-0027), the importer reports:

- in-scope records whose hash changed, as `stale`;
- in-scope keys that were added or removed.

Stale encodings fail the coverage gate until someone reviews them.

5e.tools applies errata in place, with no per-record version, and releases about weekly. Without a hash, a changed record would silently make the solver wrong.

## Considered Options

- Reading the changelog by hand. Rejected: easy to miss, with no guarantee.
- Never bumping the pin. Rejected: errata and new books would be frozen out.

Research: [08](../research/08-5etools-ingestion-personal-use.md)

---
status: proposed
---

# 5e.tools is the local rules corpus, pinned to an exact commit

All rules data, for both rulesets, comes from the `data/` directory of `5etools-mirror-3/5etools-src`. The pinned version is recorded in a committed `upstream.lock.toml`: repository URL, tag, commit SHA, and a sha256 for each file. The roughly 13 MB the MVP needs is downloaded into a local cache outside the repo, file by file, and checked against the per-file hashes. The download runs from the app (ADR-0041) or from `mm-cli fetch` / `cargo xtask fetch`, all using the same library code. The 5e.tools JSON schema (`TheGiddyLimit/5etools-utils`) is pinned by tag the same way.

The corpus has the best-structured metadata of any source and covers 2024 books through November 2026. Its 2014 records are identical to those in `5etools-2014-src`, so one pin serves both rulesets. Each release is a single tagged commit, roughly weekly, so SHA pins are clean. Bumping the pin is a deliberate step that runs the staleness review (ADR-0031).

Supersedes ADR-0001 under the personal-use scope (ADR-0026). How the data is kept out of the public repo is ADR-0028.

## Considered Options

- SRD-only data. Rejected: no GWM, Sharpshooter, Dual Wielder, most subclasses, or smites.
- `5etools-2014-src` for the 2014 sandbox. Rejected: a second pin for identical data.
- A git submodule. Rejected: it pulls the full 180 MB site and visibly ties the public repo to the mirror.
- The release zip as the main fetch. Rejected: 50 MB, mostly site code.
- Tracking `main` without a pin. Rejected: errata would change results silently.

Research: [08](../research/08-5etools-ingestion-personal-use.md)

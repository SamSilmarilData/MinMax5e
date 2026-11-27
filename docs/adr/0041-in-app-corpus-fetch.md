---
status: proposed
---

# The app downloads and updates its own copy of the corpus

Every install builds its own data from the public 5e.tools mirror, so friends never receive data from the author (ADR-0026's sharing rule), and nobody needs developer tooling.

**First launch:**

1. With no packs in the cache, the app shows a setup screen. It says what will be downloaded and where it comes from, and offers a **Download rules data** button.
2. `mm-app-core` calls `mm-import`. It downloads the files listed in the app's built-in `upstream.lock.toml` (pinned commit, per-file sha256) from GitHub, verifies the hashes, and builds the packs into the platform cache directory.
3. Progress and errors (no network, a hash mismatch, GitHub rate limits) are reported in the UI. The download is resumable and makes no other network calls.

The download goes through the `Fetcher` trait, and packs are stored through `Storage` (ADR-0042), so a browser build could use the same flow. `raw.githubusercontent.com` allows cross-origin requests.

**Updates.** The pin is part of each app version, because encodings are reviewed against a specific corpus commit (ADR-0031). A friend gets new data by getting a new app version, never by tracking `main`. The app may offer an **Update rules data** action only when its built-in pin is newer than the cached packs. There is no automatic background update.

**Offline and manual fallback.** A *Load from folder…* option builds packs from a local `5etools-src` checkout or sparse clone the user made themselves, verified against the same hashes.

## Considered Options

- The author sends friends prebuilt packs or a cache folder. Rejected: that is redistributing the data, which ADR-0026 rules out.
- The app tracks the latest 5e.tools release automatically. Rejected: encodings would go stale unreviewed (ADR-0031), and friends' results would diverge from the author's.
- A developer-only fetch command (`cargo xtask fetch`), as originally planned. Rejected for friends: it requires Rust tooling. It is kept for development.

Research: [08](../research/08-5etools-ingestion-personal-use.md)

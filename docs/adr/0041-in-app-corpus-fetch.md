---
status: proposed
---

# The app downloads and updates its own copy of the corpus

> *Amended 2026-11-27 (web delivery, ADR-0043). Verified 2026-11-27 unless marked.*
>
> - **Source order: jsDelivr first, raw GitHub second.**
>   - **jsDelivr URL:** `https://cdn.jsdelivr.net/gh/5etools-mirror-3/5etools-src@<commit-sha>/data/<path>`, pinned by **full commit SHA** (`x-jsd-version-type: commit`).
>   - **jsDelivr responses:** `access-control-allow-origin: *`, `content-encoding: br`, and `cache-control: public, max-age=31536000, immutable`. jsDelivr keeps files at a commit hash "effectively forever" (its README).
>   - **jsDelivr limits:** single GitHub files up to 20 MB (our largest, `items.json`, is 3.0 MB). The terms forbid using it as a general-purpose file host or for scraping; a few dozen JSON files per install is ordinary use.
>   - **Raw GitHub:** GitHub tightened unauthenticated limits on `raw.githubusercontent.com` in May 2025 without publishing a number. Its REST docs give 60 requests per hour for unauthenticated API calls. With about 40 files per import, raw GitHub is the fallback, not the default.
>   - **Unusable:** `codeload.github.com` tarballs only allow CORS from `render.githubusercontent.com`.
>   - **Integrity:** per-file sha256 checks make the mirror's trustworthiness irrelevant.
> - **Size on the wire:** the 23 MVP data files measured 9.7 MB raw and **1.26 MB brotli** from jsDelivr. The first-run cost is small.
> - **Start straight away.** On the web, `index.html` starts the corpus fetch alongside the app download. The import then runs in the solver worker and writes packs to OPFS (ADR-0038).
> - **Consent:** the blocking "Download rules data" button becomes an inline notice that says what was downloaded and from where. The notice links to a settings page with "Re-download" and "Load from file…" (the File API, which takes the release zip or a folder).
> - **Eviction:** Safari deletes script-writable storage after 7 days of Safari use without interaction on the site. That includes OPFS, IndexedDB, Cache API, and service-worker registrations. Home Screen web apps keep their own day counter and are not expected to lose data ([WebKit](https://webkit.org/blog/10218/full-third-party-cookie-blocking-and-more/)).
>   - When packs are missing, the app silently re-downloads and re-imports, with a progress bar. It never shows an error.
>   - iPhone visitors get a one-time "Add to Home Screen" tip.
> - The native build keeps the original flow below.

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

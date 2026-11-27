---
status: accepted
---

# The public repository contains nothing derived from upstream data

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-27 (workspace foundation, #13): details of the leak scan.*
>
> - It reads **staged contents** from the git index, because that is what a commit records. The allowlist is read from the index too.
> - **The allowlist is checked as well.** An exemption with no reason, with no checks, or for a path that is no longer tracked fails the scan.
> - **A JSON fixture that carries the marker passes the data-file check,** because the marker is its proof of being synthetic. The marker must be a top-level field. Any other tracked JSON still needs an allowlist entry.
> - **A fixture Content ID must carry its `|source`,** and the source must be `tst`. An ID without a source fails, because ADR-0008 makes the source part of the identity, and leaving it off would dodge the `tst` check.
> - **Fixture files are RON or JSON** with the marker. Markdown notes about the fixtures are also allowed, and their Content IDs are still checked. Any other file type under `fixtures/` fails.
> - **The `relaxed-simd` check also covers untracked files that git doesn't ignore,** so a local cargo config can't carry the feature either. It matches the hyphenated feature name only; prose about relaxed SIMD does not trip it. Like the other checks, a tracked file may be exempted from it with a reason: the ADRs and research notes that explain the ban name the feature. An untracked file cannot be exempted.

> *Amended 2026-11-27 (walking-skeleton lock): how enforcement works.*
>
> - **Cache location.** The local cache is `directories::ProjectDirs` cache dir (`~/Library/Caches/minmax5e/` on macOS), holding `corpus/<lockhash>/`, `packs/<lockhash>/<ruleset>.pack` and `results/`. `MINMAX5E_CACHE` overrides it, and an override inside a git working tree is refused. The in-repo `.gitignore` entries stay as a second guard.
> - **`cargo xtask leak-scan`** runs in CI and in `.githooks/pre-commit` (enabled with `git config core.hooksPath .githooks`). It fails on:
>   - `{@tag` markup (`\{@[a-z]+[ |}]`) in any tracked file not on the per-file allowlist `xtask/leak-allow.toml`, where every entry states its reason (research notes and ADRs that describe the markup);
>   - any tracked `*.json`, `*.pack` or `*.postcard` outside that allowlist;
>   - any file under a `fixtures/` directory without the synthetic marker (`// fixture: synthetic` as the first line of RON, `"_fixture": "synthetic"` in JSON), or with a Content ID whose source is not `tst`.

> *Amended 2026-11-27:* the **deployed web bundle** (`dist/`, served by ADR-0044) counts as a published artifact, under the same rules as a release binary. It contains code, our encodings, and our own assets only. A CI check fails the deploy on any unexpected file type or any `{@` markup. The corpus is fetched by each visitor's browser, never by CI or the host.

Every recorded takedown notice from WotC or D&D Beyond (2017 and 2024, read in full) targets **publicly hosted copies**, in one case a single file in an unrelated personal repository. MinMax5e's repo is public. Using the data locally is the low-risk case; committing it is the case that gets taken down. So:

- **Must never be committed:** raw 5e.tools JSON, derived JSON or packs, corpus snapshots (including `insta` snapshots and golden files generated from the corpus), Rust source generated from the data, and binaries or Release assets that embed it.
- **Where it lives instead:** a user-level cache outside the working tree. The only committed references to upstream are `upstream.lock.toml` and `rulesets.toml`.
- **Test fixtures** are synthetic (invented names such as `Test Blade|TST`), SRD material with CC-BY attribution, or our own paraphrase. Each is marked `"_fixture": "synthetic"`.
- **Encodings** describe mechanics in our own words and never paste rules text. LLM prompts and outputs that contain upstream prose stay in the cache.
- **Enforcement:** a pre-commit hook and a CI check look for `{@tag …}` markup and untracked JSON. CI never fetches the corpus; any opt-in corpus job is manual-trigger only, with no cache, no artifacts, and logs that show counts only. Every crate sets `publish = false`.
- **Binaries and releases:** a GitHub Release or a zip sent to a friend may contain the app binary only, with no packs, cache, or corpus files. ADR-0029 makes embedding the data impossible by construction.
- **If something leaks:** rewrite history with `git filter-repo` and force-push, assuming forks may already hold copies.

Making the repository **private** would remove most of the remaining risk; that is the owner's call. Full checklist: [research 08 §8](../research/08-5etools-ingestion-personal-use.md).

Supersedes ADR-0003.

## Considered Options

- Committing "numbers-only" derived data. Rejected: it still reproduces the corpus's selection and arrangement.
- Fetching in CI with a cache. Rejected: exposure through public logs and caches, for little benefit.
- Relying on review discipline alone. Rejected: one mistake needs a history rewrite.

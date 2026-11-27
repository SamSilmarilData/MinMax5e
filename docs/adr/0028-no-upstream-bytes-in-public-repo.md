---
status: proposed
---

# The public repository contains nothing derived from upstream data

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

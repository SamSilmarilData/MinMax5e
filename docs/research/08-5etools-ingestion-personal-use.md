# 08: Ingesting 5e.tools data for a personal-use Rust tool

*Researched 2026-11-27. Engineering research, not legal advice. Every claim links to a primary source. **[unverified]** marks what I could not confirm at a primary source, **[estimate]** marks my own numbers, **[design]** marks my recommendations. All data observations come from `5etools-mirror-3/5etools-src` at tag **v2.41.3**, commit `2512db75eb48ef06f6190b086a82430887906dca` (2026-11-23). I fetched it to a scratch directory outside the repo. No 5e.tools data was added to this repo, and this note quotes no non-SRD rules prose.*

## Question

Notes 01 and 04 and ADRs 0001–0005 and 0008 assumed a public product. The project's premises have since changed:

- MinMax5e is **mainly for personal use**: no website and no sales.
- 5e.tools data from its GitHub mirror is the source for all non-SRD rules.
- The engine is **written entirely in Rust**.
- The GitHub repo `SamSilmarilData/MinMax5e` is **public**.

This note asks:

1. Where does the canonical data live, how is it versioned, and how do we pin and fetch only what we need?
2. What must the public repo never contain, and what pattern keeps it clean, including in CI?
3. Which files, source codes, flags and inheritance mechanisms matter for the MVP and the 2014 P1 sandbox?
4. How much of the MVP can a Rust importer derive automatically, and how much must still be written by hand?
5. How do we split content into the 2024 and 2014 rulesets, and what should content identity be now?
6. In Rust, how do we model the JSON, validate it, where does importing run, and how do we cache the result?

## TL;DR

- **Source.** The data lives in `data/` of [`5etools-mirror-3/5etools-src`](https://github.com/5etools-mirror-3/5etools-src). That one repo holds **both** editions (PHB and XPHB side by side).
  - Releases are git tags `vX.Y.Z`, and each release is a single squashed commit. Cadence is 2–7 releases a month (Oct 2025 – Nov 2026).
  - Each release also has a ~50 MB zip asset of the whole site.
  - `data/` is 119 MB. The MVP subset is ~13 MB.
  - Fetch options: a sparse, blobless clone of `data/` at a tag (measured at 16 s), or per-file raw URLs pinned to a commit SHA.
  - A sibling repo, [`5etools-2014-src`](https://github.com/5etools-mirror-3/5etools-2014-src), is a classic-only build. I compared PHB feats, spells, weapons, species and Fighter features: its 2014 records are identical to those in `5etools-src` once metadata fields are ignored. **One pin on `5etools-src` covers both rulesets.**
- **Repo hygiene.** The recorded WotC notices target places where the content is **hosted**:
  - two GitHub Pages mirrors in 2024 ([notice](https://github.com/github/dmca/blob/master/2024/08/2024-08-07-wizards-of-the-coast.md));
  - an earlier GitHub Pages 5etools in 2017 ([notice](https://github.com/github/dmca/blob/master/2017/2017-12-04-WizardsoftheCoast.md)), now verified at the primary source, which 01 could not do;
  - a single PDF file inside an unrelated personal repo ([notice](https://github.com/github/dmca/blob/master/2024/08/2024-08-08-dndbeyond.md)).

  So the public MinMax5e repo must hold **no upstream bytes and no text derived from them**: no raw JSON, no derived JSON or packs, no snapshots, no generated Rust embedding text, and no Release binaries with data baked in.

  The pattern [design]:
  - a committed **lockfile** that holds a SHA and per-file hashes only;
  - a fetch command that writes into a **gitignored cache**;
  - a separate **importer binary**, never `build.rs`;
  - **synthetic fixtures** for tests, with invented names, SRD text or paraphrase;
  - CI that never fetches the corpus by default.
- **Data layout.**
  - Class files hold `class`, `subclass`, `classFeature` and `subclassFeature` arrays.
  - `edition` is set only on classes, subclasses, base items, species and backgrounds. It is **absent from every feat, spell and optional feature**.
  - In all 1,038 spells, 317 feats and 233 optional features, edition can only come from the source code.
  - Wherever `edition` is present, it **never disagrees** with the rule 5e.tools itself uses: a source counts as classic if it was published before XPHB ([`SourceUtil.isClassicSource`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L658)). Checked on 512 records, 0 conflicts.
  - **The 2024 MVP core (XPHB) uses no `_copy` at all.** It only needs `_versions` expansion, for 5 species and Magic Initiate.
  - `_copy` with `_mod` matters for the 2014 sandbox (variant backgrounds and races, subclass features) and for later 2024 books.
- **Automation.** All of these are fully structured and can be imported without hand work:
  - weapons: dice, type, properties, mastery and range;
  - class chassis and tables: Sneak Attack dice, Rage damage, Martial Arts die, mastery counts, Fighting Style and Epic Boon levels, spell slots;
  - feat prerequisites and ability increases;
  - background ability options and origin feats;
  - spell level, casting time, concentration, saves and damage types.

  Spell **dice** are nearly structured: of 139 XPHB damage spells, 105 have exactly one `{@damage}` tag and 62 have a `{@scaledamage base|levels|step}` tag for upcasting.

  What a feature **does** (triggers, conditions, once-per-turn limits) is still prose. For the MVP I estimate **~110–150 hand-authored behaviour encodings** [estimate]. That is fewer than 04's 150–200, because chassis, feats and backgrounds now import automatically.
- **Identity [design].** Keep ADR-0008's ruleset prefix, but **derive the body from the 5e.tools key**: `2024:feat/great-weapon-master|xphb`. Store the exact upstream key and a record hash next to it.
  - A bare `Name|SOURCE` cannot be the identity. Under ADR-0009, `Spirit Shroud|TCE` must exist in *both* rulesets with different encodings.
  - A slug without the source collides: 25 collisions in the 2014 data (mostly species).
- **Rust [design].**
  - `serde` types cover the closed fields. `#[serde(untagged)]` handles the few small unions.
  - `entries` (prose) and `additionalSpells` stay as `serde_json::Value`.
  - `_copy` and `_versions` are resolved on `Value` *before* typed deserialisation.
  - The [`jsonschema`](https://crates.io/crates/jsonschema) crate (0.63.2, draft 2020-12) validates against the pinned 5etools-utils schema as an optional check.
  - Output is a compiled pack in the cache, serialised with [`postcard`](https://crates.io/crates/postcard). **Not bincode:** [bincode 3.0.0](https://crates.io/crates/bincode) is a tombstone release whose `lib.rs` is just a `compile_error!`.
- **Biggest open item [design].** Making the GitHub repo **private** would remove most of the residual risk at no cost to a personal tool. The hand-authored encodings of named non-SRD content (01's open question 1) would no longer be published.

---

## 1. Where the data lives, versioning, pinning and fetching

### 1.1 Repositories

`gh api users/5etools-mirror-3/repos`, 2026-11-27:

| Repo | Size (GitHub, KB) | Created | What it is |
|---|---|---|---|
| [`5etools-src`](https://github.com/5etools-mirror-3/5etools-src) | 181,208 | 2024-09-14 | Current site: code, `data/`, tests. Holds **both** editions. MIT-licensed code ([LICENSE.md](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/LICENSE.md)). |
| [`5etools-2014-src`](https://github.com/5etools-mirror-3/5etools-2014-src) | 174,863 | 2024-09-18 | Classic-only site. Fighter file has one class (`Fighter|PHB`), no `edition` field and no synthetic copies. No `spells-xphb.json`. Tags `v1.221.x`, released in lockstep with `5etools-src` (both pushed 2026-11-23). |
| [`5etools-img`](https://github.com/5etools-mirror-3/5etools-img) / [`5etools-2014-img`](https://github.com/5etools-mirror-3/5etools-2014-img) | 6,995,066 / 4,907,523 | | Images (about 7 GB and 5 GB). **Not needed.** |

There is **no separate data repo**. The data is the `data/` directory of `5etools-src`.

I diffed the 2014 records of `5etools-2014-src@v1.221.9` against `5etools-src@v2.41.3`, ignoring `reprintedAs`, `edition`, `srd`, `basicRules`, `otherSources`, `referenceSources` and `hasFluff*`. They were **identical** for:

- 42/42 PHB feats;
- 361/361 PHB spells;
- 91/91 PHB base items;
- 9/9 PHB species;
- the PHB Fighter class and its 14 class features.

So the second repo adds nothing for P1 except a cleaner shape. Use `5etools-src` for both editions and filter (§5).

The README carries only the MIT line and links ([README](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/README.md)). CONTRIBUTING says only WotC-published data goes in, aims for "a 1:1 copy of the original data", and says to use "the latest version of any published material" ([CONTRIBUTING.md](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/CONTRIBUTING.md)). That last point means **errata change records in place between tags**, with no version marker on the record. See the staleness hash in §7.

### 1.2 Release and tag cadence

- **Tags.** Semver tags such as `v2.41.3` → `2512db75…`, `v2.41.2` → `c286d156…`, `v2.41.1` → `c3a5185c…` ([tags](https://github.com/5etools-mirror-3/5etools-src/tags)).
- **One commit per release.** The `main` history is one commit per release, and its message is the version: `2512db75eb v2.41.3`, `c286d15651 v2.41.2`, … ([commits](https://github.com/5etools-mirror-3/5etools-src/commits/main)). A tag and its commit SHA are therefore interchangeable pins.
- **GitHub Releases.** Each tag has a Release with one asset, `5etools-vX.Y.Z.zip`, of about 50 MB ([releases](https://github.com/5etools-mirror-3/5etools-src/releases)).
  - I downloaded v2.41.3: 50,024,225 bytes, sha256 `492cbd7aa38ebc6051c1b785073ac4ac85321ae95644702af7376890a12770ca`.
  - The zip has no top-level folder; `data/...` sits at its root, with 557 data entries. Its `data/feats.json` is byte-identical in size (419,597) to the tagged file.
- **Cadence.** Releases per month: 2025-10: 4, 11: 6, 12: 3; 2026-01: 6, 02: 2, 03: 5, 04: 4, 05: 4, 06: 3, 07: 5, 08: 3, 09: 4, 10: 3, 11: 7. That is roughly weekly, with bursts around new book releases (`WoW` was published 2026-11-17).

### 1.3 Size, and fetching only the data

| Fetch method | Measured / observed | Pin | Notes |
|---|---|---|---|
| `git clone --depth 1 --branch vX --filter=blob:none --sparse` + `git sparse-checkout set data` | 16 s; 119 MB working tree + 28 MB `.git` | tag → SHA (verify with `git rev-parse HEAD`) | Same pattern [ttrpg-convert-cli's CI uses](https://github.com/ebullient/ttrpg-convert-cli/blob/main/.github/actions/data-cache/action.yml). The sparse set can narrow to the ~13 MB MVP subset (`data/class`, `data/spells`, `feats.json`, `items-base.json`, `items.json`, `races.json`, `backgrounds.json`, `optionalfeatures.json`, `books.json`, `magicvariants.json`, `variantrules.json`, `actions.json`, `conditionsdiseases.json`, `skills.json`, `senses.json`, two `generated/gendata-*` lookups). |
| Raw per-file: `https://raw.githubusercontent.com/5etools-mirror-3/5etools-src/<SHA>/data/<path>` | HTTP 200; `content-length: 419597` for `feats.json` | commit SHA in URL, plus sha256 per file | Easiest from Rust (`ureq` + `sha2`). About 40 files for the MVP, with no git dependency. |
| Release zip asset | 50 MB; sha256 above | tag + sha256 | Whole site, including JS and HTML. Fine as an offline fallback. |
| `codeload.github.com/.../tar.gz/<SHA>` | HTTP 200 | SHA | Whole repo. GitHub doesn't guarantee archive checksums are stable **[unverified]**, so prefer the per-file hash. |

**Recommendation [design].** Make raw per-file at a commit SHA the default fetcher, verified against a committed manifest of per-file sha256 hashes. Keep the sparse clone as an alternative for bulk exploration. A git submodule would also be possible, since it stores only a URL and SHA. I reject it because a recursive clone pulls the whole 180 MB site, and because a visible submodule link from a public repo looks like the project is built around the mirror.

### 1.4 Schema

- **Where.** JSON Schema (draft 2020-12) lives in [`TheGiddyLimit/5etools-utils/schema/site`](https://github.com/TheGiddyLimit/5etools-utils/tree/master/schema/site). The latest tag is `v0.17.40` (pushed 2026-11-20). There are also `site-fast`, `brew` and `ua` variants; `site` and `site-fast` were byte-identical for `entry.json`, `class/class.json`, `spells/spells.json` and `util.json` at this tag.
- **Licence.** 01 recorded the licence as unclear because the repo has no LICENSE file and the GitHub API reports `license: null`. However, [`package.json`](https://github.com/TheGiddyLimit/5etools-utils/blob/master/package.json) declares `"license": "MIT"`. **Resolved: MIT, declared in package.json.**
- **How 5e.tools validates it.** It uses Ajv2020 with `allowUnionTypes`, custom no-op keywords `version` and `markdownDescription`, a custom `date` format, and a patched float-safe `multipleOf` ([UtilAjv.js](https://github.com/TheGiddyLimit/5etools-utils/blob/master/lib/UtilAjv.js)).
- **Structure.** Schemas cross-reference with relative `$ref`s such as `../util.json#/$defs/source`. Across `schema/site` they use 379 `oneOf`, 101 `anyOf`, 75 `patternProperties`, 3 `multipleOf`, one `unevaluatedProperties` and no `$dynamicRef`.
- **Edition tester.** There is a tool for checking which edition each source belongs to: [`bin/test-edition-sources.js`](https://github.com/TheGiddyLimit/5etools-utils/blob/master/bin/test-edition-sources.js) and [`lib/BrewTester/BrewTesterEdition.js`](https://github.com/TheGiddyLimit/5etools-utils/blob/master/lib/BrewTester/BrewTesterEdition.js).

### 1.5 Existing parsers

| Project | Lang | Status | Relevance |
|---|---|---|---|
| [ebullient/ttrpg-convert-cli](https://github.com/ebullient/ttrpg-convert-cli) | Java, Apache-2.0 | Active (pushed 2026-11-09), 384★ | The most complete third-party 5e.tools reader: it resolves `_copy`, tags and sources. Its README asks users to "only include content you own" and defaults to SRD output. Its CI **does fetch the corpus** (sparse clone of the latest release into `actions/cache`) ([action.yml](https://github.com/ebullient/ttrpg-convert-cli/blob/main/.github/actions/data-cache/action.yml)). It is a precedent, not an endorsement. |
| [tome-vtt/vault-forge](https://github.com/tome-vtt/vault-forge) | Rust wrapper, Apache-2.0 | Active (2026-11-26) | A GUI over ttrpg-convert-cli. README: "does not include or point to any game data"; the user pastes the download address. |
| [TheTercept/fiveetoolsapi](https://github.com/TheTercept/fiveetoolsapi) | Rust, no licence | 2025-02 | "A portable api for 5eTools without the data". It uses the 5etools-utils schemas, with data in `./user_data/`. Same posture as this note. |
| [retool](https://crates.io/crates/retool) ([repo](https://github.com/eth0net/retool-rs)) | Rust, MIT | Last release 2023-05 | Converts 5e.tools data for Reroll. Stale and pre-2024. |
| [Steelbirdy/dnd_5etools_rs](https://github.com/Steelbirdy/dnd_5etools_rs) | Rust | 2021 | Stale. |

No maintained Rust crate models the 2024 data, so we write our own types (§6). The JS reference implementation of `_copy` and `_mod` is in [`js/utils.js` L5484–6290](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L6113) (`_doMod_handleProp`, `getCopy`).

---

## 2. Repo hygiene: what the DMCA record actually shows

### 2.1 The notices (github/dmca, read in full)

| Notice | Target | Grounds stated |
|---|---|---|
| [2017-12-04 WotC](https://github.com/github/dmca/blob/master/2017/2017-12-04-WizardsoftheCoast.md) | `astranauta.github.io/5etools.html` (a GitHub Pages site) | The site "incorporates and allows a player to search for every rules element", including content from a book "just published last Friday". |
| [2024-08-07 WotC](https://github.com/github/dmca/blob/master/2024/08/2024-08-07-wizards-of-the-coast.md) | `5etools-mirror-1.github.io` and `5etools-mirror-2.github.io` (GitHub Pages) | Sites "consist almost exclusively of content copied verbatim". The notice quotes the CONTRIBUTING "only official data" rule as evidence, and claims copyright in "details on characters, classes, races, feats, spells, powers, items…". |
| [2024-08-08 D&D Beyond](https://github.com/github/dmca/blob/master/2024/08/2024-08-08-dndbeyond.md) | **One file**, a PHB PDF, inside an otherwise unrelated personal repo (`kwmorris/DnD`) | The unreleased 2024 PHB. The form asks for "URLs for each file, or if the entire repository" and asks about forks. |

Facts only, with no legal conclusion:

1. Every recorded target is a **publicly served copy** of the content: a Pages site or a file in a repo.
2. Notices can target **individual files** in repos that are otherwise unrelated. One stray fixture or snapshot in a public repo is enough to be named.
3. A GitHub code search of `github/dmca` on 2026-11-27 for `5etools` returned only the 2017 and 2024-08-07 notices. Code search covers only part of that repo, so absence is **[unverified]**.
4. `5etools-mirror-3/5etools-src` is live and active at the time of writing. That says nothing about its legal status.
5. No notice I found names a tool that fetches the data onto the user's own machine. Such tools exist publicly (ttrpg-convert-cli, vault-forge, fiveetoolsapi). Absence of a notice is not permission.

**Personal use vs hosting [factual framing, not advice].** The difference the sources support is about **distribution and public serving**.

- A local cache on one machine that is never redistributed is not the pattern any notice targets.
- A public repo that contains upstream bytes or derived text **is** that pattern, whatever the author's intent.
- The sources do not address whether *downloading and using* a copy from the mirror is itself lawful. That is a question for counsel, not this note.
- ADR-0004's Fan Content Policy constraints applied to a public product. They don't bear on a private tool that publishes nothing.

### 2.2 What the public repo must never contain

"Upstream" means anything from `5etools-src`, including derived forms.

1. **Raw data.** No copied `data/**/*.json`, whether whole, trimmed or reformatted (YAML, TOML, CSV).
2. **Derived data.** No compiled packs (`.postcard`, `.rkyv`, `.bin`, `.json`), even when "numbers only". Taken together, names, catalogue structure and numbers reproduce the selection and arrangement. 01 §2 rated "derived numbers plus names" Medium risk.
3. **Text.** No `entries` prose, `{@tag}`-marked strings, feature or spell descriptions, or table captions from non-SRD sources. That rules out doc comments, test names that paste rules text, and `README` examples too.
4. **Test artefacts from real data.** No `insta` snapshots, golden files or recorded fixtures produced from the corpus. Snapshot tests over real data write to the cache directory, or assert counts and hashes only.
5. **Generated Rust.** No `build.rs` output and no checked-in `generated.rs` that embeds upstream strings or tables. No `include_str!` or `include_bytes!` pointing into the cache.
6. **Binaries and Releases.** No GitHub Release assets, container images or `cargo publish` packages that bundle a pack. Set `publish = false` in every crate. If binaries are ever published, they must load packs at runtime from a user-side cache.
7. **CI outputs.** No upload of the corpus or packs as workflow artifacts, and no printing of record text to logs, which are public on a public repo. Prefer that CI never fetches at all (§2.3).
8. **Git history.** If any of the above is ever committed, deleting it in a later commit is not enough. The history must be rewritten and force-pushed, and forks and caches may keep copies.

What the repo **may** contain [design]:

- code, including the importer and the knowledge of field names such as `scalingLevelDice`;
- the lockfile (URL, tag, SHA, per-file sha256);
- hand-authored encodings in our own words, keyed by Content ID;
- synthetic fixtures;
- SRD 5.2.1 / 5.1 text with the CC-BY-4.0 attribution;
- golden DPR numbers.

The encodings of *named* non-SRD content are the residual grey area. See open question 1.

### 2.3 Concrete pattern [design]

```
MinMax5e/ (public repo)
├── upstream.lock.toml        # repo, tag, sha, schema tag, [[file]] path + sha256  (no content)
├── encodings/2024/…          # hand-authored Effect DSL, keyed by Content ID, + upstream_hash
├── crates/…                  # engine, importer, upstream model; publish = false
├── tests/fixtures/synthetic/ # invented records ("Test Blade|TST"), SRD/paraphrase only
└── .gitignore                # /.cache/  /target/  *.pack  *.postcard  *.rkyv  /data/
~/.cache/minmax5e/            # OR ./.cache (gitignored): raw/<sha>/data/…, packs/<lock-hash>/2024.pack
```

- **Fetch.** `cargo xtask fetch` reads the lock, downloads each file to `~/.cache/minmax5e/raw/<sha>/`, and verifies sha256. A user-level cache dir (e.g. via the `dirs` crate) puts the data outside the working tree entirely, which is safer than a gitignored folder inside it.
- **Upgrade.** `cargo xtask bump vX.Y.Z` rewrites the lock and runs the importer diff (§7).
- **CI.** Default: build plus unit tests on synthetic fixtures only; **no corpus fetch**.
  - Optional: a local `cargo xtask corpus-test` that imports the pinned corpus, checks coverage, and prints counts only.
  - If corpus CI is ever wanted, limit it to `workflow_dispatch`, with no `actions/cache` save, no artifacts, and output limited to counts. ttrpg-convert-cli does fetch in CI, but that is not a reason to.
  - Whether Actions caches on public repos can be read by third parties is **[unverified]**, so avoid them.
- **Guards.**
  - A pre-commit hook and a CI "hygiene" job fail if:
    - any tracked file matches `\{@(variantrule|spell|damage|dice|condition|action)\s`, outside an allowlist (this note, research notes, the importer's tag-parser tests on synthetic strings);
    - any `*.json` outside `tests/fixtures/synthetic/` is tracked;
    - any fixture lacks a `"_fixture": "synthetic"` marker;
    - any tracked file is over 256 KB.
  - Add `cargo deny` / `cargo-about` only if dependencies grow. They are not hygiene tools.

---

## 3. Data layout for the MVP and P1

### 3.1 Files and source codes

| Content | 2024 (P0) | 2014 (P1) | File(s) |
|---|---|---|---|
| Classes, subclasses, features | `XPHB` class + 24 XPHB subclasses. Later 2024 subclasses: FRHoF 4, WoW 5, AU 2, RHW 2. | `PHB` class + 55 subclasses (XGE 18, PHB 16, TCE 12, SCAG 4, FTD 2, EGW/BGG/DMG 1 each) | [`data/class/class-{fighter,rogue,barbarian,ranger,paladin,monk}.json`](https://github.com/5etools-mirror-3/5etools-src/tree/v2.41.3/data/class), index [`class/index.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/class/index.json) |
| Optional features (maneuvers, invocations, 2014 fighting styles) | XPHB: 20 `MV:B`, 28 `EI`, 10 `MM` | PHB/TCE: `MV:B`, `FS:F/P/R`, `EI` | [`optionalfeatures.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/optionalfeatures.json) |
| Feats | 77 XPHB: 43 `G`, 10 `O`, 12 `EB`, 10 `FS` + `FS:P` + `FS:R` (fighting styles are feats in 2024) | 42 PHB + 15 XGE + 15 TCE (fighting styles are *optional features*) | [`feats.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/feats.json) |
| Weapons, properties, masteries | 40 XPHB weapons, all with `mastery`; `itemProperty` and 8 `itemMastery` (XPHB) | 39 PHB weapons + DMG | [`items-base.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/items-base.json) |
| Magic items (+1/+2/+3) | XDMG | DMG | [`magicvariants.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/magicvariants.json), [`items.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/items.json) |
| Species | 10 XPHB (`edition:"one"`), 5 with `_versions` lineages | 9 PHB + 15 PHB subraces | [`races.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/races.json) |
| Backgrounds | 16 XPHB | 14 PHB (+6 PHB `_copy` variants) | [`backgrounds.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/backgrounds.json) |
| Spells | `spells-xphb.json` (391) | `spells-phb.json` (361), `-xge` (95), `-tce` (21) | [`data/spells/`](https://github.com/5etools-mirror-3/5etools-src/tree/v2.41.3/data/spells), index `spells/index.json` |
| Spell → class lists | `spells/sources.json` (`class`, `classVariant`) | same | [`spells/sources.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/spells/sources.json) |
| Source metadata | `books.json` (`published` dates; no `edition` field) | same | [`books.json`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/data/books.json), `adventures.json` |
| Rules glossary (Advantage etc.) | `variantrules.json`, `actions.json`, `conditionsdiseases.json` | same | for tag resolution only |

Source codes, from `books.json`, by `published` date:

| Code | Book | Published |
|---|---|---|
| `PHB` | | 2014-08-19 |
| `DMG` | | 2014-12-09 |
| `SCAG` | | 2015-11-03 |
| `XGE` | | 2017-11-21 |
| `EGW` | | 2020-03-17 |
| `TCE` | | 2020-11-17 |
| `FTD` | | 2021-10-26 |
| `BGG` | | 2023-08-15 |
| **`XPHB`** | | **2024-09-17** |
| `XDMG` | | 2024-11-12 |
| `XMM` | | 2025-02-18 |
| `FRHoF` | | 2025-11-11 |
| `EFA` | | 2025-12-09 |
| `RHW` | | 2026-06-16 |
| `AU` | | 2026-09-15 |
| `WoW` | World of Warcraft | 2026-11-17 |

`WoW` is in [`Parser.SOURCES_UNIVERSES_BEYOND`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/parser.js).

MVP spells observed:

- **XPHB versions:** Divine Smite, Searing, Thunderous, Wrathful, Shining (the old Branding), Blinding, Staggering and Banishing Smite, Hunter's Mark, Hex and True Strike all exist.
- **Spirit Shroud** is `TCE` only (no XPHB reprint). `spells/sources.json` lists it under `classVariant` for **`Paladin|XPHB`** (`definedInSource: "TCE"`). In other words, 5e.tools itself attaches Tasha's expanded spells to the 2024 classes. That is relevant to ADR-0009.
- **Booming Blade and Green-Flame Blade** are `TCE` only.
- **The 2014 Divine Smite** is a PHB Paladin *class feature*, not a spell.

### 3.2 Edition, reprint and SRD fields

| Field | Where present (observed) | Semantics |
|---|---|---|
| `edition` (`"classic"` / `"one"`) | Classes (29 of 42), subclasses (203 of 361), base items (176 of 230), species (30 of 175), backgrounds (74 of 175). **Never** on feats, spells or optional features. | A "context-sensitive behaviour hint" ([util-edition.json](https://github.com/TheGiddyLimit/5etools-utils/blob/master/schema/site/util-edition.json)). Not a reliable partition key on its own. |
| `reprintedAs` | On the old record, pointing to the new one. 2-part keys (`"Greatsword|XPHB"`), 4-part for subclasses (`"Battle Master|Fighter|XPHB|XPHB"`) = `Name|Class|ClassSource|Source`. Can **rename** (`Branding Smite → Shining Smite|XPHB`, `Path of the Totem Warrior → Wild Heart`, `Fey Touched → Fey-Touched|XPHB`) and can point to **later books** (`Arcane Archer → |AU`, `Swashbuckler → |WoW`, `Phantom → |RHW`). | Drives the ADR-0009 rule that reprinted items are always replaced. |
| `srd` / `basicRules` | 2014 records | `bool` or `string`; a string means the SRD name differs |
| `srd52` / `basicRules2024` | 2024 records. 17 of 77 XPHB feats; every XPHB weapon; 9 of 10 species (not Aasimar); 1 subclass per class | Same. Usable for an "SRD-only" view of the data, though it grants no licence to 5e.tools' copy. |
| `_isCopy`, `reskinOf`, `includeSubclassesFrom` | `Rogue|WoW`, `Paladin|WoW` and `Monk|WoW` are `_copy` **reskins** of the XPHB classes | Must be dropped (or aliased) in our model. |

### 3.3 `_copy`, `_mod`, `_versions`: what an importer must resolve

The `_meta.internalCopies` header lists the kinds that contain copies (e.g. `["class","subclass"]` in `class-rogue.json`). I classified every `_copy` in the relevant files:

| Ruleset | Kind | Count | Kind of copy |
|---|---|---|---|
| 2014 (by subclass source) | subclass | 124 | **Synthetic cross-edition**: a 2014 subclass (`classSource: PHB`) cloned onto the XPHB class. Example: `Samurai|XGE` with `classSource:"XPHB"` and `_copy` → `classSource:"PHB"` plus `_preserve`. **Drop.** |
| 2024 | subclass | 28 | WoW reskin copies (`classSource: WoW`) of XPHB subclasses. **Drop.** |
| 2024 | class | 13 | `reskinOf` classes (WoW and others). **Drop.** |
| 2014 | background / race / subrace / item / subclassFeature | 26 / 11 / 1 / 50 / 75 | Real variants. **Resolve** for P1. |
| 2024 | race / item / magicvariant / subclassFeature | 10 / 40 / 5 / 1 | Real, in later 2024 books and XDMG items. **Resolve** when in scope. |
| **2024 XPHB core** | any MVP kind | **0** | Nothing to resolve. |

**`_mod` modes in use** across class, feat, item, species, background, optional-feature and spell files: `replaceArr` 53, `insertArr` 45, `appendArr` 39, `appendIfNotExistsArr` 17, `setProp` 16, `remove` 11, `removeArr` 5, `replaceTxt` 4. The full set in the JS has 30 modes, including bestiary-specific ones ([utils.js L6113–6160](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L6113)). Implementing those 8 covers everything in scope.

**Other merge semantics to copy exactly:**

- `null` in the copy deletes the key.
- Keys listed in `_MERGE_REQUIRES_PRESERVE_BASE` are only inherited if `_preserve` names them or `"*"`. Those keys are: `page`, `otherSources`, `referenceSources`, `srd`, `srd52`, `basicRules`, `basicRules2024`, `reprintedAs`, `hasFluff*`, `_versions` ([L5484](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L5484)).
- `_mod` keys `"*"` (all entry props) and `"_"` (root) are applied last.
- `_templates` are used 0 times in scope.
- Variables `{{…}}` are resolved before mods.

**`_versions`** expands one record into variants via `_mod`, or via `_abstract` + `_implementations` + `_variables` (Dragonborn colours). It appears in 28 species and 2 feats (`Magic Initiate|XPHB` per class list; `Strike of the Giants|BGG`). **The 2024 MVP needs `_versions` (5 species and Magic Initiate) but no `_copy`.** That lets the first importer reject `_copy` in 2024 scope with a hard error, and add the resolver with P1.

### 3.4 References and `{@tag}` markup

- **Reference strings** are pipe-delimited, and the source defaults when omitted:
  - class features: `"Extra Attack|Fighter|XPHB|5"`, i.e. `Name|Class|ClassSource|Level[|Source]`;
  - subclass features: `"Battle Master|Fighter|XPHB|Battle Master|XPHB|3"`;
  - spell lists: lower-cased (`"hunter's mark|xphb"`), so key comparison must be **case-insensitive**;
  - `classFeatures[]` items are either a string or `{classFeature, gainSubclassFeature: true}`, which marks the subclass level.
- **Tags.** The renderer defines **121 tag kinds** ([`Renderer.tag.TAGS`, render.js L6611](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/render.js#L6611)). Format: `{@tag text|source|display…}`. When `source` is empty, the tag's `defaultSource` applies: `PHB` for spell, item, feat and so on ([`Parser.getTagSource`](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/parser.js#L4492)).
  - In XPHB records, **every** reference tag I counted (3,262: `variantrule` 1,554, `spell` 448, `item` 389, `condition` 364, …) carries an explicit source.
  - Unqualified tags and property codes (`"F"`, `"H"` on PHB weapons, versus `"F|XPHB"`) occur in 2014 data. Default them to `PHB` / `DMG`.
- **Scaled dice.** `{@scaledamage base|levels|step}`, e.g. base `2d8`, levels `1-9`, step `1d8`, is a machine-parseable upcast rule ([`Renderer.parseScaleDice`, render.js L3042](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/render.js#L3042)).
- **Tagging is partly hand-done.** 5e.tools' own [NOTES_AUTOMATION.md](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/NOTES_AUTOMATION.md) says tagging is "partially automated … many false positives", and that spell `miscTags` are maintained by hand. Treat tags as hints, not ground truth.

Tag census of the MVP class and subclass features (my count):

| Ruleset | Records | Words | Records with a dice/damage/DC tag | Top tags |
|---|---|---|---|---|
| 2024 | 325 | 18,967 | 37 | `variantrule` 368, `spell` 85, `condition` 83, `action` 59 |
| 2014 | 559 | 39,453 | 58 | |

Structured entry nodes in the 2024 records: `refSubclassFeature` 59, `refOptionalfeature` 25, `refClassFeature` 13, `table` 9.

---

## 4. Automation: what is structured, and how far it gets us

### 4.1 Structured fields (observed)

- **Weapons** (`items-base.json`, XPHB). Every one of the 40 has `dmg1`, `dmgType`, `weaponCategory` and `mastery`; 37 have `property`, 16 `range` and 9 `dmg2` (versatile). Example structure, with no prose:

  `{"name":"Shortsword","source":"XPHB","weaponCategory":"martial","property":["F|XPHB","L|XPHB"],"mastery":["Vex|XPHB"],"dmg1":"1d6","dmgType":"P"}`

  Note that `value` is in copper. Property and mastery *meanings* are prose (`itemProperty` and `itemMastery` `entries`). They are a closed set: 10 property codes and 8 masteries, authored once in the kernel.
- **Class chassis** (XPHB class record):
  - `hd`, `primaryAbility`, `proficiency`, `startingProficiencies`, `multiclassing`;
  - `featProgression`: Fighter `FS` at L1 and `EB` at L19; Paladin `["FS","FS:P"]` at L2; Ranger `["FS","FS:R"]` at L2;
  - `itemMasteriesProgression`: Fighter `{1:3,4:4,10:5,16:6}`, Barbarian `{1:2,4:3,10:4}`, Rogue, Paladin and Ranger `{1:2}`, Monk none;
  - `casterProgression`: Paladin and Ranger are `"artificer"`, i.e. half-caster rounded up;
  - `preparedSpellsProgression`;
  - `additionalSpells`: Paladin always prepares Divine Smite at L2; Ranger always prepares Hunter's Mark at L1;
  - ASI levels come from `classFeatures` entries named "Ability Score Improvement", and the subclass level from `gainSubclassFeature`.
- **Class tables** (`classTableGroups`). Typed cells:
  - Rogue `Sneak Attack`: `{type:"dice",toRoll:[{number:1,faces:6}]}` … `10d6`;
  - Barbarian `Rages` `"2"` … `"6"`, `Rage Damage` `{type:"bonus",value:2}` … `4`;
  - Monk `Martial Arts` `1d6` … `1d12`, `Focus Points` (number), `Unarmored Movement` `{type:"bonusSpeed"}`;
  - Fighter `Second Wind`, `Weapon Mastery`;
  - spell slots via `rowsSpellProgression`.

  Column labels can contain tags (`{@filter Prepared Spells|…}`).
- **Subclasses.** `optionalfeatureProgression` (Battle Master `MV:B` `{3:3,7:5,10:7,15:9}`), `additionalSpells` (oath spells), and Eldritch Knight / Arcane Trickster `casterProgression` plus `subclassTableGroups`.
- **Feats.**
  - `category`;
  - `prerequisite`: an **array of alternatives**, each an AND of `level`, `ability`, `feature`, `proficiency`, `race`. Dual Wielder is `[{level:4,ability:[{str:13}]},{level:4,ability:[{dex:13}]}]`.
  - `ability`: fixed `{str:1}`, or `{choose:{from:[…]}}`, with optional `max` (Epic Boons have `max:30`).
  - `additionalSpells`, `repeatable`, and `*Proficiencies`.
- **Backgrounds** (XPHB). `ability` is a weighted choice over three abilities (`weights:[2,1]` or `[1,1,1]`). `feats` holds the origin feat, e.g. `{"savage attacker|xphb":true}`. Skill and tool proficiencies are also structured.
- **Species** (XPHB). `size` (Human is `["S","M"]`), `speed` (a number, or an object for other modes), `darkvision`, `resist`, `additionalSpells`, `feats` (Human: `anyFromCategory O`), and `_versions` lineages.
- **Spells.**
  - `level`, `school`, `components`, `range`, `duration`;
  - `time[].unit` (`action` / `bonus` / `reaction`);
  - `time[].condition`: free text, present on 15 XPHB spells, including every smite's "immediately after hitting…";
  - `duration[].concentration`: 161 of 391 XPHB spells;
  - `savingThrow`, `spellAttack` (`M`/`R`), `damageInflict`, `conditionInflict`, `areaTags`;
  - `scalingLevelDice` for cantrips: an object, or an **array** for two-part cantrips such as Booming Blade and True Strike;
  - `miscTags`: `AAD` "Additional Attack Damage", `UBA` "Uses Bonus Action", `ADV`, `SGT` and others ([spells.json schema](https://github.com/TheGiddyLimit/5etools-utils/blob/master/schema/site/spells/spells.json)).

  Several things changed between editions:
  - **concentration:** every 2014 smite is concentration; 2024 Searing, Thunderous, Wrathful, Blinding and Staggering are not;
  - **damage type:** Wrathful Smite deals psychic damage in 2014 and necrotic in 2024;
  - **Hunter's Mark:** 2014 has no `damageInflict`; 2024 has `force`.

  All of these are **picked up automatically** by importing per edition.
- **Damage dice in spells (semi-structured).** Of 139 XPHB damage spells:
  - 105 have exactly one `{@damage X}` in `entries`, 25 have two, 4 have three or more, and 5 have none;
  - 62 have a `{@scaledamage}` in `entriesHigherLevel`;
  - 18 have `scalingLevelDice`.

  The 2014 PHB, XGE and TCE spells look similar: 112 / 28 / 30 / 3 of 173.
- **Foundry overlays.** `data/class/foundry.json` (177 class-feature and 479 subclass-feature overlays) and `data/foundry-*.json` hold partial Active-Effect automation, such as Improved Critical as a crit-threshold change ([01 §1.4](01-data-source-and-licensing.md)). They are useful as a **cross-check or drafting hint**, not as an input.

### 4.2 Auto-derivable vs hand-authored, MVP (2024) roster

| Area | Records | Auto-imported (no human) | Semi-auto (prefilled from tags, reviewed) | Hand-authored behaviour [estimate] |
|---|---|---|---|---|
| Class chassis | 6 classes | All: HD, saves, proficiencies, MC prerequisites, ASI and subclass levels, FS/EB feat slots, mastery counts, caster progression and slots, class-table scalars | n/a | 0 |
| Class features | 166 | Name, level, ordering, reference graph | 27 `dice` and 17 `damage` tags | **~35–45** DPR-relevant (Extra Attack, Sneak Attack, Cunning Strike, Steady Aim, Rage, Reckless, Brutal Strike, Action Surge, Martial Arts, Flurry, Stunning Strike, Radiant Strikes, Foe Slayer, Weapon Mastery …). The rest are `no-dpr-impact` |
| Subclasses (24 XPHB + 13 in later 2024 books) | 159 XPHB features + 92 | Feature lists, maneuver counts, oath and domain spells, EK/AT spellcasting | dice tags | **~30–50** (Champion crit range, Battle Master dice and maneuvers, Gloom Stalker, Hunter, Vengeance, Zealot, Berserker, Open Hand, Assassin …) |
| Feats | 77 XPHB | **All build-legality data**: category, OR-of-AND prerequisites, ASI choices, granted spells, proficiencies | n/a | **~20–25** combat feats (GWM, Sharpshooter, Dual Wielder, PAM, Crossbow Expert, Savage Attacker, the 12 FS feats, a few Boons) |
| Weapons | 40 + 10 properties + 8 masteries | **All weapon stats** | n/a | 10 property semantics + 8 masteries, once, in the kernel or DSL |
| Species | 10 (+ lineages) | Size, speed, darkvision, resistances, granted spells and feats | n/a | ~4–6 combat traits |
| Backgrounds | 16 | **All** | n/a | 0 |
| Rider spells | ~15 | Level, cast-time unit, "after hitting" condition present, concentration, save, damage type, cantrip scaling | Base dice (`{@damage}`), upcast (`{@scaledamage}`) | ~15 templated encodings (on-hit rider vs persistent rider vs mark) |

**Bottom line [estimate].** About 110–150 hand-written encodings, against 04's 150–200. The cut comes from three sources:

- build legality (prerequisites, ASIs, origin feats, mastery counts, spell grants) is now 100% imported;
- weapons are 100% imported;
- spell dice come prefilled from tags.

Encodings shrink to *behaviour only*: a reference to the upstream key plus triggers, conditions and costs, with numbers pulled from the import wherever they exist. For example, Sneak Attack's dice come from `classTableGroups`, not from the encoding.

### 4.3 Where tags help, and where they mislead

- **Safe.** `{@scaledamage}` and `{@damage}` in spells with a single damage tag. Cross-check against `damageInflict`.
- **Unsafe without review.**
  - Multi-tag spells, e.g. first hit vs ongoing damage.
  - Feature prose. The 5e.tools tagger itself produces false positives ([NOTES_AUTOMATION.md](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/NOTES_AUTOMATION.md)).
  - `time[].condition` is free text. Use it only as an "after hitting" flag that **asserts** the hand encoding's trigger, not as the trigger itself.
- **Use of LLM drafting.** Per ADR-0002, an LLM may draft encodings from the *local* prose. The prompt and its output stay in the cache, and only the reviewed DSL is committed.

---

## 5. Ruleset partitioning with 5e.tools as the source

### 5.1 Rule [design]

1. **Source to ruleset.** `ruleset(source) = 2024` if `books.json[source].published ≥ books.json["XPHB"].published`, else `2014`. This is exactly 5e.tools' own `SourceUtil.isClassicSource` ([utils.js L658](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L658)). Freeze it as an **explicit allowlist table** in the lockfile's companion config (`PHB→2014, XGE→2014, TCE→2014, XPHB→2024, XDMG→2024, FRHoF→2024, …`), so a new book is a reviewed one-line change rather than an automatic inclusion.
   - Exclude prerelease and UA sources (`UA*`), Universes Beyond (`WoW`), adventure-only sources and homebrew unless they are allowlisted.
   - Seven sources in scope files have no date in `books.json` or `adventures.json`: `TftYP`, `UATheMysticClass`, `HAT-LMI`, `RoTOS`, `EEPC`, `MCV2DC`, `EET`. These are not allowlisted, so they are excluded.
2. **`edition` as an assertion.** Where `edition` is present, it must agree with rule 1. On 2026-11-27 it agreed in all 512 records that carry it, so a mismatch fails the import.
3. **Subclasses.** A subclass belongs to `ruleset(subclass.source)`, and its `classSource` must belong to the same ruleset. Records whose `_copy.classSource` differs from `classSource` are **dropped**: 124 synthetic 2014-on-2024 copies and 28 WoW reskin copies.
   - Their 2014 originals are kept in the 2014 pack.
   - A 2014 subclass that is wanted under 2024 rules (ADR-0009) is added as an explicit whitelist entry with its **own** encoding and levels, never by importing the synthetic copy. 5e.tools itself warns that "feature levels may have to be adjusted" ([util-edition.json](https://github.com/TheGiddyLimit/5etools-utils/blob/master/schema/site/util-edition.json)).
4. **Reskins.** Classes with `reskinOf` and their copied subclasses are dropped.
5. **`reprintedAs`.** A 2014 record with a `reprintedAs` target in an allowlisted 2024 source is **never** eligible for the ADR-0009 legacy-compatible whitelist; this is ADR-0009's "always replaced" rule. A record with no `reprintedAs`, such as `Spirit Shroud|TCE`, `Booming Blade|TCE` or `Samurai|XGE`, *can* be whitelisted.
   - `spells/sources.json` `classVariant` entries for `XPHB` classes, e.g. Spirit Shroud for `Paladin|XPHB`, provide a candidate list that the importer can surface automatically. Inclusion stays a committed decision.

### 5.2 Content ID: revisit ADR-0008

| Option | Example | Verdict |
|---|---|---|
| A. 5e.tools key as identity | `Great Weapon Master|XPHB` | **Reject.** Under ADR-0009, `Spirit Shroud|TCE` lives in both packs with different encodings (2014 vs 2024 rules), so one key would name two entities. The key also doesn't say which ruleset it belongs to, and subclass and feature keys are 4–6-part. |
| B. Ruleset-prefixed slug, key as alias (ADR-0008 today) | `2024:feat/great-weapon-master` | **Reject as specified.** A slug without the source collides: **25 collisions in the 2014 data** (e.g. `orc` from DMG, ERLW, MPMM and VGM; `shawm` from PHB and SCAG), and new books will add more in 2024. Adding a disambiguator *later* renames existing IDs. It also needs a hand-maintained alias table. |
| **C. Ruleset-prefixed, body derived from the key [recommend]** | `2024:feat/great-weapon-master|xphb`; `2024:subclass/fighter|xphb/battle-master|xphb`; `2024:classfeature/rogue|xphb/1/sneak-attack|xphb`; `2024:spell/spirit-shroud|tce` (whitelisted) alongside `2014:spell/spirit-shroud|tce` | **Adopt.** The ID is deterministic from (ruleset, kind, upstream key), so no mapping table is needed. It is unique as long as 5e.tools keys are unique. The ruleset is explicit, so ADR-0008's load-time cross-pack check still works. It is greppable from a key. Each record also stores `upstream_key` (exact case) and `upstream_hash`. Hand-written content with no upstream (house rules, test content) uses the reserved source `mm5e`. |

Slug rule: lower-case, with runs of non-alphanumerics collapsed to `-`. The source code is lower-cased. The upstream key parser is case-insensitive, as 5e.tools refs are.

---

## 6. Rust specifics

### 6.1 serde modelling [design; not compiled, as no Rust toolchain was available on the research machine]

Polymorphic fields observed in scope:

| Record | Field | Shapes |
|---|---|---|
| race | `speed` | number or object |
| race | `lineage` | string, null or bool |
| race | `resist[]` | string or object |
| feat | `resist[]` | string or object |
| spell | `scalingLevelDice` | object or array |
| spell | `srd`, `srd52` | bool or string |
| class | `classFeatures[]` | string or object |
| baseitem | `property[]` | string or object |
| baseitem | `reprintedAs[]` | string or object |
| all | `entries[]` | string or object (recursive) |
| class tables | cells | string, number, or `{type:dice|bonus|bonusSpeed}` |

Plan:

1. **Two-phase load.**
   - Parse each file to `serde_json::Value`.
   - Resolve `_copy` and `_versions` on `Value`. Copies lack required fields until merged, so typed deserialisation must wait.
   - Deserialise the resolved records into typed structs with `serde_json::from_value`, wrapped in [`serde_path_to_error`](https://crates.io/crates/serde_path_to_error), so failures report `feat[37].prerequisite[1].ability`.
2. **Closed small unions** use `#[serde(untagged)]` enums. Examples: `enum Speed { Walk(u32), Modes(SpeedModes) }` and `enum ScalingLevelDice { One(Scaling), Many(Vec<Scaling>) }`. Put the most specific variant first, because untagged enums try variants in order and give poor errors. Use `serde_with` helpers for "one or many".
3. **Internally tagged** table cells: `#[serde(tag = "type")] enum Cell { Dice{toRoll}, Bonus{value}, BonusSpeed{value} }`, wrapped in an outer untagged enum with `Text(String)` and `Num(i64)`.
4. **Escape hatches.**
   - `entries: serde_json::Value`. The engine never reads prose, and the importer's tag scanner walks `Value` strings.
   - `additionalSpells: Value` until a spell-granting feature is in scope. Its keys include `"1e"`, `"_"`, `daily`, `innate` and `choose` filters.
   - `#[serde(flatten)] extra: BTreeMap<String, Value>` on top-level records, or [`serde_ignored`](https://crates.io/crates/serde_ignored). Report unknown keys as **upgrade drift**, so a bump that adds fields is visible rather than silently lost. Don't use `deny_unknown_fields`, because upstream adds fields often.
5. **Key types.** Newtypes `UpstreamKey { name, source }` and `ClassFeatureRef { name, class, class_source, level, source }`, with `FromStr` for the pipe format and case-insensitive `Eq`/`Hash` (store a normalised lower-case form).
6. **Dice.** A tiny parser for the `NdM[+K]` dice strings found in `dmg1` and `{@damage}`, plus the `{@scaledamage a|lvls|step}` grammar mirroring `Renderer.parseScaleDice`. Add property tests (`proptest`) over synthetic strings.

### 6.2 Schema validation

- **Validator choice.** [`jsonschema`](https://crates.io/crates/jsonschema) 0.63.2 (2026-11-26) advertises draft 2020-12 compliance on Bowtie, plus custom keywords and formats (README). [`boon`](https://crates.io/crates/boon) 0.6.1 (2025-01) is a smaller alternative.
- **Wiring.** Load the pinned 5etools-utils `schema/site/**` (fetched into the cache by the same lock, as MIT code). Register each file as a resource under its `$id` so relative `../util.json` refs resolve. Mirror Ajv's set-up: `date` format, ignore `version` and `markdownDescription`. **[unverified]** whether jsonschema's `multipleOf` needs the float workaround Ajv uses (`UtilAjv.js`) and whether unknown keywords are silently ignored. Test on the first run.
- **Role.** Make validation an opt-in `mm5e-import --validate` diagnostic run on bumps. The typed serde layer is the contract the engine relies on.
- **Generated types.** [`typify`](https://crates.io/crates/typify) 0.10.2 could generate types from the schema. With 379 `oneOf` and 75 `patternProperties`, the output is likely unwieldy **[unverified]**. Hand-written types for the ~10 kinds in scope are simpler.

### 6.3 Where importing runs: `build.rs` vs importer binary vs runtime

| Option | Verdict |
|---|---|
| `build.rs` reads the cache and generates Rust | **Reject.** Every `cargo build`, including CI and a fresh clone, would depend on gitignored data. It invites `include_bytes!`, which embeds the data in every binary, so any binary that is shared becomes a redistribution. It also hides an expensive step in incremental builds. |
| Runtime parse of the raw JSON on each start | Viable, since 13 MB parses in well under a second **[estimate]**. But `_copy` resolution, partitioning and the encoding join would rerun every time, and errors would surface late. |
| **A separate `mm5e-import` binary that writes a compiled pack to the cache, loaded by the engine at runtime [recommend]** | Explicit and diffable, with fast startup. The engine crates have no data dependency, so CI builds and tests them on synthetic fixtures. |

### 6.4 Caching the parsed form

| Format | Notes | Verdict |
|---|---|---|
| [bincode](https://crates.io/crates/bincode) | 3.0.0 (2025-12-16) is a tombstone: its README says it is unmaintained and `lib.rs` contains only `compile_error!`. It recommends wincode, postcard or rkyv. | **Reject.** |
| [postcard](https://crates.io/crates/postcard) 1.2.0 | serde-native, compact, stable wire format. | **Adopt for packs.** The pack is small (thousands of records), so load time isn't critical. |
| [rkyv](https://crates.io/crates/rkyv) 0.8.18 | Zero-copy and fastest to load, but needs its own derives and has schema-evolution friction. | Use later, only if a benchmark shows load time matters, e.g. for precomputed pools (ADR-0020). |
| JSON | Human-readable. | Keep as a `--dump-json` debug output to the cache. Never commit it. |

Pack header: magic, format version, lock hash (sha256 of `upstream.lock.toml`), encodings git hash, and ruleset. The engine refuses a stale pack.

---

## 7. Recommended pipeline

```
                       PUBLIC REPO (no upstream bytes)                     LOCAL ONLY (~/.cache/minmax5e)
 ┌──────────────────────────────────────────────┐
 │ upstream.lock.toml                           │   cargo xtask fetch
 │  repo=5etools-mirror-3/5etools-src           │ ───────────────────────► raw/<sha>/data/**.json
 │  tag=v2.41.3 sha=2512db75…                   │   (raw.githubusercontent  (sha256-verified per file)
 │  [[file]] path, sha256  (~40 files)          │    @SHA, or sparse clone)  schema/<tag>/** (5etools-utils)
 │  schema_tag=v0.17.40                         │
 │ rulesets.toml  source→ruleset allowlist,     │
 │  legacy-compatible whitelist (ADR-0009)      │
 └──────────────────────────────────────────────┘
                     │                                         │
                     ▼                                         ▼
 ┌───────────────────────────── mm5e-import (binary) ─────────────────────────────┐
 │ 1 load Value      2 resolve _versions (+ _copy for P1)    3 partition:         │
 │                                                             source→ruleset,    │
 │                                                             assert `edition`,  │
 │                                                             drop synthetic     │
 │                                                             copies & reskins   │
 │ 4 typed serde (+path_to_error, unknown-key drift report)                       │
 │ 5 derive IR: weapons, class tables, progressions, prereqs, spell dice          │
 │   ({@damage}/{@scaledamage}), cast-time, concentration                         │
 │ 6 join encodings/<ruleset>/** by Content ID; compare upstream_hash → STALE?    │
 │ 7 coverage report: encoded | no-dpr-impact | missing | stale   (fail on missing)│
 │ [--validate: jsonschema against pinned schema]   [--dump-json: debug]          │
 └────────────────────────────────────────────────────────────────────────────────┘
                     │ writes                                  │
                     ▼                                         ▼
          packs/<lockhash>/2024.pack, 2014.pack (postcard)   reports/<lockhash>/{coverage,diff}.txt
                     │
                     ▼  load at runtime
 ┌──────────── engine / solver / cli (Rust, data-agnostic crates) ────────────┐
 │ rules-core kernel + rules-2024 / rules-2014 pack logic (ADR-0008)          │
 └────────────────────────────────────────────────────────────────────────────┘

 CI (public): cargo build + test on tests/fixtures/synthetic only + hygiene guard. No fetch.
 Bump: xtask bump vX → fetch → import → diff report (added/removed/changed keys in scope,
       stale encodings) → human review → commit lock + encoding fixes.
```

**Staleness detection [design].** Each encoding stores `upstream_hash`: the sha256 of the canonicalised, resolved upstream record. Because CONTRIBUTING applies errata in place, a bump that changes a record flags its encoding as `stale` until a human re-reviews it. A hash is not content, so committing it is fine.

---

## 8. Repo-hygiene rules (checklist)

- [ ] `.gitignore` covers `/target/`, `/.cache/`, `/data/`, `*.pack`, `*.postcard`, `*.rkyv`, `**/5etools*/`, `.DS_Store`. There is currently **no `.gitignore`** in the repo.
- [ ] The cache defaults to a user-level directory outside the working tree.
- [ ] Only `upstream.lock.toml` (URL, tag, SHA, per-file sha256) and `rulesets.toml` refer to upstream. No upstream bytes.
- [ ] No tracked JSON except `tests/fixtures/synthetic/**`, and each fixture carries a `"_fixture":"synthetic"` marker. Fixtures use invented names and sources (`Test Blade|TST`), SRD 5.2.1 / 5.1 material with CC-BY attribution, or our own paraphrase.
- [ ] No `{@tag …}` rules text in tracked files outside an allowlist. Enforce with a pre-commit hook and a CI grep.
- [ ] No `build.rs` reads the cache. No `include_str!` / `include_bytes!` reaches outside the crate.
- [ ] No `insta` snapshots or golden files generated from the corpus are committed. Corpus snapshots live in the cache.
- [ ] Every crate sets `publish = false`. No GitHub Releases or images with packs embedded.
- [ ] CI never fetches the corpus by default. Any opt-in corpus job is `workflow_dispatch` only, with no cache, no artifacts, and counts-only logs.
- [ ] Hand-authored encodings contain mechanics in our own words only: no pasted descriptions.
- [ ] LLM drafting prompts and outputs that include upstream prose stay in the cache.
- [ ] Before the first push of the currently untracked `docs/research/*`, audit the quotes: SRD 5.2.1 quotes with attribution are fine; non-SRD rules prose must be paraphrased.
- [ ] If anything slips in, rewrite history (`git filter-repo`) and force-push. Assume forks and caches may already hold copies.
- [ ] Consider making the repo private (open question 1).

---

## 9. Open questions

1. **Public or private repo?** Making it private removes the residual "hosting" exposure of the hand-authored encodings of named non-SRD content (01's open question 1). The cost is that nobody else sees the code. For a personal tool this looks cheap. *User decision.*
2. **Legality of downloading and using the mirror copy** for personal use. None of the primary sources here addresses it. *Counsel, if it matters to the user.*
3. **2024 scope beyond XPHB.** Which later 2024 books are allowlisted for the MVP: XDMG (magic weapons), FRHoF, AU, RHW? Is `WoW` (Universes Beyond, with reskin classes) excluded?
4. **ADR-0009 whitelist.** Should the importer *propose* candidates from `spells/sources.json` `classVariant` (e.g. Spirit Shroud for `Paladin|XPHB`) and from subclasses with no `reprintedAs`, with inclusion staying manual? Recommended: yes.
5. **Bump policy.** Track every release, which is weekly, or bump deliberately per book release? Recommended: deliberately, since each bump costs a review of stale encodings.
6. **Display text.** A personal CLI or TUI could show feature prose straight from the local cache, which is fine locally. Should the pack carry it or not? Recommended: a separate local "display" pack, so the engine pack stays prose-free.
7. **jsonschema crate compatibility** with the 5etools-utils schemas (relative `$ref`s, `multipleOf`, unknown keywords) is untested **[unverified]**. Spike it on the first importer milestone.
8. **Foundry overlays** (`class/foundry.json`). Should they be used as an automatic cross-check, e.g. that the encoded crit threshold matches a `weaponCriticalThreshold` change? This is cheap and catches errors.

---

## Candidate ADRs / revisions

### Existing ADRs

| ADR | Action | Why |
|---|---|---|
| **0001** No 5e.tools corpus | **Supersede** with new ADR A | The premise has flipped. For a personal tool, the 5e.tools corpus is the local source for all non-SRD rules. The reasons 0001 gave (DMCA, Fan Content Policy) were about *shipping and hosting*. They become the repo-hygiene rules in ADR B rather than a ban. |
| **0002** Hand-authored mechanics | **Amend** | Behaviour stays hand-authored, and prose is still never parsed into triggers. The amendment widens "structured fields may be imported" to an explicit list: chassis, class tables, progressions, prerequisites, ASIs, weapon stats, spell metadata, and tag-derived spell dice with review. Encodings reference upstream numbers instead of restating them, and carry `upstream_hash` for staleness. |
| **0003** User-imported content stays local | **Supersede** with ADR A/B (generalised) | There is no "user" separate from the developer and no server. The rule becomes "all upstream-derived data stays in a local cache, never in the repo or any published artefact". |
| **0004** Free, no paywall | **Deprecate** | There is no product, no sales and no site. The Fan Content Policy constraints it encoded don't apply to a private tool. Revisit only if a public release is ever considered, in which case ADR-0001's original reasoning returns. |
| **0005** No WotC trademarks in branding | **Amend, narrow scope** | No product branding is left to protect. Keeping the `MinMax5e` name for the *public repo* still costs nothing and avoids trademark use in a public place. Reduce it to a one-line convention. |
| **0008** Rules Packs and Content IDs | **Amend** | Keep separate packs per ruleset and the cross-pack load check. Change the ID format to `<ruleset>:<kind>/<slug>|<source>`, derived from the upstream key (§5.2). Replace "the 5e.tools key may be kept as an alias … it is not the identity" with "every record stores `upstream_key` and `upstream_hash`". The rejected option "5e.tools keys as identity, because we don't use 5e.tools" loses its second reason but keeps its first (one key in two rulesets). |
| 0009 Legacy-compatible whitelist | Amend (minor) | Add the `reprintedAs` exclusion and importer-proposed candidates from `classVariant`. Synthetic cross-edition copies are never the vehicle. |
| 0019 Client-side compute, 0021 TypeScript engine, 0022 pnpm monorepo | **Supersede** (outside this note's brief, flagged for completeness) | No website, and the engine is all Rust. These become a Cargo workspace of data-agnostic crates plus the `mm5e-import` binary (ADR C). ADR-0020's "shipped precomputed pools" becomes "locally cached pools". |

### New candidate ADRs

**A. 5e.tools (`5etools-mirror-3/5etools-src`) is the local rules corpus, pinned by commit.**
- *Decision:* All non-SRD rules data comes from `data/` of `5etools-src`, pinned in `upstream.lock.toml` by tag, commit SHA and per-file sha256. It is fetched by `cargo xtask fetch` into a local cache. Both rulesets come from this one repo; `5etools-2014-src` is not used. The schema comes from `TheGiddyLimit/5etools-utils`, pinned by tag.
- *Rationale:* It has the best-structured metadata of any source (§4) and covers 2024 books through 2026-11. One repo holds both editions, with 2014 records identical to the classic-only mirror (§1.1). Single-commit releases make SHA pins clean.
- *Rejected:*
  - SRD-only, which lacks GWM, Sharpshooter, most subclasses and smites;
  - `5etools-2014-src` for P1, a second pin for identical data;
  - a git submodule, which pulls the full 180 MB site and visibly couples the public repo to the mirror;
  - the release zip as the primary fetch, which brings 50 MB of site code;
  - tracking `main` unpinned, which brings silent errata drift.

**B. The public repository contains no upstream-derived bytes.**
- *Decision:* §8 checklist. Raw data, derived JSON or packs, corpus snapshots, generated Rust and data-embedding binaries live only in the local cache. A pre-commit hook and a CI guard enforce this. CI never fetches the corpus. Tests use synthetic fixtures.
- *Rationale:* Every recorded WotC/D&D Beyond notice targets publicly served copies, including a single file in an unrelated repo (§2.1). The repo is public. Undoing a leak means rewriting history.
- *Rejected:*
  - committing "numbers-only" derived data (01 rated it Medium risk; it still reproduces selection and arrangement);
  - fetching in CI with `actions/cache` (ttrpg-convert-cli does this; it adds public-log and cache exposure for little benefit);
  - relying on review discipline alone without a guard.

**C. Import is a separate binary that writes a compiled pack to the cache; no `build.rs`.**
- *Decision:* `mm5e-import` resolves, partitions, derives, joins encodings, reports coverage, and writes `packs/<lockhash>/<ruleset>.pack` (postcard, versioned header). The engine loads packs at runtime and refuses stale ones.
- *Rationale:* Builds and CI stay independent of gitignored data. It prevents embedding data into binaries, makes errors explicit at import time, and gives fast startup.
- *Rejected:* `build.rs` codegen or `include_bytes!` (§6.3); parsing raw JSON on every start; bincode (unmaintained tombstone); rkyv for now (deferred until a benchmark needs it).

**D. Ruleset assignment uses a source-date allowlist, with `edition` as an assertion and synthetic copies dropped.**
- *Decision:* `rulesets.toml` maps each allowlisted source code to a ruleset, using 5e.tools' published-before-XPHB rule. `edition` mismatches fail the import. Subclasses whose `_copy.classSource` differs from `classSource`, and `reskinOf` classes, are dropped. `reprintedAs` disqualifies a 2014 record from the ADR-0009 whitelist.
- *Rationale:* `edition` is missing on every feat, spell and optional feature (§3.2), so it can't be the key. The date rule is what 5e.tools itself uses ([utils.js L658](https://github.com/5etools-mirror-3/5etools-src/blob/v2.41.3/js/utils.js#L658)) and agreed with `edition` in all 512 records that have it. Synthetic copies are exactly the cross-edition mixing the spec forbids.
- *Rejected:* `edition` alone (too sparse); an automatic date rule with no allowlist (new or Universes Beyond books would enter silently); keeping synthetic copies with a warning (the 5e.tools note admits their feature levels may be wrong).

**E. Content IDs are ruleset-prefixed and derived from the 5e.tools key.** *(Amends 0008.)*
- *Decision:* `<ruleset>:<kind>/<slug(name)>|<source>`, with composite paths for subclasses and features. Each record also stores `upstream_key` and `upstream_hash`. Hand-made content uses source `mm5e`.
- *Rationale:* Deterministic, with no alias table. The ruleset is explicit for pack isolation. One upstream key can carry two encodings (2014 rules and legacy-compatible 2024). No collisions, against 25 for slug-only IDs.
- *Rejected:* bare `Name|SOURCE` (ambiguous across packs); slug without source (collisions, unstable when books are added); opaque UUIDs (lose greppability and the join).

**F. Encodings carry an upstream record hash; version bumps produce a staleness review.**
- *Decision:* Each encoding records the sha256 of its canonical resolved upstream record. On a bump, the importer reports `stale` for changed records, plus added and removed keys in scope. Stale encodings fail the coverage gate until reviewed.
- *Rationale:* 5e.tools applies errata in place, with no per-record version (CONTRIBUTING). Releases are weekly. A silently changed record means a silently wrong solver.
- *Rejected:* manual changelog reading; never bumping (freezes errata and new books).

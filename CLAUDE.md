# MinMax5e

A personal build solver for 5th-edition TTRPG characters (Rust, egui). It runs as a native app for the owner and as a WebAssembly build that friends open from an invite link. The design lives in `docs/adr/` (decisions) and `docs/architecture.md` (the map), and the research behind it in `docs/research/`. Code is a Cargo workspace; crates are added as the build order in `docs/architecture.md` §11 needs them.

## Project rules

- **Rules data stays local.** 5e.tools data, and anything derived from it (packs, caches, snapshots, generated code), lives only in the local cache outside the repo. The repo is public and holds code, our own encodings, and synthetic fixtures. Rules text in tracked files is either an attributed SRD quote or our own paraphrase. See ADR-0028.
- **The deployed site is a published artifact.** `dist/` follows the same rules as the repo (ADR-0028). Invite tokens and the signing key live only in Cloudflare Worker secrets, never in tracked files (ADR-0044).
- **ADRs outrank the spec.** `docs/spec.md` predates the ADRs. Where they disagree (data source, tech stack, product name), the ADRs win.
- **Before revisiting a decision**, read the ADR and the research note it cites. Record changes as an amendment or a superseding ADR, never as a silent edit.

## Working on a ticket

- **Each ticket gets its own branch** (`<issue>-<slug>`), pushed early so CI runs on it. Reviews compare against `main`.
- **Repo guards:** `cargo xtask deps`, `cargo xtask leak-scan` (exemptions in `xtask/leak-allow.toml`) and `cargo xtask adr-check`. CI runs them with fmt, clippy, tests, the wasm32 check and `cargo deny`.
- **Reviews** use `CODING_STANDARDS.md`. The review loop ends when a round finds no standards breaches and no spec gaps; smells alone don't call for another round.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues (SamSilmarilData/MinMax5e) via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `GLOSSARY.md` + `docs/adr/`. See `docs/agents/domain.md`.

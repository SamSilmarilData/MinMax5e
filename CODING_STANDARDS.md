# Coding standards

Read during review. Tooling already enforces fmt, clippy (`all` and `pedantic`, the float bans in `clippy.toml`), the crate dependency rules (`cargo xtask deps`), the leak scan, Content ID kinds named against GLOSSARY.md's _Avoid_ lists (`cargo xtask glossary`; avoided words in prose and identifiers stay with review), ADR immutability (`cargo xtask adr-check`) and native/WebAssembly bit identity of the hashed fixture Builds (`cargo xtask xhash`), so review spends its attention on the judgement calls below.

## Where the conventions live

- Workspace, crate boundaries and CI: [ADR-0034](docs/adr/0034-cargo-workspace.md) and its amendments.
- What may be committed: [ADR-0028](docs/adr/0028-no-upstream-bytes-in-public-repo.md).
- Determinism and banned float operations: [ADR-0037](docs/adr/0037-deterministic-parallel-solving.md), [ADR-0050](docs/adr/0050-solve-as-deterministic-job-plan.md).
- Toolchain, lints and determinism rules: [research 09](docs/research/09-rust-engine-core.md) §5–6.
- Vocabulary: [GLOSSARY.md](GLOSSARY.md).

## Judgement calls

- **One home per list.** A list held in config or code (`clippy.toml`, `deny.toml`, the crate table in `xtask/src/deps.rs`, `xtask/leak-allow.toml`), and the fixture list in `crates/mm-testkit/fixtures/README.md`, is the single source of truth. Other docs name the categories and point to it.
- **Deviations travel with their record.** Code that departs from a ticket, a spec or an ADR, including a rename of anything the spec names, carries a dated amendment saying what changed and why. A deviation is ADR-level when an ADR's stated outcome no longer holds, even when another ADR forces it. An ADR's amendment is a dated section in the ADR (`> *Amended YYYY-MM-DD (why):*`, newest first; `cargo xtask adr-check` enforces the form), in the same commit. A spec's or ticket's amendment is a dated comment on its GitHub issue, linked from the commit message. The amendment matches the code it describes, and points to other ADRs rather than repeating them.
- **A changed concept changes everywhere it is stated.** When a diff moves or redefines something an ADR, README, glossary entry, CI comment or issue amendment describes, review searches the repo for the old wording, and every statement of it is updated or amended.
- **Research errors go to the errata.** A test whose expected value departs from a figure in a research note adds an entry to the Errata in [`docs/research/README.md`](docs/research/README.md); the note itself stays as written.
- **Scope is the ticket's.** Work a ticket defers (to a later ticket or build-order step) is recorded as deferred in an amendment, and a review treats it as done.
- **Tests sit at agreed seams** (spec #12, "Testing Decisions"), assert behaviour through public interfaces, and take expected values from an independent source: a worked example, the spec, a hand-computed literal.
- **Floats compare bit for bit or within a stated bound.** The bounds and their sources live in `mm_testkit::tolerance`; a bound written inline is a breach.

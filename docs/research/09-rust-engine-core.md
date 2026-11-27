# 09: The Rust engine and solver core (crates, architecture, performance)

Status: research note, 2026-11-27. Nothing here is decided. The decisions worth recording are listed under **Candidate ADRs / revisions**.

Conventions:
- Every external claim cites a URL. Each was fetched in this session unless it is marked otherwise.
- **[unverified]** means the claim comes from my own knowledge or reasoning and I did not confirm it against a primary source in this session.
- **[estimate]** marks back-of-envelope arithmetic that a spike needs to confirm.
- **[design]** marks a proposal.
- Scope: the engine and solver core only. The UI and app shell are covered by a separate note and are not discussed here.
- **No measurements were taken.** Neither `cargo`/`rustc` nor `node` is installed on this machine (`command not found` for all three, and there is no `~/.cargo` or `~/.rustup`). Following my instructions, I did not install a toolchain. The micro-benchmark is specified as Spike S1 under Open questions so it can be run once Rust is installed.

---

## Question

The user has decided that MinMax5e is mainly for personal use (no public website) and should be "full Rust", with the best technology for each part. That raises six questions:

1. What does Rust actually buy over TypeScript for *this* workload: small-PMF convolution, DP over turn states, enumeration or branch-and-bound over 10^7–10^10 candidates, and memoisation by hash? How much does native multithreading on an M-series Mac add, and how does that change research 05's design?
2. Which crates should we use for each concern, and why?
3. How should the Effect DSL (ADR-0010) be expressed in Rust? Data files or macros? Should hooks be scripted?
4. How does Cargo enforce edition isolation (ADR-0008, ADR-0022)?
5. How do we keep results deterministic across threads?
6. Which toolchain, lints and CI setup should we use?

## TL;DR

- **Expect native Rust with rayon to be about 10–30× faster than single-threaded JS on this workload (≈20× central) [estimate].** That is 2–5× per core, times about 6× from the M5's 4 performance plus 6 efficiency cores. Against the Web Worker pool that research 07 planned, the gain is only 2–5×. Per-core ratios come from the Benchmarks Game: n-body 2.5×, spectral-norm 1.9×, fannkuch 1.6×, k-nucleotide (hash-heavy) 5.4×.
- **Native speed moves each problem class about one tier, not more.** The search space spans about 7 orders of magnitude (05) and Rust buys a little over one. Most of 05's algorithmic design (canonicalisation, dominance, kit memoisation, Screen→Refine, level-DAG DP, beams) is still required.
- **What changes:** live search becomes feasible for:
  - single-class snapshots (sub-second to a few seconds);
  - two-class snapshots once the user locks some choices (seconds);
  - single-class curves with a locked origin and weapon archetype (seconds).

  "Precomputed candidate pools" stop being shipped artifacts. They become (a) an in-memory pool kept from the last solve, so sliders still respond within 100 ms, and (b) an on-disk cache for the expensive partitions (unconstrained curves, three classes).
- **Crates:**
  - parallelism: `rayon`;
  - PMFs: plain `Box<[f64]>`/`Vec<f64>` plus an offset, scalar loops first and `wide` only if profiling demands it (not ndarray, not nightly `std::simd`);
  - hashing and caching: `rustc-hash` (FxHash), with `dashmap` for the cross-thread kit memo (moka deferred);
  - formats: `serde` + **RON** for hand-authored encodings, JSON for imports, **postcard** for the cache (rkyv if pool files get large);
  - **avoid bincode**: development has ceased (RUSTSEC-2025-0141) and the repository is archived;
  - errors: `thiserror` + `miette`;
  - tests: `num-rational` exact oracles, `proptest`, `insta` (RON snapshots), `divan`, `cargo-nextest`, `samply`.
- **DSL:** closed Rust enums matched exhaustively, deserialised from **RON data files loaded at runtime**. Hooks are **`fn` pointers in a per-ruleset registry**, each carrying a declaration (stage, costs, outputs). The approach rejects:
  - a macro DSL (compile times, no runtime loading);
  - trait objects and enum_dispatch (the hook set is open across crates but stateless);
  - embedded scripting such as rhai or mlua (opaque to the solver, and Rhai's own benchmarks put it 2–3× slower than Python).

  Predicates compile to bitmask tests in the Combat Kit, so dispatch cost never reaches the hot loop.
- **Edition isolation becomes type-level:**
  - a `Ruleset` trait;
  - `Id<R, Kind>` content IDs, so a cross-edition reference is a compile error in Rust hooks and a load error in data;
  - `rules-2024` and `rules-2014` crates that only `mm-cli` and the app shell may depend on, enforced by `cargo-deny` `bans.deny` + `wrappers`.
- **Determinism:** parallelise only *across* independent candidates. Each evaluation is sequential and pure. Reductions are top-K selections under a total order: score compared with `total_cmp` after quantising, then a stable canonical build key. Never use `par_iter().sum()` on floats (rayon documents it as non-deterministic). Prune strictly (`bound < incumbent`).
- **Toolchain:**
  - edition 2024;
  - a pinned stable toolchain (currently 1.100), with `rust-version` = the pinned version;
  - `[workspace.dependencies]` and `[workspace.lints]` (forbid `unsafe_code`; clippy `all` + selected pedantic);
  - `Swatinem/rust-cache@v2` in CI;
  - `cargo-deny` for both advisories and the edition-isolation bans.

---

## 1. Rust vs TypeScript for this workload

### 1.1 The workload, restated from research 03, 05 and 07

| Component | Shape | Source |
|---|---|---|
| PMF convolution | Dense f64 arrays of 40–900 entries, convolved with short kernels (dice, mixtures). Moment mode carries `(P, E, E²)` per state | 03 §2.1, §2.4 |
| Turn and rotation DP | Map `state → unnormalised PMF`, branching miss/hit/crit, merging equal states. Rotation MDP of 10^4–10^5 states for a Paladin 20 | 03 §3; 05 §3.3 |
| Enumeration / B&B | ~5×10^7 candidates (single class, L8), ~10^10 state evaluations (single class, curve), 10^13+ (three classes) | 05 §1.4 |
| Memoisation by hash | Kit hash → value; dice PMF cache; d20 table | 05 §2.1; 03 §7.1 |

### 1.2 What the published benchmarks say (native Rust vs Node.js)

The Computer Language Benchmarks Game compares Node.js v23.8.0 with Rust 1.84.1. The page reports elapsed seconds and **CPU seconds**, and CPU seconds are a fair proxy for per-core work ([node-rust](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/node-rust.html)). The hardware is a quad-core Intel machine **[unverified: the page fetched did not name the machine]**. I picked the closest proxies for our components and compared the best *non-flagged* program in each language where one exists. Flagged (`*`) programs use hand-written SIMD, unsafe code or FFI.

| Proxy | Why it resembles our work | Rust CPU-s | Node CPU-s | Per-core ratio |
|---|---|---|---|---|
| n-body (both single-threaded) | Scalar f64 loops over small arrays, like PMF mixing | 3.46 (Rust #3) | 8.55 (Node #6) | **2.5×** |
| spectral-norm | Dense f64 multiply-adds, like convolution | 2.85 (Rust #4) | 5.40 (Node #7, single thread) | **1.9×** |
| mandelbrot | f64 loops that vectorise well | 3.78 (Rust #4) | 15.86 (Node #3) | 4.2× |
| fannkuch-redux | Small-integer permutation enumeration, like candidate enumeration | 36.04 (Rust #2); 15.06 (flagged #6) | 58.31 (Node #8) | **1.6×** (3.9× flagged) |
| k-nucleotide | Hash-table heavy, like memoisation | 8.13 (Rust #7) | 43.91 (Node #3) | **5.4×** |
| binary-trees (marked "contentious") | Allocation heavy, like naive outcome trees | 3.84 (Rust #5) | 28.29 (Node #7) | 7.4× |

Memory is the other half of the story. Typical Rust entries use about 3–4 MB and Node entries about 66 MB or more on the same programs ([node-rust](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/node-rust.html)). That matters for big memo tables and DP layers.

**Compiling to WASM erases part of the gap.** Jangda et al. (USENIX ATC 2019) measured WebAssembly running 45% (Firefox) to 55% (Chrome) slower than native on average, with peaks of 2.08× and 2.5× ([USENIX](https://www.usenix.org/conference/atc19/presentation/jangda)). This is 2019 data and browsers have improved since **[unverified how much]**. Surma's binary-heap kernel ran in Rust→WASM at 1.9× (idiomatic) and 1.2× (optimised) the *time* of JS ([Surma](https://surma.dev/things/js-to-asc/)). He warns against generalising from it. Research 07 leaned on that result. It does not describe native Rust, and it was a single allocation-heavy kernel.

### 1.3 Mapping to our components [estimate]

| Component | Best proxy | Per-core Rust vs V8 | Notes |
|---|---|---|---|
| PMF convolution and mixture | n-body / spectral-norm | ~2× (up to ~3–4× with explicit SIMD) | Apple's NEON is only 2 × f64 wide, which caps SIMD gains (see §2.2) |
| Moment-mode Screen `(P,E,E²)` × 16 ACs | n-body | ~2–3× | Straight-line f64 code; V8 does well here too |
| Turn-state DP and memo | k-nucleotide | ~3–5× | JS `Map` needs a primitive or string key for composite states. Rust packs the state into a `u64` and hashes it with Fx **[unverified for JS key cost, widely known]** |
| Enumeration / B&B | fannkuch | ~1.6–4× | Mostly integer and branchy code |
| Naive object-per-node outcome trees | binary-trees | up to ~7× | Avoidable in either language with flat arrays, so it shouldn't drive the decision |

Blending these gives a **per-core factor of about 2–5×, central ≈ 3×**.

### 1.4 Native multithreading on this Mac

- The machine is an Apple M5: `sysctl` reports 10 cores, 4 performance and 6 efficiency. Apple describes the M5 as "a 10-core CPU made up of six efficiency cores and up to four performance cores" ([Apple Newsroom](https://www.apple.com/newsroom/2025/10/apple-unleashes-m5-the-next-big-leap-in-ai-performance-for-apple-silicon/)).
- Rayon starts one thread per logical processor unless `RAYON_NUM_THREADS` or `num_threads` says otherwise ([ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)).
- Candidate evaluation is embarrassingly parallel. If an efficiency core gives about 0.3–0.5 of a performance core's throughput **[unverified]**, 10 threads deliver roughly **5.8–7 performance-core equivalents**. At about 90% parallel efficiency that is a **≈5–6.5× speed-up** over one performance core **[estimate]**.

### 1.5 Bottom line

| Comparison | Expected speed-up of native Rust + rayon [estimate] |
|---|---|
| vs single-threaded TypeScript | **≈10–30×, central ≈20×** (3× per core × ~6× threads) |
| vs a TypeScript Web Worker pool on the same Mac (research 07's plan) | ≈2–5× (the per-core factor only) |
| vs Rust→WASM in a browser worker pool | ≈1.5× (Jangda) plus no COOP/COEP friction for shared memory |

**Honest reading:** Rust is not a magic fix. The decisive factors remain (a) the Screen→Refine split and (b) the pruning and memoisation in 05. Rust's real advantages for a personal-use tool:

1. Real shared-memory threads with no browser isolation headers.
2. Predictable performance, with no JIT warm-up or deoptimisation ([Surma](https://surma.dev/things/js-to-asc/)).
3. Compact state packing (u8/u16 fields, `u64` keys), which shrinks memo tables by about an order of magnitude ([memory column](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/node-rust.html)).
4. Exhaustive `match` on enums, where a missing arm is error E0004 ([Rust book](https://doc.rust-lang.org/book/ch06-02-match.html)).

Rust's costs:

- compile times;
- slower iteration on rules encodings, *unless* encodings are data loaded at runtime (§3.2), which this note recommends for that reason.

### 1.6 How native speed changes research 05's design

Planning figures, all **[estimate]** pending Spike S1:

- **Screen** (policy-aware moments for all 16 ACs, per kit): **1 µs** native per core, 3 µs in single-thread TypeScript. 05 measured 8.3 µs for its *toy* evaluator in CPython, and a real evaluator does more work.
- **Refine** (exact PMFs plus the rotation MDP): **0.03–1 ms** native. 05 estimates about 1 ms for a Paladin 20 MDP.
- **Parallelism:** 6× effective.

| Workload (counts from 05 §1.4, §1.2) | Count | Single-thread TS, Screen only | Native + rayon, Screen only | With kit memo + B&B (assume ÷10–100) [estimate] | Verdict with native |
|---|---|---|---|---|---|
| Single class, L8 snapshot | ~5×10^7 candidates | ~150 s | ~8 s | **~0.1–0.8 s** | **Live** on constraint change (1 s budget). Sliders re-score the in-memory pool (500 × 1 ms / 6 ≈ 80 ms) |
| ≤2 classes, L8 snapshot | ~3×10^9 [estimate: ~70× single class from 111 vs 6 end states × subclass pairs] | ~2.5 h | ~8 min | **~5–50 s** | Live *with progress* when the user locks a class or subclass; otherwise cached |
| Single class, L1–20 curve, origin + weapon archetype locked | 2.6×10^6 DP states | ~8 s | ~0.4 s | — | **Live** |
| Single class, L1–20 curve, unconstrained | ~10^10 | ~8 h | ~28 min | ~0.3–3 min | Background job → disk cache, one-off per pack version |
| ≤2 classes, curve, per origin | 2.95×10^8 | ~15 min | ~50 s | ~1–10 s | Live per locked origin; unconstrained needs beams (×10^4 origins) |
| ≤3 classes, curve, per origin | 6.0×10^9 | ~5 h | ~17 min | ~10–100 s | Beam DP + bounds with an optimality-gap certificate (05 §6), even when native |

Consequences [design]:

1. **ADR-0020's "offline pools shipped with the static site" goes away.** There is no static site. Pools become:
   - (a) the **in-memory pool** from the last live solve: the union of top-N over the AC × rounds × advantage × resistance grid, which keeps slider moves within 100 ms;
   - (b) an **on-disk cache** (§2.4) for partitions too expensive to solve live.

   The ADR-0020 "fallback" B&B becomes the *primary* path for single-class and constrained two-class queries.
2. **Screen→Refine stays.** Exact PMFs for 10^7 candidates are still not interactive in any language (07 §2).
3. **Kit memoisation matters more than the language.** A shared `DashMap<KitKey, Arc<KitValue>>` across rayon tasks means each distinct kit is solved once per process. A per-pack-version on-disk cache means once per pack version.
4. **The rotation MDP cost is the biggest uncertainty.** Policy-aware Screen moments (ADR-0013) for slot-using kits need the MDP. If that costs about 1 ms, Screen drops to Refine speed. Mitigation: B&B upper bounds from kit relaxation (05 §2.5) need no MDP, so the MDP runs only for survivors. Spike S2 measures it.
5. **Smooth-curve objectives** need V*_L per level (05 §4.3). Precompute the 20 per-level frontiers in the background on first run and cache them on disk.

---

## 2. Crate choices

Versions are those docs.rs showed on 2026-11-27.

| Concern | Choice | Why | Rejected or deferred |
|---|---|---|---|
| Parallelism | **rayon 1.12** ([docs](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html)) | Work-stealing `par_iter` over candidate chunks; `map_init` gives per-thread scratch buffers; a pool sized by `available_parallelism` by default ([ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)) | `std::thread::scope` by hand: fine but reinvents load balancing. tokio: async I/O runtime, wrong tool for CPU-bound work [unverified, well known] |
| PMF storage | **`Box<[f64]>` + `offset: i32`**, scratch `Vec<f64>` reused per thread [design] | 1-D, a few hundred entries. A plain slice gives LLVM the best chance to auto-vectorise, adds no dependency, and keeps exactly-reproducible operation order | **ndarray**: n-dimensional machinery we don't need [unverified: no ndarray docs fetched]. **FFT**: no gain at these sizes and negative probabilities (03 §2.1) |
| SIMD | **None at first; `wide` 1.7 if profiling shows convolution dominates** | `wide` runs on stable Rust and uses explicit intrinsics on "x86, x86_64, wasm32 and aarch64 neon"; "Aarch64 always enables the NEON extension" ([wide README](https://github.com/Lokathor/wide), [docs](https://docs.rs/wide/latest/wide/)) | **`std::simd`**: still behind `#![feature(portable_simd)]`, stabilisation "mostly blocked" ([tracking issue](https://github.com/rust-lang/rust/issues/86656)). **pulp 0.22**: its strength is runtime dispatch across x86 feature levels ([docs](https://docs.rs/pulp/latest/pulp/)). It has an `aarch64` module ([aarch64 docs](https://docs.rs/pulp/latest/aarch64-apple-darwin/pulp/)), but NEON is baseline on Apple, so dispatch buys nothing here |
| Hashing | **rustc-hash 2.1** (`FxHashMap`) | Fast, unkeyed and deterministic: "a polynomial hash finished with a single bit rotation" plus a wyhash-inspired string compressor ([docs](https://docs.rs/crate/rustc-hash/latest)). Not DoS-resistant, which doesn't matter for keys we build ourselves | **ahash 0.8**: keyed and DoS-resistant, but keys come from the OS by default, so hashes vary between runs, and output "may" differ across machines or versions ([docs](https://docs.rs/ahash/latest/ahash/)). That cost buys nothing for a local tool |
| Cross-thread memo | **dashmap 6.2** for kit → value ([docs](https://docs.rs/dashmap/latest/dashmap/)) | Sharded concurrent map. Caveat: `insert`/`entry` "may deadlock if called when holding any sort of reference into the map" ([DashMap](https://docs.rs/dashmap/latest/dashmap/struct.DashMap.html)). Rule: clone the `Arc` out and never hold a guard across a call | Per-thread `FxHashMap` merged afterwards: simpler, but solves duplicates per thread. Use it for the *small* caches (dice PMFs, d20 table) |
| Bounded cache | **Deferred: moka 0.12** | Concurrent, Caffeine-inspired, TinyLFU admission, bounded by entry count or weight ([docs](https://docs.rs/moka/latest/moka/)). Adopt only if memo memory becomes a problem | An LRU written by hand |
| Hand-authored encodings | **serde + RON 0.12** | RON supports "all of Serde's data model", including enum variants written like Rust (`Struct(x: 1.0)`), with comments ([docs](https://docs.rs/ron/latest/ron/)). Encodings are mostly enums | JSON: no comments, and enums need tag objects. TOML: awkward for deeply nested enum trees [unverified, well known] |
| Imports / interop | **serde_json** into the same types | User imports (ADR-0003) and any 5e.tools-shaped structured fields arrive as JSON. The same `#[derive(Deserialize)]` types serve both formats | — |
| Cache format | **postcard 1.2** | "As of v1.0.0, postcard has a documented and stable wire format"; serde-based ([docs](https://docs.rs/postcard/latest/postcard/)). Compact: 725 KB vs 1.83 MB for serde_json on the `log` dataset ([rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark)) | **rkyv 0.8** (zero-copy; readable only while schema, format flags and semver-compatible version stay the same ([docs](https://docs.rs/rkyv/latest/rkyv/))): adopt for memory-mapped pool files if load time matters. **bitcode 0.6**: fastest in the same benchmark, but I did not verify its format-stability policy. **bincode: avoid.** The GitHub repo was "archived by the owner on Aug 15, 2025" and moved to SourceHut ([repo](https://github.com/bincode-org/bincode)). RUSTSEC-2025-0141 (issued 2026-01-07) says "the bincode team has taken the decision to cease development permanently" and suggests wincode, postcard, bitcode or rkyv ([advisory](https://rustsec.org/advisories/RUSTSEC-2025-0141)) |
| Interning / IDs | **Dense typed indices built at pack load** (`Id<R, Kind>(u32)`, `Vec` arenas) [design]. Load-time name → id map in an `FxHashMap<Box<str>, u32>` | The content set is frozen after load, so a `Vec` indexed by a newtype is the cheapest possible arena | **lasso 0.7** (32-bit `Spur`; `RodeoReader`/`RodeoResolver` read-only views ([docs](https://docs.rs/lasso/latest/lasso/))) and **string-interner 0.20** ([docs](https://docs.rs/string-interner/latest/string_interner/)): only useful if open-ended strings reach the hot path, which typed facts prevent. **slotmap 1.1** (generational keys, `SecondaryMap` ([docs](https://docs.rs/slotmap/latest/slotmap/))): for collections with deletion, such as user-edited plans in a UI, not for frozen content |
| Library errors | **thiserror 2.0**, which "deliberately does not appear in your public API" ([docs](https://docs.rs/thiserror/latest/thiserror/)) | Typed `CompileError`, `Illegal(reason)` and `LoadError` enums | anyhow in libraries (fine in `mm-cli` only) |
| Authoring diagnostics | **miette 7.6**: `#[derive(Diagnostic)]`, `#[label]` spans, `NamedSource`, `fancy` feature in the top-level binary only ([docs](https://docs.rs/miette/latest/miette/)) | Pack-load errors such as "unknown Content ID `2024:feat/great-weapn-master`" point at the file and span | Plain `Display` errors. **Gap:** RON reports positions for *syntax* errors, but I found no `Spanned<T>` wrapper for *semantic* errors like the one the `toml` crate has **[unverified for both]**. Fallback: report the Content ID plus the field path |
| Exact oracle | **num-rational 0.4** (`Ratio`, `Rational64`, `BigRational` "arbitrary precision rationals") ([docs](https://docs.rs/num-rational/latest/num_rational/)) | Golden tests per ADR-0023. Use `BigRational`: denominators of 20^k × S^n overflow 64 bits within a few rounds [estimate] | Rationals at runtime (ADR-0023: too slow) |
| Property tests | **proptest 1.x**: explicit `Strategy` values, composable, with shrinking. It "mainly sees passive maintenance" ([repo](https://github.com/proptest-rs/proptest)) | Builds and encounters need constrained generators, so strategies beat type-driven generation | **quickcheck**: type-only generation, hand-written shrinkers, and it can't express constraints (per proptest's comparison). Proptest can be "up to an order of magnitude slower" at generation, which is acceptable |
| Snapshots | **insta 1.49** with `assert_ron_snapshot!`, redactions, `glob!` and `cargo insta review` ([docs](https://docs.rs/insta/latest/insta/)) | Golden solver outputs (top-3 plus Turn Plan) as reviewable RON diffs | Hand-rolled golden files |
| Benchmarks | **divan 0.1** (`#[divan::bench]`, `AllocProfiler` to catch hot-loop allocations) ([docs](https://docs.rs/divan/latest/divan/)) | Low ceremony, and allocation profiling directly checks the "no allocation per candidate" rule | **criterion 0.8** (moved to the criterion-rs org ([old repo](https://github.com/bheisler/criterion.rs), [crate](https://docs.rs/crate/criterion/latest))): keep as the option if CI needs statistical baseline comparisons |
| Test runner | **cargo-nextest**: "Up to 3× faster than `cargo test`", per-test isolation, retries, JUnit output ([nexte.st](https://nexte.st/)) | Fast local loop and a CI profile | Plain `cargo test`, still needed for doctests **[unverified: nextest doesn't run doctests]** |
| Profiling | **samply**: uses the Firefox Profiler UI, runs on macOS ([repo](https://github.com/mstange/samply)) | No root needed, and it shows rayon threads | **cargo-flamegraph**: on macOS it relies on `xctrace` ([repo](https://github.com/flamegraph-rs/flamegraph)). Fine as a second option |

### 2.1 PMF kernel sketch [design]

```rust
// crates/mm-dice/src/pmf.rs
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pmf { offset: i32, p: Box<[f64]> }        // P(X = offset + i) = p[i]

pub struct Scratch(Vec<f64>);                         // one per rayon worker via map_init

impl Pmf {
    pub fn die(sides: u8, gwf: bool) -> Pmf { /* … */ }
    /// Direct convolution in a fixed loop order → bit-identical on every run.
    pub fn conv_into(&self, other: &Pmf, out: &mut Scratch) -> Pmf {
        let (a, b) = (&self.p, &other.p);
        let buf = &mut out.0; buf.clear(); buf.resize(a.len() + b.len() - 1, 0.0);
        for (i, &ai) in a.iter().enumerate() {            // b is the short kernel (a die)
            for (j, &bj) in b.iter().enumerate() { buf[i + j] += ai * bj; }
        }
        Pmf { offset: self.offset + other.offset, p: buf.as_slice().into() }
    }
    pub fn mix(parts: &[(f64, &Pmf)], out: &mut Scratch) -> Pmf { /* weighted sum */ }
    pub fn mean(&self) -> f64 { /* sequential sum, fixed order */ }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Moments { pub p: f64, pub e: f64, pub e2: f64 }   // Screen tier (03 §2.4)
```

**SIMD note.** NEON holds 2 × f64. The inner loop has a short trip count (die widths of 2–12), so loop overhead and memory traffic matter more than lanes. That is why `wide` is deferred until samply shows convolution dominating [estimate].

### 2.2 Interning and hashing of turn states [design]

`TurnState` packs into a `u64`: hands (2×5 bits), action/bonus/reaction (3 bits), attack slots (3 bits), once-per-turn flags (16 bits), `vex_pending` (1 bit), concentration id (8 bits), slots remaining (5 levels × 3 bits). The memo is an `FxHashMap<u64, usize>` into a `Vec<Pmf>`. Any state that doesn't fit uses a `u128` key. No allocation per state, and hashing is a single multiply.

### 2.3 The on-disk cache [design]

- Key: `blake3(engine_version, pack_content_hash, rulings, query_partition)` → `postcard` file under `~/Library/Caches/minmax5e/`. The hash crate is **[unverified]**; any stable hash works.
- The engine version is part of the key, so the format does not have to stay stable across releases. That is why postcard is enough, and rkyv is optional.

---

## 3. The Effect DSL in Rust (ADR-0010)

### 3.1 Closed primitives as enums

```rust
// crates/mm-core/src/effect.rs (edition-agnostic; no 2024/2014 knowledge)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deserialize, Serialize)]
pub enum Stage {                      // ADR-0011: fixed, no priorities
    TurnStart, AttackDeclare, AttackModifiers, AttackRoll,
    AttackHit, AttackCrit, AttackMiss, DamageRiders,
    DamageAfterHitDecide, DamageResolve, AttackAfter, TurnEnd, RoundBoundary,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Fact {                       // typed facts, not strings (04 open question 7)
    OutcomeHitOrCrit, DealtDamage, UsesAttackRoll,
    MeleeWeapon, RangedWeapon, UnarmedStrike,
    WeaponProperty(WeaponProperty), WeaponMastery(MasteryProperty),
    NetAdvantage, NetDisadvantage,
    AllyAdjacentToTarget,             // Encounter toggle / probability (04 OQ3)
    TargetType(CreatureType),
    TargetHasStatusFromSelf(IdRef),   // IdRef = unresolved "2024:status/…" string in data
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Pred { Fact(Fact), All(Vec<Pred>), Any(Vec<Pred>), Not(Box<Pred>) }

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Dice {
    Fixed { count: u8, die: u8 },
    Scaled { count: ScaleRef, die: u8 },                       // e.g. sneak_dice
    PerSlotLevel { base: u8, per_level_above_1: u8, die: u8 }, // Divine Smite
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Op {
    Advantage,
    ExtraDamage { dice: Dice, damage_type: DamageTypeRef, from_attack: bool },
    GrantStatus { status: IdRef, on: Subject, expires: Expiry, consumed_by: Consumption },
    FlatModifier { target: Stat, value: i8 },
    Upgrade { target: Stat, value: i8 }, Set { target: Stat, value: i8 },
    When { cond: Pred, then: Box<Op> },
    All(Vec<Op>),
    Hook(HookRef),                    // escape hatch, resolved against R::hooks() at load
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Rider {
    pub on: Stage,
    #[serde(default)] pub when: Option<Pred>,
    #[serde(default)] pub limit: Option<Limit>,      // OncePerTurn, OncePerRound, UsesPer(Rest, n)
    #[serde(default)] pub optional: bool,            // true → Policy decision (ADR-0014)
    #[serde(default)] pub cost: Option<CostSpec>,    // OneOf([...]) of BA / slot / feature uses
    pub effect: Op,
    #[serde(default)] pub consume: bool,
}
```

Every consumer `match`es these exhaustively. Adding a primitive breaks compilation until every stage handles it, which is the property ADR-0010 relies on ([E0004](https://doc.rust-lang.org/book/ch06-02-match.html)).

### 3.2 Encodings as RON data, loaded at runtime (not a macro DSL)

| Criterion | RON/JSON data loaded at runtime | Rust embedded DSL (`macro_rules!` / proc macro) |
|---|---|---|
| Edit → re-run loop | Edit the file, re-run. No rebuild | Recompile the rules crate and everything downstream on every edit **[unverified: seconds to tens of seconds; proc macros are the slowest part]** |
| Runtime loading (user imports, ADR-0003) | Native: the same types deserialise imported JSON | Impossible without a second, data-based path anyway |
| Type checking | At load, into the same Rust enums. Errors come with miette diagnostics. A `#[test]` loads every shipped pack, so `cargo test` catches typos | At compile time, but macro error messages are notoriously poor **[unverified]** |
| Solver inspectability | Full: it's data | Full, if the macro expands to the same enums |
| LLM-drafted encodings (ADR-0002) | Draft against the schema, validate by loading | Generated Rust code needs a compile cycle |

**Decision [design]:** encodings live in `data/rules-2024/**/*.ron`. In release builds they are embedded with `include_str!` so the binary is self-contained. A `--pack-dir` flag overrides them with live files during authoring. JSON is accepted for imports. A JSON Schema could be generated for editor completion with `schemars` **[unverified]**.

### 3.3 Hooks: `fn` pointers in a per-ruleset registry

```rust
// crates/mm-core/src/hook.rs
pub struct HookDecl<R: Ruleset> {
    pub id: &'static str,                    // "2024:hook/rogue/cunning-strike"
    pub stage: Stage,                        // declared: the solver can inspect without running
    pub costs: &'static [CostKind],
    pub outputs: &'static [OutputKind],
    pub run: fn(&TurnView<'_, R>, &Event, &HookArgs) -> EffectList,   // pure: no &mut, no globals
}

// crates/mm-rules-2024/src/hooks.rs
pub static HOOKS: &[HookDecl<Rules2024>] = &[
    HookDecl { id: "2024:hook/rogue/cunning-strike", stage: Stage::DamageRiders,
               costs: &[CostKind::ScaledDice("sneak_dice")], outputs: &[OutputKind::SaveEffect],
               run: cunning_strike },
];
```

| Option | Verdict |
|---|---|
| **`fn` pointer + static declaration** | **Choose.** Hooks are stateless and pure, and parameters come from data (`HookArgs`), so there is no captured state. Pointers are `Copy`, `Send + Sync` and storable in a `static` slice. The rules crate owns them, and the engine sees only `R::hooks()`. |
| `Box<dyn Hook>` / `&'static dyn Hook` | Works, but adds nothing over `fn` for stateless functions. A trait tempts authors to add state and methods, which erodes purity. |
| `enum_dispatch` ("up to 10x" faster than dynamic dispatch in its own benchmarks ([docs](https://docs.rs/enum_dispatch/latest/enum_dispatch/))) | Reject. It needs one enum listing every implementing type at compile time, but the hook set lives in separate ruleset crates. And the speed doesn't matter (next paragraph). |

**Dispatch cost never reaches the hot loop.** The engine *compiles* a Build into a Combat Kit. Each rider becomes a row with a precompiled predicate in disjunctive normal form, tested as `required & facts == required && forbidden & facts == 0` over a per-attack `FactSet(u64)` bitmask [design]. Hooks run during kit compilation, or at most once per (state, outcome) node in the turn DP, never per candidate × AC × outcome inside the Screen loop.

### 3.4 Embedded scripting (rhai, mlua): recommend against for now

- **Rhai 1.27:** a pure-Rust embedded language with a `sync` feature ([docs](https://docs.rs/rhai/latest/rhai/)). Its own benchmark page says the AST interpreter is "roughly 2-3x slower than Python 3" ([Rhai benchmarks](https://rhai.rs/book/about/benchmarks.html)).
- **mlua 0.12:** Lua 5.1–5.5, LuaJIT and Luau. It is `!Send` unless `feature = "send"` ([docs](https://docs.rs/mlua/latest/mlua/)), and it brings a C dependency.
- **Arguments against:**
  1. A scripted hook is just as opaque to the solver as Rust code, so it still needs the same declaration.
  2. ADR-0010 rejected arbitrary script hooks for purity and testability, and that argument doesn't depend on whether packs are trusted.
  3. Hooks are expected to be about 10–15% of the roughly 150–200 MVP encodings (04 §2.1). Writing those in Rust is cheap and gets them tested.
  4. Interpreter speed would matter if a hook ran in the turn DP.
- **When to revisit:** personal use removes the "unsafe imported packs" concern. If user imports routinely need behaviour the primitives can't express, add a Rhai hook kind that runs only at kit-compile time, with a declared signature.

### 3.5 The three reference encodings (pack `rules-2024`, RON)

Content IDs follow ADR-0008 (`<ruleset>:<kind>/<slug>`). Rules readings follow research 04 §3; the Rulings are marked.

**Vex (mastery property)**

```ron
// data/rules-2024/masteries/vex.ron
Mastery(
    id: "2024:mastery/vex",
    riders: [
        Rider(
            on: AttackAfter,
            when: Some(All([Fact(OutcomeHitOrCrit), Fact(DealtDamage), Fact(WeaponMastery(Vex))])),
            effect: GrantStatus(
                status: "2024:status/vex-pending",
                on: Target,                        // per-target, crosses the turn boundary (03 §3.2)
                expires: EndOfYourNextTurn,
                consumed_by: YourNextAttackRollAgainstIt,
            ),
        ),
    ],
    statuses: [
        Status(id: "2024:status/vex-pending", riders: [
            Rider(on: AttackModifiers,
                  when: Some(Fact(TargetHasStatusFromSelf("2024:status/vex-pending"))),
                  effect: Advantage, consume: true),
        ]),
    ],
)
```

**Sneak Attack (2024 Rogue)**

```ron
// data/rules-2024/classes/rogue/sneak-attack.ron
Feature(
    id: "2024:feature/rogue/sneak-attack",
    scales: { "sneak_dice": ByClassLevel(class: "2024:class/rogue",
              table: [1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10]) },
    riders: [
        Rider(
            on: DamageRiders,
            limit: Some(OncePerTurn),              // any creature's turn; keyed by whose turn (ADR-0012)
            optional: false,                       // Ruling R-SA-1: apply at first eligible hit
            when: Some(All([
                Fact(OutcomeHitOrCrit), Fact(UsesAttackRoll),
                Any([Fact(WeaponProperty(Finesse)), Fact(RangedWeapon)]),
                Any([
                    Fact(NetAdvantage),            // net, after the tri-state collapse
                    All([Fact(AllyAdjacentToTarget), Not(Fact(NetDisadvantage))]),
                ]),
            ])),
            effect: ExtraDamage(dice: Scaled(count: "sneak_dice", die: 6),
                                damage_type: WeaponDamageType, from_attack: true),  // doubles on crit
        ),
    ],
)
```

**Divine Smite (2024 spell) with Paladin's Smite**

```ron
// data/rules-2024/spells/divine-smite.ron
Spell(
    id: "2024:spell/divine-smite",
    level: 1, casting_time: BonusAction, concentration: false,
    trigger: Rider(
        on: DamageAfterHitDecide,                  // "immediately after hitting"
        optional: true,                            // Policy decides (ADR-0014, ADR-0015)
        when: Some(All([Fact(OutcomeHitOrCrit), Any([Fact(MeleeWeapon), Fact(UnarmedStrike)])])),
        cost: Some(OneOf([
            Cost(bonus_action: 1, spell_slot: Some(Slot(min_level: 1, upcast: true))), // sets slot_spent_this_turn
            Cost(bonus_action: 1, uses: Some("2024:feature/paladin/paladins-smite")),  // no slot; Ruling: cast at 1st level
        ])),
        effect: All([
            ExtraDamage(dice: PerSlotLevel(base: 2, per_level_above_1: 1, die: 8),
                        damage_type: Radiant, from_attack: true),                 // Ruling R-DS-1: doubles on crit
            When(cond: Any([Fact(TargetType(Fiend)), Fact(TargetType(Undead))]),
                 then: ExtraDamage(dice: Fixed(count: 1, die: 8), damage_type: Radiant, from_attack: true)),
        ]),
    ),
)
```

None of the three needs a hook. `cost` and `optional` make the smite decision visible to the rotation MDP, and `limit` makes Sneak Attack's once-per-turn flag a known bit of the `TurnState` key. All three load into the same Rust enums, and an unknown variant name is a load error with a miette span.

---

## 4. Type-level edition isolation (ADR-0008, ADR-0022)

### 4.1 The `Ruleset` trait and typed IDs [design]

```rust
// crates/mm-core/src/ruleset.rs
pub trait Ruleset: Sized + Send + Sync + 'static {
    const PREFIX: &'static str;                 // "2024" | "2014"; checked on every IdRef at load
    type Economy: EconomyRules;                 // 2024: one slot per turn; 2014: BA-spell rule
    type Crit: CritRule;
    fn stacking() -> StackingRules;
    fn hooks() -> &'static [HookDecl<Self>];
}

pub struct Id<R: Ruleset, K>(u32, PhantomData<fn() -> (R, K)>);   // Copy, Eq, Hash; 4 bytes

pub struct Pack<R: Ruleset> { /* frozen, index-resolved content; only constructible via load_pack */ }
```

- **In data:** an `IdRef` string whose prefix is not `R::PREFIX` fails `load_pack::<R>` (ADR-0008).
- **In Rust hooks:** a `rules-2014` hook can't even name `Id<Rules2024, Feat>`. The crate doesn't depend on `rules-2024`, and the type parameter keeps a 2014 ID from flowing into a `Pack<Rules2024>`.
- **The engine is generic over `R`,** so it contains no `if edition` branches. Monomorphisation compiles the engine twice, once per ruleset. That costs compile time, not runtime [estimate].

### 4.2 Workspace layout [design]

```
minmax5e/
├─ Cargo.toml                 # [workspace] resolver = "3"; members = ["crates/*"]
│                             # [workspace.package] edition = "2024", rust-version = "1.100"
│                             # [workspace.dependencies] rayon, serde, ron, … ; [workspace.lints]
├─ rust-toolchain.toml        # channel = "1.100", components = ["rustfmt", "clippy"]
├─ deny.toml                  # advisories (bincode etc.) + edition-isolation bans
├─ .config/nextest.toml
├─ data/
│  ├─ rules-2024/**.ron       # hand-authored encodings (ADR-0002), embedded at release build
│  └─ rules-2014/**.ron
└─ crates/
   ├─ mm-dice/                # Distribution Kernel           deps: (none; opt. wide)
   ├─ mm-core/                # IDs, Ruleset trait, Effect DSL, deps: mm-dice, serde, thiserror
   │                          # HookDecl, Build/Choice/Encounter/Ruling types
   ├─ mm-load/                # RON/JSON → Pack<R>, coverage   deps: mm-core, ron, serde_json, miette
   ├─ mm-engine/              # compile → CombatKit, TurnState, deps: mm-core, mm-dice, rustc-hash
   │                          # legal_steps/apply, evaluator, rotation DP
   ├─ mm-solver/              # enumeration, dominance, B&B,   deps: mm-engine, rayon, dashmap
   │                          # level-DAG DP, Screen→Refine, top-K/MMR, cancellation
   ├─ mm-cache/               # on-disk result cache           deps: mm-solver (types), postcard
   ├─ mm-rules-2024/          # Rules2024: Ruleset + hooks     deps: mm-core ONLY
   ├─ mm-rules-2014/          # Rules2014: Ruleset + hooks     deps: mm-core ONLY
   ├─ mm-testkit/ (dev)       # MC oracle, BigRational oracle, proptest strategies, golden fixtures
   └─ mm-cli/                 # bin: solve | validate-packs | golden | bench   (+ anyhow, miette fancy)
   (UI shell crate(s): out of scope for this note; they depend on mm-solver + packs like mm-cli)
```

Dependency rules:

1. `mm-dice` → nothing. `mm-core` → `mm-dice`. `mm-engine` → `mm-core`. `mm-solver` → `mm-engine`.
2. **`mm-engine`, `mm-solver`, `mm-load` and `mm-cache` must not depend on any `mm-rules-*` crate.** Packs are injected through `R`.
3. `mm-rules-2024` and `mm-rules-2014` depend only on `mm-core`, and never on each other.
4. Only the binaries (`mm-cli`, the UI shell) and `mm-testkit` may depend on rules crates.

Enforcement uses three layers:

- **The Cargo graph itself.** A crate can't `use` what it doesn't depend on.
- **`cargo-deny` `bans.deny` with `wrappers`.** This "allows specific crates to have a direct dependency on the banned crate but denies all transitive dependencies on it" ([cargo-deny bans](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html)):
  ```toml
  [bans]
  deny = [
    { crate = "mm-rules-2024", wrappers = ["mm-cli", "mm-app", "mm-testkit"] },
    { crate = "mm-rules-2014", wrappers = ["mm-cli", "mm-app", "mm-testkit"] },
    { crate = "bincode" },
  ]
  ```
  Whether cargo-deny applies bans to *workspace-member* path crates as well as registry crates is **[unverified]**.
- **A fallback test.** An `xtask`-style test reads `cargo metadata` and asserts rules 1–4, so the boundary holds whatever the cargo-deny answer turns out to be.

### 4.3 Public interfaces (deep modules, narrow interfaces) [design]

| Crate | Public interface (all of it) | Hides |
|---|---|---|
| `mm-dice` | `Pmf` (`die`, `conv_into`, `mix`, `shift`, `halve_floor`, `cap_at`, `mean`, `var`, `quantile`, `tail_ge`), `Moments`, `d20::outcome(bonus, ac, crit_on, mode) -> OutcomeProbs`, `Scratch` | Loop order, SIMD, epsilon/truncation policy, exact integer evaluation of `((t−1)/20)^k` |
| `mm-core` | `Ruleset`, `Id<R,K>`, Effect DSL enums, `HookDecl<R>`, `Build<R>`, `Choice`, `Encounter`, `RulingSet` | Nothing much; this is the shared vocabulary, kept small |
| `mm-load` | `load_pack::<R>(sources) -> Result<Pack<R>, Diagnostics>`, `coverage(&Pack<R>) -> CoverageReport` | RON/JSON parsing, ID resolution and prefix checks, hook resolution, DNF predicate compilation |
| `mm-engine` | `compile(&Pack<R>, &Build<R>, &RulingSet) -> Result<CombatKit, CompileError>`; `CombatKit::key() -> KitKey`; `validate(&CombatKit, &TurnPlan) -> Result<(), Illegal>`; `evaluate(&CombatKit, &Encounter, Mode, Fidelity) -> Evaluation` (`Fidelity::{Moments, Exact}`) | `TurnState` packing, `legal_steps`/`apply`, outcome-tree merging, rotation backward induction, attribution |
| `mm-solver` | `solve(&Pack<R>, &Query, &Control) -> SolveResult`; `Control { cancel: &AtomicBool, progress: &dyn Fn(Progress) + Sync }`; `rescore(&Pool, &Encounter) -> Ranked` | Enumeration order, canonicalisation, dominance, bounds, B&B incumbent, DP/beam, kit memo, top-K heaps, MMR |
| `mm-cache` | `Cache::open(dir)`, `get(&CacheKey) -> Option<Pool>`, `put` | Key hashing, postcard/rkyv, invalidation by engine version and pack hash |
| `mm-rules-20xx` | `pub struct Rules20xx; impl Ruleset`, `pub fn embedded_sources() -> Sources` | Hooks, economy and crit rules |

---

## 5. Determinism

The facts:

- Rayon's `sum` order "is not specified" and "floating point numbers" make it "not fully deterministic" ([ParallelIterator](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html)).
- `reduce` needs an associative operator "or else the results will be non-deterministic", and `fold` split points are nondeterministic (same page).
- Task order "is not intended to be guaranteed" ([ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html)).
- Rust's basic float operations "follow IEEE 754-2008". Only NaN bit patterns are nondeterministic, and functions outside the specified set (e.g. `powf`) have no guarantee ([RFC 3514](https://rust-lang.github.io/rfcs/3514-float-semantics.html)).

The rules [design]:

1. **Parallelise across candidates, never within one evaluation.** Each `evaluate` is sequential with a fixed loop order, so its bits don't depend on the thread.
2. **Reductions are selections, not sums.** Top-K uses a total order: `(quantise(score), canonical_build_key)`. Here `quantise` rounds to 1e-9 relative, so 1-ulp differences (e.g. a scalar vs a future SIMD path, or aarch64 vs x86) can't reorder near-ties; the key breaks ties. Selection under a total order is associative and commutative, so any rayon split gives the same top-K.
3. **No `par_iter().sum::<f64>()` for reported numbers.** If an aggregate is needed, `collect` into a `Vec` indexed by candidate and sum sequentially, or sum fixed-size chunks and combine them in chunk-index order.
4. **Prune strictly.** B&B prunes only when `bound < incumbent_k`, never `<=`. Ties survive and the key decides, so timing-dependent incumbent updates change *how much* work is done, not the answer. The incumbent is an `AtomicU64` holding the f64 bits; it only grows.
5. **Memo races are benign by construction.** Two threads may compute the same kit at once. Both results are bit-identical because evaluation is pure, so whichever `DashMap` insert wins doesn't matter.
6. **No transcendental functions in scored values.** d20 probabilities are computed as integers `(t−1)^k` and `20^k`, then one division.
7. **No fast-math and no implicit FMA.** Use `mul_add` either everywhere in a kernel or nowhere.
8. **No output depends on `HashMap` iteration order.** FxHash is unkeyed, but parallel insertion order isn't fixed. Sort by key before emitting.
9. **Test it.** An insta snapshot of the full `SolveResult` must be byte-identical with `RAYON_NUM_THREADS=1` and `=10` (a property test across random queries).

---

## 6. Recommended toolchain

- **Edition 2024**, stabilised in Rust 1.85.0 on 2025-02-20 ([edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)). Cargo resolver v3 (MSRV-aware) is the 2024 default **[unverified detail; listed in the edition guide summary]**.
- **Toolchain pinned** in `rust-toolchain.toml` to the current stable, **1.100.0** ([releases.rs](https://releases.rs/); 1.101 is due 2026-12-24).
  - For a personal project the MSRV is simply the pinned version, so set `rust-version = "1.100"` in `[workspace.package]`.
  - Concrete reason to be at least 1.99: in edition-2024 packages, a member's `default-features = false` overrides the workspace dependency's setting only from Rust 1.99 ([Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)).
  - Bump deliberately, about once per quarter.
- **`[workspace.dependencies]`** for every third-party crate, and `[workspace.lints]` inherited with `[lints] workspace = true` (respected since 1.74) ([Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)):
  ```toml
  [workspace.lints.rust]
  unsafe_code = "forbid"            # wide/rayon/dashmap encapsulate their own unsafe
  missing_debug_implementations = "warn"
  [workspace.lints.clippy]
  all = { level = "warn", priority = -1 }
  pedantic = { level = "warn", priority = -1 }
  module_name_repetitions = "allow"
  must_use_candidate = "allow"
  float_cmp = "deny"                # force total_cmp / explicit tolerance
  unwrap_used = "warn"              # allow in tests via clippy.toml
  dbg_macro = "deny"
  ```
  The `priority` syntax and the group membership of individual lints are **[unverified]**. Check them against the clippy lint list when wiring this up.
- **rustfmt:** use the defaults. Avoid unstable options such as `imports_granularity`, which would need nightly **[unverified]**.
- **Profiles:**
  - `[profile.release] lto = "thin", codegen-units = 1, debug = "line-tables-only"` (line tables for samply);
  - `[profile.dev.package."*"] opt-level = 2`, so that tests calling the solver aren't painfully slow in debug builds;
  - all of the above **[unverified as optimal; benchmark it]**.
- **CI (GitHub Actions):**
  - run on `macos-latest` (Apple Silicon) and `ubuntu-latest`;
  - install the pinned toolchain *before* the cache step, because "the cache key uses the rustc version";
  - then `Swatinem/rust-cache@v2`, which caches `~/.cargo` and `./target` and supports `workspaces`, `shared-key` and `cache-on-failure` ([rust-cache](https://github.com/Swatinem/rust-cache)).

  Jobs:
  - `cargo fmt --check`;
  - `cargo clippy --all-targets -- -D warnings`;
  - `cargo nextest run`, plus `cargo test --doc`;
  - `cargo deny check` (advisories, bans, licences);
  - `mm-cli validate-packs`, which also produces the coverage report (ADR-0002);
  - insta goldens with no pending snapshots allowed;
  - a divan bench smoke run with a regression budget.

---

## 7. Open questions and spikes

1. **Spike S1: micro-benchmark (not run; no toolchain here).**
   - Setup: in `/private/tmp/claude-502/mm5e-bench/`, `cargo new --lib`, with divan.
   - (a) Convolve 2d6+GWF with mixture weights for 2–4 attacks into a 900-wide PMF.
   - (b) A toy enumeration: 10^7 candidates × 16 ACs closed-form expected DPR, using `rayon` with `map_init` scratch, at 1 thread vs 10 threads.
   - (c) The same toy in Node (needs Node installed).
   - Pass criteria: confirm the 2–5× per-core and ≈6× thread factors, and the ≤1 µs Screen per kit.
2. **Spike S2: rotation-MDP cost per kit,** for a Paladin 20 with smite timing and a Rogue/Fighter with Vex→Nick. If it is above ~1 ms, the Screen tier needs an MDP-free policy-aware bound, or a cache of rotation solutions keyed by the kit "shape" (07 spike 2).
3. **How many distinct kits per partition?** The kit-memo hit rate decides whether the single-class L8 snapshot really runs in under a second. Count kits with the real compiler.
4. **Does cargo-deny ban workspace path crates?** If not, rely on the `cargo metadata` test.
5. **RON semantic-error spans.** Can we get field spans for semantic validation (e.g. via a two-pass parse into `ron::Value`)? Or do we accept "Content ID + field path" diagnostics?
6. **Floating-point policy for a future SIMD path.** Accept cross-path ulp differences, with quantised ranking making them harmless, or require the SIMD loop to match scalar order bit for bit?
7. **wasm32 portability.** Should `mm-engine` stay buildable for `wasm32` in case the UI note picks a web shell? Rayon on wasm needs threads plus COOP/COEP (07 §1), so `mm-solver` would need a sequential feature flag. This depends on the UI note.

---

## Candidate ADRs / revisions

### Existing ADRs

| ADR | Recommendation | Why |
|---|---|---|
| **0010** Hybrid Effect DSL | **Amend** | The decision stands. Replace "TypeScript hook" with "Rust `fn` hook registered per ruleset crate with a static `HookDecl` (stage, costs, outputs)". Add: encodings are RON data deserialised into closed Rust enums at runtime, and there is no embedded scripting. |
| **0015** Build search separate from rotation | **Keep** (minor amendment) | Unchanged in substance. Specify that the kit hash is a canonical compact `KitKey`, memoised in a process-wide `DashMap` and persisted in the on-disk cache per pack version. |
| **0019** All solving runs in the browser | **Supersede** | No public website, and the compute is native. Replacement: "All solving runs locally, natively, multithreaded on the user's machine; no server." The ADR-0003 consequence (imports stay local) still holds. The `SharedArrayBuffer`/COOP rationale disappears. |
| **0020** Precomputed pools + two-tier evaluation | **Amend** | Keep Screen→Refine, top-K-per-AC lookups and the 100 ms slider budget. Change: pools are no longer shipped build artifacts. They are (a) the in-memory pool from the last live solve and (b) an on-disk cache for expensive partitions, computed in the background. Live B&B becomes the primary path for single-class and constrained two-class queries (§1.6). |
| **0021** TypeScript engine with a WASM seam | **Supersede** | By the new ADR "Native Rust engine and solver". The `dice` seam survives as the `mm-dice` crate boundary. |
| **0022** pnpm monorepo | **Supersede** (engine part) | By the Cargo workspace ADR below. The UI part belongs to the UI note. |
| **0023** Exact distributions, not Monte Carlo | **Keep** (minor amendment) | Unchanged. Add: f64 PMFs in `Box<[f64]>`; `num-rational` `BigRational` as the golden-test oracle; and a pointer to the determinism ADR. |

### New candidate ADRs

1. **The engine and solver are native Rust, parallelised with rayon** (supersedes 0021).
   - *Decision:* `mm-dice`, `mm-core`, `mm-engine` and `mm-solver` are Rust (edition 2024) and run natively, using rayon over candidates.
   - *Rationale:*
     - about 10–30× over single-threaded TypeScript and 2–5× over a JS worker pool [estimate, §1.5];
     - real shared-memory threads;
     - compact state keys;
     - exhaustive enums.

     Personal use removes the browser-distribution constraint that drove 0021.
   - *Rejected:*
     - TypeScript plus a worker pool (slower per core, and memo memory about 15× larger per the Benchmarks Game memory columns);
     - Rust→WASM in a browser (about 1.5× slower than native, COOP/COEP needed for threads, and the wasm-pack toolchain risk noted in 07);
     - GPU compute (not evaluated: the branchy DP is a poor fit [unverified]).
2. **A Cargo workspace whose ruleset crates implement a generic `Ruleset` trait; isolation is enforced by the crate graph and cargo-deny** (supersedes 0022's engine part; implements 0008).
   - *Decision:* the layout and dependency rules in §4.2. The engine is generic over `R: Ruleset`, `Id<R, K>` IDs are typed, and a foreign prefix is a load error.
   - *Rationale:* cross-edition references become compile errors (hooks) or load errors (data), with no `isLegacy` branches.
   - *Rejected:*
     - `dyn Ruleset` (loses the ID typing);
     - edition flags inside one crate (the dnd5e anti-pattern, 04 §2.6);
     - separate repositories.
3. **Encodings are RON data files loaded at runtime into closed Rust enums; there is no macro DSL and no embedded scripting.**
   - *Decision:* `data/rules-*/**/*.ron`, embedded with `include_str!` in release builds and overridable with `--pack-dir`. JSON is accepted for user imports into the same types. `miette` diagnostics; a `cargo test` that loads every pack.
   - *Rationale:* an edit-and-re-run loop without recompiling, runtime user imports (ADR-0003), and solver-inspectable data.
   - *Rejected:*
     - `macro_rules!`/proc-macro DSL (compile times, no runtime path);
     - JSON as the authoring format (no comments, clumsy enums);
     - TOML (poor for nested enums);
     - rhai or mlua hooks (opaque, rhai is 2–3× slower than Python, and they reopen ADR-0010's purity concern).
4. **Hooks are pure `fn` pointers with static declarations, registered per ruleset crate.**
   - *Decision:* `HookDecl<R> { id, stage, costs, outputs, run: fn(&TurnView<R>, &Event, &HookArgs) -> EffectList }`, resolved to an index at load. Predicates compile to bitmask tests in the Combat Kit.
   - *Rationale:* stateless, `Send + Sync`, inspectable without running, and dispatch kept out of the hot loop.
   - *Rejected:*
     - `Box<dyn Hook>` (invites state);
     - `enum_dispatch` (needs a closed type list across crates);
     - scripted hooks.
5. **Deterministic parallel solving.**
   - *Decision:* the rules in §5:
     - parallel only across candidates;
     - total-order top-K on (quantised score, canonical key);
     - no parallel float sums;
     - strict pruning;
     - integer-exact d20 maths;
     - a 1-thread vs N-thread snapshot test.
   - *Rationale:* rayon's documented non-determinism for float `sum`/`reduce`; ADR-0023's promise that rankings never flicker.
   - *Rejected:*
     - single-threaded solving (gives up about 6×);
     - accepting non-determinism (rankings would flicker between runs);
     - exact rationals at runtime (too slow, per 0023).
6. **A local on-disk result cache in postcard, keyed by engine version + pack hash + query partition; bincode is banned.**
   - *Decision:* `mm-cache` with postcard (stable wire format since 1.0). rkyv is allowed later for memory-mapped pools. `bincode` goes in `deny.toml`.
   - *Rationale:*
     - it replaces 0020's shipped pools;
     - the version in the key makes format stability across releases unnecessary;
     - bincode is unmaintained (RUSTSEC-2025-0141; repo archived 2025-08-15).
   - *Rejected:*
     - bincode;
     - serde_json for the cache (about 2.5× larger and about 9× slower to serialise on the `log` benchmark);
     - no cache (unconstrained curve partitions take minutes even natively).

Not ADR-worthy (record in `CONTRIBUTING`/config instead): the crate picks for testing and profiling (proptest, insta, divan, nextest, samply), the lint set, the pinned toolchain version and the CI caching setup. All are easy to reverse.

## Glossary candidates

- **Combat Kit** (already implied by ADR-0015): the compiled, flat, ruleset-free form of a Build that the evaluator runs on. Its **Kit Key** is the canonical hash used for memoisation.
- **Hook Declaration:** the static description of a code hook's stage, costs and outputs that the solver reads without running the hook.
- **Live pool:** the in-memory union of top-N candidates over the encounter grid, kept from the last solve so slider moves only re-score. Avoid "precomputed pool" for it.

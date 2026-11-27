# 07: Technology Stack and System Architecture

Status: research note (2026-11-26). Nothing here is decided. The decisions worth recording are listed under **Candidate ADRs**.

Conventions:
- Every claim cites a URL. **[unverified]** means the claim comes from my own knowledge or reasoning and I did not confirm it against a primary source in this session.
- **[estimate]** marks back-of-envelope arithmetic. It needs a spike to confirm.
- Vendor and author benchmarks are labelled as such. I did not reproduce them.

---

## Question

What technology stack and system architecture let MinMax5e do the following?
- (a) Encode 2024 (P0) and 2014 (P1) rules as executable logic, with the editions isolated from each other.
- (b) Search a very large space of builds under user constraints.
- (c) Compute exact damage distributions rather than just means.
- (d) Drive a live dashboard whose sliders (AC 10–25, rounds, toggles) update results "immediately" (spec §5.3).
- (e) Keep hosting cost and legal exposure low.

The MVP covers 2024 rules only, six martial and half-caster classes, and single-target Sustained and Nova DPR (spec §5.1).

## TL;DR

- **Where the compute runs:** in the user's browser. The site is a static site, and the engine runs in a pool of Web Workers. There is no server-side compute in the MVP.
  - Hosting is close to free.
  - Slider latency does not depend on the network.
  - The project can ship only SRD 5.2 data (CC-BY-4.0), while users load any other rules data locally.
  - A server or offline precompute tier can be added later as a cache. It should not be on the critical path.
- **Engine language:** TypeScript (strict mode). It gives the best developer velocity, lets the engine and UI share types for free, and its discriminated unions with `never` exhaustiveness are good enough for modelling rules.
  - The numeric hot path (probability-distribution convolution) sits behind a narrow **Distribution Kernel** interface built on `Float64Array`. It can be swapped for Rust→WASM if a spike shows TypeScript is too slow.
  - Do not start the whole engine in Rust. Rust's advantage over well-written JavaScript is real but modest and workload-dependent (Surma measured 1.0–1.9x *slower* to 0.6x on his kernels). Moving rich rule objects across the JS↔WASM boundary also costs serialisation work.
- **The architectural move that matters more than the language: two-tier evaluation.**
  - Screen millions of candidate builds with a cheap *mean-DPR* evaluator. Thanks to linearity of expectation, it can compute a candidate's value at all 16 target ACs in one pass.
  - Then compute exact damage distributions only for the top K. This makes the AC slider a lookup into precomputed results rather than a re-solve.
- **Frontend:** React with Vite. Charts use Observable Plot (line, `areaY` bands and stacked bars in a few lines of code), with uPlot held in reserve.
  - The data volumes are tiny (3 builds × 16 AC points), so chart rendering speed is not the bottleneck. The solver is.
  - Worker RPC uses Comlink. In-flight solves are cancelled by a *generation token* checked between solver chunks, with `worker.terminate()` as a last resort.
- **Data:** rules packs are authored as TypeScript plus JSON data and validated with **TypeBox**, whose schemas *are* JSON Schema, so third-party pack authors get a real schema.
  - Each edition is its own workspace package. The edition-agnostic engine never imports a pack; it is handed one at runtime.
- **Repo:** a pnpm workspace (`packages/*`, `apps/web`, `apps/cli`), with Vitest plus fast-check for property tests, a CLI for golden tests, and GitHub Actions. Add Turborepo only when CI gets slow.
- **Credible alternative:** a Rust core in a Cargo workspace, compiled to WASM with wasm-bindgen. Types are exported to the UI with ts-rs, tests use proptest, and the React frontend stays the same.
  - Note that the rustwasm org was sunset in 2025. wasm-bindgen moved to a new organisation, and **wasm-pack was slated for archiving**.

---

## 1. Where the compute runs

### Options

| | Fully client-side (static site + Workers / WASM) | Server-side API | Hybrid (offline precompute → client query) |
|---|---|---|---|
| Hosting cost | Static hosting. Cloudflare Pages Free allows 20,000 files, 25 MiB per asset and 500 builds/month ([CF Pages limits](https://developers.cloudflare.com/pages/platform/limits/)). Bandwidth limits are not listed on that page | Pay per CPU-second for every solve. Cost grows with users × slider drags **[unverified, general]** | Pay for precompute once per data release. Query serving is static |
| Slider latency | No network round trip. Bounded by local CPU, which can be parallelised over workers | Network round trip on every change, plus queueing | Lookup only for precomputed parameter points. Misses need a fallback |
| Legal angle | Project can ship only SRD 5.2 (CC-BY-4.0, [D&D Beyond SRD](https://www.dndbeyond.com/srd)). User-supplied non-SRD data never leaves the device | The server holds whatever rules data it computes on, so it is effectively hosting the data | Precomputed tables derived from non-SRD data are arguably derived content that the project would host **[unverified, not legal advice]** |
| Offline | Works offline once loaded (PWA) **[unverified, standard service-worker behaviour]** | No | Partially |
| User-supplied data / homebrew | Natural fit | Requires upload | Precompute is invalidated by any custom data |
| Weak devices | Slow phones solve slowly | Uniform | Uniform for cached points |

### Notes

- **Workers are mandatory.** web.dev's guidance is explicit: "If you execute this on the main thread, with truly CPU-intensive tasks, you risk blocking the entire app" ([web.dev Wasm performance patterns](https://web.dev/articles/webassembly-performance-patterns-for-web-apps)).
- **Shared memory needs cross-origin isolation.** `SharedArrayBuffer` (needed for zero-copy shared state or Atomics-based cancel flags) requires a secure context and cross-origin isolation. Without it, `SharedArrayBuffer` is hidden and `postMessage` of one throws ([MDN SharedArrayBuffer](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer)).
  - Isolation requires the COOP and COEP response headers. Cloudflare Pages can set headers through a `_headers` file ([CF Pages limits](https://developers.cloudflare.com/pages/platform/limits/)).
  - GitHub Pages cannot set these headers. There the workaround is `coi-serviceworker`, which costs one forced page reload and must be served from your own origin ([coi-serviceworker](https://github.com/gzuidhof/coi-serviceworker)).
  - **Recommendation:** design the MVP so it does *not* need `SharedArrayBuffer`. Use transferable `ArrayBuffer`s, which are zero-copy moves ([MDN Transferable objects](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects)), and message-based cancellation. Host on Cloudflare Pages so isolation can be switched on later.
- **Legal.** The spec names "Complete 5e.tools Corpus" as the data source (spec, Metadata Summary).
  - SRD 5.2 is CC-BY-4.0 and covers the 5.5e/2024 core rules ([D&D Beyond SRD](https://www.dndbeyond.com/srd)). The rest of the 2024 PHB content is not in the SRD.
  - Client-side compute allows a split: the project ships an SRD pack, and users import other data locally.
  - Whether that split is sufficient is a legal question for a separate research note. **[unverified, not legal advice]**

**Verdict:** fully client-side for the MVP. Keep a hybrid precompute tier as a possible later optimisation, for example shipping pre-pruned candidate sets per class and level for the SRD pack.

---

## 2. Engine language

### How big is the workload? [estimate]

**Full distributions.**
- A per-attack damage PMF (probability mass function) such as 2d6+5 with crits, a Graze floor and riders has a support of about 40 to 60 integer values.
- Four attacks per turn convolve to about 200 values. Three rounds give about 600.
- That is roughly Σ(n·m) ≈ a few ×10⁵ multiply-adds for one build at one AC. Over 16 ACs it is about 5×10⁶, which is a few milliseconds per build in either JS or WASM.
- **Exact distributions for a million builds are therefore not interactive in any language.**

**Means.**
- By linearity of expectation, a build's mean DPR at one AC is a sum over attacks of p(hit)·E[dmg|hit] + p(crit)·E[extra] + p(miss)·graze, and so on. That is O(attacks), so tens of operations.
- Computing all 16 ACs is a 16-wide loop. Screening 10⁶ candidates is then about 10⁸ to 10⁹ simple operations, which is seconds on one core and less across a worker pool.

**Consequence:** the decisive architecture is **Screen (means) → Refine (exact PMFs for top-K)**, not the choice of language. Language matters for the constant factor of the Refine step and of rotation search.

### Comparison

| | TypeScript (V8 / JIT) | Rust → WASM | Python (NumPy server / Pyodide) | Go → WASM | Kotlin → Wasm |
|---|---|---|---|---|---|
| Numeric speed | V8 is "really good at executing JavaScript". Surma's measured kernels ran at 0.6x to 1.9x of JS time depending on language and tuning; idiomatic Rust was 1.9x *slower* than JS on his heap benchmark ([Surma, "Is WebAssembly magic performance pixie dust?"](https://surma.dev/things/js-to-asc/)) | No warmup and no deopts, so "more reliable performance" ([Surma](https://surma.dev/things/js-to-asc/)). SIMD128 is available on stable Rust with `-C target-feature=+simd128` ([V8 SIMD](https://v8.dev/features/simd)) | NumPy convolution is fast on a server **[unverified]**. Pyodide is a WASM CPython; the full distribution is "200+ megabytes", though apps load only the packages they need ([Pyodide deploying](https://pyodide.org/en/stable/usage/downloading-and-deploying.html)). Per-element Python loops (search) are slow **[unverified]** | Minimum binary about 2 MB, ~500 kB gzipped, "10MB+ is common" ([Go wiki: WebAssembly](https://go.dev/wiki/WebAssembly)). No runtime numbers given there | Beta. Needs browser support for Wasm GC and legacy exception handling ([Kotlin/Wasm overview](https://kotlinlang.org/docs/wasm-overview.html)) |
| Combinatorial search | Fine. Allocation-heavy code needs care (object pooling, typed arrays) **[unverified]** | Best constant factors. Manual memory layout | Poor in pure Python **[unverified]** | Good natively. WASM size is a cost | Unknown |
| Modelling rules (ADTs, exhaustiveness) | Discriminated unions plus `never` give compile-time exhaustiveness: "Type 'Triangle' is not assignable to type 'never'" ([TS handbook: narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html)). Structural typing means it is unsound in places **[unverified, well known]** | Native enums with exhaustive `match`; a missing arm is error E0004 ([Rust book ch06-02](https://doc.rust-lang.org/book/ch06-02-match.html)) | Weak (runtime only, mypy optional) **[unverified]** | No sum types **[unverified, well known]** | Sealed classes **[unverified]** |
| Sharing types with the UI | Same language | Codegen: ts-rs turns Rust enums into TS unions and honours serde `tag`/`content` ([ts-rs](https://github.com/Aleph-Alpha/ts-rs)) | Manual or codegen | Manual | Manual |
| Developer velocity | Highest. One toolchain | Lower: two toolchains and boundary glue **[unverified]** | High on a server. Pyodide in a worker is awkward | Medium | Low (Beta) |
| Tooling status | Stable | rustwasm org sunset in July 2025. wasm-bindgen moved to a new `wasm-bindgen` org; **wasm-pack is on the archive list** ([Inside Rust blog](https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org/)). wasm-bindgen remains active ([wasm-bindgen repo](https://github.com/wasm-bindgen/wasm-bindgen)) | Stable | Stable | Beta |

### The JS↔WASM boundary

- **Calls themselves are cheap.** Since 2018, JS↔WASM calls in SpiderMonkey are faster than non-inlined JS→JS calls. In the 100M-call benchmark, JS→wasm fell from about 5500 ms to about 450 ms ([Mozilla Hacks](https://hacks.mozilla.org/2018/10/calls-between-javascript-and-webassembly-are-finally-fast-%F0%9F%8E%89/)).
- **Rich data is the problem.** WASM value types are only ints and floats, so rich data must be mediated by JS or glue code ([Mozilla Hacks](https://hacks.mozilla.org/2018/10/calls-between-javascript-and-webassembly-are-finally-fast-%F0%9F%8E%89/)).
- **serde-wasm-bindgen** is the preferred way to move structured data. It is smaller and faster than JSON but still converts every object ([serde-wasm-bindgen](https://github.com/RReverser/serde-wasm-bindgen)). wasm-bindgen maps numeric boxed slices to typed arrays ([wasm-bindgen boxed slices](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/boxed-slices.html)). Whether that copies was not stated **[unverified: believed to copy into/out of linear memory]**.
- **Design rule:** if WASM is used, cross the boundary **once per batch**, never once per build. For example, pass a compiled candidate table as a flat `Float64Array`/`Int32Array` and get back a flat score array. That is exactly the shape of the Distribution Kernel seam below.

**Verdict:** a TypeScript engine with a kernel seam that can be replaced by WASM.

---

## 3. Frontend framework, charts and reactivity

### Framework

| | React | Svelte 5 | Solid | Vue |
|---|---|---|---|---|
| Reactivity | VDOM. `useDeferredValue` gives interruptible, adaptive-lag updates that need no fixed delay and are "interruptible by default" ([react.dev useDeferredValue](https://react.dev/reference/react/useDeferredValue)) | Runes: `$state` makes deep reactive proxies, and `$state.raw` skips proxying for large immutable data ([Svelte $state](https://svelte.dev/docs/svelte/$state)) | Fine-grained signals, no VDOM **[unverified: docs URL 404'd]** | Proxies plus VDOM **[unverified]** |
| Ecosystem and charts | Largest (Recharts, visx, wrappers for all) **[unverified]** | Good | Smaller | Good |
| Raw DOM benchmarks | I could not load the js-framework-benchmark results; the page renders dynamically ([krausest](https://krausest.github.io/js-framework-benchmark/current.html)). **[unverified: Solid/Svelte usually rank ahead of React]** | | | |

**Assessment.**
- The dashboard has a handful of charts with fewer than 100 points each, plus a few tables. DOM throughput is irrelevant; solver latency dominates.
- Choose React for ecosystem and familiarity, and to give agents the most prior art. **[judgement]**
- Svelte 5 is an equally good choice if the team prefers it. This is easy to reverse as long as the engine stays framework-free, so it is **not** an ADR candidate.

### Charts (spec §5.3: AC sensitivity multi-line, DPR distribution and reliability band, damage source attribution)

| Library | Multi-line | Bands / distribution | Stacked attribution | Interactive performance | Notes |
|---|---|---|---|---|---|
| **Observable Plot** | yes | `areaY` with y1/y2 **[unverified: docs rate-limited (429)]** | stacked `barY` via stack transform **[unverified]** | SVG, so fine at this scale **[unverified]** | Layered grammar of graphics, ISC licence ([observablehq/plot](https://github.com/observablehq/plot)). Imperative `Plot.plot()` returns a DOM node, wrapped in a 10-line React component **[unverified]** |
| Vega-Lite | yes | yes | stacking built in | Selections for interaction ([Vega-Lite](https://vega.github.io/vega-lite/)) | Declarative JSON. Heavier runtime **[unverified]** |
| ECharts | yes | yes | yes | Canvas is recommended above ~1k points; the SVG renderer got 2–10x faster with a VDOM in 5.3 ([ECharts canvas vs SVG](https://echarts.apache.org/handbook/en/best-practices/canvas-vs-svg/)) | ~1000 KB full bundle per the uPlot author's table ([uPlot](https://github.com/leeoniya/uPlot)). Tree-shakable |
| Recharts | yes | yes | yes | Docs advise memoising props and throttling (50 ms example) ([Recharts performance](https://recharts.github.io/en-US/guide/performance/)) | Idiomatic React. SVG **[unverified]** |
| visx / D3 | anything | anything | anything | You own the rendering | Most work. Low-level primitives **[unverified: site redirected]** |
| uPlot | best | via fill between series **[unverified]** | `opts.stack.groups` | ~47.9 KB. 166k points in ~25 ms (author's benchmark) ([uPlot](https://github.com/leeoniya/uPlot)) | No animations or data processing; canvas ([uPlot](https://github.com/leeoniya/uPlot)) |

**Pick:** Observable Plot for all three MVP charts, because one library covers lines, bands and stacks with little code. Fall back to uPlot if a future progression view (20 levels × many builds) ever needs canvas speed.

### "Constraints → solver → results" reactivity, debouncing and cancellation

Proposed pattern (design; component APIs cited):

1. **State shape.**
   - `SearchParams` are the inputs that change the *search*: level, classes, multiclass, mode, ruleset.
   - `EncounterParams` are the inputs that only change *scoring*: AC, rounds, resistance, advantage.
   - The two are separated so that each slider is classified as either a re-search or a re-score.
2. **AC slider = lookup.** The Screen pass scores every candidate at all ACs from 10 to 25 and keeps the top K *per AC*. Moving the AC slider then selects precomputed results with no worker call. **[design, estimate]**
3. **Other changes trigger a solve.**
   - The UI increments a `generation` counter and sends `solve(params, generation)` to the pool via Comlink.
   - Comlink is about 1.1 kB brotli RPC over `postMessage`/Proxy. `Comlink.transfer()` moves `ArrayBuffer`s, and `Comlink.proxy()` passes callbacks for progress ([Comlink](https://github.com/GoogleChromeLabs/comlink)).
   - **Comlink has no documented AbortSignal support** ([Comlink](https://github.com/GoogleChromeLabs/comlink)). Cancellation is therefore cooperative: the solver works in chunks of about 5–10 ms, and between chunks it checks a "latest generation" value that the UI updates through a proxied callback or a separate message.
   - Stale results are dropped by generation on the UI side.
   - If a worker overruns, `worker.terminate()` stops it immediately with no chance to finish ([MDN Worker.terminate](https://developer.mozilla.org/en-US/docs/Web/API/Worker/terminate)). The pool then respawns it. To make that cheap, compile WASM once and send the `WebAssembly.Module` to workers ([web.dev](https://web.dev/articles/webassembly-performance-patterns-for-web-apps)).
4. **Progressive results.** Stream "best so far" top-3 lists while the solve runs, so the UI is never blank. Use `useDeferredValue` for the expensive panels rather than a fixed debounce ([react.dev](https://react.dev/reference/react/useDeferredValue)).
   - A short debounce (~100 ms) is still useful *before dispatching* a re-search, because it is a compute request, which matches React's own note that debounce suits non-render work ([react.dev](https://react.dev/reference/react/useDeferredValue)).
5. **State library.** A small external store (for example Zustand), or React state plus a `useSolver` hook, is enough. **[unverified/judgement]** Avoid putting solver results in a server-cache library; there is no server.

---

## 4. Data pipeline

### Validation libraries

| | TypeBox | Zod 4 | Ajv + JSON Schema files | serde (+ schemars) |
|---|---|---|---|---|
| Source of truth | TS builder that *is* JSON Schema. `Type.Static` infers TS types ([TypeBox](https://github.com/sinclairzx81/typebox)) | Zod schema, exported via `z.toJSONSchema()` (Draft 2020-12 by default; transforms, dates and so on are unrepresentable) ([Zod JSON Schema](https://zod.dev/json-schema)) | Hand-written JSON Schema plus codegen | Rust types |
| Speed | Compiler claims to beat Ajv 8 in most of its own benchmarks ([TypeBox](https://github.com/sinclairzx81/typebox)) | Slower than compiled validators **[unverified]** | Fast (compiled) **[unverified]** | Native |
| Third-party pack authors | Get a standard JSON Schema directly | Get exported JSON Schema (lossy edge cases) | Native | schemars output **[unverified]** |
| Notes | 1.x is ESM-only and targets TS 6–7; 0.x is the LTS line for TS 5 ([TypeBox](https://github.com/sinclairzx81/typebox)) | Most popular, best ergonomics **[unverified]** | Two artefacts to keep in sync | Only for the Rust alternative |

**Pick:** TypeBox. The rules-pack format is a published contract (users import their own data), so JSON Schema should be the primary artefact rather than an export. Zod 4 is the close alternative.

### Pipeline shape

```
5e.tools-style JSON / SRD 5.2 text      hand-authored encodings (TS)
            │                                     │
            ▼                                     ▼
   packs/<edition>/import/*  ──►  normalise ──►  Effect DSL objects
                                                  │
                     validate against @minmax/rules-schema (TypeBox, build-time + load-time)
                                                  │
                                                  ▼
             pack bundle: { meta: {edition, version, sourceHash, license}, data }
                                                  │
                         web app: SRD pack bundled; user packs loaded from disk → IndexedDB
```

- **Versioning.** Every pack carries `edition` (`"2024"` | `"2014"`), a semver `packVersion` and a `schemaVersion`. A solve result records the pack versions it used, so golden tests and shared links can be reproduced. **[design]**
- **Isolation at build level.**
  - `rules-2024` and `rules-2014` are separate workspace packages. Neither depends on the other, and the engine depends on neither.
  - pnpm gives each project symlinks only to its *declared* dependencies, so an undeclared cross-import fails ([pnpm workspaces](https://pnpm.io/workspaces)). `disallowWorkspaceCycles` blocks cycles ([pnpm workspaces](https://pnpm.io/workspaces)).
  - Add a lint rule (for example `no-restricted-imports` or dependency-cruiser) as a belt-and-braces check **[unverified: not fetched]**.
- **Runtime isolation.**
  - The engine is generic over a `Ruleset` value. The type system makes `Effect<'2024'>` and `Effect<'2014'>` distinct via a branded `edition` discriminant, so a 2014 feat cannot be passed into a 2024 solve.
  - This enforces spec §2 "hard boundaries in the rule-parsing layer". **[design]**

---

## 5. Monorepo layout, tooling, testing, CI

```
minmax5e/
├─ packages/
│  ├─ dice/           # Distribution Kernel: PMF ops on Float64Array (convolve, mix, max/min, clamp, add-const). Zero deps. Swappable for WASM.
│  ├─ rules-schema/   # TypeBox schemas + inferred types for the Effect DSL and pack format. Edition-generic.
│  ├─ engine/         # Character compiler (build → CompiledCharacter), turn state machine / action economy, Rotation policy, Evaluator (mean + exact).
│  ├─ solver/         # Search space enumeration, pruning, Screen→Refine, top-K per AC, chunked + cancellable.
│  ├─ rules-2024/     # P0 pack (SRD subset shipped; importer for user data)
│  ├─ rules-2014/     # P1 pack (post-MVP; empty placeholder keeps the boundary honest)
│  └─ worker/         # Worker entry + Comlink API + pool; the only place threads exist
├─ apps/
│  ├─ web/            # Vite + React dashboard
│  └─ cli/            # node CLI: batch solves, golden-file generation, pack validation
└─ pnpm-workspace.yaml
```

### Tooling

- **Vite.**
  - Workers: Vite detects `new Worker(new URL(...), ...)` only when `new URL()` is written inside the constructor, and it also supports `?worker` imports.
  - WASM: `?init` gives manual instantiation ([Vite features](https://vite.dev/guide/features)).
- **pnpm workspaces** use the `workspace:` protocol, which refuses to fall back to the registry ([pnpm workspaces](https://pnpm.io/workspaces)).
- **Turborepo** adds dependency-ordered, parallel, cached tasks and a remote cache. It works with pnpm and a single `turbo.json`, and can be adopted incrementally ([Turborepo docs](https://turborepo.dev/docs)). Defer it until `pnpm -r` builds become slow.

### Testing

- **Vitest** is Vite-native, reads `vite.config`, has a projects mode for monorepos, a browser mode and benchmarking. It requires Vite ≥ 6.4 and Node ≥ 22.12 ([Vitest guide](https://vitest.dev/guide/)).
- **fast-check** provides property-based tests and is runner-agnostic, so it works with Vitest ([fast-check](https://fast-check.dev/)). Properties worth encoding:
  - Every PMF sums to 1 ± ε.
  - mean(convolve(a,b)) = mean(a)+mean(b).
  - Hit chance is non-increasing in AC.
  - Advantage ≥ normal ≥ disadvantage in expected damage for attacks with non-negative damage.
  - The Screen mean equals the Refine exact mean for the same build. This is the key cross-check between the two tiers.
  - No illegal turn is ever produced, for example two concentration spells or two spell slots in one turn (spec §5.2).
- **Golden tests:** the CLI solves a fixed corpus of reference builds (hand-verified against community maths) and snapshots the outputs. Any change to the rules or kernel shows up as a diff.
- **Monte Carlo oracle:** a slow simulator in `engine/test` checks exact PMFs statistically.
- In the Rust alternative, **proptest** fills the same role. It is Hypothesis-inspired, shrinks failing cases, and is now passively maintained ([proptest book](https://proptest-rs.github.io/proptest/intro.html)). Hypothesis is the Python equivalent ([Hypothesis](https://hypothesis.readthedocs.io/en/latest/)).

### CI (GitHub Actions)

- `actions/setup-node` with `cache: 'pnpm'` needs pnpm v6.10+ and pnpm pre-installed, for example via `pnpm/action-setup` ([setup-node](https://github.com/actions/setup-node)).
- Jobs: typecheck → unit + property tests → validate all packs against the schema → golden tests → solver benchmark with a regression budget → build `apps/web` → deploy previews to Cloudflare Pages.

---

## 6. Recommended architecture

### Recommended stack

TypeScript (strict) everywhere, with:
- pnpm workspaces
- Vite + React web app
- Web Worker pool via Comlink
- TypeBox rules schemas
- Observable Plot charts
- Vitest + fast-check, and a CLI for golden tests
- GitHub Actions, deployed to Cloudflare Pages

### Credible alternative

- Rust core (`dice`, `engine`, `solver` crates in a Cargo workspace), compiled with wasm-bindgen into a worker.
- serde + schemars for the pack schema, ts-rs to export types to TypeScript, and proptest for property tests.
- The same React/Plot frontend and pnpm workspace for `apps/web`.
- **When to choose it:** spikes 1–2 show TS throughput is more than about 3x short of the interactive budget, *and* a kernel-only WASM swap does not close the gap.
- **Risk:** wasm-pack's archived status ([Inside Rust blog](https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org/)) means building with `wasm-bindgen-cli` directly, or with a Vite WASM plugin, instead.

### Component diagram

```
 ┌──────────────────────────── Browser (static site, no server compute) ────────────────────────────┐
 │                                                                                                 │
 │  apps/web (React, main thread)                                                                  │
 │  ┌─────────────────────┐   SearchParams    ┌───────────────────────┐   EncounterParams (AC…)    │
 │  │ Constraint panel    ├──────────────────►│ useSolver (gen token, ├──────────┐                 │
 │  │ Target-dummy sliders│                   │ debounce, progress)   │          │ lookup only     │
 │  └─────────────────────┘                   └──────────┬────────────┘          ▼                 │
 │  ┌──────────────────────────────────────┐             │ Comlink RPC   ┌──────────────────┐      │
 │  │ Top-3 cards · Turn script · Charts   │◄────────────┼───────────────┤ SolveResult      │      │
 │  │ (Observable Plot)                    │             │ (transferable │ (top-K per AC,   │      │
 │  └──────────────────────────────────────┘             │  buffers)     │  PMFs for top-3) │      │
 │                                                       ▼               └──────────────────┘      │
 │  packages/worker ── pool of N Workers (N ≈ hardwareConcurrency−1 [unverified])                   │
 │  ┌──────────────────────────────────────────────────────────────────────────────────────────┐   │
 │  │ solver: enumerate ─► prune ─► SCREEN (mean DPR × 16 ACs) ─► top-K ─► REFINE (exact PMF)   │   │
 │  │            │                         │                                   │                │   │
 │  │            ▼                         ▼                                   ▼                │   │
 │  │ engine: compile(Build, Ruleset) → CompiledCharacter ; plan(Rotation) ; evaluate()         │   │
 │  │                                                                          │                │   │
 │  │ dice: Distribution Kernel (Float64Array ops)  ◄── optional Rust/WASM implementation       │   │
 │  └───────────────▲──────────────────────────────────────────────────────────────────────────┘   │
 │                  │ Ruleset value (injected; engine never imports a pack)                         │
 │  rules-2024 (SRD, bundled)   rules-2014 (P1)   user packs (IndexedDB, validated by rules-schema) │
 └─────────────────────────────────────────────────────────────────────────────────────────────────┘
                 apps/cli (Node) reuses engine/solver/packs for golden tests and batch runs
```

### Module boundaries (deep modules, narrow interfaces) [design]

| Module | Interface (narrow) | Hides (deep) |
|---|---|---|
| `dice` | `Pmf` (offset + `Float64Array`); `convolve`, `mix(p, a, b)`, `addConst`, `scale`, `mean`, `variance`, `quantile` | Truncation/epsilon policy, FFT vs direct convolution, WASM/SIMD implementation |
| `rules-schema` | `RulesPack<E>` type + `validatePack(json): Result<RulesPack>` | JSON Schema details, migrations between schema versions |
| `engine` | `compile(build, ruleset): CompiledCharacter`; `evaluate(char, encounter, mode): Evaluation` (mean-only or exact) | Action-economy state machine, concentration/slot/bonus-action legality, mastery and weapon-swap sequencing, rotation optimisation, attribution bookkeeping |
| `solver` | `solve(params, ruleset, {onProgress, isCancelled}): AsyncIterable<SolveResult>` | Enumeration order, symmetry/dominance pruning, Screen→Refine thresholds, top-K-per-AC heaps, chunking |
| `worker` | `createSolverPool(n)` → `{solve, cancel}` | Comlink, transfer lists, worker respawn, sharding |
| `rules-2024` | `export default ruleset: Ruleset<'2024'>` | Every feature's encoding |

---

## Spike list (riskiest unknowns first)

1. **Search-space size and Screen throughput.**
   - Count legal Level-8 and Level-20 builds for the six MVP classes: species × origin feat × point-buy arrays × class order × feats × weapon loadouts × subclass, with multiclassing on and off.
   - Measure TS mean-evaluator throughput per core.
   - Pass condition: the full Screen finishes in under about 1–2 s on a mid-range laptop with 4 workers. If not, pruning (dominance, symmetry, monotone ASI choices) becomes the main design problem. **This is the biggest unknown.**
2. **Rotation optimisation cost.**
   - "Best turn sequence" (Vex→Nick ordering, weapon draw/stow, Bonus Action contention between Smite and an off-hand attack) is itself a search *inside* each build evaluation.
   - Find out whether a per-build-shape cache (rotation depends on features, not on exact numbers) makes it amortised O(1).
3. **Exact-PMF kernel speed in TS vs Rust/WASM.**
   - Benchmark convolve-heavy Refine for 3 builds × 16 ACs × 1–3 rounds, including the crit and Graze mixture.
   - Target: under 16 ms in TS. Only port to WASM if this misses by a wide margin.
4. **Effect DSL expressiveness.**
   - Encode the 10 hardest MVP features end to end: Vex, Nick, Graze, Topple, 2024 GWM, Divine Smite (BA + 1 slot/turn), Sneak Attack (once/turn, including off-turn), Hunter's Mark, Action Surge, and Extra Attack with mastery swapping.
   - If any needs an escape hatch (code rather than data), decide how packs carry code safely when a user imports one. User packs may need to be data-only.
5. **Cancellation and progressive UX.** Measure end-to-end latency from slider drag to updated charts with Comlink, generation tokens, chunking and respawn-on-overrun, and confirm the AC slider never touches a worker.
6. **Determinism across shards.** Confirm that a sharded solve gives the same top-3 as a single-threaded solve, with tie-breaking by a stable build key and no floating-point order dependence beyond ε.

## Open questions

- Is "millions of builds" (spec §1) a real requirement after legal pruning, or does dominance pruning collapse the space to thousands? This should be settled by spike 1 before any language debate.
- Does the Smooth Leveling Curve mode (spec §3.1, §5.1) multiply the search by 20 levels, or can it reuse incremental per-level evaluations? This affects the Screen budget.
- Must user-imported packs be data-only, so they cannot execute code? This bears on both security and schema design.
- Is a mobile-first experience required? Phones have fewer cores, which would push toward server precompute for the SRD pack. **[unverified assumption that mobile is secondary]**
- Legal: is shipping only SRD 5.2 plus a local importer for user-supplied data acceptable? Defer to the legal research note.
- Shareable URLs: should a solve be encoded in the URL (params + pack versions) so results are reproducible without a server?

---

## Candidate ADRs

1. **Engine computes client-side in Web Workers; no server compute in MVP.**
   - **Decision:** ship a static site. All solving runs in a worker pool, and users' non-SRD rules data never leaves their device.
   - **Rationale:** near-zero hosting cost ([CF Pages limits](https://developers.cloudflare.com/pages/platform/limits/)), no network latency on sliders, offline use, and legal separation (ship SRD 5.2 CC-BY-4.0 only, [SRD](https://www.dndbeyond.com/srd)).
   - **Hard to reverse because** it shapes data distribution, the licensing posture and the cancellation model.
   - **Rejected:** server API (cost, latency, hosts rules data). Hybrid precompute as the primary path (invalidated by user data). It remains an optional later cache.
2. **TypeScript engine, with the numeric kernel behind a WASM-swappable seam.**
   - **Decision:** write the engine, solver and packs in strict TypeScript. `packages/dice` exposes a typed-array API that a Rust→WASM build can implement later.
   - **Rationale:** shared types with the UI, the fastest iteration on rules encoding, and exhaustive discriminated unions ([TS handbook](https://www.typescriptlang.org/docs/handbook/2/narrowing.html)). WASM's speed edge is workload-dependent ([Surma](https://surma.dev/things/js-to-asc/)) and the boundary penalises rich objects ([Mozilla Hacks](https://hacks.mozilla.org/2018/10/calls-between-javascript-and-webassembly-are-finally-fast-%F0%9F%8E%89/)).
   - **Rejected:** a full Rust core (velocity, type-sharing glue, the wasm-pack archive risk), Python/Pyodide (size, search speed), Go (binary size), Kotlin/Wasm (Beta).
3. **Two-tier evaluation: mean-DPR Screen across all target ACs, then exact distributions for top-K.**
   - **Decision:** the solver ranks candidates by expected value computed in closed form for every AC from 10 to 25 at once, and computes full PMFs only for finalists.
   - **Rationale:** exact PMFs for millions of builds are not interactive **[estimate]**. Precomputing per-AC top-K turns the AC slider into a lookup.
   - **Surprising because** variance and reliability (spec §4) do not influence the ranking unless the objective is reformulated. A risk-adjusted objective would need a variance term in the Screen; the variance of independent sums is also additive, so this is cheap.
   - **Rejected:** exact everywhere (too slow), Monte Carlo everywhere (noisy rankings, non-deterministic).
4. **pnpm monorepo with edition-specific rules packs as separate packages injected into an edition-agnostic engine.**
   - **Decision:** `rules-2024` and `rules-2014` are independent packages. The engine is generic over `Ruleset<E>` and never imports a pack, and pnpm's strict per-package dependency linking ([pnpm](https://pnpm.io/workspaces)) plus lint rules enforce the boundary.
   - **Rationale:** spec §2 demands hard isolation between editions, and the same seam supports user-imported packs.
   - **Rejected:** one rules package with edition flags (pollution risk), separate repos (shared engine drift).
5. **Rules-pack format is JSON Schema-first via TypeBox.**
   - **Decision:** pack schemas are authored in TypeBox, so the JSON Schema *is* the published contract and the TS types are inferred from it ([TypeBox](https://github.com/sinclairzx81/typebox)).
   - **Rationale:** third parties and users author packs, and validation runs at both build time and load time.
   - **Rejected:** Zod 4 (JSON Schema is an export with unrepresentable cases, [Zod](https://zod.dev/json-schema)), hand-written JSON Schema plus codegen (two sources of truth).
   - Only marginally hard to reverse; include it if the pack format is published early.

## Glossary candidates

| Term | Definition | Avoid |
|---|---|---|
| **Ruleset** | The edition environment (2024 or 2014) that a solve is locked to, represented in code as the injected `Ruleset<E>` value. | edition mode, sandbox (in code) |
| **Rules Pack** | A versioned, schema-validated bundle of rules data and effect encodings for exactly one Ruleset. | dataset, corpus, module |
| **Effect DSL** | The structured, executable encoding of a feature, feat, spell or item (triggers, conditions, effects) that the engine interprets. | rule graph, logic, script |
| **Build** | A complete character specification: species, background/origin feat, ability scores, class level order, feats, loadout. | character, sheet, template |
| **Candidate** | A Build generated by the solver that has not yet been ranked. | option, combo |
| **Compiled Character** | A Build resolved against a Ruleset into the numeric and feature state the Evaluator consumes. | stat block, snapshot |
| **Rotation** | The per-turn sequence of Actions, Bonus Actions, reactions and weapon draw/stow steps that a Build executes, by phase (pre-combat, round 1, rounds 2+). | combo, script, workflow (in code) |
| **Encounter** | The target-dummy assumptions: AC, saves, resistances, rounds, adv/disadv toggles. | enemy, monster, scenario |
| **Screen** | The first solver tier: fast mean-DPR scoring of every Candidate at every target AC. | prefilter, rough pass |
| **Refine** | The second solver tier: exact damage-distribution computation for the top-K Candidates. | detailed pass, deep eval |
| **Damage Distribution** | The exact probability mass function of damage over a turn or encounter, held as a `Pmf`. | histogram, spread, curve |
| **Distribution Kernel** | The `dice` package's narrow numeric API over `Pmf`s; the only module that may be WASM-accelerated. | math lib, dice engine |
| **Solve** | One cancellable solver run, identified by a generation token, that produces a stream of results. | query, job, request |

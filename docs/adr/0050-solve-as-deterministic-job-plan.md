---
status: accepted
---

# A Solve is a deterministic job plan; interchangeable executors run it, and only the final result is promised identical

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

> *Amended 2026-11-27 (workspace foundation, #13): the float bans, with sources.*
>
> - **`sqrt` is allowed.** Rust guarantees `f64::sqrt` is "the rounded infinite-precision result … specified by IEEE 754 as `squareRoot` and guaranteed not to change" ([std docs](https://doc.rust-lang.org/std/primitive.f64.html#method.sqrt)), and WebAssembly's `fsqrt` rounds to nearest. It is an IEEE basic operation, so the rule below already allows it. Spec #12's ban on `sqrt` is withdrawn.
> - **`powi` is banned.** Rust documents its precision as "non-deterministic", varying "by platform, Rust version, and … from one invocation to the next". Multiply explicitly.
> - **`f64::max`/`min` are banned outright** in `clippy.toml`, not only "where NaN is possible", because a lint can't tell where NaN is possible. Use explicit comparisons or `total_cmp`.
> - **`mul_add` stays banned** even though Rust guarantees its rounding: a fused result differs from the unfused `a * b + c` used elsewhere, so mixing them changes bits between code paths (research 09 §5, rule 7).
> - **"Transcendentals" means every `f32`/`f64` method whose precision Rust leaves unspecified,** not only the three examples in the rule below: `powf`, `exp`, `exp2`, `exp_m1`, `ln`, `ln_1p`, `log`, `log2`, `log10`, the trigonometric and hyperbolic functions and their inverses (`sin`, `cos`, `tan`, `sin_cos`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`), `cbrt` and `hypot`. `clippy.toml` holds the list. `xtask/tests/clippy_bans.rs` checks three things: every entry is rejected, each method is banned for both `f32` and `f64`, and the bans #13 names are still present.
> - **Display code may use a banned method, one site at a time.** The rules cover scored code only. A helper that only formats output (for example, turning a variance into a standard deviation for display) may call a banned method under `#[expect(clippy::disallowed_methods, reason = "…")]`, with the reason saying why the value is never scored.

`mm-solver` has no threads, channels or clocks. It exposes four pure functions:

- `plan(pack, query) -> JobPlan`, ordered by (bound descending, canonical key);
- `run_job(ctx, job, hint) -> JobOut`;
- `merge(acc, out, chunk_ix)`, called in chunk order;
- `snapshot(acc)`.

Jobs are serialisable data: join slices, Screen and PoolFill blocks for one Kit Shape, and Exact finalists. Three **executors** run the same plan:

- `Inline`, for tests and oracles;
- `Rayon`, for the native owner's build;
- `WorkerPool`, for the web build (ADR-0045).

Screen jobs are routed by shape affinity, so each worker traces a shape once. Affinity affects speed only. The branch-and-bound incumbent is a monotone **hint**: pruning is strict (`bound < hint`), so a stale hint prunes less but never prunes wrongly.

**Determinism has two levels:**

- **Outcome determinism (guaranteed):** a `Done` result is bit-identical for every executor, thread count and target. [measured: identical at 1–10 threads, and one result hash across native aarch64, `wasm32` and `wasm32`+`simd128`, research 13 §4]
- **Trajectory determinism (not promised):** `BestSoFar` snapshots, progress counts and the state of a cancelled solve depend on scheduling. Tests compare `Done` results only, and the UI never presents an intermediate snapshot as final.

This is stronger than Gurobi, which guarantees repeatability only on the same machine. It holds because scored values use only IEEE-754 basic operations. Rapier ships a WASM build that is deterministic across browsers on the same basis.

**Rules for scored code, CI-enforced:**

- no WASM `relaxed-simd` target feature (its FMA may round once or twice, depending on the host);
- no `f64::mul_add`;
- no transcendentals (`powf`, `exp`, `ln`);
- no `f64::max`/`min` where NaN is possible;
- no fast-math;
- no parallel float sums (ADR-0037);
- `+simd128` is allowed.

**Budgets and performance gates count work, not time.** Beam budgets (ADR-0057) are counted in kits evaluated, like Gurobi's `WorkLimit`, so a budgeted result doesn't depend on the machine. CI performance gates use deterministic counters: tape nodes × lanes, kits screened, prune ratio. Wall-clock benchmarks are advisory and reported with confidence intervals, not best-of-N (Georges et al., OOPSLA 2007), because timings on the M5's mixed cores varied by up to about 1.8× (research 13).

CI runs a **cross-target result hash**: a fixed synthetic job set must hash to the same value natively and under `wasm32-wasip1` with wasmtime.

## Considered Options

- **Solver-owned threading** (rayon inside `mm-solver`). Rejected: the web has no rayon without nightly Rust (ADR-0045), and the solver would need two code paths.
- **Promising identical intermediate snapshots.** Rejected: that would force synchronous rounds and cost the parallel speed-up.
- **Wall-clock budgets.** Rejected: the result would depend on the machine.
- **Per-machine determinism only** (the Gurobi model). Rejected: Share Links (ADR-0020) must reproduce exactly in a friend's browser.

Extends ADR-0037; ADR-0040 is unchanged. Research: [13](../research/13-tape-spike.md), [14](../research/14-prior-art-and-refinements.md), [15](../research/15-problems-ahead-and-methods.md), [09](../research/09-rust-engine-core.md) §5

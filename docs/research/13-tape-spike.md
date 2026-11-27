# 13: Spike: traced tapes, resource canonicalisation, and WASM parity

Status: research note, 2026-11-27. **Measured**, not estimated, on the owner's Apple M5 (4 performance + 6 efficiency cores) with Rust 1.100.0 and wasmtime 51.0.0. Code: [`spikes/tape-spike/`](../../spikes/tape-spike/). It is standalone, synthetic, std plus `rustc-hash`.

Run `cargo run --release -- <mode>` with mode = *(none)* | `canon` | `threads` | `lagrange` | `mc` | `hash`. For WASM: `cargo build --release --target wasm32-wasip1`, then `wasmtime target/wasm32-wasip1/release/tape-spike.wasm <mode>`.

## Question

[`docs/architecture.md`](../architecture.md) proposes to trace each Kit Shape's rotation MDP once into a flat **Tape** and evaluate it over lanes. Research 09 §1.6 names the rotation-MDP cost per kit (spike S2) as the largest performance uncertainty. Five things to settle:

1. Is the tape faster than the per-kit hash-map DP, and does it give the same numbers?
2. Does batching several kits per lane block help?
3. How bad is the large-resource case (Paladin-like, five slot levels, 10–12 rounds), and can it be tamed?
4. Do threads and WASM keep the results bit-identical, and what does WASM cost per core?
5. Are policy-aware moments (E, E²) correct against an independent oracle?

## The synthetic model

The mechanics are our own simplifications, with no rules text. A kit gets 2 attacks per turn. After a hit or crit it may optionally spend one slot of any level (**at most one slot per turn**, and the spend uses the Bonus Action) for extra dice, doubled on a crit. After the attacks, an optional Bonus-Action extra attack is allowed if a crit was seen. A hit grants advantage on the next attack roll, and that flag persists across turns (Vex-like). The objective is expected total damage over N rounds, with E and E² carried policy-aware. The d20 maths uses exact integer counts with one division (research 03 §1).

The baseline ("hash-map DP") is a memoised recursion over a `u64` state key in an `FxHashMap`, with **16 ACs as a vector** per state. That is research 05's per-kit vector, the strongest reasonable baseline. The tape is built from the **same** `step()` function, so there is one legality authority (ADR-0012). It stores nodes in post-order, uses liveness-based slot reuse, and keeps values as SoA `[slot][lane]`.

## Results

### 1. Tape vs hash-map DP (per kit, 16 ACs, single thread)

| Shape | Tape nodes | Live slots | Hash-map DP | Tape (1 kit) | Tape (best block) | Bit-identical |
|---|---|---|---|---|---|---|
| 2 atk, no slots, 3 rds | 49 | 8 | 15.9 µs | 4.1 µs | 2.8 µs (16 kits) | yes |
| slots 4/3, 3 rds | 323 | 33 | 63.9 µs | 14.1 µs | 10.3 µs (256) | yes |
| slots 4/3, 10 rds | 3,370 | 72 | 331 µs | 97.5 µs | 78.3 µs (16) | yes |
| slots 4/3/3/3/2, 3 rds | 1,268 | 127 | 122 µs | 35.3 µs | 29.6 µs (16) | yes |
| slots 4/3/3/3/2, 10 rds | 74,173 | 2,245 | 11,800 µs | 2,493 µs | 2,064 µs (16) | yes |

- **The tape is 3.4–6.2× faster** than the hash-map DP and **bit-identical** in every case.
- **Liveness reuse keeps 2–16% of nodes live.** The working set of the largest tape fits in L2.
- **Batching kits barely helps:** going from 1 to 16+ kits per block adds only 1.2–1.4×. The gain comes from removing hashing and pointer-chasing, not from SIMD over kits.
- **Tracing costs 0.04–6 ms per shape,** which is 1–10 per-kit evaluations. Sharing a tape across the kits of a shape is therefore worth 2–10× on tracing alone.

### 2. The large-resource case, and two ways to tame it

The table above shows the problem: 74k nodes and about 2.5 ms per kit at 10 rounds, and 119k nodes and about 4.2–7.3 ms at 12 rounds. Twelve rounds is the default Adventuring Day Model (4 encounters × 3 rounds) with long-rest slots. That confirms the S2 risk.

**(a) Lagrangian resource pricing (research 05 §3.3): ruled out.** The slot count is replaced by a per-level price λ (the "priced" tape has 76–301 nodes), and λ is minimised per lane by coordinate ternary search.

- **Validity:** the bound is always valid. There were zero violations across 48 kits × 16 ACs × 5 shapes, as duality theory predicts.
- **Cost:** reaching a 1% median gap takes 193–481 evaluations, which costs **more than the exact tape** (610–9,800 µs per kit).
- **Gap:** at 10–12 rounds the gap **stalls at about 1–3% median and 6–11% max**. That is the duality gap between the expected-budget and almost-sure constraints, so more iterations don't close it.
- **Ranking:** pairwise order agreement with the exact values is 97.8–100%. That isn't good enough for a top-3.

**(b) Horizon-bounded resource canonicalisation: adopted.** With R turns left and at most one slot spendable per turn, only the R highest slots can still matter. At each turn boundary the tracer keeps the top R slots by level and drops the rest. States that differ only in slots that can never be spent then merge.

| Shape | Nodes, plain → canonical | Eval per kit, plain → canonical | Bit-identical |
|---|---|---|---|
| 4/3, 3 rds | 323 → 78 (4.1×) | 11.5 → 3.7 µs | yes |
| 4/3, 10 rds | 3,370 → 1,953 (1.7×) | 148 → 86 µs | yes |
| 4/3/3/3/2, 3 rds | 1,268 → 130 (9.8×) | 51 → 5.3 µs | yes |
| 4/3/3/3/2, 10 rds | 74,173 → 3,584 (20.7×) | 2,552 → 122 µs | yes |
| 4/3/3/3/2, 12 rds | 118,911 → 9,586 (12.4×) | 4,206 → 339 µs | yes |
| 4/3/3/3/2, 20 rds | 310,370 → 143,941 (2.2×) | 11,137 → 5,281 µs | yes |

The result is **exact**: 64 kits × 16 ACs are bit-identical to the plain tape. The default adventuring day costs 0.34 ms per kit, not 4–7 ms. It helps least when the slot total is below the horizon (20 rounds with 15 slots), where nothing can be truncated at the start.

*Why it is sound [design, not proved]:* this relies on two properties. Each turn caps spending (the 2024 one-slot rule), and slots are monotone (a higher slot is never worse for any use). The architecture makes both **declared properties of a resource**, and adds a differential property test (canonical == plain) so that a resource breaking them is caught.

### 3. Threads (job queue, one kit per job, canonical tapes)

| Threads | 1 | 2 | 4 | 6 | 8 | 10 |
|---|---|---|---|---|---|---|
| slots 4/3, 3 rds (20,000 kits) | 1.0× (2.32 µs/kit) | 1.6× | 2.6× | 3.2× | 3.7× | **4.3×** (0.54 µs/kit) |
| 4/3/3/3/2, 12 rds (600 kits) | 1.0× (345 µs/kit) | 1.9× | 3.3× | 4.3× | 5.1× | **5.5×** (63 µs/kit) |

Every thread count gave bit-identical results (ADR-0037). Scaling falls a little short of research 09's estimate of 5–6.5×. The small-kit case is held back by per-call allocation in the spike, which per-thread scratch buffers would remove (`map_init`, research 09 §2.1).

### 4. WASM: per-core cost and cross-target bit identity

These figures come from wasmtime 51, which compiles with Cranelift. Browsers compile with V8 or JavaScriptCore, so treat this as a proxy until spike W2 runs in a real browser.

| Canonical shape | Native | WASM | WASM + simd128 | WASM / native |
|---|---|---|---|---|
| 4/3, 3 rds | 3.7 µs | 3.5 µs | 3.2 µs | ≈1.0× |
| 4/3, 10 rds | 86 µs | 94 µs | 86 µs | 1.0–1.1× |
| 4/3/3/3/2, 3 rds | 5.3 µs | 6.0 µs | 5.4 µs | 1.0–1.1× |
| 4/3/3/3/2, 10 rds | 122 µs | 185 µs | 167 µs | 1.4–1.5× |
| 4/3/3/3/2, 12 rds | 339 µs | 510 µs | 468 µs | 1.4–1.5× |

- **WASM costs 1.0–1.5× native per core,** in line with ADR-0045's 1.45–1.55×. The penalty grows with tape size, which suggests memory behaviour rather than arithmetic.
- **SIMD128 is worth about 8–10%.** It's worth enabling but not worth hand-writing intrinsics for.
- **Bit identity across targets:** a hash of every result bit (3 shapes × plain and canonical × 64 kits × 16 ACs × E and E²) is **`d96857c6c55ce1c9` on native aarch64, WASM and WASM+simd128 alike.** Rust doesn't contract to FMA by default, so the same `Fp` yields the same numbers on the owner's Mac and in a friend's browser.

### 5. Correctness of policy-aware moments against an independent oracle

The oracle is a Monte Carlo over the **state machine**, not the tape: 10⁶ trajectories per case. It follows the argmax-E policy, and each damage term is sampled from a two-point law with the same E and E², which is enough to test E and E² of a sum of independent terms. Across 3 kits × ACs 13/17/21 on 4/3/3/3/2 with 4 rounds, **every |z(E)| ≤ 1.67** and **SD agrees within 0.17%**. The update `E[(D+R)²] = E[D²] + 2E[D]E[R] + E[R²]` under the recorded policy is confirmed.

## Caveats

- **Synthetic mechanics.** Real kits will have more riders and possibly more shapes. The figures measure the evaluation technique; they don't predict full-solve times.
- **Kits per partition is still unknown.** Turning per-kit cost into Solve time needs the number of distinct kits after the projected join. That needs real content (the remainder of S2′).
- **Timing noise.** On the M5, per-kit times for small tapes varied up to about 1.8× between runs (P-core vs E-core scheduling). Figures are best of 2–3 runs. Benchmarks in CI should pin a thread count and compare ratios, not absolutes.
- **Lagrangian search.** The coordinate ternary search is simple. A smarter dual method (subgradient on expected slot use, carried as a linear quantity) would need fewer evaluations, but the measured duality gap of 1–3% at long horizons remains.

## Candidate ADRs / revisions

| Target | Recommendation |
|---|---|
| New ADR: **Kit Shape / Kit Params split and traced Tapes** | The tape is a recording of `legal_steps`/`apply` with liveness slot reuse, evaluated per kit over 16 AC lanes. Measured 3.4–6.2× over hash-map DP, bit-identical. Kit batching is optional (1.2–1.4×). |
| New ADR: **Horizon-bounded resource canonicalisation** | Resources declare a per-turn spend cap and monotonicity; the tracer clamps resource state to what can still be spent. Exact; up to 21× fewer nodes; makes the 12-round adventuring day about 0.34 ms per kit. |
| Research 05 §3.3 / ADR-0020 | Don't use Lagrangian λ-pricing to evaluate the Adventuring Day Model. The exact canonical MDP is cheaper and has no duality gap. Keep λ only as an optional valid bound. |
| ADR-0037 | Confirmed: identical across thread counts *and* across native/WASM/SIMD128. Add the cross-target result hash to CI. |
| ADR-0045 | Per-core WASM cost measured at 1.0–1.5× (Cranelift proxy). SIMD128 is worth about 10%. |

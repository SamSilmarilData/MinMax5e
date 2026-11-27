---
status: proposed
---

# In the browser, the solver runs on a pool of Web Workers on stable Rust, not on WASM threads

> *Amended 2026-11-27 (measurements, [research 13 §4](../research/13-tape-spike.md)):*
>
> - **Per-core cost:** WASM measured at **1.0–1.5× native** for the tape evaluator (wasmtime/Cranelift as a proxy; spike W2 confirms in browsers). SIMD128 adds about 8–10%.
> - **Bit identity:** results are bit-identical to native.
> - **Job planning:** the solver itself is a job plan (ADR-0050). Compute workers route jobs by shape affinity so each traces a Tape once. No shared memo is needed, because equal kits merge in the join (ADR-0049).

**Layout in the browser:**

- **Main thread:** the egui UI only (ADR-0043).
- **Coordinator worker:** owns a Solve. It splits the Solve into ADR-0037's deterministic chunks, hands them out, merges per-chunk top-K **in chunk order**, and streams `SolveEvent` snapshots to the UI.
- **Compute workers (N):** each runs chunks. Branch-and-bound incumbents are broadcast between them as messages, so pruning still works across workers, a little later than with shared memory.
- **Interactive worker:** holds the in-memory Candidate pool and answers slider re-scores (ADR-0020), so a heavy solve never delays slider feedback.

**Mechanics** [verified unless marked]:

- **Two WASM modules.** The engine and solver are a separate binary (`mm-worker`) from the UI (`mm-gui`). Trunk 0.22 builds both from one `index.html`: `<link data-trunk rel="rust" data-bin="worker" data-type="worker">`. Trunk's own `webworker` and `webworker-gloo` examples show the pattern. `gloo-worker` 0.7 gives typed messages.
- **Compile once.** The worker module is compiled once and posted to each worker as a `WebAssembly.Module`, so none of them recompile. WebKit documents that it passes a reference to the same machine code ([WebKit](https://webkit.org/blog/7691/webassembly/)). MDN documents the same pattern.
- **Messages.** Payloads are `postcard` bytes in transferable `ArrayBuffer`s. Messages are small: inputs, chunk ranges, top-K lists, and progress.
- **Worker count.** Default to `navigator.hardwareConcurrency − 1`, at least 2, adjustable in settings. Current WebKit reports **4** when a device has fewer than 8 cores, and **8** otherwise, as a fingerprinting defence (`NavigatorBase.cpp` on WebKit `main`). So a 6-core iPhone shows 4, and the 10-core M5 shows 8. Spike W2 checks whether more workers than reported actually helps on iOS.
- **SIMD.** WASM SIMD128 works in Chrome 91+, Firefox 89+, and Safari 16.4+ (MDN compat data), and `core::arch::wasm32` intrinsics have been stable since Rust 1.54. `mm-dice` may use them behind `cfg(target_feature = "simd128")`, with the web build compiled with `+simd128`.
- **Determinism.** Same as ADR-0037: the result depends on chunk order, not on which worker ran which chunk. The 1-vs-N snapshot test runs for both executors.

**Expected speed** [estimate, pending W2]:

- **Per core:** WASM ran 1.45–1.55× slower than native on SPEC CPU, with a peak of 2.5× ([Jangda et al., USENIX ATC '19](https://www.usenix.org/conference/atc19/presentation/jangda); dated). Tight numeric loops usually do better.
- **Whole search:** with 4–8 workers, live searches should land within about 2× of the native rayon build. Heavy solves stay background jobs, as on native.

## Considered Options

- **True WASM threads (`wasm-bindgen-rayon` 1.3) running the same rayon code.** Not now:
  - it needs nightly Rust, `-Z build-std`, and `+atomics,+bulk-memory`, and build-std is still unstable (Rust project goal 2026, RFCs 3873–3875 accepted, in progress);
  - it needs `SharedArrayBuffer`, which needs COOP/COEP `require-corp`, because Safari lacks `credentialless`;
  - the solver's chunks are coarse and independent, so shared memory buys little.

  The host can serve COOP/COEP through `_headers` (ADR-0044), so this stays a drop-in upgrade if W2 shows the message-passing pool losing more than about 20% against threads.
- **One worker, single-threaded.** Rejected: it gives up 3–7× on multi-core machines.
- **Solving on the main thread, in yielding chunks** (the old ADR-0042 fallback). Rejected: frame drops during solves.

Research: [11](../research/11-web-first-delivery.md), [09](../research/09-rust-engine-core.md) §5

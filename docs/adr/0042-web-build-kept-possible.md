---
status: superseded by ADR-0043
---

# The engine stays buildable for the browser (wasm32), but no web version is deployed

> **Superseded 2026-11-27 by [ADR-0043](0043-friends-use-the-web-build.md).** The web build is now how friends use MinMax5e. The wasm32 constraints below still apply. Two things changed: the browser runs the solver on a Web Worker pool (ADR-0045), not a single yielding thread, and it is hosted on Cloudflare Workers behind invite links (ADR-0044), not on GitHub Pages with `coi-serviceworker`.

A friend may want to use MinMax5e on a phone. The native app (ADR-0032) can't reach phones without app-store friction, but egui (ADR-0039) can render the same UI in a browser. To keep that door open cheaply, every crate except the native shells must compile for `wasm32-unknown-unknown`:

- **Threads are optional.** rayon sits behind a `parallel` cargo feature, on by default for native builds. Without it, the solver runs the same chunked loop on one thread and yields between chunks. Results are identical either way (ADR-0037 already asserts that 1 thread and N threads match).
- **I/O sits behind traits.** The engine, solver, loader, and importer never touch `std::fs`, `std::net`, or `std::time::Instant` directly. `mm-app-core` defines `Storage` (packs and cache) and `Fetcher` (corpus download). Native implements them with the filesystem, `directories`, and an HTTP client. A web build would implement them with OPFS/IndexedDB and `fetch`.
- **CI** runs `cargo check --target wasm32-unknown-unknown` on the non-shell crates, so this can't silently rot.

**What a web build would look like** (not built now):

- `eframe` web on a static host. Each visitor's browser downloads its own corpus copy straight from `raw.githubusercontent.com`. That server sends `Access-Control-Allow-Origin: *` (verified 2026-11-27), so ADR-0026's "share the app, never the data" still holds.
- **Speed:** GitHub Pages can't set the COOP/COEP headers that `SharedArrayBuffer` needs, and therefore WASM threads. The `coi-serviceworker` shim works around this. Without it, the browser build is single-threaded.
- **Phones:** one weak core makes searches roughly 10–50× slower than the M5 [estimate]. Evaluating a build and searching with constraints locked stay usable; open-ended searches don't.
- **Policy:** a public website is the trigger in ADR-0026 to revisit the original legal concerns, even though the site would host no rules data. Prefer an unlisted URL shared privately.

## Considered Options

- Native only, with no wasm constraints. Rejected: retrofitting `std::fs` and thread assumptions out of the engine later is expensive. The cost now is a feature flag and two traits.
- Ship a web build now. Rejected: not needed yet, and it brings forward the public-site question.
- A native mobile app (egui on Android or iOS). Rejected: iOS distribution outside the App Store needs a paid developer account or sideloading, and the browser reaches both platforms.

Research: [10](../research/10-rust-ui-and-app-shell.md), [07](../research/07-tech-stack-and-architecture.md) (web constraints)

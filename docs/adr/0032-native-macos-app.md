---
status: proposed
---

# MinMax5e is a native desktop app, macOS first and cross-platform-capable, and all compute runs in-process on local threads

> *Amended 2026-11-27:* the native app is now **the owner's build**. Friends use the same app through the browser (ADR-0043), so native release binaries for friends, and the unsigned-app right-click → Open step, are no longer needed. "No compute inside a browser" now applies to the native build only. On the web, compute runs in Web Workers on the visitor's device (ADR-0045), which keeps this ADR's intent: all compute local, with no server.

MinMax5e ships as a single native Rust binary, built first for the author's Mac. There is no hosted site, no server, and no compute inside a WebView or browser. The solver runs on an in-process rayon pool (cores − 1; the machine is an Apple M5 with 4 performance and 6 efficiency cores). It works fully offline, and all data stays local.

*Amended 2026-11-27 for sharing with friends (ADR-0026):*

- **Platforms:** keep the code cross-platform. egui/eframe, rayon, and crossbeam all run on Windows and Linux, so no macOS-only APIs outside `mm-gui`. Use the `directories` crate for cache paths rather than hard-coding `~/Library/Application Support`. Windows and Linux builds are produced on demand (GitHub Actions matrix or the friend building from source), not maintained as release targets.
- **macOS distribution:** unsigned, so no Apple Developer membership is needed. Friends open the app once with right-click → Open, or remove the quarantine flag.
- **Rejected:** paying for notarisation (US$99/yr for two or three users).
- **Phones and browsers:** not served by this native app. A browser build is kept *possible* but not deployed (ADR-0042).

Native threads remove the constraints of WASM threading in a browser (nightly Rust, `build-std`, and cross-origin isolation headers for `wasm-bindgen-rayon`), and a browser brings no benefit for a single user.

Supersedes ADR-0019 (static site with Web Workers).

## Considered Options

- Static site with Web Workers (ADR-0019). Rejected: web-only constraints with nothing gained for personal use.
- A web app served on localhost. Rejected: an extra server and a browser tab, for no benefit.
- Tauri with a Rust UI compiled to WASM (Leptos, Yew). Rejected: two build targets and a JSON IPC boundary.

Research: [10](../research/10-rust-ui-and-app-shell.md)

---
status: proposed
---

# Friends use MinMax5e by opening a link: the same egui app, compiled to WebAssembly

The owner wants a friend to get from "I was sent a link" to "I'm using the tool" with no install, on any device (Windows, Linux, macOS, iOS, Android). Once there, it should feel as smooth as the native app. So `mm-gui` is built for **two targets from one codebase**:

- **Web (how friends use it):** eframe's web backend, served as a static site (ADR-0044). The solver runs in Web Workers (ADR-0045). Each browser downloads and imports its own corpus copy (ADR-0041).
- **Native (the owner's build):** the same app with rayon on all cores, AccessKit, and no download step (ADR-0032).

There is no second UI. The widgets, egui_plot charts, view models, and `mm-app-core` session code are shared. Platform differences sit behind the `Storage`, `Fetcher`, and solver-executor seams.

**Why egui is good enough on the web as of late 2026** [verified at the changelogs]:

- **eframe 0.36.0 (2026-08-05):** "drastically improves the mobile keyboard experience (when using eframe web)": autocomplete, autocorrect, and IMEs work on iOS and Android (#8045). The text agent is anchored to the canvas, and focus no longer scrolls the host page.
- **0.37.0 (2026-10-15):** kinetic scrolling with rubber-banding. eframe web rasterises glyphs through the browser, so system fonts, emoji, and non-Latin scripts render.
- `egui_plot` 0.38 targets egui 0.37.

**Known limits on the web, accepted:**

- The browser's find-in-page, page translation, and native text selection don't apply to canvas content. The combat script gets copy buttons and selectable labels.
- Screen-reader support on the web is eframe's speech-synthesis reader (`web_screen_reader`). AccessKit is native-only. Every chart keeps its data-table view (ADR-0039).
- The first download is larger than a DOM app's. Measured: the published `eframe_template` (eframe 0.36, default features with `wgpu`, `opt-level = 2`, no LTO) is 7.78 MB raw and 3.12 MB gzip −9; xz −9 gives 2.27 MB, a lower bound on brotli's ratio. egui's bundled default fonts are 1.4 MB of TTF. Spike W1 measures our real build with `glow` vs `wgpu`, `opt-level` `z`, LTO, Trunk's built-in `wasm-opt`, and fonts dropped where the browser glyph rasteriser covers them. Target: about 2.5 MB brotli or less for the UI module. The engine ships as a separate worker module (ADR-0045), so the UI module carries no solver code.

**Boot path (first visit):**

1. `index.html` (a few KB) paints a loading screen at once.
2. The UI module is compiled while it downloads (`WebAssembly.instantiateStreaming`, Safari 15+, Chrome 60+). In parallel, an inline script starts the corpus fetch (ADR-0041).
3. On return visits, the app shell comes from the service worker, and V8 reuses its cached compiled code for a stable `.wasm` URL ([V8](https://v8.dev/blog/wasm-code-caching)).
4. **Optional install:** a manifest with icons (as in `eframe_template`) supports Chrome or Edge "Install", Safari "Add to Dock", and "Add to Home Screen". The Home Screen version also protects Safari storage from eviction (ADR-0041).

**Smoothness rules:** the UI thread does layout and painting only. It never runs solver work, on either target. Re-scoring the in-memory pool when a slider moves is a worker round trip of under one frame (ADR-0020, ADR-0045).

Supersedes ADR-0042.

## Considered Options

- **Native downloads for each OS (ADR-0032 as originally amended).** Rejected for friends: unsigned binaries need right-click → Open or a quarantine override, and an iPhone can't run them at all.
- **A DOM framework for the web (Leptos 0.8 or Dioxus 0.7) with hand-written SVG charts.** Rejected: it means a second UI. The mobile-input problems that motivated it were fixed in eframe 0.36.
- **TypeScript UI over a Rust WASM engine.** Rejected: breaks full Rust (ADR-0033), and the egui UI already exists.
- **Server-side solving (a VPS, or the owner's Mac through a tunnel).** Rejected for now: ops, cost, and uptime, and derived results would leave the device. Kept as a later option behind the same solver interface.
- **A native mobile app.** Rejected: app-store friction, and the browser reaches every phone.

Research: [11](../research/11-web-first-delivery.md)

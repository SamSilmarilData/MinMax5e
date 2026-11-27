# 10: Rust UI framework, charting and app shell

Status: research note (2026-11-27). Nothing here is decided. The decisions worth recording are listed under **Candidate ADRs / revisions**.

Conventions (same as [07](07-tech-stack-and-architecture.md)):
- Every claim cites a URL. **[unverified]** means the claim comes from my own knowledge or reasoning and I did not confirm it against a primary source in this session.
- **[estimate]** marks back-of-envelope figures that a spike should confirm.
- Version numbers, release dates, star counts and "last push" dates were read from the GitHub REST API (`/repos/{r}` and `/repos/{r}/releases`) and the crates.io API (`/api/v1/crates/{c}`) on 2026-11-27. They are cited by the repo or crate URL.
- Scope: the app shell, UI framework, charting and the UI↔solver plumbing. The engine and solver crates are covered by a separate note; this one only defines the seam they must expose.

---

## Question

The project changed direction: MinMax5e is now mainly a **personal tool that runs on the user's Mac**, with no public website or hosting, and it should be **"full Rust"** using the best technology available. Given that:

1. Which delivery shape is best: (a) a native pure-Rust desktop GUI, (b) Tauri v2 with a Rust→WASM UI, (c) a Rust web app on localhost, or (d) a CLI/TUI alongside one of these?
2. Which charting option can draw the dashboard in spec §5.3 and research [06 §4](06-market-users-and-ux.md): an AC-sensitivity multi-line chart with direct labels and crossover markers, quantile dotplots, horizontal stacked attribution bars, action-economy swimlanes and data-table fallbacks, with hover, legend toggling and annotations?
3. What is the developer experience for a solo developer (build times, hot reload, code volume, layout iteration)?
4. What is the recommended stack, a fallback, and the architecture for a CPU-heavy multithreaded solver that streams progress and is cancelled when a slider moves?

## TL;DR

- **Going native removes the hardest problem from research 07.** In a browser, parallel Rust needs `wasm-bindgen-rayon`, which needs cross-origin isolation, a pinned nightly toolchain and `build-std` with atomics ([wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon)). A native process gets real OS threads, rayon, atomics and plain channels for free. Every option below that runs the solver natively (egui, iced, Slint, Dioxus desktop, Tauri's backend) gets this; options that run the solver in the browser do not.
- **Primary recommendation: `eframe`/`egui` 0.37 + `egui_plot` 0.38, with the solver on a rayon pool fed by `crossbeam-channel` and cancelled by an atomic generation token.**
  - egui is the most mature and most active pure-Rust GUI: 0.37.0 shipped on 2026-10-15 after 0.36.x in Aug and Sep ([egui releases](https://github.com/emilk/egui/releases)); it is MIT/Apache-2.0 and backed by Rerun ([egui README](https://github.com/emilk/egui)).
  - `egui_plot` already has everything the charts need: `Line` with dash styles and fills, `Points` with marker shapes, `Text` annotations, `VLine`/`HLine`, horizontal **stacked** `BarChart` (`stack_on`, `horizontal`), `BoxPlot`, hover `label_formatter`, and **click-to-toggle legends** (alt-click to solo a series) ([egui_plot items](https://docs.rs/egui_plot/latest/egui_plot/), [legend.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/overlays/legend.rs), [bar_chart.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/items/bar_chart.rs)).
  - Immediate mode fits a "live sandbox": every frame reads the latest solver snapshot, so there is no reactive graph to keep in sync. `egui::Context` is `Send + Sync`, and `request_repaint()` from a worker thread wakes the UI ([egui Context docs](https://docs.rs/egui/latest/egui/struct.Context.html)).
  - Accessibility: eframe's `accesskit` feature exposes widgets to VoiceOver ([eframe Cargo.toml](https://github.com/emilk/egui/blob/main/crates/eframe/Cargo.toml)), and 0.37 improved the tree further ([egui 0.37.0](https://github.com/emilk/egui/releases/tag/0.37.0)). Plots are painted shapes, so every chart needs the data-table fallback that research 06 already asks for.
  - Costs: layout is harder than in CSS ("a fundamental shortcoming of immediate mode GUIs"), styling is "not yet as powerful as say CSS", and "new releases will have breaking changes" ([egui README](https://github.com/emilk/egui)). Hot patching works only through a community recipe using Dioxus's `subsecond` ([egui-subsecond-example](https://github.com/enomado/egui-subsecond-example), community, **[unverified]** beyond its README).
- **Fallback: Dioxus 0.7 desktop (system WebView, Rust runs natively) with charts hand-written as SVG in `rsx!`.** It keeps a native Rust process (real threads, no WASM, no IPC), gives HTML/CSS layout, WebKit's native accessibility for free, and has first-party **hot patching of Rust code** on macOS desktop ([Dioxus 0.7 release](https://dioxuslabs.com/blog/release-070/), [Dioxus desktop guide](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop)). Its costs: no Rust charting library is mature enough (`dioxus-charts` 0.4.0 has about 9k recent downloads, [crates.io](https://crates.io/crates/dioxus-charts)), so we hand-roll SVG charts (about 150–300 lines each **[estimate]**). Also 0.8 is in alpha with breaking changes ([v0.8.0-alpha.3](https://github.com/DioxusLabs/dioxus/releases/tag/v0.8.0-alpha.3)).
- **Rejected for this project:** Tauri + Leptos/Yew (two compile targets, a JSON IPC boundary and a WASM UI, all to get what Dioxus desktop gives in one process); a localhost web app (same costs, plus a server and a browser tab); iced (no accessibility, [iced#552](https://github.com/iced-rs/iced/issues/552), and no release since 0.14.0 on 2025-12-07); Xilem ("experimental", [Xilem README](https://github.com/linebender/xilem)); GPUI (pre-1.0, "read the Zed source code" as documentation, [GPUI README](https://github.com/zed-industries/zed/tree/main/crates/gpui)); Makepad (no tagged releases, sparse docs, [Makepad](https://github.com/makepad/makepad)). Slint is a credible second fallback (declarative `.slint` markup with live reload since 1.13, GPLv3 or a free royalty-free licence, [Slint LICENSE](https://github.com/slint-ui/slint/blob/master/LICENSE.md)), but it has no charting library, so every chart would be hand-built.
- **Charting:** `egui_plot` is the only pure-Rust option that is interactive out of the box *and* actively released (0.38.0 on 2026-10-19). `plotters` draws well but is static: its last release is 0.3.7 from 2024-09-08 ([crates.io](https://crates.io/crates/plotters)). `charming` is a Rust façade over ECharts' **JavaScript**, which it runs in the browser or in embedded `deno_core` ([charming](https://github.com/yuankunzhang/charming)), so it is not "full Rust" at runtime. The Rust Vega effort (`avenger`) is still a "WIP experimental" renderer ([avenger](https://github.com/jonmmease/avenger)).
- **CLI:** keep a `clap` CLI for batch and golden runs from day one; add a `ratatui` 0.30 TUI ([ratatui](https://ratatui.rs/)) only if it earns its place. Both depend on the same UI-agnostic `app-core` crate as the GUI.
- **ADRs:** supersede 0019 (static site, browser compute), 0021 (TS engine) and 0022 (pnpm monorepo); amend 0020 (pools become an optional local cache, not shipped artefacts). There are five new candidate ADRs below.

---

## 1. Delivery options compared

### 1.1 What "talking to the solver" needs

From spec §5.3 and research 07 §3: sliders must update charts "immediately". Some changes are lookups (AC); others start a new solve that must **cancel the previous one**, **stream best-so-far results**, and never block the UI thread. On a native process this needs:

- a long-lived thread pool (rayon) that is *not* the UI thread;
- a progress channel from workers to the UI, and a way to wake the UI when something arrives;
- a cancellation signal that workers poll cheaply between chunks (an `AtomicU64` generation counter, or `tokio_util::sync::CancellationToken`, [docs.rs](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) **[unverified: page not fetched]**).

rayon has no built-in task cancellation **[unverified, consistent with the rayon API on [docs.rs](https://docs.rs/rayon/latest/rayon/)]**, so cancellation is cooperative in every option. The options differ in how a result gets back to the UI.

### 1.2 Comparison table

| | **egui / eframe** | **iced** | **Slint** | **Dioxus desktop (WebView)** | **Dioxus native (Blitz)** | **Tauri v2 + Leptos/Yew (WASM UI)** | **Localhost web app (Leptos/Dioxus fullstack, axum)** | **Xilem / Makepad / GPUI** | **ratatui (TUI)** |
|---|---|---|---|---|---|---|---|---|---|
| Latest release (2026-11-27) | egui 0.37.0, 2026-10-15; eframe 0.37.0; egui_plot 0.38.0, 2026-10-19 ([releases](https://github.com/emilk/egui/releases), [egui_plot](https://github.com/emilk/egui_plot)) | 0.14.0, 2025-12-07; `master` pushed 2026-11-19 ([releases](https://github.com/iced-rs/iced/releases)) | 1.19.1, 2026-11-24; 1.19.0 on 11-16, 1.18.1 on 09-21 ([releases](https://github.com/slint-ui/slint/releases)) | Stable 0.7.10 on crates.io ([crates.io](https://crates.io/crates/dioxus)); 0.8.0-alpha.3, 2026-10-16 ([release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.8.0-alpha.3)) | Same releases as Dioxus; Blitz "work in progress" ([0.7 blog](https://dioxuslabs.com/blog/release-070/)) | tauri 2.12.7, 2026-11-18 ([releases](https://github.com/tauri-apps/tauri/releases)); Leptos 0.8.23 stable, 0.9.0-rc1, 2026-10-31 ([releases](https://github.com/leptos-rs/leptos/releases)); Yew 0.23.0, 2026-03-10 ([releases](https://github.com/yewstack/yew/releases)) | Leptos as left; cargo-leptos 0.3.11, 2026-10-02 ([releases](https://github.com/leptos-rs/cargo-leptos/releases)) | Xilem 0.4.0, 2025-10-29 ([repo](https://github.com/linebender/xilem)); Makepad: no GitHub releases, `makepad-widgets` 1.0.0 on 2025-05-13 ([crates.io](https://crates.io/crates/makepad-widgets)); GPUI 0.2.2 on crates.io, 2026-10-30 ([crates.io](https://crates.io/crates/gpui)) | 0.30.2, 2026-06-19 ([releases](https://github.com/ratatui/ratatui/releases)) |
| Maturity signal | 32.5k stars; "professional looking applications, like the Rerun Viewer"; breaking changes each release ([README](https://github.com/emilk/egui)) | 32.1k stars; slow release cadence (about one a year) | 24.7k stars; commercial vendor; 1.x API stability | 39.9k stars; 0.x with breaking alphas | WIP | Tauri 113k stars, very mature; Leptos 21.6k stars | Mature server stack, but heavy for a single-user Mac tool | All self-described experimental or pre-1.0: Xilem "experimental" ([README](https://github.com/linebender/xilem)); GPUI "still pre-1.0… often breaking changes" ([README](https://github.com/zed-industries/zed/tree/main/crates/gpui)) | 23.4k stars, stable and widely used |
| Licence | MIT / Apache-2.0 | MIT | GPLv3, **or royalty-free (no cost) for desktop apps**, or commercial ([LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md)) | MIT / Apache-2.0 | MIT / Apache-2.0 | Tauri MIT/Apache-2.0; Leptos MIT; Yew MIT/Apache-2.0 **[Tauri/Yew dual licence unverified; API reports Apache-2.0]** | MIT | Xilem Apache-2.0; Makepad MIT; GPUI Apache-2.0 **[unverified: API reports NOASSERTION for the Zed repo]** | MIT |
| Reactivity model | Immediate mode: UI is a function run every frame; repaints only on input, animation or `request_repaint` ([README](https://github.com/emilk/egui)) | Elm architecture (`update`/`view`/`Message`) with reactive rendering in 0.14 ([0.14.0](https://github.com/iced-rs/iced/releases/tag/0.14.0)) | Declarative `.slint` DSL with property bindings; Rust callbacks **[unverified detail]** | Signals + virtual DOM, `rsx!` ([docs](https://dioxuslabs.com/learn/0.7/)) | As left | Leptos: fine-grained signals; Yew: React-like components **[unverified detail]** | Signals, plus server functions | Xilem: reactive view tree; Makepad: retained + live DSL; GPUI: hybrid retained/immediate **[unverified]** | Immediate mode ([ratatui.rs](https://ratatui.rs/)) |
| Rendering quality | GPU (wgpu or glow); non-native look ("If you want a GUI that looks native, egui is not for you", [README](https://github.com/emilk/egui)); 0.37 renders system glyphs and colour emoji ([0.37.0](https://github.com/emilk/egui/releases/tag/0.37.0)) | GPU (wgpu) or software | GPU or software; polished widget styles **[unverified]** | WebKit (WKWebView on macOS) via wry: CSS-grade typography and layout ([desktop guide](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop)) | wgpu-painted HTML/CSS; "not all CSS" ([0.7 blog](https://dioxuslabs.com/blog/release-070/)) | WebKit | Whatever browser you use | Vello/wgpu (Xilem), Metal (GPUI, Makepad) | Terminal cells |
| Accessibility | AccessKit via the `accesskit` eframe feature ([Cargo.toml](https://github.com/emilk/egui/blob/main/crates/eframe/Cargo.toml)); plots are not exposed | **None yet**: [iced#552](https://github.com/iced-rs/iced/issues/552) "Implement accessibility support" is open | Has accessibility support **[unverified]** | Native WebKit a11y tree; SVG `<title>`/ARIA usable **[unverified for wry specifically]** | AccessKit integration **[unverified]** | WebKit a11y | Browser a11y | Xilem: AccessKit ([README](https://github.com/linebender/xilem)); others unclear | Screen-reader support in terminals is poor **[unverified]** |
| Hot reload | No first-party support. Community: `dx serve --hot-patch` with eframe, state kept, struct-layout changes need a restart ([egui-subsecond-example](https://github.com/enomado/egui-subsecond-example)) | 0.14 adds hot reloading and a `comet` time-travel debugger ([0.14.0](https://github.com/iced-rs/iced/releases/tag/0.14.0)) | `.slint` live preview inside the running app (`--features slint/live-preview` + `SLINT_LIVE_PREVIEW`) since 1.13 ([live preview docs](https://docs.slint.dev/latest/docs/slint/guide/tooling/live-preview/), [1.13 blog](https://slint.dev/blog/slint-1.13-released)) | **First-party Rust hot patching** (Subsecond) on macOS desktop via `dx serve`; RSX/CSS hot reload; experimental, no automatic state migration ([0.7 blog](https://dioxuslabs.com/blog/release-070/)) | As left | Trunk rebuild and reload of the WASM UI; Rust backend restarts ([Tauri Leptos guide](https://v2.tauri.app/start/frontend/leptos/)) | cargo-leptos: CSS hot reload and browser live-reload, no Rust hot patching ([cargo-leptos](https://github.com/leptos-rs/cargo-leptos)) | Makepad: "live-editable design language" ([README](https://github.com/makepad/makepad)) | `subsecond` has a ratatui starter integration ([0.7 blog](https://dioxuslabs.com/blog/release-070/)) |
| **Solver off the UI thread** | Direct: `std::thread`/rayon in the same process | Direct; results flow back as `Message`s from a `Task` | Direct; marshal results with `slint::invoke_from_event_loop` **[unverified API name]** | Direct: Rust runs natively, "use `std::thread::spawn` on desktop/mobile" for expensive work ([async docs](https://dioxuslabs.com/learn/0.7/essentials/basics/async)) | Direct | Backend: direct. **UI is WASM in the WebView, so all data crosses Tauri IPC** (commands + `Channel`) ([calling Rust](https://v2.tauri.app/develop/calling-rust/)) | Server: direct. UI talks over HTTP/WebSocket/server functions | Direct | Direct |
| **Streaming progress** | Worker sends a snapshot over `crossbeam-channel`, then calls `ctx.request_repaint()`, which "will wake up and run" the UI thread ([Context docs](https://docs.rs/egui/latest/egui/struct.Context.html)). UI drains with `try_recv` each frame | `Task::run`/`Task::stream` map a `Stream` to `Message`s; `Task::sip` maps progress and output separately ([Task docs](https://docs.rs/iced/latest/iced/struct.Task.html)) | Push into a model from the event loop **[unverified]** | `spawn` a future that awaits a `futures`/`tokio` channel and writes a signal **[unverified pattern]** | As left | Tauri `Channel`: "the recommended mechanism for streaming data", ordered; events are JSON-only and may arrive out of order ([calling Rust](https://v2.tauri.app/develop/calling-rust/)) | SSE/WebSocket **[unverified]** | Varies | Same as egui |
| **Cancelling on slider move** | Bump an `AtomicU64` generation; workers poll between chunks; the UI drops stale snapshots | `Task::abortable` returns a `Handle` ([Task docs](https://docs.rs/iced/latest/iced/struct.Task.html)), but that aborts the *future*, not the rayon work, so the atomic token is still needed | Atomic token | Atomic token; Dioxus `spawn` tasks also cancel on unmount or `Task::cancel` ([async docs](https://dioxuslabs.com/learn/0.7/essentials/basics/async)) | Atomic token | Atomic token held in Tauri state; cancel via a command | Atomic token, plus a request per cancel | Atomic token | Atomic token |
| Fit for this project | **Best** | Good model, but no a11y and a slow release cadence | Good, but no charts | **Strong fallback** | Too early | Works, but adds a WASM target and IPC for no gain over Dioxus desktop | Works, but adds a server process and a browser tab for one user | Too early | Complement only |

### 1.3 Notes per option

**(a) Native pure Rust.**
- **egui/eframe.** Strongest balance of maturity, charting and release cadence. Immediate mode means the dashboard code reads like the spec: "draw the controls, draw the top-3 cards, draw the charts from the latest snapshot". The `egui_extras` `TableBuilder` gives the data tables and the swimlane grid (egui_extras 0.37.0, [crates.io](https://crates.io/crates/egui_extras)). `egui_kittest` now records integration-test videos ([0.37.0](https://github.com/emilk/egui/releases/tag/0.37.0)), which helps visual regression tests. eframe also has an `inspection` feature that lets an external tool (e.g. an `egui_mcp` server) read the AccessKit tree, inject input and take screenshots ([Cargo.toml](https://github.com/emilk/egui/blob/main/crates/eframe/Cargo.toml)), which suits agent-driven UI testing **[unverified beyond the feature comment]**.
- **iced.** The Elm architecture plus `Task::sip` is the cleanest model for "progress + final output" ([Task docs](https://docs.rs/iced/latest/iced/struct.Task.html)), and 0.14 added hot reloading and time-travel debugging ([0.14.0](https://github.com/iced-rs/iced/releases/tag/0.14.0)). But there is no accessibility ([#552](https://github.com/iced-rs/iced/issues/552)) and no release for almost a year. Charting is weak: `plotters-iced` last released 0.11.0 in 2024-09 ([repo](https://github.com/Joylei/plotters-iced)), and `iced_plot` has about 570 recent downloads ([crates.io](https://crates.io/crates/iced_plot)). Otherwise you draw on `canvas`.
- **Slint.** Very active (1.19.x in Nov 2026), a free licence for a desktop app either way ([LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md)), and the best *layout* live preview of the native options ([live preview](https://docs.slint.dev/latest/docs/slint/guide/tooling/live-preview/)). There is no charting crate, so every chart is hand-built from `Path` elements or rendered to an image **[unverified: no Slint charting crate found on crates.io (`slint-plotters` does not exist)]**.
- **Dioxus native (Blitz).** Interesting but "work in progress", and "not all CSS features" are supported ([0.7 blog](https://dioxuslabs.com/blog/release-070/)). Revisit in 2027.
- **Xilem / Masonry.** "An experimental Rust architecture for reactive UI" ([README](https://github.com/linebender/xilem)).
- **Makepad.** Has a live-editable DSL, but no tagged releases and documentation is mostly the README and examples ([README](https://github.com/makepad/makepad)).
- **GPUI.** Usable outside Zed in principle (it is on crates.io, [gpui](https://crates.io/crates/gpui)), and `gpui-component`/GPUI Kit (v0.11.0, 2026-11-23) claims to be "Production Ready" with line, bar, area, pie, radar, candlestick and sankey charts with hover tooltips ([gpui-component](https://github.com/longbridge/gpui-component), [GPUI Kit chart](https://gpui-kit.com/component/chart)). But GPUI says "the best way to learn about these APIs is to read the Zed source code" and warns of frequent breaking changes ([GPUI README](https://github.com/zed-industries/zed/tree/main/crates/gpui)), and GPUI Kit pins a GPUI snapshot. It is too high a risk for a solo developer today; it is the one to watch.

**(b) Tauri v2 + Rust→WASM UI.** Tauri itself is excellent and active (2.12.7, [releases](https://github.com/tauri-apps/tauri/releases)), and its backend runs rayon natively. But the UI then lives in WASM inside WebKit, so every snapshot crosses IPC as JSON (`Channel` for streams, [calling Rust](https://v2.tauri.app/develop/calling-rust/)). You also maintain two targets (native backend + `wasm32` frontend built by Trunk), and Tauri's Leptos guide is "accurate only for Leptos 0.6" and supports SSG only ([Tauri Leptos guide](https://v2.tauri.app/start/frontend/leptos/)). Dioxus desktop gives the same WebView rendering with the Rust UI code running **natively in the same process**, so there is no serialisation boundary. **(b) is dominated by Dioxus desktop for this project.**

**(c) Localhost web app.** It has the same WebView-class rendering, plus a server process, a port, a browser tab, and either WASM hydration or server round-trips on every slider move. Nothing about it helps a single-user Mac tool. Browser-side parallelism would need `wasm-bindgen-rayon`'s nightly + `build-std` + cross-origin isolation ([wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon)), but here the solver would run server-side anyway, which makes the browser just a remote display. **Rejected.**

**(d) CLI/TUI.** A `clap` CLI over the same `app-core` crate is needed regardless, for golden tests, batch solves and benchmarks (as in ADR-0022's `apps/cli`). ratatui 0.30.2 has charts, sparklines and tables with "sub-millisecond rendering… and immediate-mode rendering" ([ratatui.rs](https://ratatui.rs/)), and its chart widgets can show an AC curve as a quick terminal check. Treat a TUI as optional and post-MVP.

---

## 2. Charting

### 2.1 Required charts (from [06 §4](06-market-users-and-ux.md))

1. **AC sensitivity:** 3 series × 16 integer AC points, point markers, distinct dash/marker per build, direct end-of-line labels, a vertical rule at the chosen AC, annotated crossovers ("#2 overtakes #1 at AC ≥ 19"), Y from 0, hover values; plus a save-bonus small multiple.
2. **Reliability:** a horizontal quantile dotplot (about 50 dots per build) with mean tick, P5 floor and P95 ceiling, and a plain-language sentence above it.
3. **Attribution:** horizontal stacked bars, one per build, fixed category order and colours, segment labels.
4. **Combat script:** action-economy swimlanes (text rows in lanes, not a chart).
5. **Data-table fallback** behind each chart; no meaning in colour alone.
6. Optional: hypothetical outcome plot (HOP) animation honouring reduced motion; levelling curve 1–20 with shaded troughs.

### 2.2 Charting comparison table

| Library | Works with | AC multi-line + direct labels + crossovers | Quantile dotplot | Horizontal stacked bars | Hover tooltips | Legend toggle | Maintenance | "Full Rust"? |
|---|---|---|---|---|---|---|---|---|
| **egui_plot 0.38** | egui/eframe | Yes: `Line` (+ `style(LineStyle)` dashes, `fill`), `Points` with marker shapes ([series.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/items/series.rs), [markers example](https://github.com/emilk/egui_plot/tree/main/examples/markers)), `Text` with `anchor` for direct labels ([text.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/items/text.rs)), `VLine` for the chosen AC, `include_y(0.0)` ([Plot docs](https://docs.rs/egui_plot/latest/egui_plot/struct.Plot.html)). Crossovers are computed in Rust and drawn as `Points` + `Text` | Yes: `Points` at computed (x, stack-row) positions. No built-in dotplot binning, about 30 lines to compute **[estimate]** | Yes: `BarChart::horizontal()` + `stack_on(&[..])` ([bar_chart.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/items/bar_chart.rs), [stacked_bar example](https://github.com/emilk/egui_plot/tree/main/examples/stacked_bar)); `element_formatter` for segment text | Yes: `label_formatter`, `show_x`/`show_y`, `find_closest` ([Plot docs](https://docs.rs/egui_plot/latest/egui_plot/struct.Plot.html)); `link_cursor` across small multiples | **Yes**: clicking an entry toggles it; alt-click solos it; `hidden_items` sets visibility from code ([legend.rs](https://github.com/emilk/egui_plot/blob/main/egui_plot/src/overlays/legend.rs), [Legend docs](https://docs.rs/egui_plot/latest/egui_plot/struct.Legend.html)) | Releases about every 1–2 months (0.36 June, 0.37 Aug, 0.38 Oct 2026, [releases](https://github.com/emilk/egui_plot/releases)) | Yes |
| **plotters 0.3.7** (+ backends) | Bitmap, SVG, HTML5 canvas, GTK/Cairo, piston ([docs](https://docs.rs/plotters/latest/plotters/)); community `plotters-iced`, `egui-plotter` | Yes, drawn statically (full control of labels and annotations) | Yes, draw circles | Yes, draw rectangles | **No**: "a drawing library"; interactivity is yours to build ([docs](https://docs.rs/plotters/latest/plotters/)) | No | Last release 0.3.7 on 2024-09-08; master commits until 2026-03 ([crates.io](https://crates.io/crates/plotters), [commits](https://github.com/plotters-rs/plotters/commits/master)) | Yes |
| **charming 0.6** (ECharts) | `HtmlRenderer` (browser), `WasmRenderer` (needs `eval`), `ImageRenderer` (embeds `deno_core` to run ECharts JS) ([charming](https://github.com/yuankunzhang/charming)) | Yes (ECharts lines, markLine/markPoint) **[unverified in charming: not on README]** | Scatter-based, possible | Stacked bars: yes | Yes (ECharts) | Yes (ECharts) | 0.6.0 on 2025-06-17; repo pushed 2026-11 ([crates.io](https://crates.io/crates/charming)) | **No**: the chart engine is JS executed by a webview or V8 |
| **GPUI Kit charts** | GPUI only | Line + area with crosshair hover; direct labels/annotations unclear ([GPUI Kit chart](https://gpui-kit.com/component/chart)) | Not built in | Bar; stacking unclear | Yes, hit-testing tooltips | Legends mentioned, no documented API | Very active, pre-1.0 base | Yes |
| **iced canvas / iced_plot / plotters-iced** | iced | Hand-draw on `canvas` | Hand-draw | Hand-draw | Hand-built | Hand-built | `iced_plot` 0.5.0, tiny user base ([crates.io](https://crates.io/crates/iced_plot)); `plotters-iced` 2024 | Yes |
| **Hand-rolled SVG in `rsx!`** (Dioxus) or Leptos | Dioxus desktop, Leptos | Full control; CSS hover states; `<title>` tooltips | Easy (`<circle>`s) | Easy (`<rect>`s) | CSS/`<title>`; custom tooltip about 40 lines **[estimate]** | Signal-driven, about 20 lines **[estimate]** | Yours | Yes |
| `dioxus-charts` 0.4 / `leptos-chartistry` 0.2 | Dioxus / Leptos | Basic line/bar; annotations unclear | No | Partial **[unverified]** | Chartistry: yes **[unverified]** | Unclear | About 9k / 43k recent downloads ([dioxus-charts](https://crates.io/crates/dioxus-charts), [leptos-chartistry](https://crates.io/crates/leptos-chartistry)) | Yes |
| JS library (Observable Plot / ECharts / uPlot) inside a WebView | Dioxus desktop via `document::eval`, Tauri | Excellent (research 07 picked Observable Plot) | Excellent (Plot `dotX` + `binX`) **[unverified]** | Excellent | Excellent | Excellent | Excellent | **No**: JS in the render path, a bundling step, and a JSON boundary for every update |
| ratatui `Chart`/`BarChart` | Terminal | Coarse; Braille line charts **[unverified widget detail]** | No | Bars only | No | No | Active | Yes |
| Vega in Rust: `avenger`, `vl-convert`, `vegafusion` | wgpu (avenger); static export via embedded Deno (vl-convert); server-side transforms (VegaFusion) | — | — | — | — | — | avenger: "WIP experimental visualization engine and renderer in Rust using wgpu" ([avenger](https://github.com/jonmmease/avenger)); vl-convert v2.0.0-rc7 ([vl-convert](https://github.com/vega/vl-convert)); VegaFusion last release 2.0.3 on 2025-09-29 ([vegafusion](https://github.com/vegafusion/vegafusion)) | Not usable as an interactive Rust chart layer in 2026. No Observable Plot port to Rust found **[unverified: absence]** |

### 2.3 What going JS would cost against "full Rust"

Using Observable Plot or ECharts inside Dioxus desktop or Tauri is the highest-quality chart path for the least code. But it brings back a JS toolchain (npm/bundling or vendored minified files), a JSON boundary for every chart update, and two languages to debug. Because `egui_plot` covers every required chart natively, **pure Rust charting is not too weak here**, so the JS path is not recommended. It is the documented escape hatch for the Dioxus fallback only, and only if hand-rolled SVG proves too slow to iterate on.

---

## 3. Developer experience for a solo developer

- **Build times.** egui/eframe has a modest dependency tree **[unverified]**. The biggest lever is the workspace layout: keep the GUI crate thin and the engine and solver in separate crates, so a UI edit recompiles one crate and relinks. Dioxus and Tauri add `dx`/Trunk and, for Tauri, a second (`wasm32`) target. Measure in spike S5 rather than trust anecdotes. **[unverified: no primary benchmark found]**
- **Hot reload.**
  - Dioxus: first-party Subsecond hot patching of Rust on macOS desktop; experimental; struct-layout changes need a restart ([0.7 blog](https://dioxuslabs.com/blog/release-070/)).
  - egui: the same Subsecond engine through a community recipe (`dx serve --hot-patch`; the CLI version must match), state preserved, about 1 s per edit as reported by its author ([egui-subsecond-example](https://github.com/enomado/egui-subsecond-example), **[unverified: author's claim]**).
  - Slint: `.slint` live reload inside the running app ([live preview](https://docs.slint.dev/latest/docs/slint/guide/tooling/live-preview/)).
  - iced 0.14: hot reloading plus the `comet` debugger ([0.14.0](https://github.com/iced-rs/iced/releases/tag/0.14.0)).
  - cargo-leptos: CSS hot reload and live reload only ([cargo-leptos](https://github.com/leptos-rs/cargo-leptos)).
- **UI code volume [estimate].** For the §5.3 dashboard (constraints panel, top-3 cards with diff highlighting, three charts plus a save-bonus small multiple, swimlane script, data tables, assumptions strip):
  - egui + egui_plot: about 1.5–2.5k lines of Rust. Charts are about 60–120 lines each because axes, hover and legend come free.
  - Dioxus desktop with hand-rolled SVG: about 2–3k lines of Rust + CSS. Each chart is about 150–300 lines (scales, ticks, tooltips).
  - Slint: similar to Dioxus, with charts the hardest part.
- **Layout iteration.** CSS (Dioxus) and `.slint` with live preview are faster for pixel-level layout than egui, whose layout is imperative and computed in one pass (see [egui README](https://github.com/emilk/egui) on the layout limits of immediate mode). For a dense analyst dashboard of panels, grids and tables, egui's `SidePanel`/`CentralPanel`/`Grid`/`TableBuilder` are adequate **[judgement]**.
- **Testing.** `egui_kittest` drives the UI headlessly and can record videos ([0.37.0](https://github.com/emilk/egui/releases/tag/0.37.0)). Golden tests belong in the CLI.
- **Agent friendliness.** egui and Dioxus have the most public example code of the Rust options **[judgement]**, which matters when coding agents write much of the UI.

---

## 4. Recommendation

### Primary: native egui/eframe + egui_plot, solver on rayon, crossbeam channels, atomic generation token

Why:
1. **The solver is native.** The rayon pool, atomics and channels need no WASM, IPC or cross-origin isolation. This deletes research 07's Comlink/Web Worker/`terminate()` machinery.
2. **It is the only pure-Rust chart stack that already does every required chart interactively** (§2.2).
3. **Immediate mode matches streaming.** The UI shows whatever snapshot is newest; there is no subscription graph or stale-closure bugs; `request_repaint` from the worker wakes it ([Context docs](https://docs.rs/egui/latest/egui/struct.Context.html)).
4. **It is the most actively released and widely used pure-Rust GUI, under MIT/Apache-2.0.**
5. **Accessibility is partly covered (AccessKit), and the data tables cover the rest.**

Accepted costs: non-native look, breaking upgrades (pin versions and upgrade deliberately), CSS-less layout, and hot patching only through a community recipe.

### Fallback: Dioxus 0.7 desktop + hand-rolled SVG charts

Switch if spike S1/S4 shows that egui layout, text quality or accessibility is unacceptable. The solver architecture below is identical: replace `request_repaint` with a Dioxus task that awaits the channel and writes a signal. Keep the `app-core` API async-agnostic so both fit. Second fallback: Slint, if a declarative native UI is preferred and hand-built charts are acceptable.

### Architecture

```
┌──────────────────────────── macOS process: minmax5e (single binary) ───────────────────────────────┐
│                                                                                                     │
│  UI THREAD (eframe event loop, egui immediate mode, AccessKit)                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │ each frame:                                                                                  │  │
│  │   1. while let Ok(ev) = events_rx.try_recv() { if ev.gen == current_gen { apply(ev) } }      │  │
│  │   2. draw Controls │ Assumptions strip │ Top-3 cards │ egui_plot charts │ Swimlanes │ Tables  │  │
│  │   3. on control change:                                                                      │  │
│  │        - lookup-only (AC slider)  → read cached per-AC top-K in UiState, no solve            │  │
│  │        - re-search (level, mode…) → gen = session.submit(params)  (debounced ~100 ms)        │  │
│  │ UiState { params, current_gen, latest: Option<Arc<SolveSnapshot>>, progress, table views }   │  │
│  └───────────────┬──────────────────────────────────────────────────────▲───────────────────────┘  │
│                  │ submit(params) → gen                                  │ events_rx (crossbeam,    │
│                  │                                                       │ bounded; plus            │
│                  ▼                                                       │ ctx.request_repaint())   │
│  ┌──────────────────────────── app-core::Session (UI-agnostic) ──────────┴───────────────────────┐ │
│  │  generation: Arc<AtomicU64>   ── submit() does generation.fetch_add(1), so old solves         │ │
│  │                                   see `token.is_stale()` and stop at the next chunk boundary  │ │
│  │  jobs_tx ──► coordinator thread (1 OS thread, owns the queue; keeps only the newest job)      │ │
│  │  cache: Arc<RwLock<PoolCache>>  (optional local pool / per-AC top-K cache, ADR-0020 amended)  │ │
│  │  notify: Box<dyn Fn() + Send + Sync>   ← GUI passes `move || ctx.request_repaint()`;           │ │
│  │                                          CLI passes a no-op or a progress-bar tick            │ │
│  └─────────────────────────────┬─────────────────────────────────────────────────────────────────┘ │
│                                │ solver::run(&params, &CancelToken{gen, my_gen}, &mut sink)        │
│                                ▼                                                                    │
│  ┌──────────────── rayon ThreadPool (num_threads = cores − 1, so the UI thread keeps a core) ────┐ │
│  │  Screen: par_chunks over candidates, all 16 ACs per pass ─┐   every chunk (~5–10 ms):         │ │
│  │  Refine: exact PMFs for top-K finalists                   ├──► if token.is_stale() { return } │ │
│  │                                                           │    sink.progress(...)             │ │
│  │                                                           └──► sink.best_so_far(top3, curves) │ │
│  └────────────────────────────────────────────────────────────────────────────┬──────────────────┘ │
│                                                                               │                    │
│                         SolveEvent{gen, Progress | BestSoFar(Arc<Snapshot>) | Done | Error}        │
│                         ─────────────────────── events_tx (crossbeam-channel) ─► UI (above)        │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘

 Crate graph (Cargo workspace):     engine/solver crates (other note) ◄── app-core ◄──┬── app-gui (eframe)
                                     never depend on UI, channels or rayon pool setup  └── app-cli (clap [+ ratatui])
```

Channel choice:
- **`crossbeam-channel`** (0.5.17, 2026-09, [crates.io](https://crates.io/crates/crossbeam-channel)) for worker→UI, because the UI polls with `try_recv` and needs no async runtime.
- **`flume`** (0.12.0, [crates.io](https://crates.io/crates/flume)) is the drop-in choice if the Dioxus fallback is taken, since it offers both sync and `async` receive **[unverified: async API not checked this session]**.
- **No tokio** in the primary design. A tokio runtime only earns its place if the app gains real async I/O, or for the Dioxus fallback (`tokio-util` `CancellationToken`).

Shared state:
- Snapshots are immutable `Arc<SolveSnapshot>` values (top-K per AC, PMFs for the top 3, attribution and rotation script). The UI swaps the pointer and never locks during a frame.
- The only shared mutable state is the generation atomic and the optional pool cache (`RwLock`, written by the coordinator, read by solves).

### Keeping the engine UI-agnostic (shared by CLI and GUI)

- The engine and solver crates expose **pure functions plus two small traits**: a `CancelToken` (`fn is_stale(&self) -> bool`) and a `ProgressSink` (`progress`, `best_so_far`). They never create threads, never depend on egui/Dioxus/channels, and never call `request_repaint`. The caller supplies the rayon pool (`pool.install(...)`) **[design]**.
- `app-core` owns the Session (generation counter, coordinator thread, channel, cache, debouncing policy and the lookup-vs-re-search classification of each parameter) and the **view models**: plain structs such as `AcCurve { build, points: [f64; 16], crossovers }`, `Dotplot { dots: Vec<f64>, mean, p5, p95, sentence }` and `Swimlane { lanes, rows }`. Charts only *draw* view models, so the same view models feed `egui_plot`, an SVG fallback, the data tables, the CLI's text/JSON output and golden files.
- `app-gui` and `app-cli` are thin. The CLI calls the same `Session::solve_blocking(params)` for golden runs. Enforce the boundary with `cargo-deny` bans or a `[workspace.dependencies]` review so `engine`/`solver` can't pull in `egui` **[unverified: tooling choice]**. This replaces ADR-0022's pnpm-based enforcement.

---

## 5. Spikes to run first

| # | Spike | Pass criterion |
|---|---|---|
| S1 | **Chart fidelity in egui_plot.** Build the AC chart (3 series × 16 points, dashed + marker per build, `Text` end labels, `VLine` at the chosen AC, crossover `Points` + annotation), a 50-dot quantile dotplot × 3 builds with mean/P5/P95 ticks, horizontal stacked attribution bars with `stack_on`, and a save-bonus small multiple with `link_cursor` | All of §2.1 items 1–3 drawn legibly in light and dark themes with an Okabe–Ito palette, plus hover readouts and a working legend toggle |
| S2 | **60 fps while a solve streams.** Run a synthetic rayon workload on `cores − 1` threads, emitting `BestSoFar` at 20 Hz into the S1 dashboard; drag the AC slider continuously | p99 UI frame time < 16.7 ms on the user's Mac; no dropped input. Also test `cores` threads to see if starving the UI thread matters |
| S3 | **Cancellation latency.** Drag a re-search slider (e.g. level) back and forth for 5 s | The stale solve stops within one chunk (target ≤ 20 ms); no stale snapshot is ever drawn; CPU returns to idle within 50 ms of release |
| S4 | **Accessibility and text.** VoiceOver over the S1 app with `accesskit`: controls, cards and data tables are announced; charts expose a label and the table equivalent | Every control is reachable by keyboard and named; chart data is reachable via the table |
| S5 | **Edit loop.** Measure incremental `cargo build` time for a one-line change in `app-gui` with `engine`/`solver` untouched; then try `dx serve --hot-patch` with the community egui recipe | Incremental rebuild + relaunch < 5 s, or hot patch < 2 s, on the user's Mac **[target, judgement]** |
| S6 | **Swimlanes and tables.** Render the combat script (Action / Bonus / Reaction / Object / Free lanes, branch rows, ruling badges) with `egui_extras::TableBuilder`, plus copy-as-plain-text | Readable at 1280 px wide; copy text matches the golden script |
| S7 | **Fallback parity (time-boxed to one day).** Re-implement the AC chart and the dotplot as SVG in Dioxus 0.7 desktop, fed by the same `app-core` channel through `flume`; try `dx serve` hot patching | Gives the evidence to choose between egui and Dioxus on layout effort and look |
| S8 | **CLI parity.** `app-cli solve --params x.json --out golden.json` uses the same Session and view models; diff against the GUI's exported snapshot | Byte-identical JSON for the same params |

## Open questions

1. **Does the user care about a native macOS look?** If yes, egui ("not for you", [README](https://github.com/emilk/egui)) loses to Dioxus desktop (WebKit-rendered HTML) or Slint.
2. **HOP animation** ([06 §4](06-market-users-and-ux.md)): egui can animate by requesting repaints; is it in the MVP? Reduced-motion preference would need reading the macOS setting **[unverified: no egui API checked]**.
3. **Is anything ever shared?** If the user later wants to share results, add static export (SVG/PNG via `plotters`, or JSON + a static viewer) rather than a hosted app.
4. **Pool cache.** With a native multi-core solver, is the offline-pool step (ADR-0020) still needed for slider latency, or does a live Screen pass fit the budget? This depends on the engine note's benchmarks.
5. **Version pinning cadence.** egui breaks APIs each release ([README](https://github.com/emilk/egui)); decide whether to upgrade every release or quarterly.
6. **Licence of the repo itself.** Irrelevant for egui and Dioxus (permissive). It matters only if Slint becomes the fallback (GPLv3 or the royalty-free licence, [LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md)).
7. **Resolved by this note:** web-only concerns in research 07 (Comlink, `SharedArrayBuffer`, Cloudflare Pages COOP/COEP, the 07 §3 framework choice of React vs Svelte, Observable Plot) no longer apply.

---

## Candidate ADRs / revisions

### Existing ADRs to supersede or amend (do not edit them here)

| ADR | Action | Why |
|---|---|---|
| **0019 Client-side compute (static site, Web Workers)** | **Supersede** with new ADR A | The product is now a personal native Mac app. "Static site", "Web Workers", "Cloudflare Pages vs GitHub Pages", COOP/COEP and the `SharedArrayBuffer` analysis no longer apply. The *intent* survives and is stronger: all compute is local, offline, and user-imported data never leaves the device (ADR-0003). |
| **0020 Precomputed pools and two-tier evaluation** | **Amend** | Keep the two-tier Screen → Refine design and "AC slider = lookup into per-AC top-K"; they are architecture, not hosting. Change "shipped precomputed pools … build artifacts shipped with the static site" to an **optional local cache** (computed on first run or by `app-cli`, stored under `~/Library/Application Support/…` **[unverified path convention]**), adopted only if the native live Screen misses the slider budget. Replace "worker pool on the device" with "rayon pool". |
| **0021 TypeScript engine with a WASM seam** | **Supersede** (jointly with the engine-core note) | "Full Rust" is now a user decision. Its rejection reasons (type-sharing glue with a TS UI, wasm-pack archive risk, JS↔WASM boundary cost) disappear when both UI and engine are native Rust. |
| **0022 pnpm monorepo** | **Supersede** with new ADR C | Replace with a Cargo workspace. Keep the package boundaries (`dice`, `rules-schema`, `engine`, `solver`, `rules-2024`, `rules-2014`, apps) and the rule that the engine never imports a pack. Enforce it with crate dependency direction instead of pnpm strict linking. `worker` becomes `app-core`; `apps/web` becomes `app-gui`. |
| 0003 User-imported content stays local | Unchanged; now trivially satisfied | No network path exists. |
| 0004 / 0005 Free posture, no WotC trademarks in branding | Unchanged, but lower stakes | Still apply if the app is ever shared. |

### New candidate ADRs

**A. MinMax5e is a native macOS desktop application; all compute runs in-process on local threads**
- *Decision:* ship a single native Rust binary for the user's Mac. There is no hosted site, server or WebView-hosted compute. The solver runs on an in-process rayon pool.
- *Rationale:* personal use; native threads remove the WASM threading constraints (nightly + `build-std` + cross-origin isolation, [wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon)); fully offline; keeps ADR-0003.
- *Rejected:* static site + Web Workers (ADR-0019: web-only constraints, no benefit for one user); localhost web app (extra server, browser tab, no benefit); Tauri + WASM UI (two targets and an IPC JSON boundary, [Tauri calling Rust](https://v2.tauri.app/develop/calling-rust/)).

**B. The GUI is egui/eframe, with charts in egui_plot; Dioxus desktop is the named fallback**
- *Decision:* `eframe` + `egui_plot` + `egui_extras` for tables and swimlanes, with AccessKit enabled. Pin minor versions.
- *Rationale:* the most mature pure-Rust GUI with the most active releases ([egui releases](https://github.com/emilk/egui/releases)); the only pure-Rust interactive charting that covers line + markers + annotations, stacked horizontal bars, dotplots, hover and legend toggling ([egui_plot](https://docs.rs/egui_plot/latest/egui_plot/)); immediate mode suits streaming snapshots.
- *Rejected:* iced (no accessibility, [#552](https://github.com/iced-rs/iced/issues/552); sparse releases; weak charting); Slint (no charts; kept as second fallback); Xilem, Makepad, GPUI (experimental or pre-1.0, thin docs); Dioxus native/Blitz (WIP); Tauri + Leptos/Yew (see A); JS charting (Observable Plot/ECharts) or `charming` (JS at runtime breaks "full Rust", [charming](https://github.com/yuankunzhang/charming)); `plotters` (static only, no release since 2024-09).
- *Revisit if:* spikes S1/S4/S5 fail (switch to Dioxus 0.7 desktop + SVG), or GPUI reaches 1.0 with documentation.

**C. Cargo workspace with UI-agnostic core crates; GUI and CLI are thin shells over `app-core`**
- *Decision:* crates `engine`/`solver`/packs (per the engine note) ← `app-core` (Session, cancellation, view models) ← `app-gui`, `app-cli`. Engine crates never depend on UI crates, channels or a specific thread pool.
- *Rationale:* golden and batch runs share the exact code path with the GUI; view models are testable without a window; swapping GUI frameworks (B's fallback) touches only `app-gui`.
- *Rejected:* a single crate (slow incremental builds, and UI types leaking into the engine); separate repos (drift, as in ADR-0022).

**D. Solves are cancelled cooperatively by a generation token and stream immutable snapshots over a channel**
- *Decision:* each re-search increments an `AtomicU64`; solver chunks of about 5–10 ms check `is_stale()`; workers send `SolveEvent{gen, …}` with `Arc` snapshots over `crossbeam-channel` and wake the UI via a notify callback; the UI drops events from stale generations. Parameters are classified as *lookup* (AC) or *re-search*; only re-search parameters dispatch, with about 100 ms debounce.
- *Rationale:* rayon has no preemptive cancellation **[unverified]**; this is deterministic, lock-free on the UI path, and identical for GUI, CLI and the Dioxus fallback. It carries research 07's generation-token idea over without Web Workers.
- *Rejected:* killing threads (unsafe in Rust; no API); a tokio runtime as the core (no async I/O to justify it); shared mutable result state behind a mutex (UI-thread contention).

**E. A CLI ships from day one; a TUI is optional**
- *Decision:* `app-cli` (clap) for solve, golden, bench and pack validation; a ratatui view is post-MVP and optional ([ratatui](https://ratatui.rs/)).
- *Rationale:* golden tests and benchmarks (research 07's CI jobs) need a headless entry point; a TUI adds little for a GUI user.
- *Rejected:* GUI-only (no headless tests); TUI as the primary UI (cannot draw the dotplots or swimlanes legibly).

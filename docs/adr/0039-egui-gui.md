---
status: proposed
---

# The GUI is egui/eframe with egui_plot charts; Dioxus desktop is the fallback

> *Amended 2026-11-27:*
>
> - **One egui UI for native and web** (ADR-0043). eframe 0.36 fixed mobile keyboards, autocorrect, and IME on the web, and 0.37 added kinetic scrolling and browser glyph rasterisation.
> - **Web accessibility:** on the web, AccessKit isn't available. eframe offers only a speech-synthesis screen reader. The data-table view for every chart is therefore required, not optional.
> - **Renderer:** native uses `wgpu`. The web uses `glow` (WebGL2) unless spike W1 shows `wgpu` costs little extra size. eframe supports both on web through `WebOptions::renderer`.
> - **Spike S4 (VoiceOver) now also covers the web screen reader.**

`mm-gui` uses `eframe` 0.37, `egui_plot` 0.38, and `egui_extras` for tables and the action-economy swimlanes, with AccessKit enabled and minor versions pinned. egui is the most actively released pure-Rust GUI (MIT/Apache-2.0). egui_plot is the only interactive pure-Rust charting that covers every dashboard chart in research 06:

- the AC sensitivity lines with markers, crossover annotations, and a rule at the chosen AC;
- horizontal stacked attribution bars;
- quantile dotplots;
- hover readouts and legend entries that toggle series.

Its immediate-mode model suits a UI that redraws from streamed solver snapshots. Plots aren't exposed to screen readers, so every chart has a data-table view.

**Revisit if** spikes S1 (chart fidelity), S4 (VoiceOver), or S5 (edit-loop build time) fail. The fallback is Dioxus 0.7 desktop with hand-drawn SVG charts: native Rust behind the system WebView, with HTML/CSS layout and hot patching. Slint is the second fallback, but it has no charting library.

## Considered Options

- iced. Rejected: no accessibility support, no release since December 2025, and weak charting.
- Xilem, Makepad, GPUI, Dioxus native/Blitz. Rejected: experimental or pre-1.0.
- Tauri with Leptos or Yew. Rejected (ADR-0032).
- JS charting via `charming` (ECharts) or Observable Plot. Rejected: JavaScript at runtime breaks "full Rust".
- plotters. Rejected: static output only, and no release since September 2024.

Research: [10](../research/10-rust-ui-and-app-shell.md)

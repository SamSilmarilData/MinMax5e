# 11: Web-first delivery: open a link, get the egui app

*Researched 2026-11-27. Engineering research, not legal advice. Every claim is listed in the **verification log** (§8) with its status:*

- ***verified:** I measured it, or read it at the primary source, today;*
- ***reported:** secondary or dated source;*
- ***estimate:** my own number;*
- ***design:** a recommendation.*

*No Rust toolchain is installed. An attempt to install one in a scratch directory to measure a real build was blocked by the session's permissions. Build sizes and speeds are therefore inferred from published artifacts and wait on the spikes in §7.*

**Decisions taken from this note:**

| Topic | Decision | ADR |
|---|---|---|
| Delivery | Same egui app built for the web | 0043 |
| Hosting | Cloudflare Workers at a free `workers.dev` URL | 0044 |
| Gate | Per-friend invite links | 0044 |
| Solver in the browser | Web Worker pool on stable Rust | 0045 |
| Amended to match | Data fetch, storage, cancellation, determinism, scope | 0020, 0026, 0028, 0029, 0032–0034, 0037–0041 |

## 1. Goal

**Ease of access.** A friend goes from "I was sent a link" to "I'm using it" with no install and no sign-up, on whatever device they have. Once in, it should be as smooth as the native egui app.

## 2. UI: egui on the web, with one codebase

The first draft of this note rejected egui for the web, citing 2024 mobile-input bugs. The changelogs show those bugs were fixed:

- **eframe 0.36.0 (2026-08-05):** autocomplete, autocorrect, and IME work on iOS and Android; the text agent is anchored to the canvas; focus no longer scrolls the page [verified].
- **0.37.0 (2026-10-15):** kinetic scrolling; browser glyph rasterisation for system fonts and emoji [verified].
- **egui_plot 0.38:** targets egui 0.37 [verified].

So the "egui setup" becomes the web app too (ADR-0043).

**Accepted limits:**

- no find-in-page or translate on the canvas;
- web accessibility is a speech-synthesis reader, because AccessKit is native-only [verified, eframe `Cargo.toml` and `docs/accessibility.md`];
- a larger first download than a DOM app.

**Size baseline [verified, measured]:**

- `eframe_template` (eframe 0.36, default `wgpu`, `opt-level = 2`, no LTO): 7.78 MB raw, 3.12 MB gzip −9, 2.27 MB xz −9.
- The full egui demo: 12.0 MB raw, 4.55 MB gzip.
- egui's bundled fonts: 1.4 MB of TTF.
- The template's `.wasm` contains `wgpu` and `naga`, so `glow` plus `opt-level = "z"`, LTO, and `wasm-opt` should shrink it materially [estimate].
- The engine ships as a separate worker module, so the UI module carries no solver code.

## 3. Getting in: invite links on Cloudflare Workers

**Host.** Cloudflare's Pages docs open with "Start new projects with Workers" [verified], so the site is Workers static assets at `minmax5e.<account-subdomain>.workers.dev`.

- **Free plan:** static asset requests are free and unlimited, and Worker invocations get 100,000 a day [verified].
- **Separate site:** `workers.dev` is on the Public Suffix List, so the hostname is its own site for cookies and storage [verified].

**Gate.** `assets.run_worker_first = ["/", "/index.html", "/join/*"]` runs the gate Worker only on entry points [verified, the option takes route arrays]. `/join/<token>` sets an HMAC-signed `HttpOnly; Secure; SameSite=Lax` cookie that lasts 400 days and is re-issued on each visit. Chrome caps cookie lifetime at 400 days [verified].

**Why the invite cookie survives in Safari:**

- ITP's 7-day cap targets *script-writable* storage [verified, WebKit 2020].
- Cookies are outside WebKit's storage-eviction policy [verified, WebKit 2023].
- WebKit recommends server-set `HttpOnly` cookies for durable sessions [verified].
- Spike W4 still tests 8+ days without a visit on iOS.

**Details that would bite:**

- **The manifest must not be gated.** Browsers fetch manifests without cookies unless `crossorigin="use-credentials"` is set [verified, MDN].
- **Headers.** Custom `_headers` rules don't apply to Worker-generated responses [verified], so assets are served directly, with immutable caching.
- **Fail closed.** If the free allowance runs out, fail-open would bypass the gate, so the route is set to fail closed (1027 error) [verified, Cloudflare docs].
- **Compression.** Cloudflare's default compression list includes `application/wasm`; the free plan uses Zstandard, then Brotli or Gzip [verified, Cloudflare docs].
- **Service-worker cache.** It must not store the gate page as `/`. Only cache 200 responses from the app's own assets.

**Rejected:**

- public and ungated;
- Cloudflare Access with email one-time PIN: a code on every device, and a session of one month at most [verified];
- a shared password;
- GitHub Pages: no gate and no headers.

## 4. Data: small, fast, and rebuildable

**jsDelivr pinned by commit SHA** [verified]:

- `x-jsd-version-type: commit`;
- `access-control-allow-origin: *`;
- brotli encoding;
- `immutable`, one-year caching.

**Measured size.** The 23 MVP files are 9.7 MB raw and **1.26 MB brotli** [verified]. For example, `items.json` is 3.0 MB raw and 367 KB brotli.

**jsDelivr limits and terms** [verified, jsDelivr README and terms]: single GitHub files up to 20 MB; no use as a general-purpose file host, and no scraping.

**Raw GitHub is the fallback, not the default:**

- GitHub tightened unauthenticated limits on raw downloads in May 2025 but published no number [verified, changelog].
- GitHub's REST docs give 60 requests per hour for unauthenticated API calls [verified].
- About 60 per hour for raw downloads is a community report only [reported].
- `codeload` tarballs can't be used: CORS allows only `render.githubusercontent.com` [verified].

**Storage and eviction** [verified, WebKit and MDN]:

- Packs live in OPFS: `getDirectory` and `FileSystemSyncAccessHandle`, in Safari 15.2+, Chrome 102+, and Firefox 111+.
- WebKit evicts whole origins under pressure, or after 7 days of Safari use without interaction. This covers Cache API and service-worker data too.
- Home Screen web apps have their own day counter and aren't expected to lose data.
- `persist()` is granted by heuristics, such as Home Screen use.
- **Design:** packs are a rebuildable cache, re-fetched silently when missing.

## 5. Compute: a Web Worker pool on stable Rust

- **Two modules.** Trunk 0.22 builds a UI module plus a worker module (`data-type="worker"`), with `wasm-opt` built in [verified, Trunk examples].
- **One compile.** The worker module is compiled once and posted as a `WebAssembly.Module` to every worker. WebKit shares the same machine code across workers [verified, WebKit blog].
- **Worker count.** WebKit reports `hardwareConcurrency` as **4** on devices with fewer than 8 cores and **8** otherwise [verified, WebKit `main` source]. This corrects the "2 on iOS" figure in the first draft, which came from older source.
- **SIMD.** Works in Safari 16.4+, Chrome 91+, and Firefox 89+, and the Rust intrinsics have been stable since 1.54 [verified].
- **Per-core speed.** WASM runs at about 1.45–1.55× native on SPEC CPU [reported, 2019 paper].
- **Why not WASM threads.** `wasm-bindgen-rayon` 1.3 needs nightly and `build-std`, which is still unstable in 2026 [verified, Rust project goals]. It also needs COOP and COEP `require-corp`, and Safari lacks `credentialless` [reported, caniuse]. It stays an upgrade path.

## 6. Experience budget [estimate, pending W1–W4]

| Moment | Laptop, good Wi-Fi | Phone, 4G |
|---|---|---|
| Tap invite link → interactive (first ever) | ≈2–4 s | ≈5–10 s |
| Return visit | under 1 s | ≈1–2 s |
| Return after Safari eviction | ≈2 s (silent re-fetch of 1.3 MB plus import) | ≈3–5 s |
| Slider move → updated charts | 1 frame | 1–2 frames |
| Snapshot search, 1 class | ≈0.2–1.5 s | ≈1–5 s |

## 7. Spikes (each a day or less, before building on ADR-0043–0045)

- **W1, size:** UI module with `glow` vs `wgpu`, `opt-level` `s` vs `z`, LTO, `wasm-opt`, and fonts dropped where possible. Confirm on the wire that Cloudflare compresses `.wasm`.
- **W2, workers:** Screen-kernel throughput with 1, 4, and 8 workers versus native rayon on the M5. Slider round-trip time. Whether more workers than `hardwareConcurrency` reports helps on iOS. Shared-Module posting on Safari, macOS and iOS.
- **W3, boot:** cold and warm time-to-interactive with the parallel prefetch and the OPFS import; again after clearing site data; again as a Home Screen app.
- **W4, gate:** invite link on iOS Safari (tab and Home Screen), Android Chrome, and desktop browsers. Does the cookie survive 8+ days without a visit? Do previews render in iMessage, Discord, and WhatsApp? Does fail-closed behave as described?

## 8. Verification log

| Claim | Status | Source |
|---|---|---|
| eframe 0.36 mobile keyboard and IME fixed on web | verified | [egui CHANGELOG](https://github.com/emilk/egui/blob/main/CHANGELOG.md) |
| eframe/egui 0.37.0 (2026-10-15), egui_plot 0.38.0 (2026-10-19), Trunk 0.22.2, gloo-worker 0.7.0, wasm-bindgen 0.2.130, Rust 1.100.0 | verified | crates.io API, `static.rust-lang.org` channel file |
| Web screen reader is speech-synthesis; AccessKit is native-only | verified | [eframe Cargo.toml](https://github.com/emilk/egui/blob/main/crates/eframe/Cargo.toml), [accessibility.md](https://github.com/emilk/egui/blob/main/docs/accessibility.md) |
| Template and demo `.wasm` sizes; font sizes | verified (measured) | `emilk.github.io/eframe_template`, `egui.rs`, GitHub API |
| Cloudflare recommends Workers over Pages for new projects | verified | [Pages docs](https://developers.cloudflare.com/pages/) |
| Static assets free; 100k invocations/day; fail open or closed | verified | [Pages pricing](https://developers.cloudflare.com/pages/functions/pricing/), [Workers limits](https://developers.cloudflare.com/workers/platform/limits/) |
| `run_worker_first` takes route arrays; `_headers` not applied to Worker responses | verified | [Worker script routing](https://developers.cloudflare.com/workers/static-assets/routing/worker-script/), [Headers](https://developers.cloudflare.com/workers/static-assets/headers/) |
| `application/wasm` in the default compression list; free plan uses Zstandard | verified (docs); wire check in W1 | [Compression rules](https://developers.cloudflare.com/rules/compression-rules/) |
| `workers.dev` and `pages.dev` on the Public Suffix List | verified | [publicsuffix.org list](https://publicsuffix.org/list/public_suffix_list.dat) |
| Chrome caps cookies at 400 days | verified | [Chrome blog](https://developer.chrome.com/blog/cookie-max-age-expires) |
| ITP 7-day cap on script-writable storage; Home Screen apps exempt | verified | [WebKit 2020](https://webkit.org/blog/10218/full-third-party-cookie-blocking-and-more/) |
| Eviction by origin, includes Cache API and service workers; cookies outside the policy; `persist()` by heuristics | verified | [WebKit 2023](https://webkit.org/blog/14403/updates-to-storage-policy/) |
| CNAME- or IP-cloaked server cookies capped at 7 days | verified (CNAME); IP rule reported | [WebKit](https://webkit.org/blog/11338/cname-cloaking-and-web-privacy/) |
| Manifest is fetched without credentials | verified | [MDN Manifest](https://developer.mozilla.org/en-US/docs/Web/Manifest) |
| jsDelivr: CORS, brotli, immutable, commit-SHA pinning, 20 MB file limit, terms | verified | `curl -I`; [jsDelivr README](https://github.com/jsdelivr/jsdelivr); [Terms](https://www.jsdelivr.com/terms) |
| 23 MVP files: 9.7 MB raw, 1.26 MB brotli | verified (measured) | `curl` against jsDelivr @ v2.41.3 |
| GitHub raw limits tightened (no number published); 60/hr is the documented unauthenticated REST limit | verified | [GitHub changelog](https://github.blog/changelog/2025-05-08-updated-rate-limits-for-unauthenticated-requests/), [REST rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) |
| codeload CORS is restricted | verified | `curl -I` |
| OPFS sync handle Safari 15.2+; `instantiateStreaming` Safari 15+; SIMD Safari 16.4+ | verified | [MDN browser-compat-data](https://github.com/mdn/browser-compat-data) |
| `WebAssembly.Module` shareable across workers without recompiling | verified | [WebKit blog](https://webkit.org/blog/7691/webassembly/), [MDN](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Module) |
| WebKit `hardwareConcurrency` reports 4 or 8 | verified | [WebKit NavigatorBase.cpp](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/page/NavigatorBase.cpp) |
| Rust WASM SIMD intrinsics stable since 1.54 | verified | [core::arch::wasm32](https://doc.rust-lang.org/core/arch/wasm32/) |
| Trunk builds worker binaries; `wasm-opt` built in | verified | [Trunk examples](https://github.com/trunk-rs/trunk/tree/main/examples) |
| `wasm-bindgen-rayon` needs nightly and `build-std`; `build-std` still unstable | verified | [README](https://github.com/RReverser/wasm-bindgen-rayon), [Rust goals 2026](https://rust-lang.github.io/rust-project-goals/2026/build-std.html) |
| Safari lacks COEP `credentialless` | reported | [caniuse](https://caniuse.com/mdn-http_headers_cross-origin-embedder-policy_credentialless) |
| WASM ≈1.45–1.55× native (SPEC) | reported (2019) | [Jangda et al.](https://www.usenix.org/conference/atc19/presentation/jangda) |
| Cloudflare Access OTP free ≤50 users; sessions up to 1 month | verified | [OTP](https://developers.cloudflare.com/cloudflare-one/identity/one-time-pin/), [sessions](https://developers.cloudflare.com/cloudflare-one/identity/users/session-management) |
| `wrangler-action` v4.1.3 (2026-09-24); `worker` (workers-rs) 0.9.0 | verified | GitHub API, crates.io |
| All timings in §6 | estimate | spikes W1–W4 |

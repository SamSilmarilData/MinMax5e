# 12: Can anyone, on any device, open it, with no legal issues?

*Researched 2026-11-27. Engineering research, **not legal advice**. Labels as in note 11: **[verified]**, **[reported]**, **[estimate]**, **[design]**. This note tests three claims: "anyone can open it", "on whatever device, with no issues", and "no legal issues". It builds on notes 01, 06, 08, and 11.*

## Verdict

| Claim | Holds? | Condition |
|---|---|---|
| Stunning, smooth, fast | **Smooth and fast: yes. Stunning: achievable with design work, within egui's limits** | §3 |
| Any device (Windows, Linux, macOS, Android, iPhone) | **Yes**, for any browser from about 2022 on | Two fallbacks: a non-SIMD module, and handling for in-app browsers (§2) |
| Anyone can open it **and** no legal issues | **Not with the current data design.** Yes with a split | Public visitors get an **Open edition** built only from material we may publish. The 5e.tools corpus stays behind the invite gate or a bring-your-own import (§4) |

"No legal issues" can't be promised for anything that uses a publisher's game. What can be achieved is **nothing to take down and nothing infringing hosted, linked, or fetched by the public site**, the same position as long-running public fan tools.

## 1. Why "anyone" changes the legal picture

The current design (ADR-0026, 0041, 0044) is safe for **private** use because the site hosts no corpus, and each invited friend's browser fetches its own copy. Opening the site to anyone changes three things:

1. **The app tells every visitor's browser to download the corpus.** In the August 2024 DMCA notice, WotC calls the 5etools sites ones that "consist almost exclusively of content copied verbatim from Wizards' D&D publications" [verified, 08 §2]. Under US law, secondary liability covers whoever "intentionally induc[es] or encourag[es] direct infringement" (*MGM v. Grokster*, 2005) and material contribution (*Perfect 10 v. Visa*, 9th Cir. 2007) [reported via the CRS and court-filing summaries]. A public tool that automatically makes each visitor copy that corpus is a much better target for those theories than a private tool or a neutral importer.
2. **It concentrates traffic on the community mirror.** `5etools-mirror-1` and `-2` were taken down in 2024. A popular public app pulling from `5etools-mirror-3` through jsDelivr draws attention to it, which could get the mirror (and our data source) removed. jsDelivr's terms also bar acting as a general file host [verified, 11 §4].
3. **The Fan Content Policy expects free and ungated.** "You can't require payments, surveys, downloads, subscriptions, or email registration" (01 §2.4) [verified earlier]. A public site has to be open to qualify, so invites can't be the public model.

**No new enforcement found.** A listing of `github/dmca` for 2025-01 to 2026-11 shows no notice filed under Wizards of the Coast, D&D Beyond, or Hasbro names. The 2025/2026 "wizard" hits are unrelated (`blue-wizard`, `gto-wizard`) [verified, filename listing only; notices filed under counsel's name would not show].

## 2. "Any device, no issues"

**Browser floor [verified, MDN compat data]:**

| Feature | Safari / iOS | Chrome | Firefox |
|---|---|---|---|
| WebAssembly streaming compile | 15 | 60 | 58 |
| OPFS sync access (packs, cache) | 15.2 | 102 | 111 |
| WASM SIMD | 16.4 | 91 | 89 |

**[design]** Ship two worker modules, SIMD and baseline, chosen at load time by `wasm-feature-detect`. WASM has no in-module feature detection, so the browser needs separate builds ([web.dev](https://web.dev/articles/webassembly-feature-detection)). The floor is then **iOS 15.2 / Safari 15.2**. That's every iPhone that runs iOS 15, back to the iPhone 6s.

**Reach on iPhone [verified, Apple, devices transacting on 2026-06-07]:**

- iOS 26 runs on 79% of all iPhones and 86% of those from the last four years ([Apple](https://developer.apple.com/support/app-store/)).
- Most of the rest run iOS 18, which is far above the floor.

**Windows, Linux, macOS, and Android** run current Chrome, Edge, or Firefox, all far above the floor.

**What could still go wrong, with mitigations:**

| Issue | Finding | Mitigation [design] |
|---|---|---|
| **In-app browsers** (a link tapped inside Discord, Instagram, or WhatsApp) | On iOS, `SFSafariViewController` stopped sharing cookies and website data with Safari in iOS 11. Apps that use `WKWebView` have their own isolated cookie jar [reported, Apple forums and Okta]. On Android, Custom Tabs share Chrome's cookies and state; WebView does not [verified, Chromium Custom Tabs FAQ] | **Invite gate:** the gate page explains "open in Safari/Chrome" and also accepts a short **invite code** typed by hand, so an invite isn't lost to an in-app browser. **Open edition:** works anywhere; only cached data is lost when the in-app browser is closed |
| **Memory on iOS** | WebKit bug 269777 ("Only ~300 MB of RAM is reliably available to WebAssembly pages", Safari 17) was closed as a duplicate of 269937, which is resolved with a fix landed. The release that carries it isn't stated [verified, WebKit Bugzilla] | Budget ≤ 128 MB per worker and ≤ about 512 MB in total; fewer workers on phones; memo tables capped |
| **Phones heating up on long solves** | Not measured | Heavy solves are background jobs with a stop button, plus share links so a laptop's result opens instantly on a phone (ADR-0020) |
| **Storage eviction** | Safari's 7-day rule (11 §4) | Packs and cache rebuild silently (ADR-0038, 0041) |
| **Screen readers** | Web egui has only a speech-synthesis reader (11 §2) | Data-table views; full AccessKit in the native build |
| **Older or low-end GPUs and drivers** | egui needs WebGL2 (Safari 15+), the same floor as above | `glow` renderer on the web (ADR-0039) |

## 3. "Stunning"

**egui's strengths:** smooth immediate-mode redraws, crisp text on high-DPI screens, theming of fills, corner radius, strokes, and shadows, custom fonts, simple tweened animation, `egui_plot` charts, and free-form painting (meshes, gradients drawn by hand).

**What the ecosystem shows:**

- **egui 0.37** added an experimental `StyleProvider` theme-plugin system for per-widget, class-based styling [verified, changelog].
- **Third-party theme kits** aim squarely at polish: `egui-elegance`, whose stated goal is UIs as polished as modern web apps, `egui_styled` with Tailwind-like hover and focus states and shadow and glow tokens, and `egui-thematic`, an editor for 73 visual properties [reported, docs.rs].

**Limits:** no CSS-grade effects such as backdrop blur, rich text layout, or complex transitions. Layout and animation are code, not stylesheets.

**[design]** "Stunning" here means a **deliberate design system**:

- one dark and one light palette;
- a display font and a UI font, both OFL-licensed and bundled;
- an 8-pt spacing scale;
- card components;
- a branded chart theme for `egui_plot`;
- short (≤ 200 ms) eased transitions on panels and values.

That's a few days of design work, not a framework change. If marketing-site visuals ever matter more than one shared codebase, a DOM UI (Leptos) is the alternative, and note 11 explains why it lost.

## 4. The split that makes "anyone, no legal issues" work

**Open edition (public, anyone, no gate):** contains only material we may publish.

1. **SRD 5.2.1** under CC-BY-4.0, with the attribution already in the research README. It covers all 12 classes (one subclass each), 17 feats, the weapon table with masteries, and SRD spells (01 §1).
2. **Our own encodings of non-SRD options** (e.g. Great Weapon Master, the other three subclasses per MVP class): **name plus mechanics in our own words, with the numbers authored by us**. The legal basis:
   - US copyright excludes "any idea, procedure, process, system, method of operation" (17 U.S.C. §102(b)).
   - The Copyright Office says copyright doesn't protect a game's idea, its name or title, or "the method or methods for playing it" (FL-108, archived) [reported].
   - Names and short phrases are uncopyrightable (Circular 33) [verified, copyright.gov search excerpt].
   - **Precedent:** public tools have rated and calculated 2024 options by name for years: RPGBOT's guides and DPR calculator, Esker's tool with PAM/GWM/CBE/SS, and spreadsheet calculators (06 §1).
3. **No corpus fetch, no mirror named, no `{@` text.** Rules text shown in the UI is SRD (attributed) or our own one-line summaries.
4. **Branding** follows the old ADR-0005: no D&D marks. Use "compatible with fifth edition" wording and the Fan Content Policy notice (06).

**Cost:** more hand-authoring. Without the corpus, the class tables come from the SRD, which has all 12. Non-SRD subclass and feat numbers we write ourselves. That's 18 subclasses and about 8 feats for the 6-class MVP [estimate]. ADR-0035's RON encodings already hold these mechanics; they gain explicit numbers instead of upstream keys.

**Extended edition, for invited friends (ADR-0044, unchanged):** the full 5e.tools corpus, fetched by each friend's browser. It's the same site and the same app: a valid invite cookie unlocks the corpus fetch and the extra content.

**Bring your own data (optional, any visitor):** "Load rules data from a file…" accepts a corpus file the user already has. The public app doesn't name, link to, or fetch any source. A neutral import feature with substantial non-infringing uses is the *Sony Betamax* position that *Grokster* preserved for products without inducement [reported]. This is weaker than not offering it. Leave it out of the public UI until the open edition proves itself.

**Repo and hygiene:** ADR-0028 is unchanged. Open-edition encodings are our own words and SRD material, so they belong in the public repo, and the `dist/` check (ADR-0044) still forbids any upstream bytes.

## 5. Recommendation

Do the split. One URL serves both:

- **Anyone:** the Open edition, SRD plus our own encodings. No gate, nothing to take down, nothing fetched from the mirror.
- **Friends:** tap the invite link once, and the Extended edition switches on.

Before publishing the URL, have someone qualified look over the Open edition's content list. It's a one-time review of a short list of names and mechanics, far cheaper than a dispute.

## Open questions

1. Make the URL public with the Open edition, or keep the whole site invite-only? This decides whether ADR-0026's "becoming public" clause is triggered and needs a superseding ADR.
2. Open edition content scope for the MVP: which non-SRD subclasses and feats to hand-author first?
3. Offer bring-your-own-data to public visitors, or only to invited friends?

## Sources

- **Apple:** [iOS usage](https://developer.apple.com/support/app-store/)
- **WebKit:** [bug 269777](https://bugs.webkit.org/show_bug.cgi?id=269777), [bug 269937](https://bugs.webkit.org/show_bug.cgi?id=269937)
- **Chromium:** [Custom Tabs FAQ](https://chromium.googlesource.com/chromium/src/+/main/docs/security/custom-tabs-faq.md)
- **Apple developer forums:** [SFSafariViewController cookies](https://developer.apple.com/forums/thread/78928)
- **WebAssembly feature detection:** [web.dev](https://web.dev/articles/webassembly-feature-detection)
- **Browser support:** [MDN browser-compat-data](https://github.com/mdn/browser-compat-data)
- **Takedown history:** [github/dmca](https://github.com/github/dmca)
- **US Copyright Office:** [Circular 33](https://www.copyright.gov/circs/circ33.pdf), [FL-108, archived](https://webharvest.gov/peth04/20041015023438/http://www.copyright.gov/fls/fl108.html)
- **Secondary liability:** [CRS report on Grokster](https://stuff.mit.edu/afs/sipb.mit.edu/contrib/wikileaks-crs/wikileaks-crs-reports/RL31998.pdf)
- **egui:** [CHANGELOG](https://github.com/emilk/egui/blob/main/CHANGELOG.md)
- **Theme kits:** [egui-elegance](https://docs.rs/crate/egui-elegance/0.7.2), [egui_styled](https://docs.rs/crate/egui_styled/0.8.0), [egui-thematic](https://docs.rs/crate/egui-thematic/latest)

---
status: proposed
---

# MinMax5e is a personal tool shared privately with a few friends, not a product

> *Amended 2026-11-27 (web delivery, ADR-0043/0044):* "sharing with friends" now means sending a friend a **personal invite link** to the web build at a `workers.dev` URL. The site is gated: no valid invite cookie, no app. It is `noindex`, and it serves only our code and encodings. Each friend's browser downloads its own corpus copy from the public mirror (ADR-0041), so the sharing rule below holds unchanged. A gated URL known only to invited friends counts as private sharing. Removing the gate, promoting the URL, or accepting strangers would count as becoming public (last paragraph).

MinMax5e is built mainly for the author's own use, and may be shared privately with a few friends. There is no public website, no hosting, no accounts, and no sales.

**Sharing rule: share the app, never the data.** Friends receive the app (a binary with no data in it, or the source) and the author's own encodings. Each install downloads its own copy of the 5e.tools corpus from the public mirror (ADR-0041). Nobody passes corpus files, compiled packs, or caches to anyone else. Build results (choices, numbers, rotations, with no rules text) may be shared freely, like a character sheet. Several earlier decisions protected a public product, and this one removes their premise:

- ADR-0004 (free posture under the Fan Content Policy) is **deprecated**.
- ADR-0005 (no trademarks in branding) is **deprecated**.
- ADR-0001 (no 5e.tools) is **superseded** by ADR-0027.
- ADR-0003 (user imports stay local) is **superseded** by ADR-0028.
- ADR-0019 (static site) is **superseded** by ADR-0032.

One public-facing surface remains: the GitHub repository `SamSilmarilData/MinMax5e`, which is **public**. Its constraints are in ADR-0028.

*Amended 2026-11-27:* the original scope was the author alone. Private sharing with a few friends fits as long as the sharing rule above holds.

A browser build is kept possible (ADR-0042). Putting it on a public URL, even GitHub Pages hosting no data, counts as "becoming public" below. An unlisted URL shared privately is the low-profile option.

This is hard to reverse. If the project ever becomes public (a website, a publicly promoted download, sales) or the sharing rule is broken (packs or corpus data handed to others), the original reasoning in ADR-0001 and ADR-0004 returns in full, and non-SRD content would need a legal review first ([research 01](../research/01-data-source-and-licensing.md)).

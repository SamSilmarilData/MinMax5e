---
status: superseded by ADR-0032
---

# All solving runs in the browser; there is no server compute in the MVP

> **Superseded 2026-11-27 by [ADR-0032](0032-native-macos-app.md). No website. The intent (all compute local and offline) survives.**

MinMax5e ships as a static site. All solving and evaluation run in a pool of Web Workers on the user's device. This keeps slider interactions off the network, costs almost nothing to host, works offline, and keeps user-imported rules data on the device (ADR-0003).

It is hard to reverse because it shapes how data is distributed, our licensing posture, and how in-flight solves are cancelled. A static host that can set cross-origin isolation headers (e.g. Cloudflare Pages, but not GitHub Pages) keeps `SharedArrayBuffer` available for later.

## Considered Options

- A server API. Rejected: cost and latency, and we would be hosting rules data.
- A server-side precomputed answer table as the main path. Rejected: user imports invalidate it. Shipped precomputed *pools* are covered in ADR-0020.

Research: [07](../research/07-tech-stack-and-architecture.md)

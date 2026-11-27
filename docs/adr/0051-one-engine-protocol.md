---
status: proposed
---

# The UI talks to the engine only through one message protocol, on both targets

`mm-app-core::proto` defines the whole UI ↔ engine contract as two enums. `ToEngine` carries `Submit`, `Rescore`, `Import` and `Cancel`. `FromEngine` carries `Progress`, `BestSoFar`, `Done` (with an `Optimality` certificate: proven optimal, or best found with gap ≤ x%), `ImportProgress` and `Error`. Every message carries the generation `gen` (ADR-0040).

- **Native:** `crossbeam-channel` carrying the enums directly, with no serialisation, plus `ctx.request_repaint()`.
- **Web:** postcard bytes over `postMessage` to the coordinator worker (ADR-0045).

The native app uses messages even though it could call the engine directly. That way `mm-gui` view code has no `cfg(target_arch)`, the two targets cannot drift apart, and a recorded message script can be replayed against both transports in CI. The CLI uses the same Session over the same protocol, so golden runs follow the GUI's code path (ADR-0034).

## Considered Options

- **Direct function calls natively, messages only on the web.** Rejected: two code paths, and bugs specific to the web would only show up on the web.
- **A JSON protocol.** Rejected: larger and slower than postcard, and postcard is already the cache format (ADR-0038).
- **Shared memory** (`SharedArrayBuffer`). Rejected for now (ADR-0045).

Research: [10](../research/10-rust-ui-and-app-shell.md), [11](../research/11-web-first-delivery.md)

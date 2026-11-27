---
status: proposed
---

# MinMax5e stays friends-only, with the full corpus; a public "Open edition" is deferred

The owner decided (2026-11-27) to keep the hosted app **invite-only** (ADR-0044) and to give invited friends the **full 5e.tools corpus** (ADR-0027, ADR-0041). There is no public edition for now.

Research 12 found that "anyone can open it" and "no legal issues" can only coexist through a split:

- a public **Open edition**: SRD 5.2.1 under CC-BY, plus our own-words mechanics with numbers we author, and no corpus fetch;
- an invite-only **Extended edition**.

Opening the current design to everyone would make the public app instruct every visitor's browser to copy a corpus WotC has called "copied verbatim". It would also concentrate traffic on the last community mirror. The owner chose to defer that work, not to rule it out.

**What this keeps true now:**

- The site stays gated, `noindex`, and unpromoted. ADR-0026's "becoming public" clause is not triggered.
- The engine, encodings, and UI are written so that an Open edition can be added later without a rewrite:
  - **Explicit provenance:** every encoding records whether it is SRD, Corpus-backed, or project-authored (the GLOSSARY's *Provenance*). The app can then filter content by edition.
  - **Corpus as a capability:** content loading goes through a content source chosen at startup (`Corpus` or, later, `OpenEdition`), so gating the corpus is one switch, not a refactor.

**Revisit when** the owner wants the app reachable without an invite, or wants it promoted in public. At that point:

1. write a superseding ADR for ADR-0026;
2. build the Open edition content set (research 12 §4: about 18 subclasses and 8 feats to hand-author for the 6-class MVP [estimate]);
3. adopt Fan Content Policy branding and notices;
4. get a one-off review of the Open edition's content list by someone qualified.

## Considered Options

- **Go public now with the Open edition.** Deferred: extra hand-authoring before the core tool exists, for users the owner isn't targeting yet.
- **Go public with the full corpus fetch.** Rejected: the inducement and mirror-exposure concerns in research 12 §1.
- **Public bring-your-own-data import.** Deferred with the Open edition. It is weaker than offering no import, and pointless without public users.

Research: [12](../research/12-open-access-for-anyone.md), [11](../research/11-web-first-delivery.md), [01](../research/01-data-source-and-licensing.md)

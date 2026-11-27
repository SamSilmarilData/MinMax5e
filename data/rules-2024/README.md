# 2024 encodings

Hand-written RON encodings of 2024-rules mechanics, one file per content item, loaded by
`mm-load` and embedded by `mm-rules-2024` at build time (ADR-0002, ADR-0010, ADR-0035).

Empty until real encodings start, after `mm-import` (build-order step 6). Until then every
mechanic is exercised through the synthetic fixtures in `crates/mm-testkit/fixtures/`.

Rules for this directory (ADR-0028):

- Encodings describe mechanics in our own words. Never paste rules text, and never commit
  anything generated from the 5e.tools corpus.
- Every Content ID uses the `2024:` prefix (ADR-0008).
- Each encoding carries a citation and a source tier (ADR-0007).
- Real encodings are tested locally, never in CI.

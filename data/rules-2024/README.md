# 2024 encodings

Hand-written RON encodings of 2024-rules mechanics, one file per content item, loaded by
`mm-load` and embedded by `mm-rules-2024` at build time (ADR-0002, ADR-0010, ADR-0035).

Empty until real encodings start, after `mm-import` (build-order step 6). Until then every
mechanic is exercised through the synthetic fixtures in `mm-testkit`.

Rules for this directory: what may be committed or run in CI (ADR-0028), Content IDs
(ADR-0008), citations and source tiers (ADR-0007), and where coverage is checked
(ADR-0002). `cargo xtask leak-scan` checks what is committed; `mm-cli validate-packs`
checks prefixes, citations and coverage.

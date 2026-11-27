# Synthetic fixtures

Invented content for tests and the CLI (ADR-0028). Every RON file starts with the
`// fixture: synthetic` marker line and every Content ID uses source `tst`;
`cargo xtask leak-scan` checks both.

- `pack/`: a pack directory (`mm-cli --pack-dir`). Files under `records/` are resolved
  records, the shape `mm-import` will produce; every other `.ron` file is an encoding,
  our mechanics for those records in the Effect DSL (ADR-0010, ADR-0035).
- `builds/`: Build files. A new Build also goes in `xtask/xhash.toml`, the set
  `cargo xtask xhash` checks for native/WebAssembly bit identity (ADR-0050).

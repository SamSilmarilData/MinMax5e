# Synthetic fixtures

Invented content for tests and the CLI (ADR-0028). Every RON file starts with the
`// fixture: synthetic` marker line and every Content ID uses source `tst`;
`cargo xtask leak-scan` checks both.

- `pack/`: a pack directory (`mm-cli --pack-dir`). Files under `records/` are resolved
  records, the shape `mm-import` will produce; every other `.ron` file is an encoding,
  our mechanics for those records in the Effect DSL (ADR-0010, ADR-0035). Each encoded
  feature cites its source at tier `Synthetic` (ADR-0007). The content is research 03
  §7.2's: the warrior class, a species, background and Origin feat with no combat effect,
  the style and heavy-weapon feats, a Graze-like mastery property, and two 2d6 Heavy weapons, one
  with that mastery property and one without. `test-heavy-striker` is the heavy-weapon feat
  without its crit-gated attack, so a variant can drop that attack alone.
- `malformed/`: one pack directory per mistake, each loaded alongside `pack/` by
  `mm-cli validate-packs`: an unknown Content ID, a wrong Ruleset prefix, a misspelled
  DSL variant, a missing citation, a record with no encoding, and a weapon naming an
  unknown mastery property.
- `assumptions/`: Assumption Set files (`mm-cli evaluate --assumptions`). `hand-count`
  is the target of the CLI tests' hand count for `warrior-1` (AC 16, 6 HP), with an
  Adventuring Day Model other than the default.
- `builds/`: Build files. `warrior-1` is #15's level-1 warrior, wielding the plainblade.
  `warrior-5` is research 03 §7.2's level-5 warrior, and each
  `warrior-5-no-*` drops one component for the note's variant table. Each Build is
  listed in `xtask/xhash.toml`, the set `cargo xtask xhash` checks for
  native/WebAssembly bit identity (ADR-0050); a test fails until a new Build is added
  there.

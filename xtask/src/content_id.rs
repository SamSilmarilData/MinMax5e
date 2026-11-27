//! The Content ID grammar the guards read (ADR-0008).

/// `<ruleset>:<kind>/<slug>|<source>`, capturing `kind` and `source`. The source is
/// optional, so the leak scan also catches an ID written without one.
pub const PATTERN: &str =
    r"\b[0-9a-z]+:(?P<kind>[a-z][a-z0-9-]*)(?:/[a-z0-9][a-z0-9-]*)+(?:\|(?P<source>[A-Za-z0-9]+))?";

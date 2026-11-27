//! `glossary`: content kinds use the project's vocabulary, never a term GLOSSARY.md avoids.
//!
//! The names checked, each read as lowercase words, against every term on an `_Avoid_:`
//! line of the staged GLOSSARY.md:
//! - `kind`: the `<kind>` segment of a Content ID (`<ruleset>:<kind>/…`), and the name a
//!   `const NAME: &'static str = "…"` gives a kind in Rust, hyphens read as spaces;
//! - `variant`: a variant of a Rust `enum Record` or `enum Encoding`, the names a pack's RON
//!   is written in, split at its capitals (`MasteryProperty` is "mastery property").
//!
//! Prose and other identifiers are left to review: the avoid lists hold common words
//! ("source", "version"), and terms one entry avoids that another defines ("corpus", "errata"),
//! which a wider scan would flag everywhere.
//!
//! Research notes stay as written (`CODING_STANDARDS.md`, "Research errors go to the errata"),
//! so files under `docs/research/` are not scanned.

use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use crate::Outcome;
use crate::content_id;
use crate::git::tracked_files;
use crate::report::{Check as _, Report, at_line};

const GLOSSARY: &str = "GLOSSARY.md";

/// Files under this directory are not scanned.
const RESEARCH_DIR: &str = "docs/research/";

/// The enums whose variant names a pack's RON is written in.
const CONTENT_ENUMS: [&str; 2] = ["Record", "Encoding"];

#[derive(Debug, Clone, Copy)]
enum Check {
    Kind,
    Variant,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::Kind => "kind",
            Check::Variant => "variant",
        }
    }
}

/// A term GLOSSARY.md says to avoid, and the entry that says so.
#[derive(Debug)]
struct Avoided {
    /// Lowercase, without any parenthetical note.
    term: String,
    entry: String,
}

/// The patterns a scan reads names with.
struct Patterns {
    content_id: Regex,
    kind_name: Regex,
    enum_start: Regex,
    variant: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            content_id: Regex::new(content_id::PATTERN).expect("valid regex"),
            kind_name: Regex::new(r#"const NAME: &'static str = "([a-z][a-z0-9-]*)""#)
                .expect("valid regex"),
            enum_start: Regex::new(r"\benum (\w+)\b[^;]*\{\s*$").expect("valid regex"),
            variant: Regex::new(r"^\s*([A-Z][A-Za-z0-9]*)\b").expect("valid regex"),
        }
    }
}

pub fn run(root: &Path) -> Result<Outcome> {
    let files = tracked_files(root)?;
    let glossary = files
        .iter()
        .find(|file| file.path == GLOSSARY)
        .with_context(|| format!("{GLOSSARY} is not tracked in {}", root.display()))?;
    let avoided = avoided_terms(&String::from_utf8_lossy(&glossary.contents));

    let patterns = Patterns::new();
    let mut report = Report::new("glossary");
    let mut scanned = 0;
    for file in files.iter().filter(|f| !f.path.starts_with(RESEARCH_DIR)) {
        scanned += 1;
        let text = String::from_utf8_lossy(&file.contents);
        for (index, check, name, words) in names(&patterns, &file.path, &text) {
            if let Some(hit) = avoided.iter().find(|a| a.term == words) {
                report.push(check.problem(
                    at_line(&file.path, index),
                    format!(
                        "the {} \"{name}\" is a term {GLOSSARY} avoids (see **{}**)",
                        check.name(),
                        hit.entry
                    ),
                ));
            }
        }
    }
    Ok(report.finish(
        &format!(
            "{scanned} tracked file(s) clean against {} avoided term(s)",
            avoided.len()
        ),
        "Name it after the glossary entry it belongs to.",
    ))
}

/// Every checked name in `text`: its line index, its check, the name, and its words.
fn names<'t>(
    patterns: &Patterns,
    path: &str,
    text: &'t str,
) -> Vec<(usize, Check, &'t str, String)> {
    let mut found = Vec::new();
    // Inside a content enum: the brace depth, 1 at the variants' level.
    let mut depth: Option<i64> = None;
    for (index, line) in text.lines().enumerate() {
        let kinds = patterns
            .content_id
            .captures_iter(line)
            .filter_map(|caps| caps.name("kind"))
            .chain(
                patterns
                    .kind_name
                    .captures_iter(line)
                    .filter_map(|caps| caps.get(1)),
            );
        for kind in kinds {
            let name = kind.as_str();
            found.push((index, Check::Kind, name, name.replace('-', " ")));
        }

        if Path::new(path).extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        match depth {
            None => {
                if patterns
                    .enum_start
                    .captures(line)
                    .is_some_and(|caps| CONTENT_ENUMS.contains(&&caps[1]))
                {
                    depth = Some(1);
                }
            }
            Some(level) => {
                if level == 1
                    && let Some(variant) = patterns.variant.captures(line).and_then(|c| c.get(1))
                {
                    let name = variant.as_str();
                    found.push((index, Check::Variant, name, words_of(name)));
                }
                let opened = line.matches('{').count() + line.matches('(').count();
                let closed = line.matches('}').count() + line.matches(')').count();
                let level =
                    level + i64::try_from(opened).unwrap_or(0) - i64::try_from(closed).unwrap_or(0);
                depth = (level > 0).then_some(level);
            }
        }
    }
    found
}

/// A `PascalCase` name as lowercase words: `MasteryProperty` is "mastery property".
fn words_of(name: &str) -> String {
    let mut words = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            words.push(' ');
        }
        words.push(c.to_ascii_lowercase());
    }
    words
}

/// Every term on an `_Avoid_:` line, with the bold entry above it.
fn avoided_terms(glossary: &str) -> Vec<Avoided> {
    let parenthetical = Regex::new(r"\s*\([^)]*\)").expect("valid regex");
    let mut entry = "";
    let mut avoided = Vec::new();
    for line in glossary.lines() {
        if let Some(name) = line.strip_prefix("**").and_then(|l| l.split("**").next()) {
            entry = name.trim_end_matches(':');
        } else if let Some(terms) = line.strip_prefix("_Avoid_:") {
            let terms = parenthetical.replace_all(terms, "");
            avoided.extend(
                terms
                    .split(',')
                    .map(|term| term.trim().to_lowercase())
                    .filter(|term| !term.is_empty())
                    .map(|term| Avoided {
                        term,
                        entry: entry.to_owned(),
                    }),
            );
        }
    }
    avoided
}

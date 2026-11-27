//! `glossary`: Content ID kinds use the project's vocabulary, never a term GLOSSARY.md avoids.
//!
//! A kind is the `<kind>` segment of a Content ID (`2024:<kind>/…`) or the name a
//! `const NAME: &'static str = "…"` gives it in Rust. Each one, read with its hyphens as
//! spaces, must not equal a term on any `_Avoid_:` line of the staged GLOSSARY.md. Prose is
//! left to review: the avoid lists hold common words ("source", "version") that a text scan
//! would flag everywhere.
//!
//! Research notes stay as written (`CODING_STANDARDS.md`, "Research errors go to the errata"),
//! so files under `docs/research/` are not scanned.

use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use crate::Outcome;
use crate::git::tracked_files;
use crate::report::{Check as _, Report, at_line};

const GLOSSARY: &str = "GLOSSARY.md";

/// Files under this prefix are not scanned.
const RESEARCH: &str = "docs/research/";

#[derive(Debug, Clone, Copy)]
enum Check {
    Kind,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::Kind => "kind",
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

pub fn run(root: &Path) -> Result<Outcome> {
    let files = tracked_files(root)?;
    let glossary = files
        .iter()
        .find(|file| file.path == GLOSSARY)
        .with_context(|| format!("{GLOSSARY} is not tracked in {}", root.display()))?;
    let avoided = avoided_terms(&String::from_utf8_lossy(&glossary.contents));

    let kind = Regex::new(
        r#"\b(?:2014|2024):([a-z][a-z0-9-]*)/|const NAME: &'static str = "([a-z][a-z0-9-]*)""#,
    )
    .expect("the kind pattern is valid");
    let mut report = Report::new("glossary");
    let mut scanned = 0;
    for file in files.iter().filter(|file| !file.path.starts_with(RESEARCH)) {
        scanned += 1;
        let text = String::from_utf8_lossy(&file.contents);
        for (index, line) in text.lines().enumerate() {
            for found in kind.captures_iter(line) {
                let name = found
                    .get(1)
                    .or_else(|| found.get(2))
                    .map_or("", |m| m.as_str());
                let words = name.replace('-', " ");
                if let Some(hit) = avoided.iter().find(|a| a.term == words) {
                    report.push(Check::Kind.problem(
                        at_line(&file.path, index),
                        format!(
                            "the kind \"{name}\" is a term {GLOSSARY} avoids (see **{}**)",
                            hit.entry
                        ),
                    ));
                }
            }
        }
    }
    Ok(report.finish(
        &format!(
            "{scanned} tracked file(s) clean against {} avoided term(s)",
            avoided.len()
        ),
        "Name the kind after the glossary entry it belongs to.",
    ))
}

/// Every term on an `_Avoid_:` line, with the bold entry above it.
fn avoided_terms(glossary: &str) -> Vec<Avoided> {
    let parenthetical = Regex::new(r"\s*\([^)]*\)").expect("the note pattern is valid");
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

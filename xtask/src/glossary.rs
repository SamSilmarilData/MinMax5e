//! `glossary`: content kinds use the project's vocabulary, never a term GLOSSARY.md avoids.
//!
//! The names checked, each read as lowercase words, against every term on an `_Avoid_:`
//! line of the staged GLOSSARY.md:
//! - `kind`: the `<kind>` segment of a Content ID (`<ruleset>:<kind>/…`), and the name a
//!   `const NAME: &'static str = "…"` gives a kind in Rust, hyphens read as spaces;
//! - `variant`: a variant of a Rust `enum Record` or `enum Encoding`, split at its capitals
//!   (`MasteryProperty` is "mastery property"). A pack's RON is written in these names, as
//!   long as no `#[serde(rename)]` changes them; none does.
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
    content_enum: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            content_id: Regex::new(content_id::PATTERN).expect("valid regex"),
            kind_name: Regex::new(r#"const NAME: &'static str = "([a-z][a-z0-9-]*)""#)
                .expect("valid regex"),
            content_enum: Regex::new(&format!(r"\benum (?:{})\b", CONTENT_ENUMS.join("|")))
                .expect("valid regex"),
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
    let (mut scanned, mut kinds, mut variants) = (0, 0, 0);
    for file in files.iter().filter(|f| !f.path.starts_with(RESEARCH_DIR)) {
        scanned += 1;
        let rust = Path::new(&file.path)
            .extension()
            .is_some_and(|ext| ext == "rs");
        let text = String::from_utf8_lossy(&file.contents);
        for found in names(&patterns, rust, &text) {
            match found.check {
                Check::Kind => kinds += 1,
                Check::Variant => variants += 1,
            }
            if let Some(hit) = avoided.iter().find(|a| a.term == found.words) {
                report.push(found.check.problem(
                    at_line(&file.path, found.line),
                    format!(
                        "the {} \"{}\" is a term {GLOSSARY} avoids (see **{}**)",
                        found.check.name(),
                        found.name,
                        hit.entry
                    ),
                ));
            }
        }
    }
    // The counts show the scan found names at all: a renamed content enum would otherwise
    // pass silently.
    Ok(report.finish(
        &format!(
            "{kinds} kind(s) and {variants} content enum variant(s) in {scanned} tracked file(s) \
             clean against {} avoided term(s)",
            avoided.len()
        ),
        "Name it after the glossary entry it belongs to.",
    ))
}

/// A name the guard checks, where it was found.
#[derive(Debug)]
struct Found {
    line: usize,
    check: Check,
    name: String,
    /// The name as lowercase words, as the avoid lists write terms.
    words: String,
}

/// Every checked name in `text`. `rust` says whether to look for content enum variants.
fn names(patterns: &Patterns, rust: bool, text: &str) -> Vec<Found> {
    let mut found = Vec::new();
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
            found.push(Found {
                line: index,
                check: Check::Kind,
                name: name.to_owned(),
                words: name.replace('-', " "),
            });
        }
    }
    if rust {
        found.extend(
            content_variants(patterns, text)
                .into_iter()
                .map(|(line, name)| Found {
                    line,
                    check: Check::Variant,
                    words: words_of(&name),
                    name,
                }),
        );
    }
    found
}

/// The variants of every content enum in Rust `text`, with their line indexes.
///
/// Comments are dropped first, so brackets in them cannot unbalance the walk. A variant is
/// the first name at the enum body's top level, after its `{` or a `,`; attributes, fields
/// and tuple types sit deeper and are skipped.
fn content_variants(patterns: &Patterns, text: &str) -> Vec<(usize, String)> {
    let code: String = text
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n");
    let mut variants = Vec::new();
    for start in patterns.content_enum.find_iter(&code) {
        let Some(open) = code[start.end()..].find('{') else {
            continue;
        };
        let body = start.end() + open;
        let mut line = code[..body].matches('\n').count();
        let mut depth = 0_u32;
        let mut expect_variant = false;
        let mut chars = code[body..].char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            match c {
                '\n' => line += 1,
                '{' | '(' | '[' | '<' => {
                    depth += 1;
                    expect_variant = depth == 1;
                }
                '}' | ')' | ']' | '>' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                ',' if depth == 1 => expect_variant = true,
                c if depth == 1 && expect_variant && c.is_ascii_uppercase() => {
                    let mut name = String::from(c);
                    while let Some(&(_, next)) = chars.peek() {
                        if !(next.is_ascii_alphanumeric() || next == '_') {
                            break;
                        }
                        name.push(next);
                        chars.next();
                    }
                    variants.push((line, name));
                    expect_variant = false;
                }
                _ => {}
            }
        }
    }
    variants
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

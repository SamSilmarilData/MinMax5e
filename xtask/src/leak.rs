//! `leak-scan`: the repository must hold nothing derived from upstream rules data (ADR-0028).
//!
//! Every tracked file is read from the git index, because that is what a commit records.
//! The scan fails on:
//! - `tag-markup`: 5e.tools tag markup (an opening brace, `@`, then a tag name);
//! - `data-file`: a tracked `*.json`, `*.pack` or `*.postcard` file, unless it is a JSON
//!   fixture carrying the synthetic marker;
//! - `fixture`: a file under a `fixtures/` directory that is not RON, JSON or Markdown, a RON
//!   or JSON fixture without the synthetic marker, or a Content ID whose source is not `tst`;
//! - [`RELAXED_SIMD`]: the WASM target feature for relaxed SIMD, whose FMA is not
//!   deterministic across hosts (ADR-0050). This one also covers untracked files that git
//!   does not ignore, so a local cargo config cannot carry it either.
//!
//! This file and its tests assemble the patterns at runtime, so the scan covers them too.
//!
//! A file may be exempted from named checks by the staged `xtask/leak-allow.toml`. Every
//! exemption states its reason, and an exemption for a path no longer tracked fails the scan.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use regex::bytes::Regex;
use serde::Deserialize;

use crate::Outcome;
use crate::git::{RepoFile, tracked_files, untracked_files};
use crate::report::{Problem, Report, at_line, lines};

/// The check's name, assembled so this file does not match it.
const RELAXED_SIMD: &str = concat!("relaxed", "-", "simd");

/// Where the allowlist lives, relative to the scanned root.
const ALLOWLIST: &str = "xtask/leak-allow.toml";

/// The name allowlist problems are reported under. It is not a check, so it can't be exempted.
const ALLOWLIST_CHECK: &str = "allowlist";

/// The RON fixture marker: the whole first line.
const RON_MARKER: &[u8] = b"// fixture: synthetic";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
enum Check {
    TagMarkup,
    DataFile,
    Fixture,
    RelaxedSimd,
}

impl Check {
    const ALL: [Check; 4] = [
        Check::TagMarkup,
        Check::DataFile,
        Check::Fixture,
        Check::RelaxedSimd,
    ];

    /// The name used in reports and in the allowlist.
    fn name(self) -> &'static str {
        match self {
            Check::TagMarkup => "tag-markup",
            Check::DataFile => "data-file",
            Check::Fixture => "fixture",
            Check::RelaxedSimd => RELAXED_SIMD,
        }
    }
}

impl TryFrom<String> for Check {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Check::ALL
            .into_iter()
            .find(|check| check.name() == name)
            .ok_or_else(|| {
                let known: Vec<_> = Check::ALL.iter().map(|c| c.name()).collect();
                format!(
                    "unknown check `{name}`; expected one of {}",
                    known.join(", ")
                )
            })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowlist {
    #[serde(default)]
    file: Vec<Exemption>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exemption {
    path: String,
    allow: Vec<Check>,
    reason: String,
}

/// Tracked path → the checks it is exempt from.
type Exemptions = BTreeMap<String, Vec<Check>>;

struct Patterns {
    tag: Regex,
    relaxed_simd: Regex,
    content_id: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            // ADR-0028: an opening brace and `@`, a lowercase tag name, then a space, pipe or brace.
            tag: Regex::new(&format!(r"\{{{}[a-z]+[ |}}]", '@')).expect("valid regex"),
            relaxed_simd: Regex::new(&format!("(?i){RELAXED_SIMD}")).expect("valid regex"),
            // `<ruleset>:<kind>/<slug>|<source>` (ADR-0008); the source is captured.
            content_id: Regex::new(
                r"\b[0-9a-z]+:[a-z][a-z0-9-]*(?:/[a-z0-9][a-z0-9-]*)+\|([A-Za-z0-9]+)",
            )
            .expect("valid regex"),
        }
    }
}

/// A finding before exemptions are applied.
struct Finding {
    check: Check,
    at: String,
    message: String,
}

impl Finding {
    fn new(check: Check, at: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            check,
            at: at.into(),
            message: message.into(),
        }
    }

    fn into_problem(self) -> Problem {
        Problem {
            at: self.at,
            check: self.check.name(),
            message: self.message,
        }
    }
}

pub fn run(root: &Path) -> Result<Outcome> {
    let files = tracked_files(root)?;
    let untracked = untracked_files(root)?;
    let mut report = Report::new("leak-scan");
    let exemptions = load_allowlist(&files, &mut report)?;
    let patterns = Patterns::new();

    for file in &files {
        let exempt = exemptions.get(file.path.as_str());
        for finding in scan_tracked(file, &patterns) {
            if !exempt.is_some_and(|allowed| allowed.contains(&finding.check)) {
                report.push(finding.into_problem());
            }
        }
    }
    for file in &untracked {
        for finding in scan_relaxed_simd(file, &patterns) {
            report.push(finding.into_problem());
        }
    }

    Ok(report.finish(
        &format!(
            "{} tracked and {} untracked files clean",
            files.len(),
            untracked.len()
        ),
        &format!("Remove the content, or exempt the file in {ALLOWLIST} with a reason (ADR-0028)."),
    ))
}

/// Reads the staged allowlist into path → allowed checks. Malformed entries are reported
/// alongside everything else, not raised as errors.
fn load_allowlist(files: &[RepoFile], report: &mut Report) -> Result<Exemptions> {
    let mut exemptions = BTreeMap::new();
    let Some(allowlist) = files.iter().find(|f| f.path == ALLOWLIST) else {
        return Ok(exemptions);
    };
    let text = std::str::from_utf8(&allowlist.contents)
        .with_context(|| format!("{ALLOWLIST} is not UTF-8"))?;
    let list: Allowlist = toml::from_str(text).with_context(|| format!("parsing {ALLOWLIST}"))?;

    for entry in list.file {
        let mut problem = |message: &str| {
            report.add(
                ALLOWLIST,
                ALLOWLIST_CHECK,
                format!("the exemption for {} {message}", entry.path),
            );
        };
        if entry.reason.trim().is_empty() {
            problem("has no reason");
        }
        if entry.allow.is_empty() {
            problem("allows no checks");
        }
        if !files.iter().any(|f| f.path == entry.path) {
            problem("matches no tracked file; remove it");
        }
        exemptions
            .entry(entry.path)
            .or_insert_with(Vec::new)
            .extend(entry.allow);
    }
    Ok(exemptions)
}

fn scan_tracked(file: &RepoFile, patterns: &Patterns) -> Vec<Finding> {
    let mut findings = scan_relaxed_simd(file, patterns);
    for (index, line) in lines(&file.contents) {
        if patterns.tag.is_match(line) {
            findings.push(Finding::new(
                Check::TagMarkup,
                at_line(&file.path, index),
                "5e.tools tag markup; upstream rules text must not be committed",
            ));
        }
    }

    let extension = Path::new(&file.path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let fixture = file.path.split('/').any(|part| part == "fixtures");
    let marked_json_fixture =
        fixture && extension.as_deref() == Some("json") && json_marked(&file.contents);

    if matches!(extension.as_deref(), Some("json" | "pack" | "postcard")) && !marked_json_fixture {
        findings.push(Finding::new(
            Check::DataFile,
            &file.path,
            "tracked JSON, pack or postcard file; rules data and anything derived from it stays in the local cache",
        ));
    }
    if fixture {
        findings.extend(scan_fixture(file, extension.as_deref(), patterns));
    }
    findings
}

fn scan_relaxed_simd(file: &RepoFile, patterns: &Patterns) -> Vec<Finding> {
    lines(&file.contents)
        .filter(|(_, line)| patterns.relaxed_simd.is_match(line))
        .map(|(index, _)| {
            Finding::new(
                Check::RelaxedSimd,
                at_line(&file.path, index),
                "this SIMD variant is not deterministic across hosts (ADR-0050)",
            )
        })
        .collect()
}

/// Whether the JSON document is an object with the top-level field `"_fixture": "synthetic"`.
fn json_marked(contents: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(contents)
        .is_ok_and(|doc| doc.get("_fixture").and_then(|v| v.as_str()) == Some("synthetic"))
}

fn scan_fixture(file: &RepoFile, extension: Option<&str>, patterns: &Patterns) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut problem = |at: String, message: String| {
        findings.push(Finding::new(Check::Fixture, at, message));
    };

    match extension {
        Some("ron") => {
            let first = lines(&file.contents).next().map_or(&[][..], |(_, l)| l);
            if first.strip_suffix(b"\r").unwrap_or(first) != RON_MARKER {
                problem(
                    file.path.clone(),
                    "fixture lacks the synthetic marker: `// fixture: synthetic` as the first line"
                        .to_owned(),
                );
            }
        }
        Some("json") => {
            if !json_marked(&file.contents) {
                problem(
                    file.path.clone(),
                    r#"fixture lacks the synthetic marker: a top-level `"_fixture": "synthetic"` field"#
                        .to_owned(),
                );
            }
        }
        // Documentation about the fixtures; its Content IDs are still checked below.
        Some("md") => {}
        _ => problem(
            file.path.clone(),
            "fixtures are RON or JSON files carrying the synthetic marker (or Markdown notes)"
                .to_owned(),
        ),
    }

    for (index, line) in lines(&file.contents) {
        for caps in patterns.content_id.captures_iter(line) {
            let source = String::from_utf8_lossy(&caps[1]);
            if source != "tst" {
                problem(
                    at_line(&file.path, index),
                    format!(
                        "Content ID {} has source `{source}`; fixtures use source `tst`",
                        String::from_utf8_lossy(&caps[0])
                    ),
                );
            }
        }
    }
    findings
}

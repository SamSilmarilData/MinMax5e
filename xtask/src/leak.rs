//! `leak-scan`: tracked files must hold nothing derived from upstream rules data (ADR-0028).
//!
//! Fails on:
//! - `tag-markup`: 5e.tools tag markup (an opening brace, `@`, then a tag name);
//! - `data-file`: a tracked `*.json`, `*.pack` or `*.postcard` file;
//! - `fixture`: a file under a `fixtures/` directory without the synthetic marker, or with a
//!   Content ID whose source is not `tst`;
//! - [`RELAXED_SIMD`]: the WASM target feature for relaxed SIMD, whose FMA is not deterministic
//!   across hosts (ADR-0050).
//!
//! This file and its tests assemble the patterns at runtime, so the scan covers them too.
//!
//! A file may be exempted from named checks by the staged `xtask/leak-allow.toml`, and
//! every exemption states its reason.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use regex::bytes::Regex;
use serde::Deserialize;

use crate::Outcome;
use crate::git::{TrackedFile, tracked_files};

/// The check's name, assembled so this file does not match it.
const RELAXED_SIMD: &str = concat!("relaxed", "-", "simd");

/// Where the allowlist lives, relative to the scanned root.
const ALLOWLIST: &str = "xtask/leak-allow.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Check {
    TagMarkup,
    DataFile,
    Fixture,
    RelaxedSimd,
    /// A problem with the allowlist itself; it cannot be exempted.
    #[serde(skip)]
    Allowlist,
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Check::TagMarkup => "tag-markup",
            Check::DataFile => "data-file",
            Check::Fixture => "fixture",
            Check::RelaxedSimd => RELAXED_SIMD,
            Check::Allowlist => "allowlist",
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

#[derive(Debug)]
struct Finding {
    path: String,
    line: Option<usize>,
    check: Check,
    message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: ", self.path)?,
            None => write!(f, "{}: ", self.path)?,
        }
        write!(f, "[{}] {}", self.check, self.message)
    }
}

struct Patterns {
    tag: Regex,
    banned_simd: Regex,
    json_marker: Regex,
    content_id: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            // ADR-0028: an opening brace and `@`, a lowercase tag name, then a space, pipe or brace.
            tag: Regex::new(&format!(r"\{{{}[a-z]+[ |}}]", '@')).expect("valid regex"),
            banned_simd: Regex::new(&format!("(?i){}-simd", "relaxed")).expect("valid regex"),
            json_marker: Regex::new(r#""_fixture"\s*:\s*"synthetic""#).expect("valid regex"),
            // `<ruleset>:<kind>/<slug>|<source>` (ADR-0008); the source is captured.
            content_id: Regex::new(
                r"\b[0-9a-z]+:[a-z][a-z0-9-]*(?:/[a-z0-9][a-z0-9-]*)+\|([A-Za-z0-9]+)",
            )
            .expect("valid regex"),
        }
    }
}

pub fn run(root: &Path) -> Result<Outcome> {
    let files = tracked_files(root)?;
    let (exemptions, mut findings) = load_allowlist(&files)?;
    let patterns = Patterns::new();

    for file in &files {
        let exempt = |check| {
            exemptions
                .get(file.path.as_str())
                .is_some_and(|allowed: &Vec<Check>| allowed.contains(&check))
        };
        for finding in scan_file(file, &patterns) {
            if !exempt(finding.check) {
                findings.push(finding);
            }
        }
    }

    if findings.is_empty() {
        println!("leak-scan: {} tracked files clean", files.len());
        return Ok(Outcome::Clean);
    }
    for finding in &findings {
        println!("{finding}");
    }
    println!(
        "leak-scan: {} problem(s). Remove the content, or exempt the file in {ALLOWLIST} with a reason (ADR-0028).",
        findings.len()
    );
    Ok(Outcome::Violations)
}

/// Reads the staged allowlist into path → allowed checks. Malformed entries are findings,
/// not errors, so they are reported alongside everything else.
fn load_allowlist(files: &[TrackedFile]) -> Result<(Exemptions, Vec<Finding>)> {
    let mut exemptions = BTreeMap::new();
    let mut findings = Vec::new();
    let Some(allowlist) = files.iter().find(|f| f.path == ALLOWLIST) else {
        return Ok((exemptions, findings));
    };
    let text = std::str::from_utf8(&allowlist.contents)
        .with_context(|| format!("{ALLOWLIST} is not UTF-8"))?;
    let list: Allowlist = toml::from_str(text).with_context(|| format!("parsing {ALLOWLIST}"))?;

    let allowlist_finding = |message: String| Finding {
        path: ALLOWLIST.to_owned(),
        line: None,
        check: Check::Allowlist,
        message,
    };
    for entry in list.file {
        if entry.reason.trim().is_empty() {
            findings.push(allowlist_finding(format!(
                "the exemption for {} has no reason",
                entry.path
            )));
        }
        if entry.allow.is_empty() {
            findings.push(allowlist_finding(format!(
                "the exemption for {} allows no checks",
                entry.path
            )));
        }
        if !files.iter().any(|f| f.path == entry.path) {
            findings.push(allowlist_finding(format!(
                "the exemption for {} matches no tracked file; remove it",
                entry.path
            )));
        }
        exemptions
            .entry(entry.path)
            .or_insert_with(Vec::new)
            .extend(entry.allow);
    }
    Ok((exemptions, findings))
}

fn scan_file(file: &TrackedFile, patterns: &Patterns) -> Vec<Finding> {
    let mut findings = Vec::new();
    let finding = |line, check, message: &str| Finding {
        path: file.path.clone(),
        line,
        check,
        message: message.to_owned(),
    };

    for (index, line) in file.contents.split(|&b| b == b'\n').enumerate() {
        let line_no = Some(index + 1);
        if patterns.tag.is_match(line) {
            findings.push(finding(
                line_no,
                Check::TagMarkup,
                "5e.tools tag markup; upstream rules text must not be committed",
            ));
        }
        if patterns.banned_simd.is_match(line) {
            findings.push(finding(
                line_no,
                Check::RelaxedSimd,
                "this SIMD variant is not deterministic across hosts (ADR-0050)",
            ));
        }
    }

    let extension = Path::new(&file.path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if matches!(extension.as_deref(), Some("json" | "pack" | "postcard")) {
        findings.push(finding(
            None,
            Check::DataFile,
            "tracked JSON, pack or postcard file; rules data and anything derived from it stays in the local cache",
        ));
    }

    if file.path.split('/').any(|part| part == "fixtures") {
        findings.extend(scan_fixture(file, extension.as_deref(), patterns));
    }
    findings
}

fn scan_fixture(file: &TrackedFile, extension: Option<&str>, patterns: &Patterns) -> Vec<Finding> {
    let mut findings = Vec::new();
    let finding = |line, message: String| Finding {
        path: file.path.clone(),
        line,
        check: Check::Fixture,
        message,
    };

    let marked = match extension {
        Some("ron") => {
            let first = file
                .contents
                .split(|&b| b == b'\n')
                .next()
                .unwrap_or_default();
            first.strip_suffix(b"\r").unwrap_or(first) == b"// fixture: synthetic"
        }
        Some("json") => patterns.json_marker.is_match(&file.contents),
        _ => {
            findings.push(finding(
                None,
                "fixtures must be RON or JSON files carrying the synthetic marker".to_owned(),
            ));
            true
        }
    };
    if !marked {
        let expected = if extension == Some("ron") {
            "`// fixture: synthetic` as the first line"
        } else {
            r#"a `"_fixture": "synthetic"` field"#
        };
        findings.push(finding(
            None,
            format!("fixture lacks the synthetic marker: {expected}"),
        ));
    }

    for (index, line) in file.contents.split(|&b| b == b'\n').enumerate() {
        for caps in patterns.content_id.captures_iter(line) {
            let source = String::from_utf8_lossy(&caps[1]);
            if source != "tst" {
                findings.push(finding(
                    Some(index + 1),
                    format!(
                        "Content ID {} has source `{source}`; fixtures use source `tst`",
                        String::from_utf8_lossy(&caps[0])
                    ),
                ));
            }
        }
    }
    findings
}

//! `comment-width`: Rust comments wrap at the width rustfmt gives code.
//!
//! rustfmt keeps code within 100 columns (its default `max_width`), but stable rustfmt does
//! not wrap comments, so a long comment line passes fmt and clippy unnoticed. This guard
//! fails on any line of a tracked `.rs` file that is a comment (`//`, `///` or `//!` after
//! optional indentation) and is wider than 100 columns, counting characters rather than
//! bytes. Files under `spikes/` are frozen (spec #12) and are not checked.
//!
//! A comment line with no space after its marker's text starts, such as a link alone on its
//! line, cannot be wrapped and passes. Comments are found line by line as text, so a line
//! inside a multi-line string literal that starts with `//` is checked as a comment, and a
//! comment after code on the same line is not checked. Neither appears in this repository.

use std::path::Path;

use anyhow::Result;

use crate::Outcome;
use crate::git::tracked_files;
use crate::report::{Check as _, Report, at_line, lines};

/// rustfmt's default `max_width`, which this repository keeps.
const WIDTH: usize = 100;

/// The frozen spike's directory.
const SPIKES_DIR: &str = "spikes/";

#[derive(Debug, Clone, Copy)]
enum Check {
    /// A comment line wider than [`WIDTH`].
    Width,
    /// A Rust file that is not UTF-8, so its columns cannot be counted.
    Parse,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::Width => "width",
            Check::Parse => "parse",
        }
    }
}

/// Checks every tracked Rust file in the repository at `root`.
pub fn run(root: &Path) -> Result<Outcome> {
    let mut report = Report::new("comment-width");
    let mut checked = 0;
    for file in tracked_files(root)? {
        let rust = Path::new(&file.path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"));
        if !rust || file.path.starts_with(SPIKES_DIR) {
            continue;
        }
        checked += 1;
        for (index, line) in lines(&file.contents) {
            let Ok(line) = std::str::from_utf8(line) else {
                report.push(Check::Parse.problem(
                    at_line(&file.path, index),
                    "the line is not UTF-8, so its width cannot be counted",
                ));
                continue;
            };
            let columns = line.chars().count();
            if columns > WIDTH && wrappable_comment(line) {
                report.push(Check::Width.problem(
                    at_line(&file.path, index),
                    format!("the comment is {columns} columns wide; wrap it at {WIDTH}"),
                ));
            }
        }
    }
    Ok(report.finish(
        &format!("{checked} tracked Rust file(s) have comments within {WIDTH} columns"),
        "Wrap each comment at 100 columns, as rustfmt does code.",
    ))
}

/// Whether `line` is a comment whose text has a space to wrap at.
fn wrappable_comment(line: &str) -> bool {
    let Some(comment) = line.trim_start().strip_prefix("//") else {
        return false;
    };
    let text = comment.trim_start_matches(['/', '!']).trim();
    text.contains(char::is_whitespace)
}

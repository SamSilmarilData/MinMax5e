//! `adr-check`: accepted ADRs change only by dated amendment (CLAUDE.md: "never as a silent
//! edit").
//!
//! Each ADR whose `status:` is `accepted` at the base commit is compared with the working
//! tree. It may gain lines inside dated amendment blocks (a blockquote whose first line starts
//! `> *Amended YYYY-MM-DD`) and blank lines, and its `status:` line may change (for example to
//! `superseded by ADR-…`). Any other added, changed or removed line fails, and so does
//! deleting the file. ADRs that are new, or not accepted at the base, are not checked.
//!
//! Every ADR in the working tree, new or old, also lists its amendment blocks newest first:
//! each block's date is on or before the date of the block above it.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::Outcome;
use crate::report::{Check as _, Report, at_line};

const ADR_DIR: &str = "docs/adr";

#[derive(Debug, Clone, Copy)]
enum Check {
    SilentEdit,
    AmendmentOrder,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::SilentEdit => "adr-silent-edit",
            Check::AmendmentOrder => "adr-amendment-order",
        }
    }
}

pub fn run(root: &Path, base: &str) -> Result<Outcome> {
    let base = git(root, &["merge-base", base, "HEAD"])?.trim().to_owned();
    let amendment_start = Regex::new(r"^> \*Amended (\d{4}-\d{2}-\d{2})").expect("valid regex");
    let mut report = Report::new("adr-check");
    let mut checked = 0;

    for path in git(
        root,
        &["ls-tree", "-r", "--name-only", &base, "--", ADR_DIR],
    )?
    .lines()
    {
        let markdown = Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !markdown {
            continue;
        }
        let old = git(root, &["show", &format!("{base}:{path}")])?;
        if status(&old) != Some("accepted") {
            continue;
        }
        checked += 1;
        let Ok(new) = std::fs::read_to_string(root.join(path)) else {
            report.push(Check::SilentEdit.problem(
                path,
                "an accepted ADR was deleted; supersede it with a new ADR instead",
            ));
            continue;
        };

        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let in_amendment = amendment_lines(&new_lines, &amendment_start);
        let (removed, added) = diff(&old_lines, &new_lines);

        for index in removed {
            if !is_status_line(old_lines[index]) {
                report.push(Check::SilentEdit.problem(
                    at_line(path, index),
                    "changes or removes accepted text; record the change in a dated amendment instead",
                ));
            }
        }
        for index in added {
            let line = new_lines[index];
            if !(line.trim().is_empty() || is_status_line(line) || in_amendment[index]) {
                report.push(Check::SilentEdit.problem(
                    at_line(path, index),
                    "adds text outside a dated amendment (`> *Amended YYYY-MM-DD …*`)",
                ));
            }
        }
    }

    check_amendment_order(root, &amendment_start, &mut report)?;

    Ok(report.finish(
        &format!("{checked} accepted ADR(s) changed only by amendment since {base:.12}"),
        "Accepted ADRs change by dated amendment or a superseding ADR (CLAUDE.md), and amendments are listed newest first.",
    ))
}

/// Reports every amendment block dated after the block above it, in every ADR in the
/// working tree.
fn check_amendment_order(
    root: &Path,
    amendment_start: &Regex,
    report: &mut Report<Check>,
) -> Result<()> {
    let dir = root.join(ADR_DIR);
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()
        .with_context(|| format!("reading {}", dir.display()))?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    });
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let name = format!(
            "{ADR_DIR}/{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        let mut above: Option<&str> = None;
        for (index, line) in text.lines().enumerate() {
            let Some(date) = amendment_start
                .captures(line)
                .and_then(|captures| captures.get(1))
            else {
                continue;
            };
            let date = date.as_str();
            if let Some(above) = above
                && date > above
            {
                report.push(Check::AmendmentOrder.problem(
                    at_line(&name, index),
                    format!(
                        "amendment dated {date} sits below one dated {above}; list amendments newest first"
                    ),
                ));
            }
            above = Some(date);
        }
    }
    Ok(())
}

/// The value of the frontmatter `status:` line.
fn status(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix("status:"))
        .map(str::trim)
}

fn is_status_line(line: &str) -> bool {
    line.starts_with("status:")
}

/// For each line, whether it belongs to a dated amendment block.
fn amendment_lines(lines: &[&str], amendment_start: &Regex) -> Vec<bool> {
    let mut inside = false;
    lines
        .iter()
        .map(|line| {
            if amendment_start.is_match(line) {
                inside = true;
            } else if !line.starts_with('>') {
                inside = false;
            }
            inside
        })
        .collect()
}

/// A line diff by longest common subsequence: the indices of `old` lines removed and of
/// `new` lines added. ADRs are short, so the quadratic table is fine.
fn diff(old: &[&str], new: &[&str]) -> (Vec<usize>, Vec<usize>) {
    let (n, m) = (old.len(), new.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old[i] == new[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut removed, mut added) = (Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            added.push(j);
            j += 1;
        } else {
            removed.push(i);
            i += 1;
        }
    }
    (removed, added)
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

//! `cargo xtask comment-width` as a process against seeded temporary git trees.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::{TempTree, xtask};

fn check(tree: &TempTree) -> common::XtaskRun {
    xtask(&["comment-width", "--root", tree.path().to_str().unwrap()])
}

/// A `///` comment line indented by four spaces, `width` characters long in all, made of
/// words so it could be wrapped.
fn doc_line(width: usize) -> String {
    let prefix = "    /// ";
    let mut body = String::new();
    while prefix.len() + body.chars().count() < width {
        body.push_str(if body.is_empty() { "word" } else { " word" });
    }
    let line: String = format!("{prefix}{body}").chars().take(width).collect();
    assert_eq!(line.chars().count(), width);
    line
}

#[test]
fn comments_within_100_columns_pass() {
    let tree = TempTree::git();
    tree.track("src/a.rs", format!("{}\nfn a() {{}}\n", doc_line(100)));
    check(&tree).assert_ok();
}

#[test]
fn a_comment_over_100_columns_fails_naming_its_line() {
    let tree = TempTree::git();
    tree.track(
        "src/a.rs",
        format!("fn a() {{}}\n\n{}\nfn b() {{}}\n", doc_line(101)),
    );
    check(&tree).assert_fails_with(&["src/a.rs:3", "[width]", "101 columns"]);
}

#[test]
fn every_comment_form_is_checked() {
    for marker in ["//", "///", "//!"] {
        let tree = TempTree::git();
        let line = doc_line(101).replacen("    ///", &format!("    {marker:<3}"), 1);
        tree.track("a.rs", format!("{line}\n"));
        check(&tree).assert_fails_with(&["a.rs:1", "[width]"]);
    }
}

#[test]
fn columns_are_characters_not_bytes() {
    // An en dash is three bytes in UTF-8 but one column.
    let tree = TempTree::git();
    let line: String = doc_line(100)
        .chars()
        .enumerate()
        .map(|(i, c)| if i > 10 && c == ' ' { '–' } else { c })
        .collect();
    assert!(line.len() > 100);
    tree.track("a.rs", format!("{line}\n"));
    check(&tree).assert_ok();
}

#[test]
fn a_comment_that_cannot_be_wrapped_passes() {
    // A link alone on its line has no space to break at.
    let tree = TempTree::git();
    let url = format!("//! <https://example.invalid/{}>", "a".repeat(120));
    tree.track("a.rs", format!("{url}\n"));
    check(&tree).assert_ok();
}

#[test]
fn code_other_files_and_the_spikes_are_not_checked() {
    let tree = TempTree::git();
    let long = doc_line(130);
    tree.track(
        "a.rs",
        format!("const A: &str = \"{}\";\n", "a ".repeat(70)),
    );
    tree.track("notes.md", format!("{long}\n"));
    tree.track("spikes/tape-spike/src/main.rs", format!("{long}\n"));
    check(&tree).assert_ok();
}

#[test]
fn a_line_that_is_not_utf8_fails_as_unreadable() {
    let tree = TempTree::git();
    tree.track("a.rs", b"fn a() {}\n// caf\xe9\n".as_slice());
    check(&tree).assert_fails_with(&["a.rs:2", "[parse]", "not UTF-8"]);
}

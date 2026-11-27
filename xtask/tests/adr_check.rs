//! `cargo xtask adr-check` as a process against seeded git repositories.
//!
//! CLAUDE.md: decision changes are recorded "as an amendment or a superseding ADR, never as
//! a silent edit". So against a base commit, an accepted ADR may gain dated amendment blocks
//! and change its `status:`, and nothing else.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::{TempTree, xtask};

const ADR: &str = "docs/adr/0042-example.md";

const ACCEPTED: &str = "---
status: accepted
---

# Use the example

> *Accepted 2026-11-27.*

We use the example, because it is simple.

- one rule;
- another rule.
";

/// A repository whose first commit holds `base` at `ADR`.
fn repo_with(base: &str) -> TempTree {
    let tree = TempTree::git();
    tree.track(ADR, base).commit("base");
    tree
}

fn check(tree: &TempTree) -> common::XtaskRun {
    xtask(&[
        "adr-check",
        "--root",
        tree.path().to_str().unwrap(),
        "--base",
        "HEAD",
    ])
}

#[test]
fn an_unchanged_adr_passes() {
    check(&repo_with(ACCEPTED)).assert_ok();
}

#[test]
fn adding_a_dated_amendment_passes() {
    let tree = repo_with(ACCEPTED);
    tree.write(
        ADR,
        ACCEPTED.replace(
            "> *Accepted 2026-11-27.*\n",
            "> *Accepted 2026-11-27.*\n\n> *Amended 2026-12-01 (#20):*\n>\n> - **A change**, with its reason.\n",
        ),
    );
    check(&tree).assert_ok();
}

#[test]
fn editing_the_accepted_text_fails() {
    let tree = repo_with(ACCEPTED);
    tree.write(
        ADR,
        ACCEPTED.replace("because it is simple", "because it is fast"),
    );
    check(&tree).assert_fails_with(&["docs/adr/0042-example.md:9: [adr-silent-edit]", "amendment"]);
}

#[test]
fn adding_text_outside_an_amendment_fails() {
    let tree = repo_with(ACCEPTED);
    tree.write(ADR, format!("{ACCEPTED}- a third rule, slipped in.\n"));
    check(&tree).assert_fails_with(&["docs/adr/0042-example.md:13: [adr-silent-edit]"]);
}

#[test]
fn changing_the_status_passes() {
    let tree = repo_with(ACCEPTED);
    tree.write(
        ADR,
        ACCEPTED.replace("status: accepted", "status: superseded by ADR-0043"),
    );
    check(&tree).assert_ok();
}

#[test]
fn editing_an_existing_amendment_fails() {
    let amended = ACCEPTED.replace(
        "> *Accepted 2026-11-27.*\n",
        "> *Accepted 2026-11-27.*\n\n> *Amended 2026-12-01:* the example is kept.\n",
    );
    let tree = repo_with(&amended);
    tree.write(ADR, amended.replace("is kept", "is dropped"));
    check(&tree).assert_fails_with(&["[adr-silent-edit]"]);
}

#[test]
fn a_proposed_adr_may_be_edited_freely() {
    let proposed = ACCEPTED.replace("status: accepted", "status: proposed");
    let tree = repo_with(&proposed);
    tree.write(
        ADR,
        proposed.replace("because it is simple", "because it is fast"),
    );
    check(&tree).assert_ok();
}

#[test]
fn a_new_adr_passes() {
    let tree = repo_with(ACCEPTED);
    tree.write("docs/adr/0043-next.md", ACCEPTED);
    check(&tree).assert_ok();
}

#[test]
fn deleting_an_accepted_adr_fails() {
    let tree = repo_with(ACCEPTED);
    std::fs::remove_file(tree.path().join(ADR)).unwrap();
    check(&tree).assert_fails_with(&["docs/adr/0042-example.md: [adr-silent-edit]", "deleted"]);
}

/// `ACCEPTED` with amendments dated `dates`, in that order, under the acceptance line.
fn amended_on(dates: &[&str]) -> String {
    let blocks = dates
        .iter()
        .map(|date| format!("\n> *Amended {date}:* a change.\n"))
        .collect::<Vec<_>>()
        .concat();
    ACCEPTED.replace(
        "> *Accepted 2026-11-27.*\n",
        &format!("> *Accepted 2026-11-27.*\n{blocks}"),
    )
}

#[test]
fn amendments_newest_first_pass() {
    let tree = repo_with(ACCEPTED);
    tree.write(ADR, amended_on(&["2026-12-02", "2026-12-01", "2026-12-01"]));
    check(&tree).assert_ok();
}

#[test]
fn an_amendment_below_an_older_one_fails() {
    let tree = repo_with(ACCEPTED);
    tree.write(ADR, amended_on(&["2026-12-01", "2026-12-02"]));
    check(&tree).assert_fails_with(&[
        "docs/adr/0042-example.md:11: [adr-amendment-order]",
        "newest first",
    ]);
}

#[test]
fn amendment_order_is_checked_in_a_new_adr_too() {
    let tree = repo_with(ACCEPTED);
    tree.write(
        "docs/adr/0043-next.md",
        amended_on(&["2026-12-01", "2026-12-02"]),
    );
    check(&tree).assert_fails_with(&["docs/adr/0043-next.md:11: [adr-amendment-order]"]);
}

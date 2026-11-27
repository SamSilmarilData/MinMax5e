//! `cargo xtask leak-scan` as a process against seeded temporary git trees (ADR-0028).

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use common::{TempTree, tag, xtask};

fn scan(tree: &TempTree) -> common::XtaskRun {
    xtask(&["leak-scan", "--root", tree.path().to_str().unwrap()])
}

#[test]
fn a_clean_tree_passes() {
    let tree = TempTree::git();
    tree.track("README.md", "# Hello\n")
        .track("src/lib.rs", "pub fn f() {}\n");
    scan(&tree).assert_ok();
}

#[test]
fn tag_markup_in_a_tracked_file_fails() {
    let tree = TempTree::git();
    tree.track(
        "notes.md",
        format!("Deals {}2d6}} damage.\n", tag("damage ")),
    );
    scan(&tree).assert_fails_with(&["notes.md:1", "tag markup"]);
}

#[test]
fn tag_markup_in_an_allowlisted_file_passes() {
    let tree = TempTree::git();
    tree.track(
        "docs/adr/0028.md",
        format!("Looks for `{}…}}`.\n", tag("tag ")),
    )
    .track(
        "xtask/leak-allow.toml",
        r#"
[[file]]
path = "docs/adr/0028.md"
allow = ["tag-markup"]
reason = "describes the markup the scan looks for"
"#,
    );
    scan(&tree).assert_ok();
}

#[test]
fn an_exemption_covers_only_the_checks_it_names() {
    let tree = TempTree::git();
    tree.track("data.json", format!("{{\"a\": \"{}x}}\"}}\n", tag("b ")))
        .track(
            "xtask/leak-allow.toml",
            "[[file]]\npath = \"data.json\"\nallow = [\"tag-markup\"]\nreason = \"r\"\n",
        );
    scan(&tree).assert_fails_with(&["data.json", "[data-file]"]);
}

#[test]
fn an_exemption_without_a_reason_fails() {
    let tree = TempTree::git();
    tree.track("a.json", "{}\n").track(
        "xtask/leak-allow.toml",
        "[[file]]\npath = \"a.json\"\nallow = [\"data-file\"]\nreason = \"  \"\n",
    );
    scan(&tree).assert_fails_with(&["[allowlist] the exemption for a.json has no reason"]);
}

#[test]
fn an_exemption_that_allows_no_checks_fails() {
    let tree = TempTree::git();
    tree.track("a.json", "{}\n").track(
        "xtask/leak-allow.toml",
        "[[file]]\npath = \"a.json\"\nallow = []\nreason = \"r\"\n",
    );
    scan(&tree).assert_fails_with(&[
        "[allowlist] the exemption for a.json allows no checks",
        "a.json: [data-file]",
    ]);
}

#[test]
fn the_allowlist_check_itself_cannot_be_exempted() {
    let tree = TempTree::git();
    tree.track("a.json", "{}\n").track(
        "xtask/leak-allow.toml",
        "[[file]]\npath = \"a.json\"\nallow = [\"allowlist\"]\nreason = \"r\"\n",
    );
    scan(&tree).assert_fails_with(&["unknown check `allowlist`"]);
}

#[test]
fn an_exemption_for_an_untracked_path_fails() {
    let tree = TempTree::git();
    tree.track("README.md", "hi\n").track(
        "xtask/leak-allow.toml",
        "[[file]]\npath = \"gone.json\"\nallow = [\"data-file\"]\nreason = \"r\"\n",
    );
    scan(&tree).assert_fails_with(&["gone.json matches no tracked file"]);
}

#[test]
fn an_unknown_check_name_in_the_allowlist_is_an_error() {
    let tree = TempTree::git();
    tree.track("a.json", "{}\n").track(
        "xtask/leak-allow.toml",
        "[[file]]\npath = \"a.json\"\nallow = [\"everything\"]\nreason = \"r\"\n",
    );
    scan(&tree).assert_fails_with(&["leak-allow.toml", "everything"]);
}

#[test]
fn tracked_json_pack_and_postcard_files_fail() {
    let tree = TempTree::git();
    tree.track("a.json", "{}\n")
        .track("b.pack", [0u8, 1, 2])
        .track("c.postcard", [3u8]);
    scan(&tree).assert_fails_with(&[
        "a.json: [data-file]",
        "b.pack: [data-file]",
        "c.postcard: [data-file]",
    ]);
}

#[test]
fn untracked_files_are_not_scanned() {
    let tree = TempTree::git();
    tree.track("README.md", "hi\n")
        .write("cache/spells.json", "{}\n");
    scan(&tree).assert_ok();
}

#[test]
fn staged_contents_are_scanned_not_the_working_copy() {
    let tree = TempTree::git();
    tree.track("notes.md", format!("{}x}}\n", tag("spell ")))
        .write("notes.md", "fixed, but not staged\n");
    scan(&tree).assert_fails_with(&["notes.md:1", "[tag-markup]"]);
}

#[test]
fn marked_synthetic_fixtures_with_tst_ids_pass() {
    let tree = TempTree::git();
    tree.track(
        "crates/mm-testkit/fixtures/build.ron",
        "// fixture: synthetic\n(class: \"2024:class/test-warrior|tst\")\n",
    )
    .track(
        "crates/mm-testkit/fixtures/corpus/feats.json",
        "{\"_fixture\": \"synthetic\", \"id\": \"2024:feat/test-heavy-hitter|tst\"}\n",
    )
    .track(
        "crates/mm-testkit/fixtures/README.md",
        "Builds use `2024:class/test-warrior|tst`.\n",
    );
    scan(&tree).assert_ok();
}

#[test]
fn a_ron_fixture_without_the_marker_fails() {
    let tree = TempTree::git();
    tree.track(
        "crates/mm-testkit/fixtures/build.ron",
        "(class: \"2024:class/test-warrior|tst\")\n// fixture: synthetic\n",
    );
    scan(&tree).assert_fails_with(&[
        "fixtures/build.ron: [fixture]",
        "`// fixture: synthetic` as the first line",
    ]);
}

#[test]
fn a_json_fixture_without_the_marker_fails() {
    let tree = TempTree::git();
    tree.track("fixtures/a.json", "{\"name\": \"x\"}\n");
    scan(&tree).assert_fails_with(&[
        "fixtures/a.json: [fixture]",
        "\"_fixture\": \"synthetic\"",
        "fixtures/a.json: [data-file]",
    ]);
}

#[test]
fn a_json_marker_must_be_a_top_level_field() {
    let tree = TempTree::git();
    tree.track(
        "fixtures/a.json",
        "{\"feat\": [{\"_fixture\": \"synthetic\"}]}\n",
    );
    scan(&tree).assert_fails_with(&["fixtures/a.json: [fixture]", "top-level"]);
}

#[test]
fn a_fixture_of_another_file_type_fails() {
    let tree = TempTree::git();
    tree.track("fixtures/notes.txt", "hello\n");
    scan(&tree).assert_fails_with(&["fixtures/notes.txt: [fixture]", "RON or JSON"]);
}

#[test]
fn a_markdown_note_among_fixtures_is_still_checked_for_content_ids() {
    let tree = TempTree::git();
    tree.track(
        "fixtures/README.md",
        "Not `2024:feat/great-weapon-master|xphb`.\n",
    );
    scan(&tree).assert_fails_with(&["fixtures/README.md:1: [fixture]", "source `xphb`"]);
}

#[test]
fn a_fixture_with_a_non_tst_content_id_fails() {
    let tree = TempTree::git();
    tree.track(
        "crates/mm-testkit/fixtures/build.ron",
        "// fixture: synthetic\n(feat: \"2024:feat/great-weapon-master|xphb\")\n",
    );
    scan(&tree).assert_fails_with(&[
        "fixtures/build.ron:2: [fixture]",
        "2024:feat/great-weapon-master|xphb",
        "source `xphb`",
    ]);
}

#[test]
fn content_ids_outside_fixtures_are_not_checked() {
    let tree = TempTree::git();
    tree.track(
        "docs/glossary.md",
        "e.g. `2024:feat/great-weapon-master|xphb`\n",
    );
    scan(&tree).assert_ok();
}

#[test]
fn the_nondeterministic_simd_feature_fails_in_a_tracked_file() {
    let feature = ["relaxed", "simd"].join("-");
    let tree = TempTree::git();
    tree.track(
        ".cargo/config.toml",
        format!("rustflags = [\"-C\", \"target-feature=+simd128,+{feature}\"]\n"),
    );
    scan(&tree).assert_fails_with(&[&format!(".cargo/config.toml:1: [{feature}]")]);
}

#[test]
fn the_nondeterministic_simd_feature_fails_in_an_untracked_file() {
    let feature = ["relaxed", "simd"].join("-");
    let tree = TempTree::git();
    tree.track("README.md", "hi\n")
        .write(".cargo/config.toml", format!("target-feature=+{feature}\n"));
    scan(&tree).assert_fails_with(&[&format!(".cargo/config.toml:1: [{feature}]")]);
}

#[test]
fn a_tracked_file_may_be_exempted_from_the_simd_check_but_an_untracked_one_may_not() {
    let feature = ["relaxed", "simd"].join("-");
    let tree = TempTree::git();
    tree.track("docs/adr/0050.md", format!("Bans `{feature}`.\n"))
        .track(
            "xtask/leak-allow.toml",
            format!(
                "[[file]]\npath = \"docs/adr/0050.md\"\nallow = [\"{feature}\"]\nreason = \"explains the ban\"\n"
            ),
        )
        .write("notes.txt", format!("try +{feature}\n"));
    let run = scan(&tree);
    run.assert_fails_with(&[&format!("notes.txt:1: [{feature}]")]);
    assert!(
        !run.stdout().contains("docs/adr/0050.md"),
        "{}",
        run.stdout()
    );
}

#[test]
fn ignored_files_are_not_scanned() {
    let feature = ["relaxed", "simd"].join("-");
    let tree = TempTree::git();
    tree.track(".gitignore", "/target/\n")
        .write("target/build.log", format!("+{feature}\n"));
    scan(&tree).assert_ok();
}

#[test]
fn plain_simd128_passes() {
    let tree = TempTree::git();
    tree.track(
        ".cargo/config.toml",
        "rustflags = [\"-C\", \"target-feature=+simd128\"]\n",
    );
    scan(&tree).assert_ok();
}

#[test]
fn a_directory_that_is_not_a_git_repository_is_an_error() {
    let tree = TempTree::new();
    let run = scan(&tree);
    assert_eq!(run.0.status.code(), Some(2), "{}", run.stderr());
}

#[test]
fn a_fixture_content_id_without_a_source_fails() {
    let tree = TempTree::git();
    tree.track(
        "crates/mm-testkit/fixtures/build.ron",
        "// fixture: synthetic\n(feat: \"2024:feat/great-weapon-master\")\n",
    );
    scan(&tree).assert_fails_with(&[
        "fixtures/build.ron:2: [fixture]",
        "2024:feat/great-weapon-master has no `|source`",
    ]);
}

#[test]
fn it_warns_when_the_pre_commit_hook_is_not_enabled() {
    let tree = TempTree::git();
    tree.track("README.md", "hi\n");
    let run = scan(&tree);
    run.assert_ok();
    assert!(
        run.stderr().contains("git config core.hooksPath .githooks"),
        "{}",
        run.stderr()
    );
}

#[test]
fn it_is_quiet_about_the_hook_once_it_is_enabled() {
    let tree = TempTree::git();
    tree.track("README.md", "hi\n");
    tree.run_git(&["config", "core.hooksPath", ".githooks"]);
    let run = scan(&tree);
    run.assert_ok();
    assert!(!run.stderr().contains("core.hooksPath"), "{}", run.stderr());
}

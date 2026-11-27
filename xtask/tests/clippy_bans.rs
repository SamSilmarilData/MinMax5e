//! The repository's `clippy.toml` bans the float operations that are not bit-identical
//! across targets (ADR-0037, ADR-0050). A throwaway crate that uses each must fail clippy.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use std::process::Command;

use common::{TempTree, repo_root};

/// Every `disallowed-methods` path in the repository's `clippy.toml`, so a new ban is
/// tested as soon as it is added.
fn banned_paths() -> Vec<String> {
    let text = std::fs::read_to_string(repo_root().join("clippy.toml")).unwrap();
    let config: toml::Table = toml::from_str(&text).unwrap();
    config["disallowed-methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_owned())
        .collect()
}

/// An expression calling the banned method `path` (e.g. `f32::powf`) on `x: f64`.
fn call(path: &str) -> String {
    let (ty, method) = path.split_once("::").unwrap();
    let args = match method {
        "mul_add" => "2.0, 3.0",
        "powi" => "2",
        "powf" | "log" | "max" | "min" | "atan2" | "hypot" => "2.0",
        _ => "",
    };
    format!("(x as {ty}).{method}({args})")
}

#[test]
fn every_ban_covers_both_f32_and_f64() {
    let banned = banned_paths();
    let methods = |ty: &str| -> std::collections::BTreeSet<String> {
        banned
            .iter()
            .filter_map(|path| path.strip_prefix(&format!("{ty}::")).map(str::to_owned))
            .collect()
    };
    assert_eq!(
        methods("f64"),
        methods("f32"),
        "clippy.toml bans differ between f64 and f32"
    );
    assert_eq!(
        banned.len(),
        methods("f64").len() * 2,
        "clippy.toml bans something other than f32/f64 methods: {banned:?}"
    );
}

fn clippy(tree: &TempTree, body: &str) -> std::process::Output {
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"banned\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
    );
    tree.write(
        "src/lib.rs",
        format!("pub fn f(x: f64) -> f64 {{\n{body}\n}}\n"),
    );
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args([
            "clippy",
            "--quiet",
            "--",
            "-D",
            "clippy::disallowed_methods",
        ])
        .current_dir(tree.path())
        .env("CLIPPY_CONF_DIR", repo_root())
        .env("CARGO_TARGET_DIR", tree.path().join("target"))
        .output()
        .unwrap()
}

#[test]
fn each_banned_float_operation_fails_clippy() {
    let tree = TempTree::new();
    let banned = banned_paths();
    assert!(
        banned.len() >= 10,
        "clippy.toml bans too little: {banned:?}"
    );
    let body = banned
        .iter()
        .map(|path| format!("    let _ = {};", call(path)))
        .chain(std::iter::once("    x".to_owned()))
        .collect::<Vec<_>>()
        .join("\n");
    let output = clippy(&tree, &body);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "clippy passed:\n{stderr}");
    for path in &banned {
        assert!(
            stderr.contains(&format!("use of a disallowed method `{path}`")),
            "{path} was not reported:\n{stderr}"
        );
    }
}

#[test]
fn basic_float_arithmetic_passes_clippy() {
    // `sqrt` is an IEEE 754 basic operation, correctly rounded on every target (ADR-0050).
    let tree = TempTree::new();
    let output = clippy(
        &tree,
        "    let y = (x * 2.0 + 3.0).sqrt();\n    if y > x { y } else { x / 2.0 }",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

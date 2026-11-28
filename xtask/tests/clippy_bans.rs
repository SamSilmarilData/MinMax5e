//! The repository's `clippy.toml` bans the float operations that are not bit-identical
//! across targets (ADR-0037, ADR-0050), and the `HashMap` methods that iterate in an order
//! that differs across targets (research 09 errata). A throwaway crate that uses each must
//! fail clippy.

#![allow(
    clippy::unwrap_used,
    reason = "test code: a panic is the failure report"
)]

mod common;

use std::process::Command;

use common::{TempTree, package_manifest, repo_root};

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

/// The prefix of the banned `HashMap` methods' paths.
const HASH_MAP: &str = "std::collections::HashMap::";

/// An expression calling the banned method `path`: a float method (e.g. `f32::powf`) on
/// `x: f64`, or a `HashMap` method on `m: &mut HashMap<u32, u32>`.
fn call(path: &str) -> String {
    if let Some(method) = path.strip_prefix(HASH_MAP) {
        return match method {
            "into_keys" | "into_values" => format!("m.clone().{method}()"),
            _ => format!("m.{method}()"),
        };
    }
    let (ty, method) = path.split_once("::").unwrap();
    let args = match method {
        "mul_add" => "2.0, 3.0",
        "powi" => "2",
        "powf" | "log" | "max" | "min" | "atan2" | "hypot" => "2.0",
        _ => "",
    };
    format!("(x as {ty}).{method}({args})")
}

/// The bans issue #13 requires by name (`sqrt` was withdrawn by the ADR-0050 amendment).
/// `clippy.toml` may ban more, never less.
const REQUIRED: &[&str] = &[
    "f64::mul_add",
    "f64::powf",
    "f64::powi",
    "f64::exp",
    "f64::ln",
    "f64::log",
    "f64::log2",
    "f64::log10",
    "f64::max",
    "f64::min",
];

#[test]
fn the_bans_the_spec_requires_are_all_present() {
    let banned = banned_paths();
    let missing: Vec<_> = REQUIRED
        .iter()
        .filter(|path| !banned.iter().any(|b| b == *path))
        .collect();
    assert!(missing.is_empty(), "clippy.toml no longer bans {missing:?}");
}

#[test]
fn every_float_ban_covers_both_f32_and_f64() {
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
    let hash_map = banned
        .iter()
        .filter(|path| path.starts_with(HASH_MAP))
        .count();
    assert_eq!(
        banned.len(),
        methods("f64").len() * 2 + hash_map,
        "clippy.toml bans something other than f32/f64 or HashMap methods: {banned:?}"
    );
}

fn clippy(tree: &TempTree, body: &str) -> std::process::Output {
    tree.write(
        "Cargo.toml",
        package_manifest("banned", "publish = false\n\n[workspace]\n"),
    );
    tree.write(
        "src/lib.rs",
        format!(
            "pub fn f(x: f64, m: &mut std::collections::HashMap<u32, u32>) -> f64 {{\n{body}\n}}\n"
        ),
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
fn each_banned_method_fails_clippy() {
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
fn basic_float_arithmetic_and_map_lookups_pass_clippy() {
    // `sqrt` is an IEEE 754 basic operation, correctly rounded on every target (ADR-0050).
    // Looking up and inserting do not depend on the map's order.
    let tree = TempTree::new();
    let output = clippy(
        &tree,
        "    m.insert(1, 2);\n    let _ = m.get(&1);\n    \
         let y = (x * 2.0 + 3.0).sqrt();\n    if y > x { y } else { x / 2.0 }",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

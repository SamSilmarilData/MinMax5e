//! Embeds every `.ron` file under `data/rules-2024/` (ADR-0035): writes `encodings.rs`, a
//! list of `(name, include_str!(path))` pairs sorted by name.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let data = manifest.join("../../data/rules-2024");
    println!("cargo::rerun-if-changed={}", data.display());

    let mut files = Vec::new();
    collect(&data, &mut files);
    files.sort();

    let mut out = String::from("&[\n");
    for path in &files {
        let name = path
            .strip_prefix(&data)
            .expect("under the data directory")
            .to_string_lossy()
            .replace('\\', "/");
        let path = path.canonicalize().expect("the file exists");
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            path.to_string_lossy()
        )
        .expect("writing to a String");
    }
    out += "]\n";
    let dest = PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo")).join("encodings.rs");
    fs::write(dest, out).expect("OUT_DIR is writable");
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("data/rules-2024 exists") {
        let path = entry.expect("a readable directory entry").path();
        println!("cargo::rerun-if-changed={}", path.display());
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|e| e == "ron") {
            files.push(path);
        }
    }
}

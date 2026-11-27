//! Reading tracked files from the git index.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// A file in the repository: its path relative to the root, and its contents.
#[derive(Debug)]
pub struct RepoFile {
    pub path: String,
    pub contents: Vec<u8>,
}

/// Every file in the index of the repository at `root`, with its staged contents.
///
/// Staged contents, not the working copy, because they are what a commit records:
/// the pre-commit hook must see exactly that.
pub fn tracked_files(root: &Path) -> Result<Vec<RepoFile>> {
    let listing = Command::new("git")
        .args(["ls-files", "--stage", "-z"])
        .current_dir(root)
        .output()
        .context("running git ls-files")?;
    if !listing.status.success() {
        bail!(
            "git ls-files failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&listing.stderr).trim()
        );
    }

    // Each entry is "<mode> <object> <stage>\t<path>\0".
    let mut entries = Vec::new();
    for entry in listing.stdout.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let (meta, path) = entry
            .split_once('\t')
            .context("unexpected git ls-files output")?;
        let mut fields = meta.split(' ');
        let mode = fields.next().unwrap_or_default();
        let object = fields.next().context("unexpected git ls-files output")?;
        if mode == "160000" {
            continue; // a submodule has no blob
        }
        entries.push((object.to_owned(), path.to_owned()));
    }
    // A conflicted path appears once per stage; scan each blob, report the path once per hit.
    entries.dedup();

    let blobs = read_blobs(root, entries.iter().map(|(object, _)| object.as_str()))?;
    Ok(entries
        .into_iter()
        .zip(blobs)
        .map(|((_, path), contents)| RepoFile { path, contents })
        .collect())
}

fn read_blobs<'a>(root: &Path, objects: impl Iterator<Item = &'a str>) -> Result<Vec<Vec<u8>>> {
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("running git cat-file")?;

    let mut request = String::new();
    for object in objects {
        request.push_str(object);
        request.push('\n');
    }
    let count = request.lines().count();
    let mut stdin = child.stdin.take().context("git cat-file stdin")?;
    // Written from a thread so a full stdout pipe cannot deadlock us.
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));

    let mut out = BufReader::new(child.stdout.take().context("git cat-file stdout")?);
    let mut blobs = Vec::with_capacity(count);
    for _ in 0..count {
        // Header: "<object> <type> <size>\n", then the contents, then "\n".
        let mut header = String::new();
        out.read_line(&mut header)?;
        let size: usize = header
            .trim_end()
            .rsplit(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .with_context(|| format!("unexpected git cat-file header {header:?}"))?;
        let mut contents = vec![0; size];
        out.read_exact(&mut contents)?;
        let mut newline = [0; 1];
        out.read_exact(&mut newline)?;
        blobs.push(contents);
    }

    writer
        .join()
        .map_err(|_| anyhow::anyhow!("git cat-file writer panicked"))??;
    if !child.wait()?.success() {
        bail!("git cat-file failed");
    }
    Ok(blobs)
}

/// Files in the working tree that git neither tracks nor ignores, with their contents.
pub fn untracked_files(root: &Path) -> Result<Vec<RepoFile>> {
    let listing = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(root)
        .output()
        .context("running git ls-files --others")?;
    if !listing.status.success() {
        bail!(
            "git ls-files --others failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&listing.stderr).trim()
        );
    }
    let mut files = Vec::new();
    for path in listing.stdout.split(|&b| b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(path).into_owned();
        // A file can vanish between listing and reading; skip it rather than fail.
        if let Ok(contents) = std::fs::read(root.join(&path)) {
            files.push(RepoFile { path, contents });
        }
    }
    Ok(files)
}

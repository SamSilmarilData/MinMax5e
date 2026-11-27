//! Helpers for running `xtask` as a process against seeded temporary trees.

#![allow(dead_code, reason = "each test binary uses a different subset")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A temporary directory, deleted on drop.
pub struct TempTree {
    root: PathBuf,
}

impl TempTree {
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("xtask-test-{}-{n}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    /// A git repository; files written with `track` are added to the index.
    pub fn git() -> Self {
        let tree = Self::new();
        tree.run_git(&["init", "--quiet"]);
        tree
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> &Self {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
        self
    }

    /// Writes a file and adds it to the git index.
    pub fn track(&self, rel: &str, contents: impl AsRef<[u8]>) -> &Self {
        self.write(rel, contents);
        self.run_git(&["add", "--force", rel]);
        self
    }

    pub fn run_git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The repository root: the parent of the xtask crate's directory.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Runs the xtask binary with the given arguments.
pub fn xtask(args: &[&str]) -> XtaskRun {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .unwrap();
    XtaskRun(output)
}

/// The exit status and output of one `xtask` process.
pub struct XtaskRun(pub Output);

impl XtaskRun {
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.0.stdout).into_owned()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.0.stderr).into_owned()
    }

    #[track_caller]
    pub fn assert_ok(&self) -> &Self {
        assert!(
            self.0.status.success(),
            "expected success\n--- stdout\n{}\n--- stderr\n{}",
            self.stdout(),
            self.stderr()
        );
        self
    }

    /// Asserts failure and that the combined output contains every needle.
    #[track_caller]
    pub fn assert_fails_with(&self, needles: &[&str]) -> &Self {
        let all = format!("{}{}", self.stdout(), self.stderr());
        assert!(!self.0.status.success(), "expected failure\n{all}");
        for needle in needles {
            assert!(all.contains(needle), "output lacks {needle:?}\n{all}");
        }
        self
    }
}

/// `{@` built at runtime, so this file never contains 5e.tools tag markup itself.
pub fn tag(name_and_rest: &str) -> String {
    format!("{{{}{name_and_rest}", '@')
}

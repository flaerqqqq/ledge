//! TempTree: a named folder under CARGO_TARGET_TMPDIR, removed on drop (docs/testing.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

/// A fresh, empty folder for one test. Tests pass distinct names, so folders never clash.
pub struct TempTree {
    path: PathBuf,
}

impl TempTree {
    /// Creates `CARGO_TARGET_TMPDIR/<name>` empty, replacing anything a previous run left behind.
    pub fn new(name: &str) -> TempTree {
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp folder");
        TempTree { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

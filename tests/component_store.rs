//! Component tests for the history and bookmark files through `RealFs` on temp folders
//! (docs/testing.md level 2). The cache folder comes from `Dirs`, as in the real program.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::fs::RealFs;
use ledge::app::store::{load_bookmarks, load_history, save_bookmarks, save_history};
use ledge::domain::bookmarks::Bookmarks;
use ledge::domain::history::History;
use ledge::domain::location::Dirs;
use ledge::domain::path::AbsPath;
use std::fs;
use std::path::Path;
use support::temp::TempTree;

/// Dirs with the XDG cache base set to `base`, so the cache folder is `base/ledge` (architecture 9.1).
/// The config folder is unused by the store.
fn dirs_with_cache(base: &Path) -> Dirs {
    Dirs::resolve(
        Some(base.join("config").to_str().expect("utf-8 temp path")),
        Some(base.to_str().expect("utf-8 temp path")),
        None,
    )
    .expect("dirs resolve")
}

/// The cache folder `base/ledge` for a test whose XDG cache base is `base`.
fn cache_folder(base: &Path) -> std::path::PathBuf {
    base.join("ledge")
}

/// Writes a cache file as a hand-edited file would be, creating the cache folder first.
fn seed_cache(base: &Path, name: &str, text: impl AsRef<[u8]>) {
    let folder = cache_folder(base);
    fs::create_dir_all(&folder).expect("create cache folder");
    fs::write(folder.join(name), text).expect("seed cache file");
}

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

fn history_of(dirs: &[&str]) -> History {
    let mut history = History::default();
    for dir in dirs {
        history.add(abs(dir));
    }
    history
}

// FR-HIST-01, architecture 9.4: a missing history file loads as an empty history
#[test]
fn missing_history_file_loads_as_empty() {
    let tree = TempTree::new("component_store-history-missing");
    let dirs = dirs_with_cache(tree.path());
    let history = load_history(&RealFs, &dirs).expect("load");
    assert!(history.entries().is_empty());
}

// FR-HIST-01, FR-HIST-02: saved history reads back newest first, with a re-opened directory moved
// to the top and listed once
#[test]
fn saved_history_reads_back_newest_first_without_duplicates() {
    let tree = TempTree::new("component_store-history-roundtrip");
    let dirs = dirs_with_cache(tree.path());
    let history = history_of(&["/p/a", "/p/b", "/p/c", "/p/a"]);
    save_history(&RealFs, &dirs, &history).expect("save");
    assert_eq!(
        fs::read_to_string(cache_folder(tree.path()).join("history")).expect("read file"),
        "/p/a\n/p/c\n/p/b\n"
    );
    assert_eq!(load_history(&RealFs, &dirs).expect("load"), history);
}

// FR-HIST-01: a 51st directory pushes out the oldest, and the file holds 50 lines
#[test]
fn saved_history_holds_at_most_fifty_lines() {
    let tree = TempTree::new("component_store-history-limit");
    let dirs = dirs_with_cache(tree.path());
    let mut history = History::default();
    for index in 0..51 {
        history.add(abs(&format!("/p/{index}")));
    }
    save_history(&RealFs, &dirs, &history).expect("save");
    let text = fs::read_to_string(cache_folder(tree.path()).join("history")).expect("read file");
    assert_eq!(text.lines().count(), 50);
    assert!(!text.lines().any(|line| line == "/p/0"));
}

// F-36: a history file longer than 50 lines loads as the newest 50, and the file is not rewritten
#[test]
fn longer_history_file_is_trimmed_on_read_and_left_unchanged() {
    let tree = TempTree::new("component_store-history-long");
    let dirs = dirs_with_cache(tree.path());
    let original: String = (0..60).map(|index| format!("/p/{index}\n")).collect();
    seed_cache(tree.path(), "history", &original);
    let history = load_history(&RealFs, &dirs).expect("load");
    assert_eq!(history.entries().len(), 50);
    assert_eq!(history.entries().last().map(AbsPath::as_str), Some("/p/49"));
    assert_eq!(
        fs::read_to_string(cache_folder(tree.path()).join("history")).expect("read file"),
        original
    );
}

// A-09, D-06: relative lines are skipped and repeated lines keep the first occurrence on read
#[test]
fn history_file_is_cleaned_on_read() {
    let tree = TempTree::new("component_store-history-clean");
    let dirs = dirs_with_cache(tree.path());
    seed_cache(tree.path(), "history", "/p/a\nrelative/x\n/p/b/\n/p/a\n");
    let history = load_history(&RealFs, &dirs).expect("load");
    let texts: Vec<&str> = history.entries().iter().map(AbsPath::as_str).collect();
    assert_eq!(texts, vec!["/p/a", "/p/b"]);
}

// D-05: saving over a longer old history leaves exactly the new text
#[test]
fn save_history_replaces_old_contents_completely() {
    let tree = TempTree::new("component_store-history-replace");
    let dirs = dirs_with_cache(tree.path());
    seed_cache(tree.path(), "history", "/old/one\n/old/two\n/old/three\n");
    save_history(&RealFs, &dirs, &history_of(&["/new"])).expect("save");
    assert_eq!(
        fs::read_to_string(cache_folder(tree.path()).join("history")).expect("read file"),
        "/new\n"
    );
}

// D-05: a write that fails at the temp file step leaves the old history file as it was. The temp
// file name is fixed by architecture 9.3 (`.<name>.tmp-<pid>`); a folder at that name makes the
// temp file cannot be created, so the write fails before the target is touched.
#[test]
fn failed_history_write_leaves_old_file_unchanged() {
    let tree = TempTree::new("component_store-history-failed-write");
    let dirs = dirs_with_cache(tree.path());
    save_history(&RealFs, &dirs, &history_of(&["/old"])).expect("first save");
    let temp = cache_folder(tree.path()).join(format!(".history.tmp-{}", std::process::id()));
    fs::create_dir(&temp).expect("block the temp file name");
    assert!(save_history(&RealFs, &dirs, &history_of(&["/new"])).is_err());
    assert_eq!(
        fs::read_to_string(cache_folder(tree.path()).join("history")).expect("read file"),
        "/old\n"
    );
}

// FR-BM-01: bookmarks are written one per line in stored order, and read back in that order
#[test]
fn saved_bookmarks_keep_insertion_order() {
    let tree = TempTree::new("component_store-bookmarks-order");
    let dirs = dirs_with_cache(tree.path());
    let bookmarks = Bookmarks::parse("/p/z\n/p/a\n/p/m\n");
    save_bookmarks(&RealFs, &dirs, &bookmarks).expect("save");
    assert_eq!(
        fs::read_to_string(cache_folder(tree.path()).join("bookmarks")).expect("read file"),
        "/p/z\n/p/a\n/p/m\n"
    );
    assert_eq!(load_bookmarks(&RealFs, &dirs).expect("load"), bookmarks);
}

// architecture 9.4: a missing bookmarks file loads as an empty list
#[test]
fn missing_bookmarks_file_loads_as_empty() {
    let tree = TempTree::new("component_store-bookmarks-missing");
    let dirs = dirs_with_cache(tree.path());
    let bookmarks = load_bookmarks(&RealFs, &dirs).expect("load");
    assert!(bookmarks.entries().is_empty());
}

// FR-BM-01, A-09, D-06: a hand-edited bookmarks file keeps its order, skips relative lines and
// keeps the first of duplicate paths
#[test]
fn hand_edited_bookmarks_file_is_cleaned_on_read() {
    let tree = TempTree::new("component_store-bookmarks-clean");
    let dirs = dirs_with_cache(tree.path());
    seed_cache(tree.path(), "bookmarks", "/p/b\nrelative\n/p/a\n/p/b/\n");
    let bookmarks = load_bookmarks(&RealFs, &dirs).expect("load");
    let texts: Vec<&str> = bookmarks.entries().iter().map(AbsPath::as_str).collect();
    assert_eq!(texts, vec!["/p/b", "/p/a"]);
}

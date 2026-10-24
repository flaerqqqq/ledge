//! Component tests for the bookmark edit window state and listing (docs/testing.md level 2): the `bm-edit-open`
//! file through `RealFs` on temp folders, and the folder listing of a project on the real file system.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::adapters::fs::RealFs;
use ledge::app::store::{load_expanded, save_expanded};
use ledge::domain::bm_edit::{visible_rows, ExpandedFolders};
use ledge::domain::bookmarks::Bookmarks;
use ledge::domain::location::Dirs;
use ledge::domain::path::AbsPath;
use ledge::ports::fs::FileSystem;
use std::fs;
use std::path::{Path, PathBuf};
use support::temp::TempTree;

mod support;

/// Dirs with the XDG cache base set to `base`, so the cache folder is `base/ledge` (architecture 9.1).
fn dirs_with_cache(base: &Path) -> Dirs {
    Dirs::resolve(
        Some(base.join("config").to_str().expect("utf-8 temp path")),
        Some(base.to_str().expect("utf-8 temp path")),
        None,
    )
    .expect("dirs resolve")
}

fn cache_folder(base: &Path) -> PathBuf {
    base.join("ledge")
}

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

/// Writes a file in the cache folder as a hand-edited file would be.
fn seed_cache(base: &Path, name: &str, text: &str) {
    fs::create_dir_all(cache_folder(base)).expect("create cache folder");
    fs::write(cache_folder(base).join(name), text).expect("seed cache file");
}

fn expanded_file_text(base: &Path) -> String {
    fs::read_to_string(cache_folder(base).join("bm-edit-open")).expect("read bm-edit-open")
}

fn folder(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create folder");
}

/// The child folder names of `dir` on the real file system (what the edit window lists).
fn real_children(dir: &AbsPath) -> Vec<String> {
    RealFs
        .list_dirs(Path::new(dir.as_str()))
        .unwrap_or_default()
}

// FR-EDIT-05: a missing bm-edit-open file loads as no folder expanded
#[test]
fn missing_expanded_file_loads_as_nothing_expanded() {
    let tree = TempTree::new("component_edit-missing");
    let dirs = dirs_with_cache(tree.path());
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    assert!(expanded.entries().is_empty());
}

// FR-EDIT-05: the expanded folders saved by one run are the ones the next run loads
#[test]
fn expanded_folders_persist_between_runs() {
    let tree = TempTree::new("component_edit-persist");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src");
    let app = format!("{}/app", tree.path().display());
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    expanded.expand(abs(&format!("{app}/src")));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(
        expanded_file_text(tree.path()),
        format!("{app}\n{app}/src\n")
    );
    assert_eq!(load_expanded(&RealFs, &dirs).expect("load"), expanded);
}

// FR-EDIT-05, D-09: when the file is written, folders that no longer exist are dropped and the others kept
#[test]
fn save_prunes_folders_that_no_longer_exist() {
    let tree = TempTree::new("component_edit-prune");
    let dirs = dirs_with_cache(tree.path());
    let project = tree.path().join("app");
    folder(tree.path(), "app/src");
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(project.to_str().expect("utf-8")));
    expanded.expand(abs(&format!("{}/src", project.display())));
    expanded.expand(abs(&format!("{}/gone", project.display())));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(
        expanded_file_text(tree.path()),
        format!("{0}\n{0}/src\n", project.display())
    );
}

// D-09: collapsing a folder removes only its entry; the descendants stay expanded for the next time
#[test]
fn collapsed_parent_keeps_expanded_descendants_after_save() {
    let tree = TempTree::new("component_edit-collapse");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src/main");
    let app = format!("{}/app", tree.path().display());
    let src = format!("{app}/src");
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    expanded.expand(abs(&src));
    expanded.collapse(&abs(&app));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(expanded_file_text(tree.path()), format!("{src}\n"));
    assert!(load_expanded(&RealFs, &dirs)
        .expect("load")
        .is_expanded(&abs(&src)));
}

// FR-EDIT-05, D-05, architecture 9.3: saving over an older file leaves exactly the new text and no temp file
#[test]
fn save_replaces_old_text_and_leaves_no_temp_file() {
    let tree = TempTree::new("component_edit-atomic");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app");
    let app = format!("{}/app", tree.path().display());
    seed_cache(
        tree.path(),
        "bm-edit-open",
        "/old/one\n/old/two\n/old/three\n",
    );
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(expanded_file_text(tree.path()), format!("{app}\n"));
    let names = RealFs.list_files(&cache_folder(tree.path())).expect("list");
    assert_eq!(names, vec!["bm-edit-open".to_string()]);
}

// A-09, D-06: a hand-edited file with relative lines, a trailing slash and a repeat is cleaned on read
#[test]
fn hand_edited_expanded_file_is_cleaned_on_read() {
    let tree = TempTree::new("component_edit-clean");
    let dirs = dirs_with_cache(tree.path());
    seed_cache(tree.path(), "bm-edit-open", "app/src\n/p/app/\n/p/app\n");
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let texts: Vec<&str> = expanded.entries().iter().map(AbsPath::as_str).collect();
    assert_eq!(texts, vec!["/p/app"]);
}

// FR-EDIT-01, FR-EDIT-04: the listing of a real project shows its subfolders in name order, skips hidden
// folders, lists the children of expanded folders only, and marks them "+" or "-" (a leaf has no marker)
#[test]
fn real_listing_shows_visible_subfolders_with_markers() {
    let tree = TempTree::new("component_edit-listing");
    let dirs = dirs_with_cache(tree.path());
    let base = tree.path();
    folder(base, "app/src/main");
    folder(base, "app/docs");
    folder(base, "app/.git");
    fs::write(base.join("app/README.md"), "text").expect("write file");
    let app = abs(&format!("{}/app", base.display()));
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&format!("{}/src", app.as_str())));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let bookmarks = Bookmarks::parse(&format!("{}/app/docs\n", base.display()));

    let rows = visible_rows(&app, &expanded, &bookmarks, &real_children);
    let shown: Vec<(&str, &str, bool)> = rows
        .iter()
        .map(|row| (row.name.as_str(), row.marker.text(), row.bookmarked))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("app", "-", false),
            ("docs", "", true),
            ("src", "-", false),
            ("main", "", false),
        ]
    );
}

// FR-EDIT-04: a folder with subfolders that is not expanded shows "+"
#[test]
fn collapsed_folder_with_subfolders_shows_plus() {
    let tree = TempTree::new("component_edit-plus");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src/main");
    let app = abs(&format!("{}/app", tree.path().display()));
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let rows = visible_rows(&app, &expanded, &Bookmarks::default(), &real_children);
    let shown: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.name.as_str(), row.marker.text()))
        .collect();
    assert_eq!(shown, vec![("app", "-"), ("src", "+")]);
}

// FR-EDIT-11: a project without subfolders gives no rows (the window does not open); hidden folders do not count
#[test]
fn project_without_visible_subfolders_gives_no_rows() {
    let tree = TempTree::new("component_edit-empty");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/.git");
    fs::write(tree.path().join("app/main.rs"), "fn main() {}").expect("write file");
    let app = abs(&format!("{}/app", tree.path().display()));
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    assert!(visible_rows(&app, &expanded, &Bookmarks::default(), &real_children).is_empty());
}

//! Component tests for `adapters::fs::RealFs` on temp folders (docs/testing.md level 2).
//! Unit tests cannot use CARGO_TARGET_TMPDIR, so the adapter tests live here. D-05 and
//! architecture 9.3 define the behaviour; no FR-ID owns the atomic write in S-01.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::fs::RealFs;
use ledge::ports::fs::FileSystem;
use std::fs;
use std::path::Path;
use support::temp::TempTree;

/// Names of the entries directly inside `dir`, sorted.
fn entry_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

// D-05: a missing file is written with exactly the given text
#[test]
fn write_atomic_creates_missing_file_with_exact_text() {
    let tree = TempTree::new("component_fs-write-creates");
    let target = tree.path().join("history");
    RealFs.write_atomic(&target, "/a\n/b\n").expect("write");
    assert_eq!(fs::read_to_string(&target).expect("read back"), "/a\n/b\n");
}

// D-05: an existing file is replaced completely, with no tail of the old text left behind
#[test]
fn write_atomic_replaces_longer_old_contents_completely() {
    let tree = TempTree::new("component_fs-write-replaces");
    let target = tree.path().join("bookmarks");
    fs::write(&target, "/a/very/long/old/line\n/another/old/line\n").expect("seed");
    RealFs.write_atomic(&target, "/new\n").expect("write");
    assert_eq!(fs::read_to_string(&target).expect("read back"), "/new\n");
}

// architecture 9.3 step 1: the parent folder is created when it is missing
#[test]
fn write_atomic_creates_missing_parent_folder() {
    let tree = TempTree::new("component_fs-write-parent");
    let target = tree.path().join("cache").join("ledge").join("history");
    RealFs.write_atomic(&target, "/a\n").expect("write");
    assert_eq!(fs::read_to_string(&target).expect("read back"), "/a\n");
}

// architecture 9.3 step 4: the temp file is renamed away, so only the target remains
#[test]
fn write_atomic_leaves_only_the_target_in_its_folder() {
    let tree = TempTree::new("component_fs-write-no-temp");
    let target = tree.path().join("history");
    RealFs.write_atomic(&target, "/a\n").expect("write");
    let names: Vec<String> = fs::read_dir(tree.path())
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, vec!["history".to_string()]);
}

// architecture 9.3 step 4, D-05: when the rename fails the temp file is removed and the error is
// returned. rename(2) cannot replace a folder with a file, so a folder at the target forces it.
#[test]
fn write_atomic_removes_temp_file_when_rename_fails() {
    let tree = TempTree::new("component_fs-write-rename-fails");
    let target = tree.path().join("history");
    fs::create_dir(&target).expect("seed folder at the target");
    assert!(RealFs.write_atomic(&target, "/a\n").is_err());
    assert_eq!(entry_names(tree.path()), vec!["history".to_string()]);
    assert!(target.is_dir());
}

// architecture 4: read_text gives None for a missing file, not an error
#[test]
fn read_text_returns_none_for_missing_file() {
    let tree = TempTree::new("component_fs-read-missing");
    let target = tree.path().join("history");
    assert_eq!(RealFs.read_text(&target).expect("read"), None);
}

// D-05: config.json is created when it does not exist
#[test]
fn create_new_writes_missing_file_and_reports_true() {
    let tree = TempTree::new("component_fs-create-new-missing");
    let target = tree.path().join("config.json");
    let created = RealFs.create_new(&target, "{}\n").expect("create");
    assert!(created);
    assert_eq!(fs::read_to_string(&target).expect("read back"), "{}\n");
}

// D-05: an existing config.json is never rewritten, and the call reports false
#[test]
fn create_new_never_rewrites_existing_file() {
    let tree = TempTree::new("component_fs-create-new-existing");
    let target = tree.path().join("config.json");
    fs::write(&target, "{\"search_paths\": [\"/mine\"]}").expect("seed");
    let created = RealFs.create_new(&target, "{}\n").expect("create");
    assert!(!created);
    assert_eq!(
        fs::read_to_string(&target).expect("read back"),
        "{\"search_paths\": [\"/mine\"]}"
    );
}

// architecture 4: list_dirs gives the sorted names of the folders directly inside, not files
#[test]
fn list_dirs_returns_sorted_folder_names_only() {
    let tree = TempTree::new("component_fs-list-dirs");
    fs::create_dir(tree.path().join("beta")).expect("mkdir");
    fs::create_dir(tree.path().join("alpha")).expect("mkdir");
    fs::write(tree.path().join("file"), "x").expect("write");
    assert_eq!(
        RealFs.list_dirs(tree.path()).expect("list"),
        vec!["alpha".to_string(), "beta".to_string()]
    );
}

// architecture 4: list_files gives the sorted names of the files directly inside, not folders
#[test]
fn list_files_returns_sorted_file_names_only() {
    let tree = TempTree::new("component_fs-list-files");
    fs::write(tree.path().join("b.txt"), "x").expect("write");
    fs::write(tree.path().join("a.txt"), "x").expect("write");
    fs::create_dir(tree.path().join("folder")).expect("mkdir");
    assert_eq!(
        RealFs.list_files(tree.path()).expect("list"),
        vec!["a.txt".to_string(), "b.txt".to_string()]
    );
}

// conventions (Structure): names that are not UTF-8 are skipped by both listings
#[cfg(unix)]
#[test]
fn list_skips_non_utf8_names() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let tree = TempTree::new("component_fs-list-non-utf8");
    let bad = OsStr::from_bytes(b"bad\xff");
    fs::write(tree.path().join("ok.txt"), "x").expect("write");
    fs::write(tree.path().join(bad), "x").expect("write non-UTF-8 file");
    fs::create_dir(tree.path().join("folder")).expect("mkdir");
    let bad_dir = OsStr::from_bytes(b"dir\xfe");
    fs::create_dir(tree.path().join(bad_dir)).expect("mkdir non-UTF-8 folder");
    assert_eq!(
        RealFs.list_files(tree.path()).expect("list files"),
        vec!["ok.txt".to_string()]
    );
    assert_eq!(
        RealFs.list_dirs(tree.path()).expect("list dirs"),
        vec!["folder".to_string()]
    );
}

// D-06, F-24: symbolic links are followed by the listings, is_dir and exists
#[cfg(unix)]
#[test]
fn symlinks_are_followed_by_listings_is_dir_and_exists() {
    use std::os::unix::fs::symlink;
    let tree = TempTree::new("component_fs-symlinks");
    fs::create_dir(tree.path().join("real-dir")).expect("mkdir");
    fs::write(tree.path().join("real-file"), "x").expect("write");
    symlink(tree.path().join("real-dir"), tree.path().join("link-dir")).expect("symlink dir");
    symlink(tree.path().join("real-file"), tree.path().join("link-file")).expect("symlink file");
    symlink(tree.path().join("nowhere"), tree.path().join("dangling")).expect("symlink dangling");

    assert_eq!(
        RealFs.list_dirs(tree.path()).expect("list dirs"),
        vec!["link-dir".to_string(), "real-dir".to_string()]
    );
    assert_eq!(
        RealFs.list_files(tree.path()).expect("list files"),
        vec!["link-file".to_string(), "real-file".to_string()]
    );
    assert!(RealFs.is_dir(&tree.path().join("link-dir")));
    assert!(!RealFs.is_dir(&tree.path().join("link-file")));
    assert!(RealFs.exists(&tree.path().join("link-file")));
    assert!(!RealFs.exists(&tree.path().join("dangling")));
}

// architecture 4: is_dir and exists tell folders, files and missing paths apart
#[test]
fn is_dir_and_exists_tell_folders_files_and_missing_paths_apart() {
    let tree = TempTree::new("component_fs-is-dir-exists");
    fs::create_dir(tree.path().join("folder")).expect("mkdir");
    fs::write(tree.path().join("file"), "x").expect("write");
    assert!(RealFs.is_dir(&tree.path().join("folder")));
    assert!(!RealFs.is_dir(&tree.path().join("file")));
    assert!(!RealFs.is_dir(&tree.path().join("missing")));
    assert!(RealFs.exists(&tree.path().join("folder")));
    assert!(RealFs.exists(&tree.path().join("file")));
    assert!(!RealFs.exists(&tree.path().join("missing")));
}

// architecture 4: create_dir_all creates nested folders and accepts a folder that exists
#[test]
fn create_dir_all_creates_nested_folders_and_accepts_existing_ones() {
    let tree = TempTree::new("component_fs-create-dir-all");
    let nested = tree.path().join("a").join("b").join("c");
    RealFs.create_dir_all(&nested).expect("create");
    assert!(nested.is_dir());
    RealFs
        .create_dir_all(&nested)
        .expect("second call on an existing folder succeeds");
}

// architecture 4: remove_file deletes the file; a missing file is an error
#[test]
fn remove_file_deletes_the_file_and_fails_for_missing_file() {
    let tree = TempTree::new("component_fs-remove-file");
    let target = tree.path().join("bookmarks");
    fs::write(&target, "/a\n").expect("seed");
    RealFs.remove_file(&target).expect("remove");
    assert!(!target.exists());
    assert!(RealFs.remove_file(&target).is_err());
}

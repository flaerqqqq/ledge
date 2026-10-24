//! Component tests for the bookmark commands through `cli::run` over the harness of tests/support (docs/testing.md
//! level 2). The bookmarks file is real, in a temp cache folder; the search path is `<root>/projects`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::cli::run;
use ledge::domain::path::AbsPath;
use ledge::error::{LedgeError, Outcome, Result};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn info(result: Result<Outcome>) -> String {
    match result {
        Ok(Outcome::Info(text)) => text,
        other => panic!("expected info output, got {other:?}"),
    }
}

fn fatal(result: Result<Outcome>) -> String {
    match result {
        Err(LedgeError::Fatal(text)) => text,
        other => panic!("expected a fatal error, got {other:?}"),
    }
}

/// A harness whose only search path is `<root>/projects` (the working directory is inside it, not a project).
fn harness(name: &str) -> Harness {
    let h = Harness::new(name);
    let search = format!("{}/projects", h.root().display());
    h.write_config(&format!("{{\"search_paths\": [\"{search}\"]}}"));
    h
}

/// `<root>/<rel>`, created as a folder.
fn folder(h: &Harness, rel: &str) -> String {
    let path = format!("{}/{rel}", h.root().display());
    fs::create_dir_all(&path).expect("create folder");
    path
}

/// The bookmarks file text, or `None` when there is no file.
fn bookmarks_text(h: &Harness) -> Option<String> {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")).ok()
}

/// Writes the bookmarks file, as a hand-edited file would be.
fn write_bookmarks(h: &Harness, text: &str) {
    let folder = PathBuf::from(h.dirs.cache.as_str());
    fs::create_dir_all(&folder).expect("create cache folder");
    fs::write(folder.join("bookmarks"), text).expect("write bookmarks");
}

// FR-BM-05: a directory inside a project is bookmarked and the file gains its path
#[test]
fn add_inside_project_prints_bookmarked_and_stores_the_path() {
    let h = harness("bm-add-inside");
    let dir = folder(&h, "projects/app/src");
    assert_eq!(
        info(run(&args(&["bm", "add", &dir]), &h.ctx())),
        format!("Bookmarked {dir}")
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{dir}\n").as_str())
    );
}

// FR-BM-07: without a directory the working directory is bookmarked
#[test]
fn add_without_directory_bookmarks_the_working_directory() {
    let mut h = harness("bm-add-cwd");
    let dir = folder(&h, "projects/app/src");
    h.cwd = AbsPath::clean(&dir, &AbsPath::root());
    assert_eq!(
        info(run(&args(&["bm", "add"]), &h.ctx())),
        format!("Bookmarked {dir}")
    );
}

// D-06, FR-BM-05: a relative directory is cleaned against the working directory, and a trailing slash is dropped
#[test]
fn add_cleans_relative_and_trailing_slash_paths() {
    let mut h = harness("bm-add-clean");
    let app = folder(&h, "projects/app");
    folder(&h, "projects/app/src");
    h.cwd = AbsPath::clean(&app, &AbsPath::root());
    assert_eq!(
        info(run(&args(&["bm", "add", "src/"]), &h.ctx())),
        format!("Bookmarked {app}/src")
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{app}/src\n").as_str())
    );
}

// FR-BM-06: adding a bookmark that exists prints "Already bookmarked" and changes nothing
#[test]
fn add_of_existing_bookmark_changes_nothing() {
    let h = harness("bm-add-twice");
    let dir = folder(&h, "projects/app/src");
    run(&args(&["bm", "add", &dir]), &h.ctx()).expect("first add");
    let before = bookmarks_text(&h);
    assert_eq!(
        info(run(&args(&["bm", "add", &dir]), &h.ctx())),
        format!("Already bookmarked: {dir}")
    );
    assert_eq!(bookmarks_text(&h), before);
}

// FR-BM-01, FR-BM-05: new bookmarks are appended, so the file keeps insertion order
#[test]
fn add_appends_to_the_stored_order() {
    let h = harness("bm-add-order");
    let zed = folder(&h, "projects/zed/src");
    let app = folder(&h, "projects/app/src");
    run(&args(&["bm", "add", &zed]), &h.ctx()).expect("add zed");
    run(&args(&["bm", "add", &app]), &h.ctx()).expect("add app");
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{zed}\n{app}\n").as_str())
    );
}

// FR-BM-04: a project itself is refused with the picker hint
#[test]
fn add_of_a_project_is_refused() {
    let h = harness("bm-add-project");
    let app = folder(&h, "projects/app");
    assert_eq!(
        fatal(run(&args(&["bm", "add", &app]), &h.ctx())),
        format!("{app} is a project; open it with 'ledge project pick'.")
    );
    assert!(bookmarks_text(&h).is_none());
}

// FR-BM-03: a directory outside every project is refused
#[test]
fn add_outside_every_project_is_refused() {
    let h = harness("bm-add-outside");
    let outside = folder(&h, "outside/x");
    assert_eq!(
        fatal(run(&args(&["bm", "add", &outside]), &h.ctx())),
        format!(
            "{outside} is not inside a project. Bookmarks go inside a directory from search_paths."
        )
    );
    assert!(bookmarks_text(&h).is_none());
}

// FR-BM-03: the search path itself is not inside a project, so it is refused
#[test]
fn add_of_the_search_path_is_refused_as_outside() {
    let h = harness("bm-add-search-root");
    let search = folder(&h, "projects");
    assert_eq!(
        fatal(run(&args(&["bm", "add", &search]), &h.ctx())),
        format!(
            "{search} is not inside a project. Bookmarks go inside a directory from search_paths."
        )
    );
}

// Architecture A-19: a missing directory echoes the argument as typed
#[test]
fn add_of_missing_directory_names_it_as_typed() {
    let h = harness("bm-add-missing");
    folder(&h, "projects/app");
    let typed = format!("{}/projects/app/nope", h.root().display());
    assert_eq!(
        fatal(run(&args(&["bm", "add", &typed]), &h.ctx())),
        format!("Directory '{typed}' not found.")
    );
}

// FR-BM-07: at most one directory for bm add
#[test]
fn add_with_two_directories_is_refused() {
    let h = harness("bm-add-two");
    assert_eq!(
        fatal(run(&args(&["bm", "add", "/a", "/b"]), &h.ctx())),
        "bm add takes at most one directory."
    );
}

// FR-BM-08: an empty list prints "No bookmarks."
#[test]
fn list_without_bookmarks_prints_no_bookmarks() {
    let h = harness("bm-list-empty");
    assert_eq!(info(run(&args(&["bm", "list"]), &h.ctx())), "No bookmarks.");
    write_bookmarks(&h, "");
    assert_eq!(info(run(&args(&["bm", "list"]), &h.ctx())), "No bookmarks.");
}

// FR-BM-08: each bookmark shows its status word and path, in stored order
#[test]
fn list_shows_ok_missing_and_outside_project_in_stored_order() {
    let h = harness("bm-list-status");
    let ok = folder(&h, "projects/app/src");
    let gone = folder(&h, "projects/lib/gone");
    let outside = folder(&h, "outside/x");
    write_bookmarks(&h, &format!("{ok}\n{gone}\n{outside}\n"));
    fs::remove_dir_all(&gone).expect("remove folder");
    let text = info(run(&args(&["bm", "list"]), &h.ctx()));
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 3, "rows: {text}");
    assert_status_row(rows[0], "ok", &ok);
    assert_status_row(rows[1], "missing", &gone);
    assert_status_row(rows[2], "outside project", &outside);
}

/// The row ends with the path and starts with the status word (the padding between them is not compared).
fn assert_status_row(row: &str, status: &str, path: &str) {
    let rest = row
        .strip_suffix(path)
        .unwrap_or_else(|| panic!("row {row:?} ends with {path}"));
    assert_eq!(rest.trim_end(), status, "row {row:?}");
}

// FR-BM-08: a bookmark whose directory is gone is listed with "missing" (D-06: still listed)
#[test]
fn list_keeps_bookmark_of_a_missing_directory() {
    let h = harness("bm-list-missing");
    let gone = folder(&h, "projects/app/gone");
    run(&args(&["bm", "add", &gone]), &h.ctx()).expect("add");
    fs::remove_dir_all(&gone).expect("remove folder");
    let text = info(run(&args(&["bm", "list"]), &h.ctx()));
    assert_status_row(&text, "missing", &gone);
}

// FR-BM-09: remove deletes the bookmark and prints the removed text
#[test]
fn remove_deletes_the_bookmark_and_prints_removed_bookmark() {
    let h = harness("bm-remove");
    let app = folder(&h, "projects/app/src");
    let lib = folder(&h, "projects/lib/src");
    write_bookmarks(&h, &format!("{app}\n{lib}\n"));
    assert_eq!(
        info(run(&args(&["bm", "remove", &app]), &h.ctx())),
        format!("Removed bookmark {app}")
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{lib}\n").as_str())
    );
}

// FR-BM-09, D-06: a bookmark of a directory that no longer exists is removed by its path
#[test]
fn remove_works_for_a_directory_that_is_gone() {
    let h = harness("bm-remove-gone");
    let gone = folder(&h, "projects/app/gone");
    run(&args(&["bm", "add", &gone]), &h.ctx()).expect("add");
    fs::remove_dir_all(&gone).expect("remove folder");
    assert_eq!(
        info(run(&args(&["bm", "remove", &gone]), &h.ctx())),
        format!("Removed bookmark {gone}")
    );
}

// FR-BM-10, D-06: removing a directory that is not bookmarked fails with the cleaned path
#[test]
fn remove_of_unknown_bookmark_fails_with_cleaned_path() {
    let h = harness("bm-remove-unknown");
    let app = folder(&h, "projects/app/src");
    let lib = folder(&h, "projects/lib/src");
    write_bookmarks(&h, &format!("{app}\n"));
    let typed = format!("{}/projects/app/../lib/src/", h.root().display());
    assert_eq!(
        fatal(run(&args(&["bm", "remove", &typed]), &h.ctx())),
        format!("'{lib}' is not a bookmark.")
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{app}\n").as_str())
    );
}

// FR-BM-10: remove with no directory
#[test]
fn remove_without_directory_is_refused() {
    let h = harness("bm-remove-none");
    assert_eq!(
        fatal(run(&args(&["bm", "remove"]), &h.ctx())),
        "bookmark remove requires a directory."
    );
}

// FR-BM-01: a stored file of relative or duplicate lines is read as the list (A-09, D-06)
#[test]
fn list_skips_relative_and_repeated_lines() {
    let h = harness("bm-list-hand-edited");
    let app = folder(&h, "projects/app/src");
    write_bookmarks(&h, &format!("relative/x\n{app}/\n{app}\n"));
    let text = info(run(&args(&["bm", "list"]), &h.ctx()));
    assert_eq!(text.lines().count(), 1, "rows: {text}");
    assert_status_row(&text, "ok", &app);
}

/// A harness whose search paths are `<root>/home/code` (under HOME) and `<root>/projects` (outside HOME).
fn home_harness(name: &str) -> Harness {
    let h = Harness::new(name);
    let code = format!("{}/home/code", h.root().display());
    let search = format!("{}/projects", h.root().display());
    h.write_config(&format!("{{\"search_paths\": [\"{code}\", \"{search}\"]}}"));
    h
}

// FR-BM-08, FR-HIST-03: a bookmark row shows a path under HOME as "~"; a path outside HOME is shown in full
#[test]
fn list_shows_bookmarks_under_home_as_tilde() {
    let h = home_harness("bm-list-tilde");
    let under = folder(&h, "home/code/app/src");
    let outside = folder(&h, "projects/app/src");
    write_bookmarks(&h, &format!("{under}\n{outside}\n"));
    let text = info(run(&args(&["bm", "list"]), &h.ctx()));
    assert_eq!(
        text,
        format!("{:<16}~/code/app/src\n{:<16}{outside}", "ok", "ok")
    );
}

// FR-BM-05, FR-BM-06, FR-BM-09: the bookmarked, already-bookmarked and removed messages show a path under HOME as
// "~", while the file keeps the absolute path
#[test]
fn add_and_remove_messages_show_home_paths_as_tilde() {
    let h = home_harness("bm-messages-tilde");
    let dir = folder(&h, "home/code/app/src");
    assert_eq!(
        info(run(&args(&["bm", "add", &dir]), &h.ctx())),
        "Bookmarked ~/code/app/src"
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{dir}\n").as_str())
    );
    assert_eq!(
        info(run(&args(&["bm", "add", &dir]), &h.ctx())),
        "Already bookmarked: ~/code/app/src"
    );
    assert_eq!(
        info(run(&args(&["bm", "remove", &dir]), &h.ctx())),
        "Removed bookmark ~/code/app/src"
    );
    let gone = folder(&h, "home/code/app/gone");
    fs::remove_dir(&gone).expect("remove folder");
    assert_eq!(
        fatal(run(&args(&["bm", "remove", &gone]), &h.ctx())),
        "'~/code/app/gone' is not a bookmark."
    );
}

// FR-BM-03, FR-BM-04, FR-EDIT-10: the refusals name a path under HOME as "~"
#[test]
fn refusals_name_home_paths_as_tilde() {
    let h = home_harness("bm-refusals-tilde");
    folder(&h, "home/code/app/src");
    let project = format!("{}/home/code/app", h.root().display());
    let notes = folder(&h, "home/notes");
    assert_eq!(
        fatal(run(&args(&["bm", "add", &project]), &h.ctx())),
        "~/code/app is a project; open it with 'ledge project pick'."
    );
    assert_eq!(
        fatal(run(&args(&["bm", "add", &notes]), &h.ctx())),
        "~/notes is not inside a project. Bookmarks go inside a directory from search_paths."
    );
    assert_eq!(
        fatal(run(&args(&["bm", "edit", &notes]), &h.ctx())),
        "~/notes is not inside a project from search_paths."
    );
}

/// The custom projects file (P-65) with one path.
fn custom_projects(h: &Harness, dir: &str) {
    h.write_custom_projects(&format!("{dir}\n"));
}

// P-65, FR-BM-02: a directory inside a custom project is accepted and listed as ok
#[test]
fn directory_inside_a_custom_project_is_accepted() {
    let h = harness("bm-custom-inside");
    let scratch = folder(&h, "scratch");
    let src = folder(&h, "scratch/src");
    custom_projects(&h, &scratch);
    assert_eq!(
        info(run(&args(&["bm", "add", &src]), &h.ctx())),
        format!("Bookmarked {src}")
    );
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{src}\n").as_str())
    );
    let listing = info(run(&args(&["bm", "list"]), &h.ctx()));
    assert!(listing.starts_with("ok"), "listing was {listing:?}");
    assert!(listing.contains(&src));
}

// P-65, FR-BM-04: the root of a custom project is a project, so it is refused with the project message
#[test]
fn custom_project_root_is_refused_as_a_project() {
    let h = harness("bm-custom-root");
    let scratch = folder(&h, "scratch");
    custom_projects(&h, &scratch);
    assert_eq!(
        fatal(run(&args(&["bm", "add", &scratch]), &h.ctx())),
        format!("{scratch} is a project; open it with 'ledge project pick'.")
    );
    assert!(bookmarks_text(&h).is_none());
}

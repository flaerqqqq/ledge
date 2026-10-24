//! Component tests for the custom project commands (proposal P-67, FR-CUST-01..03) through `cli::run` over the harness of
//! tests/support (docs/testing.md level 2). The custom projects file and the bookmarks file are real, in a temp cache
//! folder; the search path is `<root>/projects`, and the sessions are the fake backend's.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::ensure_session;
use ledge::cli::run;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::error::{LedgeError, Outcome, Result};
use ledge::ports::backend::SessionBackend;
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

/// Starts the managed session of `dir` on the fake backend.
fn live(h: &Harness, dir: &str) {
    ensure_session(
        &h.backend,
        &AbsPath::clean(dir, &AbsPath::root()),
        &Profile::basic(),
    )
    .expect("create session");
}

fn session_names(h: &Harness) -> Vec<String> {
    h.backend
        .list_sessions()
        .expect("list sessions")
        .into_iter()
        .map(|session| session.name)
        .collect()
}

// FR-CUST-01: `custom add DIR` records an existing directory outside every project and prints that it was added
#[test]
fn add_records_the_directory() {
    let h = harness("custom-add");
    let scratch = folder(&h, "outside/scratch");
    assert_eq!(
        info(run(&args(&["custom", "add", &scratch]), &h.ctx())),
        format!("Added custom project {scratch}")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{scratch}\n")));
}

// FR-CUST-01: without a directory the working directory is recorded
#[test]
fn add_without_directory_records_the_working_directory() {
    let mut h = harness("custom-add-cwd");
    let scratch = folder(&h, "outside/scratch");
    h.cwd = AbsPath::clean(&scratch, &AbsPath::root());
    assert_eq!(
        info(run(&args(&["custom", "add"]), &h.ctx())),
        format!("Added custom project {scratch}")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{scratch}\n")));
}

// FR-CUST-01: a relative directory is cleaned against the working directory, and a trailing slash is dropped
#[test]
fn add_cleans_relative_and_trailing_slash_paths() {
    let mut h = harness("custom-add-clean");
    let outside = folder(&h, "outside");
    folder(&h, "outside/scratch");
    h.cwd = AbsPath::clean(&outside, &AbsPath::root());
    assert_eq!(
        info(run(&args(&["custom", "add", "scratch/"]), &h.ctx())),
        format!("Added custom project {outside}/scratch")
    );
}

// FR-CUST-01, FR-OPEN-03: a directory that does not exist is refused with the directory message
#[test]
fn add_of_a_missing_directory_is_refused() {
    let h = harness("custom-add-missing");
    let missing = format!("{}/nowhere", h.root().display());
    assert_eq!(
        fatal(run(&args(&["custom", "add", &missing]), &h.ctx())),
        format!("Directory '{missing}' not found.")
    );
    assert!(h.custom_projects_text().is_none());
}

// FR-CUST-01: a project itself is refused with the project message, as the command-line open decides (should_record)
#[test]
fn add_of_a_project_is_refused_with_the_project_message() {
    let h = harness("custom-add-project");
    let app = folder(&h, "projects/app");
    assert_eq!(
        fatal(run(&args(&["custom", "add", &app]), &h.ctx())),
        format!("{app} is a project; open it with 'ledge project pick'.")
    );
    assert!(h.custom_projects_text().is_none());
}

// FR-CUST-01: a directory inside a project is refused with the inside message
#[test]
fn add_inside_a_project_is_refused() {
    let h = harness("custom-add-inside");
    let src = folder(&h, "projects/app/src");
    assert_eq!(
        fatal(run(&args(&["custom", "add", &src]), &h.ctx())),
        format!("{src} is inside a project; a custom project lies outside every project.")
    );
    assert!(h.custom_projects_text().is_none());
}

// FR-CUST-01: a directory inside a custom project is refused with the same inside message
#[test]
fn add_inside_a_custom_project_is_refused() {
    let h = harness("custom-add-inside-custom");
    let scratch = folder(&h, "outside/scratch");
    let sub = folder(&h, "outside/scratch/sub");
    h.write_custom_projects(&format!("{scratch}\n"));
    assert_eq!(
        fatal(run(&args(&["custom", "add", &sub]), &h.ctx())),
        format!("{sub} is inside a project; a custom project lies outside every project.")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{scratch}\n")));
}

// FR-CUST-01: a directory that is already listed is an info, and the file does not change
#[test]
fn add_of_a_listed_directory_is_an_info() {
    let h = harness("custom-add-listed");
    let scratch = folder(&h, "outside/scratch");
    h.write_custom_projects(&format!("{scratch}\n"));
    assert_eq!(
        info(run(&args(&["custom", "add", &scratch]), &h.ctx())),
        format!("Already a custom project: {scratch}")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{scratch}\n")));
}

// FR-CUST-02: `custom remove DIR` removes the entry, deletes the bookmarks inside it and kills the sessions under it
#[test]
fn remove_deletes_the_entry_its_bookmarks_and_its_sessions() {
    let h = harness("custom-remove");
    let scratch = folder(&h, "outside/scratch");
    let src = folder(&h, "outside/scratch/src");
    let other = folder(&h, "projects/other");
    let kept = folder(&h, "outside/keep");
    h.write_custom_projects(&format!("{scratch}\n{kept}\n"));
    write_bookmarks(&h, &format!("{src}\n{other}\n"));
    live(&h, &scratch);
    live(&h, &src);
    live(&h, &kept);
    assert_eq!(
        info(run(&args(&["custom", "remove", &scratch]), &h.ctx())),
        format!("Removed custom project {scratch}")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{kept}\n")));
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{other}\n").as_str())
    );
    let names = session_names(&h);
    assert!(!names.contains(&session_name(&AbsPath::clean(&scratch, &AbsPath::root()))));
    assert!(!names.contains(&session_name(&AbsPath::clean(&src, &AbsPath::root()))));
    assert!(names.contains(&session_name(&AbsPath::clean(&kept, &AbsPath::root()))));
}

// FR-CUST-02: a directory that is not listed is an error with its own message, and nothing changes
#[test]
fn remove_of_an_unlisted_directory_is_an_error() {
    let h = harness("custom-remove-unlisted");
    let scratch = folder(&h, "outside/scratch");
    let other = folder(&h, "outside/other");
    h.write_custom_projects(&format!("{scratch}\n"));
    assert_eq!(
        fatal(run(&args(&["custom", "remove", &other]), &h.ctx())),
        format!("'{other}' is not a custom project.")
    );
    assert_eq!(h.custom_projects_text(), Some(format!("{scratch}\n")));
}

// FR-CUST-02: removing a custom project whose directory is gone still works (the path is not required to exist)
#[test]
fn remove_of_a_missing_custom_project_works() {
    let h = harness("custom-remove-gone");
    let gone = format!("{}/outside/gone", h.root().display());
    h.write_custom_projects(&format!("{gone}\n"));
    assert_eq!(
        info(run(&args(&["custom", "remove", &gone]), &h.ctx())),
        format!("Removed custom project {gone}")
    );
    assert_eq!(h.custom_projects_text(), Some(String::new()));
}

// FR-CUST-03: `custom list` prints the custom project paths, one per line, in stored order
#[test]
fn list_prints_the_paths_in_stored_order() {
    let h = harness("custom-list");
    let zed = folder(&h, "outside/zed");
    let apple = folder(&h, "outside/apple");
    h.write_custom_projects(&format!("{zed}\n{apple}\n"));
    assert_eq!(
        info(run(&args(&["custom", "list"]), &h.ctx())),
        format!("{zed}\n{apple}")
    );
}

// FR-CUST-03: a path under the home folder is shown with `~`
#[test]
fn list_shows_the_home_folder_as_tilde() {
    let h = harness("custom-list-home");
    let notes = folder(&h, "home/notes");
    h.write_custom_projects(&format!("{notes}\n"));
    assert_eq!(info(run(&args(&["custom", "list"]), &h.ctx())), "~/notes");
}

// FR-CUST-03: with no custom projects `custom list` prints the empty message
#[test]
fn list_with_no_custom_projects_says_so() {
    let h = harness("custom-list-empty");
    assert_eq!(
        info(run(&args(&["custom", "list"]), &h.ctx())),
        "No custom projects."
    );
    h.write_custom_projects("");
    assert_eq!(
        info(run(&args(&["custom", "list"]), &h.ctx())),
        "No custom projects."
    );
}

// FR-CUST-01: the added directory is listed by `custom list` with the same text (round trip through the file)
#[test]
fn add_then_list_shows_the_new_project() {
    let h = harness("custom-add-list");
    let scratch = folder(&h, "outside/scratch");
    run(&args(&["custom", "add", &scratch]), &h.ctx()).expect("add");
    assert_eq!(info(run(&args(&["custom", "list"]), &h.ctx())), scratch);
}

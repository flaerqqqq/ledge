//! Component tests for the history commands through `cli::run` over the harness of tests/support (docs/testing.md
//! level 2). The files are real, in a temp cache folder; HOME is a temp folder too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::cli::run;
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

/// Writes a file in the cache folder, as a hand-edited file would be.
fn write_cache(h: &Harness, name: &str, text: &str) {
    let folder = PathBuf::from(h.dirs.cache.as_str());
    fs::create_dir_all(&folder).expect("create cache folder");
    fs::write(folder.join(name), text).expect("write cache file");
}

fn read_cache(h: &Harness, name: &str) -> Option<String> {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join(name)).ok()
}

// FR-HIST-03: entries are listed one per line, newest first, as stored
#[test]
fn list_prints_one_entry_per_line_in_stored_order() {
    let h = Harness::new("hist-list-order");
    let first = h.project("alpha");
    let second = h.project("beta");
    write_cache(&h, "history", &format!("{second}\n{first}\n"));
    let text = info(run(&args(&["history", "list"]), &h.ctx()));
    assert_eq!(text, format!("{second}\n{first}"));
}

// FR-HIST-03: the home directory is shown as "~"; other paths are shown in full
#[test]
fn list_shows_home_directory_as_tilde() {
    let h = Harness::new("hist-list-tilde");
    let home = format!("{}/home", h.root().display());
    let under_home = format!("{home}/code/app");
    let elsewhere = format!("{}/projects/app", h.root().display());
    write_cache(&h, "history", &format!("{under_home}\n{elsewhere}\n"));
    let text = info(run(&args(&["history", "list"]), &h.ctx()));
    assert_eq!(text, format!("~/code/app\n{elsewhere}"));
}

// FR-HIST-03: an empty history prints "No history."
#[test]
fn list_of_empty_history_prints_no_history() {
    let h = Harness::new("hist-list-empty");
    write_cache(&h, "history", "");
    assert_eq!(
        info(run(&args(&["history", "list"]), &h.ctx())),
        "No history."
    );
}

// FR-HIST-03: a missing history file is an empty history
#[test]
fn list_without_history_file_prints_no_history() {
    let h = Harness::new("hist-list-missing");
    assert_eq!(
        info(run(&args(&["history", "list"]), &h.ctx())),
        "No history."
    );
    assert!(h.history_text().is_none(), "list creates nothing");
}

// FR-HIST-06: clean deletes the history file and prints the success text
#[test]
fn clean_deletes_the_history_file_and_prints_the_success_text() {
    let h = Harness::new("hist-clean-delete");
    let dir = h.project("app");
    write_cache(&h, "history", &format!("{dir}\n"));
    assert_eq!(
        info(run(&args(&["history", "clean"]), &h.ctx())),
        "Project history cache clear successfully."
    );
    assert!(h.history_text().is_none());
}

// FR-HIST-07: clean with no history file prints "No history cache found." and creates nothing
#[test]
fn clean_without_history_file_prints_no_history_cache_found() {
    let h = Harness::new("hist-clean-missing");
    assert_eq!(
        info(run(&args(&["history", "clean"]), &h.ctx())),
        "No history cache found."
    );
    assert!(h.history_text().is_none());
}

// FR-HIST-08: clean leaves the bookmarks file as it is
#[test]
fn clean_leaves_bookmarks_untouched() {
    let h = Harness::new("hist-clean-bookmarks");
    let dir = h.project("app");
    write_cache(&h, "history", &format!("{dir}\n"));
    write_cache(&h, "bookmarks", &format!("{dir}/src\n"));
    run(&args(&["history", "clean"]), &h.ctx()).expect("clean runs");
    assert_eq!(
        read_cache(&h, "bookmarks").as_deref(),
        Some(format!("{dir}/src\n").as_str())
    );
}

// FR-HIST-03, D-11: extra words after `history list` are refused
#[test]
fn list_with_extra_words_is_refused() {
    let h = Harness::new("hist-list-extra");
    assert_eq!(
        fatal(run(&args(&["history", "list", "x"]), &h.ctx())),
        "'history list' takes no arguments."
    );
}

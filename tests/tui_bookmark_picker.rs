//! TUI model tests for the bookmark picker (docs/testing.md level 3, picker test pattern) and the delete key of the
//! other pickers (FR-ACT-11). The services in `app::pickers` run with `FakeBackend`, `FakeTerminalFactory` (scripted
//! keys, `TestBackend`) and `RealFs` on temp folders. Covers FR-BM-11, FR-BM-12, FR-BM-13, FR-ACT-11, with D-12, D-17
//! and A-19, A-39.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::ensure_session;
use ledge::app::pickers::{bookmark_picker, project_picker};
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{Outcome, Result};
use ledge::ports::backend::SessionBackend;
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

const BOOKMARK_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark";
const TMUX_SET: &str = "/tmp/tmux-1000/default,1,0";

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

fn enter() -> Input {
    key(KeyCode::Enter)
}

fn esc() -> Input {
    key(KeyCode::Esc)
}

fn down() -> Input {
    key(KeyCode::Down)
}

/// The session name of a directory path, as the flow computes it (D-14).
fn name_of(dir: &str) -> String {
    session_name(&AbsPath::clean(dir, &AbsPath::root()))
}

/// The last screen drawn by the last picker, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("a picker opened a terminal")
        .borrow()
        .lines
        .clone()
}

/// The first screen row that contains `needle`.
fn line_with<'a>(lines: &'a [String], needle: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(needle))
        .map(String::as_str)
        .unwrap_or_else(|| panic!("no row with {needle:?} in {lines:#?}"))
}

/// Names of the sessions on the fake server.
fn live_names(h: &Harness) -> Vec<String> {
    h.backend
        .list_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|session| session.name)
        .collect()
}

/// The bookmarks file in the cache folder.
fn bookmarks_path(h: &Harness) -> PathBuf {
    PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")
}

/// Writes the bookmarks file with the given text (one absolute path per line).
fn write_bookmarks(h: &Harness, lines: &[&str]) {
    let path = bookmarks_path(h);
    fs::create_dir_all(path.parent().expect("cache folder has a parent")).expect("create cache");
    fs::write(path, lines.join("\n") + "\n").expect("write bookmarks");
}

/// The bookmarks file text, empty when there is no file.
fn bookmarks_text(h: &Harness) -> String {
    fs::read_to_string(bookmarks_path(h)).unwrap_or_default()
}

/// A managed session of `dir` with the basic profile.
fn live(h: &Harness, dir: &str) {
    ensure_session(
        &h.backend,
        &AbsPath::clean(dir, &AbsPath::root()),
        &Profile::basic(),
    )
    .expect("create session");
}

/// Runs the bookmark picker with no profile option.
fn run_picker(h: &Harness) -> Result<Outcome> {
    bookmark_picker(&h.ctx(), &ProfileOpt::Unset)
}

// FR-BM-11: the rows are the bookmarks in stored order, not sorted
#[test]
fn rows_are_listed_in_stored_order() {
    let h = Harness::new("tui_bookmark_picker-stored-order");
    h.terminal.set_size(220, 24);
    let projects = h.root().join("projects").display().to_string();
    h.project("zeta");
    h.project("alpha");
    write_bookmarks(
        &h,
        &[&format!("{projects}/zeta"), &format!("{projects}/alpha")],
    );
    h.terminal.script(vec![esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    let lines = screen(&h);
    let zeta = lines
        .iter()
        .position(|line| line.contains("/projects/zeta"))
        .expect("zeta row");
    let alpha = lines
        .iter()
        .position(|line| line.contains("/projects/alpha"))
        .expect("alpha row");
    assert!(zeta < alpha, "stored order is zeta then alpha: {lines:#?}");
}

// FR-BM-11, FR-TUI-08, A-04: a bookmark whose directory is gone is dimmed, with the ` (missing)` cue on its row
#[test]
fn missing_directory_is_marked_missing_and_present_one_is_not() {
    let h = Harness::new("tui_bookmark_picker-missing-cue");
    h.terminal.set_size(220, 24);
    let projects = h.root().join("projects").display().to_string();
    h.project("app");
    write_bookmarks(
        &h,
        &[&format!("{projects}/app"), &format!("{projects}/gone")],
    );
    h.terminal.script(vec![esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    let lines = screen(&h);
    let gone = line_with(&lines, "/projects/gone");
    assert!(gone.contains("(missing)"), "{gone}");
    let app = line_with(&lines, "/projects/app");
    assert!(!app.contains("(missing)"), "{app}");
}

// FR-BM-12, D-12: no bookmarks prints the message, exits with no window
#[test]
fn no_bookmarks_prints_the_message_and_opens_no_window() {
    let h = Harness::new("tui_bookmark_picker-empty");
    assert_eq!(
        run_picker(&h),
        Ok(Outcome::Info(
            "No bookmarks. Use 'ledge bm add' to add one.".to_string()
        ))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// FR-BM-12, D-12: an empty bookmarks file is the same as no bookmarks
#[test]
fn empty_bookmarks_file_prints_the_message_and_opens_no_window() {
    let h = Harness::new("tui_bookmark_picker-empty-file");
    write_bookmarks(&h, &[]);
    assert_eq!(
        run_picker(&h),
        Ok(Outcome::Info(
            "No bookmarks. Use 'ledge bm add' to add one.".to_string()
        ))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// A-19: enter opens the bookmarked directory's own session
#[test]
fn enter_opens_the_session_of_the_bookmarked_directory() {
    let h = Harness::new("tui_bookmark_picker-enter");
    let app = h.project("app");
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![enter()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&app)));
}

// FR-PICK-09 (as the bookmark picker reports it): enter on a missing directory keeps the picker open with the message
#[test]
fn enter_on_a_missing_directory_keeps_the_picker_open() {
    let h = Harness::new("tui_bookmark_picker-enter-missing");
    h.terminal.set_size(220, 24);
    let gone = format!("{}/projects/gone", h.root().display());
    write_bookmarks(&h, &[&gone]);
    h.terminal.script(vec![enter(), esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None);
    assert!(line_with(&screen(&h), "Missing directory:").contains(&gone));
}

// FR-BM-13: ctrl-d deletes the selected bookmark, kills its session, and the list is shown again without it
#[test]
fn delete_key_removes_the_bookmark_kills_its_session_and_reopens_the_list() {
    let h = Harness::new("tui_bookmark_picker-delete");
    h.terminal.set_size(220, 24);
    let projects = h.root().join("projects").display().to_string();
    let app = h.project("app");
    let lib = h.project("lib");
    live(&h, &app);
    live(&h, &lib);
    write_bookmarks(&h, &[&app, &lib]);
    h.terminal.script(vec![down(), ctrl('d'), esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    assert_eq!(bookmarks_text(&h), format!("{app}\n"));
    let names = live_names(&h);
    assert!(!names.contains(&name_of(&lib)), "{names:?}");
    assert!(names.contains(&name_of(&app)), "{names:?}");
    let lines = screen(&h);
    assert!(
        lines
            .iter()
            .all(|line| !line.contains(&format!("{projects}/lib"))),
        "deleted row still shown: {lines:#?}"
    );
}

// FR-BM-13, D-17, A-39: ctrl-d on the session the user is in deletes the bookmark and keeps the session; the status
// line says the session cannot be killed
#[test]
fn delete_key_on_the_current_session_deletes_the_bookmark_and_keeps_the_session() {
    let mut h = Harness::new("tui_bookmark_picker-delete-current");
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    let lib = h.project("lib");
    live(&h, &app);
    live(&h, &lib);
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend
        .switch_client(&name_of(&app))
        .expect("switch client");
    write_bookmarks(&h, &[&app, &lib]);
    h.terminal.script(vec![ctrl('d'), esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    assert_eq!(bookmarks_text(&h), format!("{lib}\n"));
    assert!(live_names(&h).contains(&name_of(&app)));
    assert!(
        line_with(&screen(&h), "cannot kill the session you are in.")
            .contains("cannot kill the session you are in.")
    );
}

// D-12, FR-BM-12: deleting the last bookmark empties the list, so the picker closes with no output
#[test]
fn deleting_the_last_bookmark_closes_the_picker() {
    let h = Harness::new("tui_bookmark_picker-delete-last");
    let app = h.project("app");
    live(&h, &app);
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![ctrl('d')]);
    assert_eq!(run_picker(&h), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), "");
    assert!(live_names(&h).is_empty());
}

// FR-ACT-11: ctrl-d does nothing on a row of the project picker (only bookmarks are deleted)
#[test]
fn delete_key_does_nothing_in_the_project_picker() {
    let mut h = Harness::new("tui_bookmark_picker-delete-noop");
    let projects = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{projects}\"]}}"));
    let app = h.project("app");
    live(&h, &app);
    h.cwd = AbsPath::clean(&app, &AbsPath::root());
    h.terminal.script(vec![ctrl('d'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(live_names(&h).contains(&name_of(&app)));
    assert!(!bookmarks_path(&h).exists());
}

// FR-ACT-01: ctrl-x kills the session of the selected bookmark, the bookmark stays and the list is shown again
#[test]
fn kill_key_kills_the_session_and_keeps_the_bookmark() {
    let h = Harness::new("tui_bookmark_picker-kill");
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    live(&h, &app);
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![ctrl('x'), esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    assert!(!live_names(&h).contains(&name_of(&app)));
    assert_eq!(bookmarks_text(&h), format!("{app}\n"));
}

// FR-ACT-02, FR-ACT-12: ctrl-r creates the missing session of the selected bookmark
#[test]
fn rebuild_key_creates_the_missing_session_of_the_bookmark() {
    let h = Harness::new("tui_bookmark_picker-rebuild");
    let app = h.project("app");
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![ctrl('r'), esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    assert!(live_names(&h).contains(&name_of(&app)));
}

// FR-ACT-04: ctrl-p opens the profile picker for the selected bookmark
#[test]
fn profile_key_opens_the_profile_picker() {
    let h = Harness::new("tui_bookmark_picker-profile");
    let app = h.project("app");
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    let profile_screen = h.terminal.records().iter().any(|record| {
        record
            .borrow()
            .lines
            .iter()
            .any(|line| line.contains("Profiles"))
    });
    assert!(profile_screen, "no profile picker was drawn");
}

// Layout: the border label, headers and the footer of the bookmark picker (architecture 6.2, screens section)
#[test]
fn label_headers_and_footer_are_shown() {
    let h = Harness::new("tui_bookmark_picker-chrome");
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    write_bookmarks(&h, &[&app]);
    h.terminal.script(vec![esc()]);
    assert_eq!(run_picker(&h), Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("Bookmarks")),
        "{lines:#?}"
    );
    let header = line_with(&lines, "Session");
    assert!(
        header.contains("Bookmark") && header.contains("Profile"),
        "{header}"
    );
    assert!(
        lines.iter().any(|line| line.contains(BOOKMARK_FOOTER)),
        "{lines:#?}"
    );
}

//! TUI model tests for the bookmark edit window (docs/testing.md level 3, picker test pattern): scripted keys through
//! `run_picker` with `FakeTerminal`, and the last drawn screen. Covers FR-EDIT-02, FR-EDIT-03, FR-EDIT-06, FR-EDIT-07
//! and FR-TUI-02 (search off, D-15). The window runs over `RealFs` on temp folders.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::bookmark_cmd::edit;
use ledge::error::Outcome;
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

/// Footer and label as architecture 6.2 gives them (the literal text, not the constant).
const FOOTER: &str = "enter toggle bookmark · →/^l open · ←/^h close · esc done";
const LABEL: &str = "Bookmarks in app";

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

fn esc() -> Input {
    key(KeyCode::Esc)
}

fn enter() -> Input {
    key(KeyCode::Enter)
}

fn down() -> Input {
    key(KeyCode::Down)
}

fn right() -> Input {
    key(KeyCode::Right)
}

fn left() -> Input {
    key(KeyCode::Left)
}

/// The last screen drawn, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("the window opened a terminal")
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

fn screen_has(h: &Harness, needle: &str) -> bool {
    screen(h).iter().any(|line| line.contains(needle))
}

fn bookmarks_path(h: &Harness) -> PathBuf {
    PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")
}

fn bookmarks_text(h: &Harness) -> String {
    fs::read_to_string(bookmarks_path(h)).unwrap_or_default()
}

fn expanded_text(h: &Harness) -> String {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bm-edit-open"))
        .unwrap_or_default()
}

/// The project `app` under the search path, with `src/main`, `docs` and the hidden `.git`. Returns its path.
/// The rows are: `app` (root), `docs`, `src` (with `main` under it once expanded).
fn fixture(h: &Harness) -> String {
    let app = h.project("app");
    fs::create_dir_all(format!("{app}/src/main")).expect("create src/main");
    fs::create_dir_all(format!("{app}/docs")).expect("create docs");
    fs::create_dir_all(format!("{app}/.git")).expect("create .git");
    let projects = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{projects}\"]}}"));
    app
}

// FR-EDIT-02: enter toggles the bookmark of the selected row and the window stays open
#[test]
fn enter_toggles_the_bookmark_and_keeps_the_window_open() {
    let h = Harness::new("tui_edit-enter-toggles");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), enter(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let docs = format!("{app}/docs");
    assert_eq!(bookmarks_text(&h), format!("{docs}\n"));
    // The last draw is the one after enter: the window was still open then, and the docs row is marked.
    assert!(line_with(&screen(&h), "docs").contains('★'));
}

// FR-EDIT-02, D-15: enter on the project root row does nothing
#[test]
fn enter_on_the_project_root_does_nothing() {
    let h = Harness::new("tui_edit-root-enter");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![enter(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), "");
    assert!(!line_with(&screen(&h), "- app").contains('★'));
}

// FR-EDIT-02: a second enter on the same row removes the bookmark again
#[test]
fn second_enter_removes_the_bookmark() {
    let h = Harness::new("tui_edit-enter-twice");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), enter(), enter(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), "");
    assert!(!line_with(&screen(&h), "docs").contains('★'));
}

// FR-EDIT-03: the right arrow expands a folder that has subfolders, and the folder is remembered
#[test]
fn right_arrow_expands_the_folder() {
    let h = Harness::new("tui_edit-right-expands");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), down(), right(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(screen_has(&h, "main"), "src is expanded: {:#?}", screen(&h));
    assert_eq!(expanded_text(&h), format!("{app}/src\n"));
}

// FR-EDIT-03: the left arrow collapses an expanded folder and removes only its entry
#[test]
fn left_arrow_collapses_an_expanded_folder() {
    let h = Harness::new("tui_edit-left-collapses");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal
        .script(vec![down(), down(), right(), left(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(
        !screen_has(&h, "main"),
        "src is collapsed: {:#?}",
        screen(&h)
    );
    assert_eq!(expanded_text(&h), "");
}

// FR-EDIT-03: the left arrow on a folder that is not expanded moves the selection to its parent row
#[test]
fn left_arrow_on_a_child_moves_to_the_parent_row() {
    let h = Harness::new("tui_edit-left-parent");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // Expand src, select main (a leaf), left moves to src, enter bookmarks src.
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        down(),
        left(),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/src\n"));
}

// FR-EDIT-03, D-15: ctrl-l expands and ctrl-h collapses, as the arrows do
#[test]
fn ctrl_l_expands_and_ctrl_h_collapses() {
    let h = Harness::new("tui_edit-ctrl-keys");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), down(), ctrl('l'), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(screen_has(&h, "main"));

    let h = Harness::new("tui_edit-ctrl-h");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal
        .script(vec![down(), down(), ctrl('l'), ctrl('h'), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(!screen_has(&h, "main"));
}

// architecture 6.5: backspace is collapse in the edit window (it is how ctrl-h arrives in raw mode)
#[test]
fn backspace_collapses_like_the_left_arrow() {
    let h = Harness::new("tui_edit-backspace");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        key(KeyCode::Backspace),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(!screen_has(&h, "main"));
    assert_eq!(expanded_text(&h), "");
}

// FR-EDIT-07: escape closes the window, with the terminal restored once and no output
#[test]
fn escape_closes_the_window() {
    let h = Harness::new("tui_edit-escape");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let record = h.terminal.last_record().expect("terminal opened");
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-02, D-15: the edit window has no type-to-filter; typed letters change nothing
#[test]
fn typing_does_not_filter_the_rows() {
    let h = Harness::new("tui_edit-no-filter");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![
        Input::Key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE)),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("docs")), "{lines:#?}");
    assert!(lines.iter().any(|line| line.contains("src")), "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.starts_with("> ")),
        "no prompt row: {lines:#?}"
    );
}

// FR-EDIT-04, FR-EDIT-06: the marker column shows the bookmark mark, the tree column shows + and -
#[test]
fn view_shows_the_marker_column_and_the_tree_markers() {
    let h = Harness::new("tui_edit-view");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    let docs = format!("{app}/docs");
    fs::create_dir_all(h.dirs.cache.as_str()).expect("cache folder");
    fs::write(bookmarks_path(&h), format!("{docs}\n")).expect("write bookmarks");
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(line_with(&lines, "docs").contains('★'), "{lines:#?}");
    assert!(!line_with(&lines, "src").contains('★'), "{lines:#?}");
    assert!(line_with(&lines, "+ src").contains("src"), "{lines:#?}");
    assert!(line_with(&lines, "- app").contains("app"), "{lines:#?}");
}

// D-15, architecture 6.2: the border label and the footer are the given texts
#[test]
fn label_and_footer_are_the_edit_window_texts() {
    let h = Harness::new("tui_edit-label");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains(LABEL)), "{lines:#?}");
    assert!(lines.iter().any(|line| line.contains(FOOTER)), "{lines:#?}");
}

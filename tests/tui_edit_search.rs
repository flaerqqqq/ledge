//! TUI model tests for the search bar of the bookmark edit window (P-53, FR-EDIT-13; docs/testing.md level 3, picker test
//! pattern). `/` opens a bar that fuzzy-filters the visible rows; enter keeps the match and closes the bar; escape clears
//! it and closes the bar; while the bar is open j and k are typed into it. The window runs over `RealFs` on temp folders
//! through `run_picker` with scripted keys (`FakeTerminal`).
//!
//! Some tests end with ctrl-c: the key map sends it to cancel (keys.rs), and the last screen drawn is the screen before
//! that key, so the test can read the bar while it is open.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::bookmark_cmd::edit;
use ledge::error::Outcome;
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ch(c: char) -> Input {
    key(KeyCode::Char(c))
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

/// The last screen drawn, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("the window opened a terminal")
        .borrow()
        .lines
        .clone()
}

fn screen_has(h: &Harness, needle: &str) -> bool {
    screen(h).iter().any(|line| line.contains(needle))
}

/// The search bar row of the last screen: the first row whose text, without the border, starts with `/`.
fn prompt_line(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .map(|line| {
            line.trim_matches(|c: char| c == '│' || c == ' ')
                .to_string()
        })
        .find(|line| line.starts_with('/'))
}

fn bookmarks_text(h: &Harness) -> String {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")).unwrap_or_default()
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

// FR-EDIT-13, P-53: / opens the search bar, whose prompt is a slash
#[test]
fn slash_opens_the_search_bar() {
    let h = Harness::new("tui_edit_search-open");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![ch('/'), ctrl('c')]);
    let _ = edit(&h.ctx(), Some(&app), false);
    let lines = screen(&h);
    assert_eq!(
        prompt_line(&lines).as_deref(),
        Some("/"),
        "the bar shows its prompt: {lines:#?}"
    );
}

// FR-EDIT-13, P-53: typed text fuzzy-filters the visible rows while the bar is open
#[test]
fn typed_text_fuzzy_filters_the_visible_rows() {
    let h = Harness::new("tui_edit_search-filter");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal
        .script(vec![ch('/'), ch('d'), ch('o'), ch('c'), ctrl('c')]);
    let _ = edit(&h.ctx(), Some(&app), false);
    let lines = screen(&h);
    assert_eq!(prompt_line(&lines).as_deref(), Some("/doc"), "{lines:#?}");
    assert!(screen_has(&h, "docs"), "docs matches doc: {lines:#?}");
    assert!(!screen_has(&h, "src"), "src is filtered out: {lines:#?}");
}

// FR-EDIT-13, P-53: enter keeps the match and closes the bar; the filtered rows stay as they are
#[test]
fn enter_keeps_the_match_and_closes_the_bar() {
    let h = Harness::new("tui_edit_search-enter");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // After enter the bar is closed, so the second enter toggles the bookmark of the selected row, docs.
    h.terminal.script(vec![
        ch('/'),
        ch('d'),
        ch('o'),
        ch('c'),
        enter(),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/docs\n"));
    let lines = screen(&h);
    assert!(screen_has(&h, "docs"), "the match is kept: {lines:#?}");
    assert!(!screen_has(&h, "src"), "the filter is kept: {lines:#?}");
}

// FR-EDIT-13, P-53: escape clears the match and closes the bar; every row is shown again
#[test]
fn escape_clears_the_match_and_closes_the_bar() {
    let h = Harness::new("tui_edit_search-escape");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // After the first escape the bar is closed and the window is still open: End selects the last row, src, and enter
    // bookmarks it (the window must not have closed on the first escape).
    h.terminal.script(vec![
        ch('/'),
        ch('d'),
        ch('o'),
        ch('c'),
        esc(),
        key(KeyCode::End),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/src\n"));
    let lines = screen(&h);
    assert_eq!(prompt_line(&lines), None, "the bar is closed: {lines:#?}");
    assert!(screen_has(&h, "docs"), "{lines:#?}");
    assert!(screen_has(&h, "src"), "the match is cleared: {lines:#?}");
}

// FR-EDIT-13, P-53: while the bar is open j and k are typed into it and do not move the selection
#[test]
fn j_and_k_type_into_the_open_bar() {
    let h = Harness::new("tui_edit_search-jk");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal
        .script(vec![down(), ch('/'), ch('j'), ch('k'), ctrl('c')]);
    let _ = edit(&h.ctx(), Some(&app), false);
    let lines = screen(&h);
    assert_eq!(
        prompt_line(&lines).as_deref(),
        Some("/jk"),
        "j and k are on the bar: {lines:#?}"
    );
}

// FR-EDIT-13, P-53: j and k typed into the bar leave the selection where it was; enter after escape bookmarks the
// row that was selected before the bar opened
#[test]
fn j_and_k_in_the_bar_do_not_move_the_selection() {
    let h = Harness::new("tui_edit_search-jk-selection");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // Select docs, open the bar, type j and k, clear with escape, then bookmark the selected row.
    h.terminal.script(vec![
        down(),
        ch('/'),
        ch('j'),
        ch('k'),
        esc(),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/docs\n"));
}

// FR-EDIT-13, P-53: a search does not change which folders are expanded
#[test]
fn search_leaves_the_expand_state_unchanged() {
    let h = Harness::new("tui_edit_search-expand");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // Expand src, search for doc (main is hidden while searching), escape. The window stays open, so End selects the
    // last row, main (under the expanded src), and enter bookmarks it.
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        ch('/'),
        ch('d'),
        ch('o'),
        ch('c'),
        esc(),
        key(KeyCode::End),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(expanded_text(&h), format!("{app}/src\n"));
    assert_eq!(bookmarks_text(&h), format!("{app}/src/main\n"));
    assert!(
        screen_has(&h, "main"),
        "src is still expanded: {:#?}",
        screen(&h)
    );
}

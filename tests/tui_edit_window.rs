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
    assert!(!line_with(&screen(&h), "app -").contains('★'));
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

// FR-EDIT-04, FR-EDIT-06: the marker column shows the bookmark mark, the tree column shows + and - after the name
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
    assert!(line_with(&lines, "src +").contains("src +"), "{lines:#?}");
    assert!(line_with(&lines, "app -").contains("app -"), "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("+ src") || line.contains("- app")),
        "{lines:#?}"
    );
}

// FR-EDIT-04, FR-TUI-09: without colour the + and - still show, after the folder name, and the branch glyphs and the
// indent stay on the left of the name
#[test]
fn folder_markers_follow_the_name_without_color() {
    let mut h = Harness::new("tui_edit-markers-no-color");
    h.terminal.set_size(80, 24);
    h.env = h.env.clone().with_var("NO_COLOR", "1");
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let app_row = line_with(&lines, "app -");
    assert!(app_row.starts_with("│ "), "{app_row:?}");
    // A folder without subfolders has no marker: the name ends the cell (the branch glyph before it is not a marker).
    let docs_row = line_with(&lines, "docs");
    assert!(
        docs_row.trim_end_matches(['│', ' ']).ends_with("docs"),
        "{docs_row:?}"
    );
}

// FR-EDIT-06, design 3.6, section 8 item 11: the nesting is drawn with branch glyphs, as the session tree draws it
#[test]
fn nesting_is_drawn_with_branch_glyphs_in_a_utf8_locale() {
    let mut h = Harness::new("tui_edit-glyphs-utf8");
    h.terminal.set_size(80, 24);
    h.env = h.env.clone().with_var("LANG", "en_US.UTF-8");
    let app = fixture(&h);
    // Expand src: docs, src (expanded) and main under it.
    h.terminal.script(vec![down(), down(), right(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(line_with(&lines, "docs").contains("├─docs"), "{lines:#?}");
    assert!(line_with(&lines, "src").contains("└─src -"), "{lines:#?}");
    assert!(line_with(&lines, "main").contains("  └─main"), "{lines:#?}");
}

// design section 7: without a UTF-8 locale the ASCII set of the session tree is used
#[test]
fn nesting_is_drawn_with_ascii_glyphs_without_a_utf8_locale() {
    let h = Harness::new("tui_edit-glyphs-ascii");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), down(), right(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(line_with(&lines, "docs").contains("|-docs"), "{lines:#?}");
    assert!(line_with(&lines, "src").contains("`-src -"), "{lines:#?}");
    assert!(line_with(&lines, "main").contains("  `-main"), "{lines:#?}");
}

// D-15, P-61 (rule under the prompt): the edit window has no prompt row, so its column header is the first row under
// the border label and no separator is drawn above it
#[test]
fn header_is_the_first_row_when_there_is_no_prompt() {
    let h = Harness::new("tui_edit-no-prompt-header");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(lines[1].contains("Directory"), "{lines:#?}");
    assert!(lines[2].starts_with("│───"), "{lines:#?}");
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

fn ch(c: char) -> Input {
    key(KeyCode::Char(c))
}

/// Runs the edit window with `script` over the fixture and returns the bookmarks text, the last screen and the
/// expanded folders (the project path is replaced by `<app>`).
fn run_edit(test: &str, script: Vec<Input>) -> (String, Vec<String>, String) {
    let h = Harness::new(test);
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(script);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let bookmarks = bookmarks_text(&h).replace(&app, "<app>");
    let expanded = expanded_text(&h).replace(&app, "<app>");
    (bookmarks, screen(&h), expanded)
}

// FR-EDIT-12: h collapses an expanded folder, as the left arrow does
#[test]
fn h_collapses_an_expanded_folder() {
    let h = Harness::new("tui_edit-h-collapses");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal
        .script(vec![down(), down(), right(), ch('h'), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(
        !screen_has(&h, "main"),
        "src is collapsed: {:#?}",
        screen(&h)
    );
    assert_eq!(expanded_text(&h), "");
}

// FR-EDIT-12: h on a leaf row moves the selection to its parent row
#[test]
fn h_on_a_leaf_moves_to_the_parent_row() {
    let h = Harness::new("tui_edit-h-leaf-parent");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // Expand src, select main (a leaf), h moves to src, enter bookmarks src.
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        down(),
        ch('h'),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/src\n"));
}

// FR-EDIT-12: h on a collapsed folder that has a parent moves the selection to the parent row
#[test]
fn h_on_a_collapsed_folder_moves_to_the_parent_row() {
    let h = Harness::new("tui_edit-h-collapsed-parent");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/src/main/deep")).expect("create deep");
    // Expand src, select main (collapsed, with a subfolder), h moves to src, enter bookmarks src.
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        down(),
        ch('h'),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/src\n"));
    assert_eq!(
        expanded_text(&h),
        format!("{app}/src\n"),
        "main stays collapsed"
    );
}

// FR-EDIT-12: l expands a folder that has subfolders, as the right arrow does
#[test]
fn l_expands_a_folder_with_subfolders() {
    let h = Harness::new("tui_edit-l-expands");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![down(), down(), ch('l'), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(screen_has(&h, "main"), "src is expanded: {:#?}", screen(&h));
    assert_eq!(expanded_text(&h), format!("{app}/src\n"));
}

// FR-EDIT-12: l on a leaf row does nothing; the selection stays on it
#[test]
fn l_on_a_leaf_does_nothing() {
    let h = Harness::new("tui_edit-l-leaf");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    // docs is a leaf: l changes nothing, so enter still bookmarks docs.
    h.terminal.script(vec![down(), ch('l'), enter(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/docs\n"));
    assert_eq!(expanded_text(&h), "");
}

// FR-EDIT-12: h and l do what the left and right arrows do, with the same expand state and selection
#[test]
fn h_and_l_match_the_arrow_keys() {
    let arrows = run_edit(
        "tui_edit-hl-arrows",
        vec![down(), down(), right(), down(), left(), enter(), esc()],
    );
    let letters = run_edit(
        "tui_edit-hl-letters",
        vec![down(), down(), ch('l'), down(), ch('h'), enter(), esc()],
    );
    assert_eq!(arrows.0, "<app>/src\n", "the arrows bookmark src");
    assert_eq!(letters, arrows);
}

// FR-EDIT-12: j and k still move the selection in the edit window
#[test]
fn j_and_k_still_move_the_selection() {
    let (down_twice, _, _) = run_edit("tui_edit-jk-j", vec![ch('j'), ch('j'), enter(), esc()]);
    assert_eq!(down_twice, "<app>/src\n");
    let (up_again, _, _) = run_edit(
        "tui_edit-jk-k",
        vec![ch('j'), ch('j'), ch('k'), enter(), esc()],
    );
    assert_eq!(up_again, "<app>/docs\n");
}

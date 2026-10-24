//! TUI model tests for the bookmark edit window (docs/testing.md level 3, picker test pattern): scripted keys through
//! `run_picker` with `FakeTerminal`, and the last drawn screen. Covers FR-EDIT-02, FR-EDIT-03, FR-EDIT-06, FR-EDIT-07
//! and FR-TUI-02 (search off, D-15). The window runs over `RealFs` on temp folders.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::bookmark_cmd::edit;
use ledge::app::build::ensure_session;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::error::Outcome;
use ledge::tui::terminal::Input;
use ledge::tui::theme::{Theme, Token};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
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
    // A folder without subfolders has no marker: the name ends the Directory cell. The next cell is the Subfolders
    // count (0), so the first text after the name is that count and not a `+` or `-`. (Moved by S-44: the row now
    // carries more columns after the name.)
    let docs_row = line_with(&lines, "docs");
    let after_name = docs_row.split("docs").nth(1).unwrap_or_default();
    assert!(
        after_name.trim_start().starts_with('0'),
        "no marker after docs: {docs_row:?}"
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

/// The character column of the first `needle` in `line`. The rows of the window are drawn with narrow glyphs, so a
/// character is a cell here.
fn column_of(line: &str, needle: &str) -> usize {
    let byte = line
        .find(needle)
        .unwrap_or_else(|| panic!("no {needle:?} in {line:?}"));
    line[..byte].chars().count()
}

/// The character at `column` of `line` (a space past its end).
fn char_at(line: &str, column: usize) -> char {
    line.chars().nth(column).unwrap_or(' ')
}

// FR-TUI-04, FR-EDIT-01: the column header row names the columns in order: Directory, Bookmark, Session, Profile and
// Subfolders; at 80 cells there is no Path column
#[test]
fn header_row_names_the_columns_in_order() {
    let h = Harness::new("tui_edit-header-labels");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let header = lines[1].as_str();
    let labels = ["Directory", "Bookmark", "Session", "Profile", "Subfolders"];
    let starts: Vec<usize> = labels
        .iter()
        .map(|label| column_of(header, label))
        .collect();
    assert!(
        starts.windows(2).all(|pair| pair[0] < pair[1]),
        "{header:?}"
    );
    assert!(!header.contains("Path"), "no Path at 80 cells: {header:?}");
}

// FR-EDIT-06: the bookmark mark is in its own column, after the folder name and under the Bookmark title; it is not
// in column 0
#[test]
fn bookmark_mark_sits_in_its_own_column_after_the_name() {
    let h = Harness::new("tui_edit-mark-column");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    let docs = format!("{app}/docs");
    fs::create_dir_all(h.dirs.cache.as_str()).expect("cache folder");
    fs::write(bookmarks_path(&h), format!("{docs}\n")).expect("write bookmarks");
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let header = lines[1].as_str();
    let docs_row = line_with(&lines, "docs");
    let mark = column_of(docs_row, "★");
    assert_eq!(mark, column_of(header, "Bookmark"), "{docs_row:?}");
    assert!(column_of(docs_row, "docs") < mark, "{docs_row:?}");
    let root_row = line_with(&lines, "app -");
    assert!(!root_row.contains('★'), "{root_row:?}");
}

// FR-EDIT-01, FR-PICK-07 (as the picker): a folder with a live managed session shows the session name and the profile
// in the Session and Profile columns; a folder without one shows neither
#[test]
fn live_session_shows_its_name_and_profile_on_the_folder_row() {
    let h = Harness::new("tui_edit-session-columns");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    let docs = format!("{app}/docs");
    ensure_session(
        &h.backend,
        &AbsPath::clean(&docs, &AbsPath::root()),
        &Profile::basic(),
    )
    .expect("create session");
    let name = session_name(&AbsPath::clean(&docs, &AbsPath::root()));
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let docs_row = line_with(&lines, "docs");
    assert!(docs_row.contains(&name), "{docs_row:?}");
    assert!(docs_row.contains("basic"), "{docs_row:?}");
    let src_row = line_with(&lines, "src +");
    assert!(!src_row.contains("basic"), "{src_row:?}");
    assert!(!src_row.contains("docs-"), "{src_row:?}");
}

// FR-EDIT-01: the Subfolders column counts the direct subfolders of each row, right-aligned under its title
#[test]
fn subfolders_column_counts_the_direct_subfolders() {
    let h = Harness::new("tui_edit-child-count");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let header = lines[1].as_str();
    let end = column_of(header, "Subfolders") + "Subfolders".chars().count() - 1;
    // The project root has docs and src; docs has none; src has main.
    for (needle, count) in [("app -", '2'), ("docs", '0'), ("src +", '1')] {
        let row = line_with(&lines, needle);
        assert_eq!(char_at(row, end), count, "{row:?}");
    }
}

// FR-TUI-04, D-15: the window is framed as the other windows: the rounded top edge carries the label, the bottom edge
// the footer keys, and the status row above it the count of rows
#[test]
fn window_is_framed_with_label_footer_and_status_count() {
    let h = Harness::new("tui_edit-frame");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let top = lines.first().expect("top edge");
    assert!(top.starts_with('╭') && top.contains(LABEL), "{top:?}");
    assert!(top.trim_end().ends_with('╮'), "{top:?}");
    let bottom = lines.last().expect("bottom edge");
    assert!(
        bottom.starts_with('╰') && bottom.contains(FOOTER),
        "{bottom:?}"
    );
    let status = lines.get(lines.len() - 2).expect("status row");
    assert!(status.contains("3 of 3"), "{status:?}");
}

// FR-EDIT-01, design 4.3 (option B): at 120 cells the Path column is shown, last, with the path of each folder
#[test]
fn path_column_is_shown_at_120_cells() {
    let h = Harness::new("tui_edit-path-120");
    h.terminal.set_size(120, 24);
    let app = fixture(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = screen(&h);
    let header = lines[1].as_str();
    let path = column_of(header, "Path");
    assert!(path > column_of(header, "Subfolders"), "{header:?}");
    let docs_row = line_with(&lines, "docs");
    assert!(docs_row.contains("app/docs"), "{docs_row:?}");
}

// P-66: the edit window title names the project by its label when another project has the same name
#[test]
fn label_names_a_same_named_project_by_its_label() {
    let h = Harness::new("tui_edit-p66-label");
    h.terminal.set_size(80, 24);
    let work = h.root().join("work");
    fs::create_dir_all(work.join("app").join("src")).expect("create work app");
    let projects = h.root().join("projects");
    fs::create_dir_all(projects.join("app").join("src")).expect("create app");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\", \"{}\"]}}",
        projects.display(),
        work.display()
    ));
    let theirs = work.join("app").display().to_string();
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&theirs), false), Ok(Outcome::Done));
    let lines = screen(&h);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Bookmarks in work/app")),
        "{lines:#?}"
    );
}

/// The `.` key (P-68), outside the search bar.
fn dot() -> Input {
    ch('.')
}

// FR-EDIT-01, P-68: hidden folders (names starting with a dot) are not shown when the window opens
#[test]
fn hidden_folders_start_hidden() {
    let h = Harness::new("tui_edit-hidden-start");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(!screen_has(&h, ".config"), "{:#?}", screen(&h));
}

// P-68: the dot key shows the hidden folders, and a second dot hides them again
#[test]
fn dot_shows_and_then_hides_hidden_folders() {
    let h = Harness::new("tui_edit-hidden-toggle");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![dot(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(screen_has(&h, ".config"), "{:#?}", screen(&h));

    let h = Harness::new("tui_edit-hidden-toggle-twice");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![dot(), dot(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(!screen_has(&h, ".config"), "{:#?}", screen(&h));
}

// P-68: the selection stays on the same folder when the toggle adds rows above it
#[test]
fn selection_stays_on_the_same_folder_when_hidden_folders_appear() {
    let h = Harness::new("tui_edit-hidden-selection");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    // docs is selected; the dot shows .config above it, and enter bookmarks the selected row.
    h.terminal.script(vec![down(), dot(), enter(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/docs\n"));
}

// P-68: a selected hidden folder that the dot hides again gives the selection to the nearest visible row above it
#[test]
fn hiding_the_selected_hidden_folder_selects_the_row_above() {
    let h = Harness::new("tui_edit-hidden-hide-selected");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/src/.cache")).expect("create src/.cache");
    // Expand src, show the hidden folders: .cache is under src. Select it and hide again: src is selected.
    h.terminal.script(vec![
        down(),
        down(),
        right(),
        dot(),
        down(),
        dot(),
        enter(),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert_eq!(bookmarks_text(&h), format!("{app}/src\n"));
    assert!(!screen_has(&h, ".cache"), "{:#?}", screen(&h));
}

// P-68: while the search bar is open the dot types into the bar and does not toggle the hidden folders
#[test]
fn dot_in_the_search_bar_types_a_dot() {
    let h = Harness::new("tui_edit-hidden-search-dot");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    // ctrl-c ends the window; the last screen drawn is the one before that key (as the search bar tests).
    h.terminal.script(vec![ch('/'), dot(), ctrl('c')]);
    let _ = edit(&h.ctx(), Some(&app), false);
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("/.")), "{lines:#?}");
    assert!(!screen_has(&h, ".config"), "{lines:#?}");
}

/// The style of the first character of `needle` on its screen row: its foreground colour and modifiers.
fn style_of_name(h: &Harness, needle: &str) -> (Option<Color>, Modifier) {
    let lines = screen(h);
    let y = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row with {needle:?} in {lines:#?}"));
    let column = column_of(&lines[y], needle);
    let record = h.terminal.last_record().expect("a terminal was opened");
    let record = record.borrow();
    let buffer = record.buffer.as_ref().expect("a frame was drawn");
    let cell = buffer
        .cell((column as u16, y as u16))
        .expect("the name is on the screen");
    (Some(cell.fg), cell.modifier)
}

// S-53, FR-TUI-08: with hidden folders shown, a folder whose name starts with a dot is dimmed like a missing row
#[test]
fn hidden_folder_is_dimmed_when_shown() {
    let h = Harness::new("tui_edit-hidden-dimmed");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![dot(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));

    let (fg, modifier) = style_of_name(&h, ".config");
    let missing = Theme::default().color(Token::Missing);
    assert_eq!(fg, Some(missing), "the dimmed row takes the missing token");
    assert!(modifier.contains(Modifier::DIM), "{modifier:?}");
}

// S-53: a normal folder in the same window is not dimmed
#[test]
fn normal_folder_is_not_dimmed_next_to_a_hidden_one() {
    let h = Harness::new("tui_edit-hidden-normal-not-dimmed");
    h.terminal.set_size(80, 24);
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![dot(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));

    let (_, modifier) = style_of_name(&h, "docs");
    assert!(!modifier.contains(Modifier::DIM), "{modifier:?}");
}

// S-53: without colour the dimmed row keeps its name on screen and gets no dim modifier
#[test]
fn hidden_folder_keeps_its_name_without_color() {
    let mut h = Harness::new("tui_edit-hidden-no-color");
    h.terminal.set_size(80, 24);
    h.env = h.env.clone().with_var("NO_COLOR", "1");
    let app = fixture(&h);
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    h.terminal.script(vec![dot(), esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));

    assert!(screen_has(&h, ".config"), "{:#?}", screen(&h));
    let (_, modifier) = style_of_name(&h, ".config");
    assert!(!modifier.contains(Modifier::DIM), "{modifier:?}");
}

//! TUI model tests for the vim navigation keys (FR-TUI-11, P-52, S-28; docs/testing.md level 3, picker test pattern).
//! j and k move the selection in the tree and edit windows, which have no type-to-filter. ctrl-j and ctrl-k move it in
//! the pickers that filter as the user types, where a plain j or k is typed into the filter (FR-TUI-02). The arrow keys
//! are the reference: every vim run must end as the arrow run does, including at the ends of the list (A-28: no wrap).
//! Every window uses the same key map (FR-TUI-07). Runs over `FakeBackend`, `FakeTerminal` and temp folders.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::bookmark_cmd::edit;
use ledge::app::build::ensure_session;
use ledge::app::pickers::{
    bookmark_picker, bookmark_tree_picker, project_picker, session_tree_picker,
};
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::domain::profile_rules::ProfileOpt;
use ledge::domain::tree::TreeScope;
use ledge::tui::keys::{self, Action};
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

/// Footers as the requirements give them (the literal text, not the constants of the code).
const PROJECT_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill";
const EDIT_FOOTER: &str = "enter toggle bookmark · →/^l open · ←/^h close · esc done";
const BOOKMARK_TREE_FOOTER: &str =
    "enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node";

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

fn ch(c: char) -> Input {
    key(KeyCode::Char(c))
}

fn j() -> Input {
    ch('j')
}

fn k() -> Input {
    ch('k')
}

fn ctrl_j() -> Input {
    ctrl('j')
}

fn ctrl_k() -> Input {
    ctrl('k')
}

fn down() -> Input {
    key(KeyCode::Down)
}

fn up() -> Input {
    key(KeyCode::Up)
}

fn enter() -> Input {
    key(KeyCode::Enter)
}

fn esc() -> Input {
    key(KeyCode::Esc)
}

fn backspace() -> Input {
    key(KeyCode::Backspace)
}

/// `make` repeated `times` times, as a script.
fn repeat(make: fn() -> Input, times: usize) -> Vec<Input> {
    (0..times).map(|_| make()).collect()
}

/// `first` followed by `rest`, as a script.
fn then(mut first: Vec<Input>, rest: Vec<Input>) -> Vec<Input> {
    first.extend(rest);
    first
}

/// The project basename of an attached session name (`<basename>-<4 hex>`, D-14). The hash depends on the temp root of
/// the test, so runs in different harnesses compare by basename.
fn base(name: Option<String>) -> Option<String> {
    name.map(|name| {
        name.rsplit_once('-')
            .map_or(name.clone(), |(base, _)| base.to_string())
    })
}

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

/// Config with the one search path `<root>/projects`.
fn search_in_projects(h: &Harness) {
    let search = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{search}\"]}}"));
}

/// Writes the bookmarks file (one absolute path per line).
fn write_bookmarks(h: &Harness, lines: &[&str]) {
    let path = PathBuf::from(h.dirs.cache.as_str()).join("bookmarks");
    fs::create_dir_all(path.parent().expect("cache folder has a parent")).expect("create cache");
    fs::write(path, lines.join("\n") + "\n").expect("write bookmarks");
}

/// The last screen drawn, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("a window opened a terminal")
        .borrow()
        .lines
        .clone()
}

fn shows(h: &Harness, needle: &str) -> bool {
    screen(h).iter().any(|line| line.contains(needle))
}

/// Projects `app`, `lib` and `web` under the search path. The launch directory is the project `launch`, so it is the
/// first row of the project picker (FR-PICK-04). Returns the harness with the picker's working directory set.
fn project_harness(test: &str, launch: &str) -> Harness {
    let mut h = Harness::new(test);
    search_in_projects(&h);
    h.project("app");
    h.project("lib");
    h.project("web");
    h.cwd = abs(&h.root().join("projects").join(launch).display().to_string());
    h
}

/// Runs the project picker with `script` and returns the session it attached to.
fn project_opened(test: &str, launch: &str, script: Vec<Input>) -> Option<String> {
    let h = project_harness(test, launch);
    h.terminal.script(script);
    let _ = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    base(h.backend.attached_to())
}

/// Project `app` with the folders `src/main`, `docs` and the hidden `.git`, under the search path. The edit window rows
/// are `app` (root), `docs`, `src`. Returns the project path text.
fn edit_fixture(h: &Harness) -> String {
    let app = h.project("app");
    fs::create_dir_all(format!("{app}/src/main")).expect("create src/main");
    fs::create_dir_all(format!("{app}/docs")).expect("create docs");
    fs::create_dir_all(format!("{app}/.git")).expect("create .git");
    search_in_projects(h);
    app
}

/// Runs the edit window with `script` and returns the bookmarks file, with the project path replaced by `<app>`.
fn edit_bookmarks(test: &str, script: Vec<Input>) -> String {
    let h = Harness::new(test);
    h.terminal.set_size(80, 24);
    let app = edit_fixture(&h);
    h.terminal.script(script);
    let _ = edit(&h.ctx(), Some(&app), false);
    let text = fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bookmarks"))
        .unwrap_or_default();
    text.replace(&app, "<app>")
}

/// Projects `app` and `lib`, with live sessions for both. Runs the session tree with `script` and returns the session it
/// attached to.
fn session_tree_opened(test: &str, script: Vec<Input>) -> Option<String> {
    let h = Harness::new(test);
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let lib = abs(&h.project("lib"));
    ensure_session(&h.backend, &app, &Profile::basic()).expect("create session");
    ensure_session(&h.backend, &lib, &Profile::basic()).expect("create session");
    h.terminal.script(script);
    let _ = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    base(h.backend.attached_to())
}

/// Projects `app` and `lib`, with `app/api` and `lib` bookmarked. Runs the bookmark tree with `script` and returns the
/// session it attached to.
fn bookmark_tree_opened(test: &str, script: Vec<Input>) -> Option<String> {
    let h = Harness::new(test);
    search_in_projects(&h);
    let app = h.project("app");
    let api = format!("{app}/api");
    fs::create_dir_all(&api).expect("create api");
    let lib = h.project("lib");
    write_bookmarks(&h, &[api.as_str(), lib.as_str()]);
    h.terminal.script(script);
    let _ = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    base(h.backend.attached_to())
}

/// Projects `app` and `lib`, with `app/api` and `lib` bookmarked. Runs the bookmark picker with `script` and returns the
/// session it attached to.
fn bookmark_picker_opened(test: &str, script: Vec<Input>) -> Option<String> {
    let h = Harness::new(test);
    search_in_projects(&h);
    let app = h.project("app");
    let api = format!("{app}/api");
    fs::create_dir_all(&api).expect("create api");
    let lib = h.project("lib");
    write_bookmarks(&h, &[api.as_str(), lib.as_str()]);
    h.terminal.script(script);
    let _ = bookmark_picker(&h.ctx(), &ProfileOpt::Unset);
    base(h.backend.attached_to())
}

// FR-TUI-11, P-52: j moves the selection down in the edit window, as the down arrow does
#[test]
fn edit_window_j_moves_down_as_the_down_arrow_does() {
    let arrow = edit_bookmarks("tui_vim-edit-j-arrow", vec![down(), enter(), esc()]);
    let vim = edit_bookmarks("tui_vim-edit-j", vec![j(), enter(), esc()]);
    assert_eq!(arrow, "<app>/docs\n", "the down arrow reaches docs");
    assert_eq!(vim, arrow);
}

// FR-TUI-11, P-52: k moves the selection up in the edit window, as the up arrow does
#[test]
fn edit_window_k_moves_up_as_the_up_arrow_does() {
    let arrow = edit_bookmarks(
        "tui_vim-edit-k-arrow",
        vec![down(), down(), up(), enter(), esc()],
    );
    let vim = edit_bookmarks("tui_vim-edit-k", vec![down(), down(), k(), enter(), esc()]);
    assert_eq!(arrow, "<app>/docs\n", "the up arrow returns to docs");
    assert_eq!(vim, arrow);
}

// FR-TUI-11, P-52: j and k stop at the ends of the edit window list, as the arrows do (no wrap, A-28)
#[test]
fn edit_window_j_and_k_stop_at_the_ends_as_the_arrows_do() {
    let bottom_arrow = edit_bookmarks(
        "tui_vim-edit-bottom-arrow",
        then(repeat(down, 5), vec![enter(), esc()]),
    );
    let bottom_vim = edit_bookmarks(
        "tui_vim-edit-bottom",
        then(repeat(j, 5), vec![enter(), esc()]),
    );
    assert_eq!(bottom_arrow, "<app>/src\n", "down clamps on the last row");
    assert_eq!(bottom_vim, bottom_arrow);

    let top_arrow = edit_bookmarks(
        "tui_vim-edit-top-arrow",
        then(repeat(up, 5), vec![enter(), esc()]),
    );
    let top_vim = edit_bookmarks("tui_vim-edit-top", then(repeat(k, 5), vec![enter(), esc()]));
    assert_eq!(
        top_arrow, "",
        "up clamps on the project root, which cannot be bookmarked"
    );
    assert_eq!(top_vim, top_arrow);
}

// FR-TUI-11, P-52: j and k move the selection in the session tree, as the arrows do, and stop at the ends
#[test]
fn session_tree_j_and_k_move_as_the_arrows_do() {
    let down_arrow = session_tree_opened("tui_vim-stree-down-arrow", vec![down(), enter(), esc()]);
    let down_vim = session_tree_opened("tui_vim-stree-j", vec![j(), enter(), esc()]);
    assert!(down_arrow.is_some(), "the down arrow reaches a session");
    assert_eq!(down_vim, down_arrow);

    let back_arrow = session_tree_opened(
        "tui_vim-stree-up-arrow",
        vec![down(), down(), up(), enter(), esc()],
    );
    let back_vim =
        session_tree_opened("tui_vim-stree-k", vec![down(), down(), k(), enter(), esc()]);
    assert_eq!(back_vim, back_arrow);

    let bottom_arrow = session_tree_opened(
        "tui_vim-stree-bottom-arrow",
        then(repeat(down, 5), vec![enter(), esc()]),
    );
    let bottom_vim = session_tree_opened(
        "tui_vim-stree-bottom",
        then(repeat(j, 5), vec![enter(), esc()]),
    );
    assert_eq!(bottom_vim, bottom_arrow, "j clamps like down");

    let top_arrow = session_tree_opened(
        "tui_vim-stree-top-arrow",
        then(repeat(up, 5), vec![enter(), esc()]),
    );
    let top_vim = session_tree_opened(
        "tui_vim-stree-top",
        then(repeat(k, 5), vec![enter(), esc()]),
    );
    assert_eq!(top_vim, top_arrow, "k clamps like up");
}

// FR-TUI-11, P-52: j and k move the selection in the bookmark tree, as the arrows do, and stop at the ends
#[test]
fn bookmark_tree_j_and_k_move_as_the_arrows_do() {
    let down_arrow = bookmark_tree_opened("tui_vim-btree-down-arrow", vec![down(), enter(), esc()]);
    let down_vim = bookmark_tree_opened("tui_vim-btree-j", vec![j(), enter(), esc()]);
    assert!(down_arrow.is_some(), "the down arrow reaches a bookmark");
    assert_eq!(down_vim, down_arrow);

    let back_arrow = bookmark_tree_opened(
        "tui_vim-btree-up-arrow",
        vec![down(), down(), up(), enter(), esc()],
    );
    let back_vim =
        bookmark_tree_opened("tui_vim-btree-k", vec![down(), down(), k(), enter(), esc()]);
    assert_eq!(back_vim, back_arrow);

    let bottom_arrow = bookmark_tree_opened(
        "tui_vim-btree-bottom-arrow",
        then(repeat(down, 8), vec![enter(), esc()]),
    );
    let bottom_vim = bookmark_tree_opened(
        "tui_vim-btree-bottom",
        then(repeat(j, 8), vec![enter(), esc()]),
    );
    assert_eq!(bottom_vim, bottom_arrow, "j clamps like down");

    let top_arrow = bookmark_tree_opened(
        "tui_vim-btree-top-arrow",
        then(repeat(up, 8), vec![enter(), esc()]),
    );
    let top_vim = bookmark_tree_opened(
        "tui_vim-btree-top",
        then(repeat(k, 8), vec![enter(), esc()]),
    );
    assert_eq!(top_vim, top_arrow, "k clamps like up");
}

// FR-TUI-11, P-52: ctrl-j moves the project picker selection down, as the down arrow does
#[test]
fn project_picker_ctrl_j_moves_down_as_the_down_arrow_does() {
    let arrow = project_opened("tui_vim-project-down-arrow", "app", vec![down(), enter()]);
    let vim = project_opened("tui_vim-project-ctrl-j", "app", vec![ctrl_j(), enter()]);
    let launch = project_opened("tui_vim-project-no-move", "app", vec![enter()]);
    assert!(arrow.is_some(), "the down arrow opens the second row");
    assert_ne!(arrow, launch, "the down arrow moves off the launch row");
    assert_eq!(vim, arrow);
}

// FR-TUI-11, P-52: ctrl-k moves the project picker selection up, as the up arrow does
#[test]
fn project_picker_ctrl_k_moves_up_as_the_up_arrow_does() {
    let arrow = project_opened(
        "tui_vim-project-up-arrow",
        "lib",
        vec![down(), down(), up(), enter()],
    );
    let vim = project_opened(
        "tui_vim-project-ctrl-k",
        "lib",
        vec![down(), down(), ctrl_k(), enter()],
    );
    assert!(arrow.is_some(), "the up arrow opens a row");
    assert_eq!(vim, arrow);
}

// FR-TUI-11, P-52: a plain j or k is typed into the project picker filter, shown on the prompt
#[test]
fn project_picker_plain_j_and_k_type_into_the_filter() {
    let h = project_harness("tui_vim-project-typed", "app");
    h.terminal.script(vec![j(), k(), esc()]);
    let _ = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("> jk")),
        "the prompt shows the typed filter: {lines:#?}"
    );
}

// FR-TUI-11, P-52: a plain j or k typed into the project picker filter does not move the selection (backspace clears it)
#[test]
fn project_picker_plain_j_and_k_do_not_move_the_selection() {
    let launch = project_opened("tui_vim-project-plain-baseline", "lib", vec![enter()]);
    let with_j = project_opened(
        "tui_vim-project-plain-j",
        "lib",
        vec![j(), backspace(), enter()],
    );
    assert!(launch.is_some(), "enter opens the launch row");
    assert_eq!(with_j, launch, "j leaves the selection on the launch row");
    let with_k = project_opened(
        "tui_vim-project-plain-k",
        "lib",
        vec![k(), backspace(), enter()],
    );
    assert_eq!(with_k, launch, "k leaves the selection on the launch row");
}

// FR-TUI-11, FR-TUI-07, P-52: the same key map is used by every window: ctrl-j and ctrl-k are the down and up keys
// of the key map, and ctrl-j moves the selection in each window as the down arrow does
#[test]
fn every_window_uses_the_same_key_map_for_vim_movement() {
    let ctrl_j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
    let ctrl_k_key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert_eq!(keys::map_key(&ctrl_j_key), Some(Action::Down));
    assert_eq!(keys::map_key(&ctrl_k_key), Some(Action::Up));

    // Project picker (filter): ctrl-j then enter opens the same row the down arrow does.
    let project_down = project_opened("tui_vim-same-project-arrow", "app", vec![down(), enter()]);
    let project_ctrl = project_opened("tui_vim-same-project-ctrl", "app", vec![ctrl_j(), enter()]);
    assert!(project_down.is_some());
    assert_eq!(project_ctrl, project_down, "project picker");

    // Bookmark picker (filter): ctrl-j moves down.
    let picker_down = bookmark_picker_opened("tui_vim-same-bpicker-arrow", vec![down(), enter()]);
    let picker_ctrl = bookmark_picker_opened("tui_vim-same-bpicker-ctrl", vec![ctrl_j(), enter()]);
    assert!(picker_down.is_some());
    assert_eq!(picker_ctrl, picker_down, "bookmark picker");

    // Edit window (no filter): ctrl-j moves down, as j does.
    let edit_ctrl = edit_bookmarks("tui_vim-same-edit-ctrl", vec![ctrl_j(), enter(), esc()]);
    assert_eq!(edit_ctrl, "<app>/docs\n", "edit window");

    // Bookmark tree and session tree: ctrl-j moves down as the down arrow does.
    let btree_down = bookmark_tree_opened("tui_vim-same-btree-arrow", vec![down(), enter(), esc()]);
    let btree_ctrl =
        bookmark_tree_opened("tui_vim-same-btree-ctrl", vec![ctrl_j(), enter(), esc()]);
    assert!(btree_down.is_some());
    assert_eq!(btree_ctrl, btree_down, "bookmark tree");

    let stree_down = session_tree_opened("tui_vim-same-stree-arrow", vec![down(), enter(), esc()]);
    let stree_ctrl = session_tree_opened("tui_vim-same-stree-ctrl", vec![ctrl_j(), enter(), esc()]);
    assert_eq!(stree_ctrl, stree_down, "session tree");
}

// FR-TUI-11, FR-TUI-03, FR-TUI-04, P-52: the footers keep their requirements text, and the arrow keys still move
#[test]
fn footers_are_unchanged_and_arrow_keys_still_move() {
    let h = project_harness("tui_vim-footer-project", "app");
    h.terminal.set_size(220, 24);
    h.terminal.script(vec![esc()]);
    let _ = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert!(
        shows(&h, PROJECT_FOOTER),
        "project footer: {:#?}",
        screen(&h)
    );
    let launch = project_opened("tui_vim-footer-project-enter", "app", vec![enter()]);
    let moved = project_opened("tui_vim-footer-project-down", "app", vec![down(), enter()]);
    assert_ne!(moved, launch, "the down arrow still moves the selection");

    let h = Harness::new("tui_vim-footer-edit");
    h.terminal.set_size(120, 24);
    let app = edit_fixture(&h);
    h.terminal.script(vec![down(), esc()]);
    let _ = edit(&h.ctx(), Some(&app), false);
    assert!(shows(&h, EDIT_FOOTER), "edit footer: {:#?}", screen(&h));

    let h = Harness::new("tui_vim-footer-btree");
    h.terminal.set_size(120, 24);
    search_in_projects(&h);
    let app = h.project("app");
    let api = format!("{app}/api");
    fs::create_dir_all(&api).expect("create api");
    write_bookmarks(&h, &[api.as_str()]);
    h.terminal.script(vec![esc()]);
    let _ = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert!(
        shows(&h, BOOKMARK_TREE_FOOTER),
        "bookmark tree footer: {:#?}",
        screen(&h)
    );
}

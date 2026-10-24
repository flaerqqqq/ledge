//! TUI model tests for the bookmark tree and the session tree windows (docs/testing.md level 3, picker test pattern).
//! The windows run through `run_picker` with scripted keys (`FakeTerminalFactory`), over `FakeBackend` and temp
//! folders. Covers FR-TREE-11 (enter on a gray connector), FR-TREE-12 (the window stays open when its last entry
//! is removed), FR-ACT-06, FR-ACT-07, FR-ACT-08, FR-ACT-09 (the alt keys of both trees), with D-12, D-17, A-38.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::ensure_session;
use ledge::app::pickers::{bookmark_tree_picker, session_tree_picker};
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::{Profile, ProfileName};
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::Outcome;
use ledge::ports::backend::{SessionBackend, OPT_PROFILE};
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;

const DEV: &str = r#"{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}"#;
const WEB: &str = r#"{"windows": [{"name": "server", "command": "npm start"}, {"name": "logs"}]}"#;
const FRESH: &str = r#"{"windows": [{"name": "fresh"}]}"#;
const TMUX_SET: &str = "/tmp/tmux-1000/default,1,0";
const SESSION_FOOTER: &str = "enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node";
const BOOKMARK_FOOTER: &str =
    "enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node";

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn alt(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT))
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
fn name_of(dir: &AbsPath) -> String {
    session_name(dir)
}

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

fn exists(h: &Harness, name: &str) -> bool {
    h.backend
        .list_sessions()
        .expect("list")
        .iter()
        .any(|session| session.name == name)
}

/// Window names with the typed lines of each window, in order.
fn windows(h: &Harness, name: &str) -> Vec<(String, Vec<String>)> {
    h.backend.windows_with_typed(name)
}

fn expected(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    rows.iter()
        .map(|(name, typed)| {
            (
                name.to_string(),
                typed.iter().map(|line| line.to_string()).collect(),
            )
        })
        .collect()
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

/// Writes the config with one search path: the `projects` folder under the temp root.
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

fn bookmarks_text(h: &Harness) -> String {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")).unwrap_or_default()
}

/// Builds the managed session of `dir` with the profile text (recorded as its profile name).
fn live(h: &Harness, dir: &AbsPath, name: &str, text: &str) {
    let profile = Profile::parse(&ProfileName::parse(name).expect("name"), text).expect("profile");
    ensure_session(&h.backend, dir, &profile).expect("create session");
}

/// Two projects, `app` and `lib`, with the managed sessions `app` (dev), `app/api` (web) and `lib` (dev). The
/// dev profile file is rewritten afterwards, so a rebuild with the recorded profile shows the new windows.
struct Fixture {
    app: AbsPath,
    api: AbsPath,
    lib: AbsPath,
}

fn session_fixture(h: &Harness) -> Fixture {
    search_in_projects(h);
    h.write_profile("dev", DEV);
    h.write_profile("web", WEB);
    let app = abs(&h.project("app"));
    let api = abs(&format!("{}/api", app.as_str()));
    let lib = abs(&h.project("lib"));
    live(h, &app, "dev", DEV);
    live(h, &api, "web", WEB);
    live(h, &lib, "dev", DEV);
    h.write_profile("dev", FRESH);
    Fixture { app, api, lib }
}

/// Makes the session `name` the one the user is in (TMUX set, client switched to it).
fn be_inside(h: &mut Harness, name: &str) {
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend.switch_client(name).expect("switch client");
}

// FR-TREE-11: enter on a gray connector does not open anything; the window stays and escape cancels
#[test]
fn enter_on_a_gray_connector_does_not_open() {
    let h = Harness::new("tui_trees-connector");
    search_in_projects(&h);
    let deep = abs(&format!("{}/src/x", h.project("app")));
    live(&h, &deep, "dev", DEV);
    h.terminal.script(vec![down(), enter(), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None, "a connector is not opened");
}

// FR-TREE-11: enter on a session entry opens its directory
#[test]
fn enter_on_a_session_entry_opens_it() {
    let h = Harness::new("tui_trees-enter-session");
    let fixture = session_fixture(&h);
    h.terminal.script(vec![enter()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(
        h.backend.attached_to(),
        Some(name_of(&fixture.app)),
        "the chosen session is attached"
    );
}

// FR-TREE-12: the bookmark tree stays open when alt-d removes its last bookmark, and says so (D-12)
#[test]
fn bookmark_tree_stays_open_when_its_last_entry_is_removed() {
    let h = Harness::new("tui_trees-last-bookmark");
    search_in_projects(&h);
    let api = format!("{}/api", h.project("app"));
    write_bookmarks(&h, &[api.as_str()]);
    h.terminal.script(vec![alt('d'), esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        h.terminal.opened(),
        1,
        "the same window stays open after the removal"
    );
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("No bookmarks.")),
        "empty tree says so: {lines:#?}"
    );
    assert!(lines.iter().any(|line| line.contains("Bookmark tree")));
    assert_eq!(bookmarks_text(&h).trim(), "");
}

// FR-TREE-12: the session tree stays open when alt-x kills its last session (D-12)
#[test]
fn session_tree_stays_open_when_its_last_entry_is_killed() {
    let h = Harness::new("tui_trees-last-session");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    live(&h, &app, "dev", DEV);
    h.terminal.script(vec![alt('x'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(!exists(&h, &name_of(&app)));
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("No ledge sessions.")),
        "empty session tree says so: {lines:#?}"
    );
}

// FR-ACT-08: alt-r rebuilds each session under the node with its recorded profile; a --profile option does not apply (A-38)
#[test]
fn alt_r_rebuilds_each_session_with_its_recorded_profile_ignoring_the_option() {
    let h = Harness::new("tui_trees-alt-r");
    let f = session_fixture(&h);
    h.terminal.script(vec![alt('r'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Named("web".to_string()));
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(windows(&h, &name_of(&f.app)), expected(&[("fresh", &[])]));
    assert_eq!(
        windows(&h, &name_of(&f.api)),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        windows(&h, &name_of(&f.lib)),
        expected(&[("editor", &["nvim"]), ("shell", &[])]),
        "the session outside the node is not rebuilt"
    );
}

// FR-ACT-09: alt-p asks one profile and rebuilds every session under the node with it; escape returns to the tree
#[test]
fn alt_p_asks_one_profile_and_rebuilds_the_node_with_it() {
    let h = Harness::new("tui_trees-alt-p");
    let f = session_fixture(&h);
    h.terminal.script(vec![alt('p')]);
    // Profile picker rows: basic, dev, web. Down twice reaches web.
    h.terminal.queue(vec![down(), down(), enter()]);
    h.terminal.queue(vec![esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.terminal.opened(), 3, "tree, profile picker, tree again");
    for dir in [&f.app, &f.api] {
        let name = name_of(dir);
        assert_eq!(
            windows(&h, &name),
            expected(&[("server", &["npm start"]), ("logs", &[])])
        );
        assert_eq!(h.backend.option(&name, OPT_PROFILE).as_deref(), Some("web"));
    }
    assert_eq!(
        windows(&h, &name_of(&f.lib)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
}

// FR-ACT-09: escape in the profile picker changes nothing and returns to the tree
#[test]
fn alt_p_escape_in_the_profile_picker_changes_nothing() {
    let h = Harness::new("tui_trees-alt-p-escape");
    let f = session_fixture(&h);
    h.terminal.script(vec![alt('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        windows(&h, &name_of(&f.app)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
}

// FR-ACT-07: alt-x on a node never kills the session the user is in; the others under the node are killed
#[test]
fn alt_x_spares_the_session_the_user_is_in() {
    let mut h = Harness::new("tui_trees-alt-x-current");
    let f = session_fixture(&h);
    be_inside(&mut h, &name_of(&f.app));
    h.terminal.script(vec![alt('x'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(exists(&h, &name_of(&f.app)), "the current session stays");
    assert!(
        !exists(&h, &name_of(&f.api)),
        "the other session under the node is killed"
    );
    assert!(
        exists(&h, &name_of(&f.lib)),
        "a session outside the node stays"
    );
}

// FR-ACT-07: alt-r on a node never rebuilds the session the user is in
#[test]
fn alt_r_leaves_the_session_the_user_is_in_alone() {
    let mut h = Harness::new("tui_trees-alt-r-current");
    let f = session_fixture(&h);
    be_inside(&mut h, &name_of(&f.app));
    h.terminal.script(vec![alt('r'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        windows(&h, &name_of(&f.app)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(
        windows(&h, &name_of(&f.api)),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
}

// FR-ACT-06: alt-d is bookmark tree only; in the session tree it does nothing
#[test]
fn alt_d_does_nothing_in_the_session_tree() {
    let h = Harness::new("tui_trees-alt-d-session");
    let f = session_fixture(&h);
    h.terminal.script(vec![alt('d'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(exists(&h, &name_of(&f.app)));
    assert!(exists(&h, &name_of(&f.api)));
}

// FR-ACT-10 (key path of alt-d): deletes every bookmark under the node and kills their managed sessions; a
// bookmark outside the node stays; the window stays open
#[test]
fn alt_d_on_a_bookmark_node_deletes_the_bookmarks_under_it() {
    let h = Harness::new("tui_trees-alt-d-bookmark");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let api = abs(&format!("{}/api", app.as_str()));
    let web = abs(&format!("{}/web", app.as_str()));
    let lib = abs(&h.project("lib"));
    let x = abs(&format!("{}/x", lib.as_str()));
    write_bookmarks(&h, &[api.as_str(), web.as_str(), x.as_str()]);
    live(&h, &api, "dev", DEV);
    live(&h, &x, "dev", DEV);
    h.terminal.script(vec![alt('d'), esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(bookmarks_text(&h).trim(), x.as_str());
    assert!(!exists(&h, &name_of(&api)));
    assert!(exists(&h, &name_of(&x)));
    assert!(screen(&h).iter().any(|line| line.contains("x")));
}

// Architecture 6.2, Screens: the footers of the two trees are the given texts
#[test]
fn trees_show_their_given_footers() {
    let h = Harness::new("tui_trees-footers");
    session_fixture(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    assert!(
        screen(&h).iter().any(|line| line.contains(SESSION_FOOTER)),
        "session tree footer"
    );

    let h2 = Harness::new("tui_trees-footers-bookmarks");
    search_in_projects(&h2);
    let api = format!("{}/api", h2.project("app"));
    write_bookmarks(&h2, &[api.as_str()]);
    h2.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h2.ctx(), &ProfileOpt::Unset).expect("bookmark tree");
    assert!(
        screen(&h2)
            .iter()
            .any(|line| line.contains(BOOKMARK_FOOTER)),
        "bookmark tree footer"
    );
}

// FR-SESS-07, FR-SESS-08: a gray folder without a session shows the session name it would get
#[test]
fn gray_folder_shows_the_session_name_it_would_get() {
    let h = Harness::new("tui_trees-gray-name");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains(&name_of(&app))),
        "gray root shows its would-be name: {lines:#?}"
    );
}

// FR-SESS-07, FR-SESS-08: enter on a gray folder without a session opens that folder under its would-be name
#[test]
fn enter_on_a_gray_folder_opens_it_under_its_would_be_name() {
    let h = Harness::new("tui_trees-gray-enter");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    // Row 0 is the gray project root (openable in the session tree); row 1 is the connector, row 2 the session.
    h.terminal.script(vec![enter()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&app)));
    assert!(exists(&h, &name_of(&app)), "the folder got its session");
}

// FR-SESS-08: enter on a deeper session entry opens that session's own directory
#[test]
fn enter_on_a_deep_session_entry_opens_its_directory() {
    let h = Harness::new("tui_trees-deep-enter");
    let f = session_fixture(&h);
    // Screens: enter opens only when the directory exists, so the session's folder must be on disk.
    fs::create_dir_all(f.api.as_str()).expect("create api folder");
    h.terminal.script(vec![down(), enter()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&f.api)));
}

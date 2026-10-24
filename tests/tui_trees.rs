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
use ledge::domain::tree::TreeScope;
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
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
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
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(bookmarks_text(&h).trim(), x.as_str());
    assert!(!exists(&h, &name_of(&api)));
    assert!(exists(&h, &name_of(&x)));
    assert!(screen(&h).iter().any(|line| line.contains("x")));
}

// FR-BM-13, FR-TUI-07: ctrl-d on a selected bookmark row deletes that bookmark and keeps the window open;
// the other bookmark stays
#[test]
fn ctrl_d_on_a_bookmark_row_deletes_that_bookmark_and_stays_open() {
    let h = Harness::new("tui_trees-ctrl-d-bookmark");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let api = abs(&format!("{}/api", app.as_str()));
    let web = abs(&format!("{}/web", app.as_str()));
    write_bookmarks(&h, &[api.as_str(), web.as_str()]);
    h.terminal.script(vec![
        down(),
        Input::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
        esc(),
    ]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    // The root row is first, so one down moves to `api` (sorted before `web`): `api` goes, `web` stays.
    assert_eq!(bookmarks_text(&h).trim(), web.as_str());
}

// FR-ACT-02, FR-SESS-12, FR-TUI-07: ctrl-x on a bookmark row with a live session kills that session
#[test]
fn ctrl_x_on_a_bookmark_row_with_a_session_kills_it() {
    let h = Harness::new("tui_trees-ctrl-x-bookmark");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let api = abs(&format!("{}/api", app.as_str()));
    write_bookmarks(&h, &[api.as_str()]);
    live(&h, &api, "dev", DEV);
    assert!(exists(&h, &name_of(&api)));
    h.terminal.script(vec![
        down(),
        Input::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        esc(),
    ]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(!exists(&h, &name_of(&api)));
    assert_eq!(bookmarks_text(&h).trim(), api.as_str());
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
    bookmark_tree_picker(&h2.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
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

// FR-SESS-06, FR-TREE-11: the session tree shows the session name first; the tree-drawn directory column comes next,
// and the header lists the columns in the same order
#[test]
fn session_tree_row_starts_with_the_session_name() {
    let h = Harness::new("tui_trees-session-first");
    let f = session_fixture(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let header = line_with(&lines, "Directory");
    let (session_title, directory_title) = (header.find("Session"), header.find("Directory"));
    assert!(session_title < directory_title, "header order: {header}");
    let name = name_of(&f.api);
    let row = line_with(&lines, &name);
    let body = row.trim_start_matches('│').trim_start();
    assert!(
        body.starts_with(&name),
        "first column is the session: {row}"
    );
    let rest = body.get(name.len()..).unwrap_or_default().trim_start();
    assert!(
        rest.starts_with("api"),
        "directory follows the session: {row}"
    );
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

// FR-TREE-05: a given directory shows only the bookmark tree that contains it, and the cursor starts on its nearest node
#[test]
fn bookmark_tree_for_a_directory_shows_only_its_tree() {
    let h = Harness::new("tui_trees-scope-tree");
    search_in_projects(&h);
    let api = h.project("app/api");
    let lib_x = format!("{}/x", h.project("lib"));
    write_bookmarks(&h, &[api.as_str(), lib_x.as_str()]);
    let dir = abs(&format!("{api}/src"));
    h.terminal.script(vec![enter()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::Directory(dir));
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(
        h.backend.attached_to(),
        Some(name_of(&abs(&api))),
        "the cursor starts on the nearest node, the api bookmark"
    );
    let lines = screen(&h);
    assert!(
        !lines.iter().any(|line| line.contains("lib")),
        "other tree hidden: {lines:#?}"
    );
}

// FR-TREE-07: a directory outside every tree shows only its root; enter on the gray root does nothing
#[test]
fn bookmark_tree_for_a_directory_outside_every_tree_shows_only_the_root() {
    let h = Harness::new("tui_trees-scope-root");
    search_in_projects(&h);
    let api = h.project("app/api");
    write_bookmarks(&h, &[api.as_str()]);
    let lib = abs(&h.project("lib"));
    h.terminal.script(vec![enter(), esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::Directory(lib));
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None, "the gray root is not opened");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("lib")),
        "root shown: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("api")),
        "only the root: {lines:#?}"
    );
}

// FR-TREE-08: with no directory and no session scope, every bookmark tree is shown
#[test]
fn bookmark_tree_without_a_directory_shows_every_tree() {
    let h = Harness::new("tui_trees-scope-all");
    search_in_projects(&h);
    let api = h.project("app/api");
    let lib_x = format!("{}/x", h.project("lib"));
    write_bookmarks(&h, &[api.as_str(), lib_x.as_str()]);
    h.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("tree");
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("api")), "{lines:#?}");
    assert!(lines.iter().any(|line| line.contains("lib")), "{lines:#?}");
}

// FR-TREE-06: `bm tree --session-scope` shows the tree of the directory of the current session
#[test]
fn session_scope_shows_the_tree_of_the_current_session_directory() {
    let mut h = Harness::new("tui_trees-session-scope");
    search_in_projects(&h);
    let api = abs(&h.project("app/api"));
    let lib_x = format!("{}/x", h.project("lib"));
    write_bookmarks(&h, &[api.as_str(), lib_x.as_str()]);
    live(&h, &api, "dev", DEV);
    be_inside(&mut h, &name_of(&api));
    h.terminal.script(vec![esc()]);
    let args = vec![
        "bm".to_string(),
        "tree".to_string(),
        "--session-scope".to_string(),
    ];
    let outcome = ledge::cli::run(&args, &h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("api")), "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.contains("lib")),
        "other tree hidden: {lines:#?}"
    );
}

// D-12, FR-TREE-08: an empty bookmark tree says `No bookmarks.` and stays open
#[test]
fn empty_bookmark_tree_says_so_and_stays_open() {
    let h = Harness::new("tui_trees-empty");
    search_in_projects(&h);
    write_bookmarks(&h, &[]);
    h.terminal.script(vec![esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.terminal.opened(), 1, "the window opened and stayed open");
    assert!(screen(&h).iter().any(|line| line.contains("No bookmarks.")));
}

/// The first screen row that contains `needle`.
fn line_with<'a>(lines: &'a [String], needle: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(needle))
        .map(String::as_str)
        .unwrap_or_else(|| panic!("no row with {needle:?} in {lines:#?}"))
}

// FR-TREE-13, P-53: a bookmark whose directory has a managed session shows the session name and its profile on its row
#[test]
fn bookmark_row_with_a_session_shows_its_name_and_profile() {
    let h = Harness::new("tui_trees-bm-session");
    search_in_projects(&h);
    let api = abs(&format!("{}/api", h.project("app")));
    fs::create_dir_all(api.as_str()).expect("create api");
    write_bookmarks(&h, &[api.as_str()]);
    live(&h, &api, "zeta", DEV);
    h.terminal.script(vec![esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    let row = line_with(&lines, &name_of(&api));
    assert!(row.contains("zeta"), "profile on the same row: {row}");
}

// FR-TREE-13, P-53: a bookmark without a managed session shows no session name and no profile
#[test]
fn bookmark_row_without_a_session_shows_no_session_text() {
    let h = Harness::new("tui_trees-bm-no-session");
    search_in_projects(&h);
    let api = abs(&format!("{}/api", h.project("app")));
    fs::create_dir_all(api.as_str()).expect("create api");
    write_bookmarks(&h, &[api.as_str()]);
    h.terminal.script(vec![esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(
        line_with(&lines, "api").contains("api"),
        "the bookmark is shown: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains(&name_of(&api))),
        "no session name for a bookmark without a session: {lines:#?}"
    );
}

// FR-TREE-13, P-53: a session created for a bookmark shows on its row once the tree is drawn again (the next reload)
#[test]
fn session_created_for_a_bookmark_shows_on_the_next_draw_of_the_tree() {
    let h = Harness::new("tui_trees-bm-next-reload");
    search_in_projects(&h);
    let api = abs(&format!("{}/api", h.project("app")));
    fs::create_dir_all(api.as_str()).expect("create api");
    write_bookmarks(&h, &[api.as_str()]);
    h.terminal.script(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let _ = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    assert!(
        !screen(&h).iter().any(|line| line.contains(&name_of(&api))),
        "no session yet: {:#?}",
        screen(&h)
    );
    live(&h, &api, "zeta", DEV);
    let _ = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
    let lines = screen(&h);
    let row = line_with(&lines, &name_of(&api));
    assert!(row.contains("zeta"), "profile on the same row: {row}");
}

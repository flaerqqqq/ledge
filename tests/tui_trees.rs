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
use support::ctx::{Harness, FIXED_NOW};

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

/// The Project cell of a session row: the first word after `text` (the session name, or `(no session)` on a
/// placeholder). The names and paths in these tests have no spaces.
fn cell_after<'a>(row: &'a str, text: &str) -> &'a str {
    let start = row.find(text).map_or(row.len(), |index| index + text.len());
    row.get(start..)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

/// Makes the session `name` the one the user is in (TMUX set, client switched to it).
fn be_inside(h: &mut Harness, name: &str) {
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend.switch_client(name).expect("switch client");
}

// FR-TREE-11: enter on a gray connector of the bookmark tree does not open anything; the window stays and escape cancels
#[test]
fn enter_on_a_gray_connector_does_not_open() {
    let h = Harness::new("tui_trees-connector");
    search_in_projects(&h);
    let app = h.project("app");
    let deep = format!("{app}/src/x");
    write_bookmarks(&h, &[deep.as_str()]);
    // Row 0 is the gray project root, row 1 the gray connector `src`, row 2 the bookmark.
    h.terminal.script(vec![down(), enter(), esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All);
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

// FR-SESS-06, P-54: the gray placeholder row of a project without its own session shows its would-be session name
// followed by "(no session)"; the sessions of the project are listed under it
#[test]
fn placeholder_row_shows_its_would_be_name_and_no_session_text() {
    let h = Harness::new("tui_trees-placeholder-text");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let row = line_with(&lines, &name_of(&app));
    assert!(
        row.contains(&format!("{} (no session)", name_of(&app))),
        "placeholder row says so: {row}"
    );
}

// FR-SESS-03, P-55: a session inside a project shows the project name in the Project column, not its directory
#[test]
fn project_session_shows_the_project_name() {
    let h = Harness::new("tui_trees-project-name");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let row = line_with(&lines, &name_of(&deep));
    assert_eq!(
        cell_after(row, &name_of(&deep)),
        "app",
        "the project name, not the directory: {row}"
    );
    assert!(!row.contains(deep.as_str()), "no full path: {row}");
}

// FR-SESS-03, P-55: outside every project, the Project column shows the directory with the home folder written as
// `~`; a directory outside the home folder shows its full path
#[test]
fn session_outside_every_project_shows_home_as_tilde() {
    let h = Harness::new("tui_trees-home-tilde");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let notes = h.root().join("home").join("notes");
    let scratch = h.root().join("scratch");
    fs::create_dir_all(&notes).expect("create notes");
    fs::create_dir_all(&scratch).expect("create scratch");
    let notes = abs(notes.to_str().expect("UTF-8 path"));
    let scratch = abs(scratch.to_str().expect("UTF-8 path"));
    live(&h, &notes, "basic", FRESH);
    live(&h, &scratch, "basic", FRESH);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let notes_row = line_with(&lines, &name_of(&notes));
    assert_eq!(
        cell_after(notes_row, &name_of(&notes)),
        "~/notes",
        "home folder as ~: {notes_row}"
    );
    let scratch_row = line_with(&lines, &name_of(&scratch));
    assert_eq!(
        cell_after(scratch_row, &name_of(&scratch)),
        scratch.as_str(),
        "full path outside home: {scratch_row}"
    );
}

// FR-SESS-06, P-54: enter on the placeholder opens the project's directory under its would-be session name
#[test]
fn enter_on_the_placeholder_opens_its_directory_under_its_would_be_name() {
    let h = Harness::new("tui_trees-placeholder-enter");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    // Row 0 is the placeholder of the project; row 1 is the session below it.
    h.terminal.script(vec![enter()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&app)));
    assert!(exists(&h, &name_of(&app)), "the project got its session");
}

// FR-SESS-06, P-54, P-55: the session tree shows the session name first, then the Project column (the name of the
// project of the session, here `app` for `app/api`), then the profile; the header lists the columns in the same order
#[test]
fn session_tree_row_starts_with_the_session_name() {
    let h = Harness::new("tui_trees-session-first");
    h.terminal.set_size(240, 40);
    let f = session_fixture(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let header = line_with(&lines, "Project");
    let (session_title, project_title) = (header.find("Session"), header.find("Project"));
    assert!(session_title < project_title, "header order: {header}");
    assert!(
        !header.contains("Directory"),
        "no Directory header: {header}"
    );
    let name = name_of(&f.api);
    let row = line_with(&lines, &name);
    // The first column may start with the tree lead (the branch glyphs, design 3.6); the name follows.
    let body = row
        .trim_start_matches('│')
        .trim_start()
        .trim_start_matches(|c: char| "|`-│├└─ ".contains(c));
    assert!(
        body.starts_with(&name),
        "first column is the session after its lead: {row}"
    );
    assert_eq!(
        cell_after(row, &name),
        "app",
        "the Project column is the project name: {row}"
    );
    assert!(
        !row.contains(f.api.as_str()),
        "the full path is not shown: {row}"
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

/// Cells before `name` on a screen row: the column where its Session cell text starts. The selection bar (design 3.1)
/// is column 0 of the selected row; the current mark and the branch glyphs (design 3.6, section 8 item 11) come before
/// the name, one cell each, so this is the indent of its depth (two cells per depth).
fn indent_of(row: &str, name: &str) -> usize {
    let at = row.find(name).expect("the name is on the row");
    row[..at].chars().count()
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

// FR-ACT-07, P-54: alt-x on the placeholder kills every session under the project folder
#[test]
fn alt_x_on_the_placeholder_kills_the_sessions_under_it() {
    let h = Harness::new("tui_trees-placeholder-alt-x");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let server = abs(&format!("{}/server", app.as_str()));
    let web = abs(&format!("{}/web", app.as_str()));
    live(&h, &server, "dev", DEV);
    live(&h, &web, "basic", FRESH);
    h.terminal.script(vec![alt('x'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(
        !exists(&h, &name_of(&server)),
        "the session under the node is killed"
    );
    assert!(
        !exists(&h, &name_of(&web)),
        "the session under the node is killed"
    );
}

// FR-SESS-06, P-54: the example of P-54 in one tree. Sessions nest by nearest session, the project without its own
// session has a placeholder, a session outside every project is at depth 0, and a project with no session is not listed
#[test]
fn session_tree_nests_the_p54_example_by_nearest_session() {
    let h = Harness::new("tui_trees-p54-example");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let ledge = abs(&h.project("ledge"));
    let docs = abs(&h.project("ledge/docs"));
    let adr = abs(&h.project("ledge/docs/adr"));
    let api = abs(&h.project("api"));
    let server = abs(&h.project("api/server"));
    let web = abs(&h.project("api/web"));
    h.project("notes");
    let scratch = abs(h.root().join("scratch").to_str().expect("UTF-8 path"));
    fs::create_dir_all(scratch.as_str()).expect("create scratch");
    live(&h, &ledge, "dev", DEV);
    live(&h, &docs, "basic", FRESH);
    live(&h, &adr, "basic", FRESH);
    live(&h, &server, "dev", DEV);
    live(&h, &web, "basic", FRESH);
    live(&h, &scratch, "basic", FRESH);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);

    // Each session cell is indented two cells per depth, from the depth 0 row of ledge. The Project cell is the
    // project name for a session inside a project, and the full path for scratch (outside every project, not under HOME).
    let base = indent_of(line_with(&lines, &name_of(&ledge)), &name_of(&ledge));
    let rows = [
        (&ledge, 0, "ledge"),
        (&docs, 1, "ledge"),
        (&adr, 2, "ledge"),
        (&server, 1, "api"),
        (&web, 1, "api"),
        (&scratch, 0, scratch.as_str()),
    ];
    for (dir, depth, project) in rows {
        let row = line_with(&lines, &name_of(dir));
        assert_eq!(
            indent_of(row, &name_of(dir)),
            base + 2 * depth,
            "depth of {}: {row}",
            dir.as_str()
        );
        assert_eq!(
            cell_after(row, &name_of(dir)),
            project,
            "project column of {}: {row}",
            dir.as_str()
        );
        assert!(
            !row.chars().any(|c| "├└─".contains(c)),
            "no tree glyphs on a session row: {row}"
        );
    }

    // The placeholder of api: its would-be name, then the text, at depth 0; its Project cell is the project name.
    let api_row = line_with(&lines, &name_of(&api));
    assert!(
        api_row.contains(&format!("{} (no session)", name_of(&api))),
        "placeholder text: {api_row}"
    );
    assert_eq!(
        indent_of(api_row, &name_of(&api)),
        base,
        "placeholder at depth 0"
    );
    assert_eq!(
        cell_after(api_row, "(no session)"),
        "api",
        "placeholder project column: {api_row}"
    );

    // Tree order inside each project: a parent row comes before the rows under it.
    let index = |dir: &AbsPath| {
        lines
            .iter()
            .position(|line| line.contains(&name_of(dir)))
            .expect("row")
    };
    assert!(index(&ledge) < index(&docs) && index(&docs) < index(&adr));
    assert!(index(&api) < index(&server) && index(&server) < index(&web));

    assert!(
        !lines.iter().any(|line| line.contains("notes")),
        "a project without a session is not listed: {lines:#?}"
    );
}

// FR-SESS-03, P-56: the session tree has an Activity column after Profile, and each session shows the age text the
// switcher shows for it
#[test]
fn session_tree_has_an_activity_column_with_the_switcher_age() {
    let h = Harness::new("tui_trees-activity-column");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let api = abs(&format!("{}/api", app.as_str()));
    let lib = abs(&h.project("lib"));
    live(&h, &app, "dev", DEV);
    live(&h, &api, "dev", DEV);
    live(&h, &lib, "dev", DEV);
    h.backend.set_activity(&name_of(&app), FIXED_NOW - 30);
    h.backend.set_activity(&name_of(&api), FIXED_NOW - 300);
    h.backend.set_activity(&name_of(&lib), FIXED_NOW - 7_200);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let header = line_with(&lines, "Activity");
    assert!(
        header.find("Profile") < header.find("Activity"),
        "Activity after Profile: {header}"
    );
    for (dir, text) in [(&app, "just now"), (&api, "5m ago"), (&lib, "2h ago")] {
        let row = line_with(&lines, &name_of(dir));
        assert!(
            row.contains(text),
            "{} row lacks {text:?}: {row}",
            dir.as_str()
        );
    }
}

// FR-SESS-03, P-56: a project placeholder row has no activity text
#[test]
fn placeholder_row_has_no_activity_text() {
    let h = Harness::new("tui_trees-placeholder-activity");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    h.backend.set_activity(&name_of(&deep), FIXED_NOW - 30);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let row = line_with(&lines, "(no session)");
    assert!(
        !row.contains("ago") && !row.contains("just now"),
        "the placeholder has no activity: {row}"
    );
}

// FR-SESS-06, P-56: sessions that share a parent are listed most recent first, in the tree window
#[test]
fn siblings_under_a_project_are_listed_most_recent_first() {
    let h = Harness::new("tui_trees-sibling-order");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let server = abs(&format!("{}/server", app.as_str()));
    let web = abs(&format!("{}/web", app.as_str()));
    live(&h, &server, "dev", DEV);
    live(&h, &web, "dev", DEV);
    h.backend.set_activity(&name_of(&server), FIXED_NOW - 900);
    h.backend.set_activity(&name_of(&web), FIXED_NOW - 50);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let index = |dir: &AbsPath| {
        lines
            .iter()
            .position(|line| line.contains(&name_of(dir)))
            .expect("row")
    };
    assert!(
        index(&web) < index(&server),
        "the more recent session first: {lines:#?}"
    );
}

// FR-SESS-06, P-56: top-level projects are listed by their most recent session, first, even when the project
// name sorts later
#[test]
fn top_level_projects_are_listed_most_recent_first() {
    let h = Harness::new("tui_trees-project-order");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let lib = abs(&h.project("lib"));
    live(&h, &app, "dev", DEV);
    live(&h, &lib, "dev", DEV);
    h.backend.set_activity(&name_of(&app), FIXED_NOW - 7_200);
    h.backend.set_activity(&name_of(&lib), FIXED_NOW - 30);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let index = |dir: &AbsPath| {
        lines
            .iter()
            .position(|line| line.contains(&name_of(dir)))
            .expect("row")
    };
    assert!(index(&lib) < index(&app), "lib first: {lines:#?}");
}

// FR-TREE-01, P-56: the bookmark tree has no Activity column; its header is unchanged
#[test]
fn bookmark_tree_has_no_activity_column() {
    let h = Harness::new("tui_trees-bookmark-no-activity");
    search_in_projects(&h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    live(&h, &app, "dev", DEV);
    h.backend.set_activity(&name_of(&app), FIXED_NOW - 30);
    write_bookmarks(&h, &[app.as_str()]);
    h.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    let lines = screen(&h);
    assert!(
        !lines.iter().any(|line| line.contains("Activity")),
        "no Activity header: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("just now")),
        "no activity text: {lines:#?}"
    );
}

// FR-TREE-07, A-13: the gray root of a directory outside every project shows it with the home folder as "~"
#[test]
fn bookmark_tree_root_outside_every_project_under_home_shows_tilde() {
    let h = Harness::new("tui_trees-root-home-tilde");
    search_in_projects(&h);
    let api = h.project("app/api");
    write_bookmarks(&h, &[api.as_str()]);
    let notes = h.root().join("home").join("notes");
    fs::create_dir_all(&notes).expect("create notes");
    let notes_text = notes.to_str().expect("UTF-8 path").to_string();
    let notes = abs(&notes_text);
    h.terminal.set_size(240, 40);
    h.terminal.script(vec![esc()]);
    let outcome = bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::Directory(notes));
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("~/notes")),
        "home folder as ~: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains(&notes_text)),
        "no full path under HOME: {lines:#?}"
    );
}

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

/// Writes the locale of a UTF-8 terminal into the environment (design 7): the branch glyphs are drawn.
fn utf8(h: &mut Harness) {
    h.env = h.env.clone().with_var("LANG", "en_US.UTF-8");
}

/// The branch fixture of design 3.6: the project `app` with no session of its own, a child `first` with a child
/// `deep`, and a second child `second`. P-56: `first` is the most recent, so it is listed first. Returns the three
/// session names.
fn branch_fixture(h: &Harness) -> (String, String, String) {
    search_in_projects(h);
    h.terminal.set_size(240, 40);
    let app = abs(&h.project("app"));
    let first = abs(&format!("{}/first", app.as_str()));
    let deep = abs(&format!("{}/first/deep", app.as_str()));
    let second = abs(&format!("{}/second", app.as_str()));
    for dir in [&first, &deep, &second] {
        fs::create_dir_all(dir.as_str()).expect("create folder");
    }
    live(h, &first, "dev", DEV);
    h.backend.set_activity(&name_of(&first), FIXED_NOW - 10);
    live(h, &deep, "dev", DEV);
    h.backend.set_activity(&name_of(&deep), FIXED_NOW - 20);
    live(h, &second, "dev", DEV);
    h.backend.set_activity(&name_of(&second), FIXED_NOW - 100);
    (name_of(&first), name_of(&deep), name_of(&second))
}

// FR-TREE-02, design 3.6, section 8 item 11: the session tree draws branch glyphs in place of the indent under UTF-8
#[test]
fn session_tree_draws_branch_glyphs_under_utf8() {
    let mut h = Harness::new("tui_trees-glyphs-utf8");
    utf8(&mut h);
    let (first, deep, second) = branch_fixture(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    assert!(line_with(&lines, &first).contains("├─"), "first");
    assert!(line_with(&lines, &deep).contains("│ └─"), "deep");
    assert!(line_with(&lines, &second).contains("└─"), "second");
}

// design section 7: without a UTF-8 locale the branch glyphs fall back to ASCII, two cells per level
#[test]
fn session_tree_draws_ascii_glyphs_without_utf8() {
    let h = Harness::new("tui_trees-glyphs-ascii");
    let (first, deep, second) = branch_fixture(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    assert!(line_with(&lines, &first).contains("|-"), "first");
    assert!(line_with(&lines, &deep).contains("| `-"), "deep");
    assert!(line_with(&lines, &second).contains("`-"), "second");
}

// FR-TREE-02, design 3.8: the bookmark tree draws branch glyphs in its Directory column
#[test]
fn bookmark_tree_draws_branch_glyphs_under_utf8() {
    let mut h = Harness::new("tui_trees-bm-glyphs");
    search_in_projects(&h);
    utf8(&mut h);
    let app = h.project("app");
    write_bookmarks(&h, &[&format!("{app}/src/x")]);
    h.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    let lines = screen(&h);
    assert!(line_with(&lines, "(src)").contains("└─(src)"));
    assert!(line_with(&lines, "└─x").contains("  └─x"));
}

// FR-TUI-08, design 3.8: a bookmark whose directory is gone shows the (missing) cue in the bookmark tree
#[test]
fn bookmark_tree_shows_the_missing_cue() {
    let h = Harness::new("tui_trees-bm-missing");
    search_in_projects(&h);
    let app = h.project("app");
    write_bookmarks(&h, &[&format!("{app}/gone")]);
    h.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    assert!(line_with(&screen(&h), "gone").contains("gone (missing)"));
}

// FR-TUI-08, design 3.6: a session whose directory is gone shows the (missing) cue in the session tree
#[test]
fn session_tree_shows_the_missing_cue() {
    let h = Harness::new("tui_trees-session-missing");
    search_in_projects(&h);
    let scratch = abs(h.root().join("scratch").to_str().expect("UTF-8 path"));
    fs::create_dir_all(scratch.as_str()).expect("create scratch");
    live(&h, &scratch, "basic", FRESH);
    fs::remove_dir_all(scratch.as_str()).expect("remove scratch");
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    assert!(line_with(&lines, &name_of(&scratch)).contains("(missing)"));
}

// FR-SESS-02, FR-PICK-05, design 3.6: the session the user is in has no star in the session tree
#[test]
fn current_session_has_no_star_in_the_session_tree() {
    let mut h = Harness::new("tui_trees-current-star");
    let fixture = session_fixture(&h);
    be_inside(&mut h, &name_of(&fixture.app));
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    let app = name_of(&fixture.app);
    assert!(line_with(&lines, &app).contains(&app), "{lines:#?}");
    assert!(!lines.iter().any(|line| line.contains('*')), "{lines:#?}");
}

// design 3.2, 4.2 (P-59): an empty session tree and an empty bookmark tree show only "esc close" and stay open
#[test]
fn empty_trees_show_only_esc_close_in_the_footer() {
    let h = Harness::new("tui_trees-empty-session-footer");
    search_in_projects(&h);
    h.terminal.script(vec![esc()]);
    session_tree_picker(&h.ctx(), &ProfileOpt::Unset).expect("session tree");
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("esc close")));
    assert!(!lines.iter().any(|line| line.contains("^r/^p/^x selected")));

    let h = Harness::new("tui_trees-empty-bookmark-footer");
    search_in_projects(&h);
    write_bookmarks(&h, &[]);
    h.terminal.script(vec![esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("esc close")));
    assert!(!lines
        .iter()
        .any(|line| line.contains("^r/^p/^x/^d selected")));
}

// FR-TUI-07, design 4.5, 4.6 (P-59): ctrl-r, ctrl-p and ctrl-x on a row with no session change nothing and say so
#[test]
fn ctrl_keys_on_a_row_without_a_session_say_so_and_change_nothing() {
    let h = Harness::new("tui_trees-no-session-keys");
    search_in_projects(&h);
    let app = abs(&h.project("app"));
    let deep = abs(&format!("{}/src/x", app.as_str()));
    live(&h, &deep, "dev", DEV);
    // Row 0 is the placeholder of `app`, which has no session of its own.
    h.terminal
        .script(vec![ctrl('r'), ctrl('p'), ctrl('x'), esc()]);
    let outcome = session_tree_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.terminal.opened(), 1, "ctrl-p opens no profile picker");
    assert!(exists(&h, &name_of(&deep)), "ctrl-x kills nothing");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No session on this row.")));
}

// FR-TREE-11, design 4.6 (P-59): enter on a gray connector says so in the status line and opens nothing
#[test]
fn enter_on_a_connector_says_nothing_to_open() {
    let h = Harness::new("tui_trees-enter-connector-message");
    search_in_projects(&h);
    let app = h.project("app");
    write_bookmarks(&h, &[&format!("{app}/src/x")]);
    h.terminal.script(vec![enter(), esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    assert_eq!(h.backend.attached_to(), None);
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("Nothing to open on this row.")));
}

// FR-BM-13, design 4.1 (P-59): ctrl-d on a row that is not a bookmark deletes nothing and says so
#[test]
fn ctrl_d_on_a_connector_says_only_a_bookmark_can_be_deleted() {
    let h = Harness::new("tui_trees-ctrl-d-connector");
    search_in_projects(&h);
    let app = h.project("app");
    let deep = format!("{app}/src/x");
    write_bookmarks(&h, &[&deep]);
    h.terminal.script(vec![ctrl('d'), esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    assert!(bookmarks_text(&h).contains(&deep), "the bookmark stays");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("Only a bookmark can be deleted.")));
}

// FR-ACT-01, design 4.5 (P-59): ctrl-x on a bookmark without a session says so in the bookmark tree
#[test]
fn ctrl_x_on_a_bookmark_without_a_session_says_so() {
    let h = Harness::new("tui_trees-ctrl-x-no-session");
    search_in_projects(&h);
    let app = h.project("app");
    write_bookmarks(&h, &[&format!("{app}/x")]);
    h.terminal.script(vec![down(), ctrl('x'), esc()]);
    bookmark_tree_picker(&h.ctx(), &ProfileOpt::Unset, &TreeScope::All).expect("bookmark tree");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No session on this row.")));
}

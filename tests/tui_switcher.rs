//! TUI model tests for the session switcher (docs/testing.md level 3; FR-SESS-01..04, D-07, D-08, D-12, FR-TUI-10).
//! `app::pickers::session_switcher` runs with `FakeBackend`, `FakeClock` (fixed time, `FIXED_NOW`), `FakeTerminal`
//! (scripted keys, `TestBackend`) and `RealFs` on temp folders. Rows, order, age text, columns, footer, label,
//! enter, ctrl-r, ctrl-p, ctrl-x and the empty state are checked on the drawn screens and on the outcome.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::ensure_session;
use ledge::app::pickers::session_switcher;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::error::Outcome;
use ledge::ports::backend::{NewSession, SessionBackend};
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use support::ctx::{Harness, FIXED_NOW};

const SWITCHER_FOOTER: &str = "enter switch · ^r rebuild · ^p profile · ^x kill";
const TMUX_SET: &str = "/tmp/tmux-1000/default,1,0";
const DEV_PROFILE: &str =
    r#"{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}"#;

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

/// Config with the one search path `<root>/projects`.
fn search_config(h: &Harness) {
    let projects = h.root().join("projects");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"]}}",
        projects.display()
    ));
}

/// Makes the test run inside tmux (FR-SESS-02, FR-SESS-04): `TMUX` is set.
fn inside_tmux(h: &mut Harness) {
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
}

/// A managed session of `dir` built from the basic profile, with its activity set to `activity`.
fn managed(h: &Harness, dir: &str, activity: i64) -> String {
    let profile = Profile::basic();
    ensure_session(&h.backend, &AbsPath::clean(dir, &AbsPath::root()), &profile)
        .expect("create session");
    let name = name_of(dir);
    h.backend.set_activity(&name, activity);
    name
}

/// A session that did not come from ledge: no managed marker, no ledge options (FR-SESS-01).
fn unmanaged(h: &Harness, name: &str) {
    h.backend
        .create_session(&NewSession {
            name: name.to_string(),
            dir: h.root().display().to_string(),
            window_name: "shell".to_string(),
        })
        .expect("create unmanaged session");
}

/// The Project cell of a switcher row: the first word after `text` (the session name).
fn cell_after<'a>(row: &'a str, text: &str) -> &'a str {
    let start = row.find(text).map_or(row.len(), |index| index + text.len());
    row.get(start..)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

/// The last screen drawn, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("the switcher opened a terminal")
        .borrow()
        .lines
        .clone()
}

/// The screen of every terminal the switcher opened, in order.
fn screens(h: &Harness) -> Vec<Vec<String>> {
    h.terminal
        .records()
        .iter()
        .map(|record| record.borrow().lines.clone())
        .collect()
}

/// The first screen row that contains `needle`.
fn line_with<'a>(lines: &'a [String], needle: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(needle))
        .map(String::as_str)
        .unwrap_or_else(|| panic!("no row with {needle:?} in {lines:#?}"))
}

/// Index of the first screen row that contains `needle`.
fn row_index(lines: &[String], needle: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row with {needle:?} in {lines:#?}"))
}

fn restores(h: &Harness) -> usize {
    h.terminal
        .last_record()
        .expect("the switcher opened a terminal")
        .borrow()
        .restores
}

fn live_names(h: &Harness) -> Vec<String> {
    h.backend
        .list_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|session| session.name)
        .collect()
}

// FR-SESS-01: only managed sessions are listed; an unmanaged session is not, and with none left the list is empty
#[test]
fn unmanaged_sessions_are_not_listed() {
    let h = Harness::new("tui_switcher-managed-only");
    search_config(&h);
    let app = h.project("app");
    let shown = managed(&h, &app, FIXED_NOW - 10);
    unmanaged(&h, "stray-1234");
    h.terminal.script(vec![esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains(&shown)));
    assert!(!lines.iter().any(|line| line.contains("stray-1234")));
}

// FR-SESS-01, D-08: with only unmanaged sessions the switcher is empty and shows the ledge wording
#[test]
fn only_unmanaged_sessions_show_the_empty_message() {
    let h = {
        let h = Harness::new("tui_switcher-only-unmanaged");
        search_config(&h);
        unmanaged(&h, "stray-1234");
        h.terminal.script(vec![esc()]);
        h
    };
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No ledge sessions.")));
}

// FR-SESS-02: the session the user is in comes first, then the rest by most recent activity
#[test]
fn current_session_first_then_most_recent_activity() {
    let mut h = Harness::new("tui_switcher-order");
    search_config(&h);
    inside_tmux(&mut h);
    let a = h.project("alpha");
    let b = h.project("bravo");
    let c = h.project("charlie");
    let alpha = managed(&h, &a, FIXED_NOW - 900);
    let bravo = managed(&h, &b, FIXED_NOW - 500);
    let charlie = managed(&h, &c, FIXED_NOW - 50);
    h.backend.set_current(&bravo);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    let first = row_index(&lines, &bravo);
    let second = row_index(&lines, &charlie);
    let third = row_index(&lines, &alpha);
    assert!(first < second && second < third, "order was {lines:#?}");
}

// FR-SESS-02, A-10: equal activity is ordered by session name
#[test]
fn equal_activity_is_ordered_by_name() {
    let h = Harness::new("tui_switcher-ties");
    search_config(&h);
    let z = h.project("zulu");
    let a = h.project("alpha");
    let zulu = managed(&h, &z, FIXED_NOW - 100);
    let alpha = managed(&h, &a, FIXED_NOW - 100);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(row_index(&lines, &alpha) < row_index(&lines, &zulu));
}

// FR-SESS-03: the activity column shows just now, <n>m ago, <n>h ago, <n>d ago
#[test]
fn activity_column_shows_the_age_text() {
    let h = Harness::new("tui_switcher-age");
    search_config(&h);
    let names = [
        (h.project("a-now"), FIXED_NOW - 30, "just now"),
        (h.project("b-min"), FIXED_NOW - 300, "5m ago"),
        (h.project("c-hour"), FIXED_NOW - 7_200, "2h ago"),
        (h.project("d-day"), FIXED_NOW - 3 * 86_400, "3d ago"),
    ];
    let mut expected = Vec::new();
    for (dir, activity, text) in &names {
        expected.push((managed(&h, dir, *activity), *text));
    }
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    for (name, text) in expected {
        let row = line_with(&lines, &name);
        assert!(row.contains(text), "{name} row {row:?} lacks {text:?}");
    }
}

// FR-SESS-03, architecture 6.2: the columns are session, project, profile and activity; the border label is
// "Sessions" with its icon; the footer is the switcher key list
#[test]
fn headers_label_and_footer_are_as_specified() {
    let h = Harness::new("tui_switcher-chrome");
    search_config(&h);
    let app = h.project("app");
    managed(&h, &app, FIXED_NOW - 10);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("◉ Sessions")));
    let header = line_with(&lines, "Activity");
    for title in ["Session", "Project", "Profile", "Activity"] {
        assert!(header.contains(title), "header {header:?} lacks {title:?}");
    }
    assert!(lines.iter().any(|line| line.contains(SWITCHER_FOOTER)));
}

// D-07, P-55: the Project column names the project of the session directory; a directory outside every search path
// shows its full path there, so the name appears in the session name and in the path
#[test]
fn session_outside_every_search_path_shows_its_directory_in_the_project_column() {
    let h = Harness::new("tui_switcher-d07");
    h.terminal.set_size(240, 40);
    search_config(&h);
    let alpha = h.project("alpha");
    let outside = format!("{}/elsewhere/zeta", h.root().display());
    fs::create_dir_all(&outside).expect("create outside folder");
    let inside_name = managed(&h, &alpha, FIXED_NOW - 10);
    let outside_name = managed(&h, &outside, FIXED_NOW - 20);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    let project_row = line_with(&lines, &inside_name);
    assert_eq!(
        cell_after(project_row, &inside_name),
        "alpha",
        "row {project_row:?}"
    );
    let outside_row = line_with(&lines, &outside_name);
    assert_eq!(
        cell_after(outside_row, &outside_name),
        outside,
        "row {outside_row:?}"
    );
    assert!(!outside_row.contains("alpha"));
}

// FR-SESS-03, P-55: a session outside every project under the home folder shows `~/...` in the Project column
#[test]
fn session_under_home_outside_every_project_shows_tilde() {
    let h = Harness::new("tui_switcher-home-tilde");
    search_config(&h);
    let notes = h.root().join("home").join("notes");
    fs::create_dir_all(&notes).expect("create notes");
    let notes = notes.display().to_string();
    let name = managed(&h, &notes, FIXED_NOW - 10);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    let row = line_with(&lines, &name);
    assert_eq!(cell_after(row, &name), "~/notes", "row {row:?}");
}

// FR-SESS-04: enter attaches to the chosen session outside tmux
#[test]
fn enter_outside_tmux_attaches_to_the_chosen_session() {
    let h = Harness::new("tui_switcher-attach");
    search_config(&h);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    h.terminal.script(vec![enter()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name));
    assert_eq!(restores(&h), 1);
}

// FR-SESS-04: enter inside tmux switches the client to the chosen session and does not attach
#[test]
fn enter_inside_tmux_switches_the_client() {
    let mut h = Harness::new("tui_switcher-switch");
    search_config(&h);
    inside_tmux(&mut h);
    let app = h.project("app");
    let lib = h.project("lib");
    managed(&h, &app, FIXED_NOW - 10);
    let lib_name = managed(&h, &lib, FIXED_NOW - 20);
    h.terminal.script(vec![down(), enter()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.current_session(), Ok(Some(lib_name)));
    assert_eq!(h.backend.attached_to(), None);
}

// FR-ACT-01, screen table: ctrl-x kills the selected session and the list stays open without it
#[test]
fn kill_key_removes_the_row_and_the_list_stays_open() {
    let h = Harness::new("tui_switcher-kill");
    search_config(&h);
    let app = h.project("app");
    let lib = h.project("lib");
    let app_name = managed(&h, &app, FIXED_NOW - 10);
    let lib_name = managed(&h, &lib, FIXED_NOW - 20);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(live_names(&h), vec![lib_name.clone()]);
    let lines = screen(&h);
    assert!(!lines.iter().any(|line| line.contains(&app_name)));
    assert!(lines.iter().any(|line| line.contains(&lib_name)));
    assert_eq!(restores(&h), 1);
}

// FR-ACT-07, D-17: ctrl-x on the session the user is in is refused; the status line says so and nothing is killed
#[test]
fn kill_key_refuses_the_session_the_user_is_in() {
    let mut h = Harness::new("tui_switcher-kill-current");
    search_config(&h);
    inside_tmux(&mut h);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    h.backend.set_current(&name);
    h.terminal.script(vec![ctrl('x'), esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    assert_eq!(live_names(&h), vec![name]);
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("cannot kill the session you are in.")));
}

// D-12, FR-TUI-10: killing the last session leaves the switcher open with the empty message
#[test]
fn killing_the_last_session_keeps_the_switcher_open_with_the_empty_message() {
    let h = Harness::new("tui_switcher-kill-last");
    search_config(&h);
    let app = h.project("app");
    managed(&h, &app, FIXED_NOW - 10);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(live_names(&h).is_empty());
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No ledge sessions.")));
    assert_eq!(h.terminal.opened(), 1);
}

// D-12: with no sessions at all the switcher opens with the message and stays open until escape
#[test]
fn empty_switcher_stays_open_until_escape() {
    let h = Harness::new("tui_switcher-empty");
    search_config(&h);
    h.terminal.script(vec![esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No ledge sessions.")));
    assert_eq!(restores(&h), 1);
}

// FR-ACT-02, FR-SESS-04: ctrl-r rebuilds the selected session in its directory
#[test]
fn rebuild_key_rebuilds_the_session_in_its_directory() {
    let h = Harness::new("tui_switcher-rebuild");
    search_config(&h);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    let before = h.backend.window_names(&name);
    h.backend
        .add_window(&name, "extra", &app)
        .expect("add a window");
    assert!(h.backend.window_names(&name).contains(&"extra".to_string()));
    h.terminal.script(vec![ctrl('r'), esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.window_names(&name), before);
    assert_eq!(live_names(&h), vec![name]);
}

// FR-PICK-09 (applied to the switcher): ctrl-r on a session whose directory is gone shows the message and builds nothing
#[test]
fn rebuild_key_on_a_missing_directory_shows_the_message() {
    let h = Harness::new("tui_switcher-rebuild-missing");
    search_config(&h);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    fs::remove_dir_all(&app).expect("remove the project folder");
    h.terminal.script(vec![ctrl('r'), esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("Missing directory:")));
    assert_eq!(live_names(&h), vec![name]);
}

// FR-PROF-13, A-08: ctrl-p opens the profile picker over the switcher; escape returns to the switcher unchanged
#[test]
fn profile_key_opens_the_profile_picker_and_escape_returns() {
    let h = Harness::new("tui_switcher-profile-escape");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    let before = h.backend.window_names(&name);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let all = screens(&h);
    assert_eq!(all.len(), 3, "switcher, profile picker, switcher");
    assert!(all[1].iter().any(|line| line.contains("Profiles")));
    assert!(all[2].iter().any(|line| line.contains("Sessions")));
    assert_eq!(h.backend.window_names(&name), before);
}

// FR-ACT-04, FR-ACT-12: ctrl-p, then a profile, rebuilds the selected session with that profile
#[test]
fn profile_key_then_choice_rebuilds_the_session_with_that_profile() {
    let h = Harness::new("tui_switcher-profile-rebuild");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    let name = managed(&h, &app, FIXED_NOW - 10);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), enter()]);
    h.terminal.queue(vec![esc()]);
    let outcome = session_switcher(&h.ctx());
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        h.backend.window_names(&name),
        vec!["editor".to_string(), "shell".to_string()]
    );
    assert_eq!(
        h.backend.option(&name, "@ledge_profile"),
        Some("dev".to_string())
    );
}

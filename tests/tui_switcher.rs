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
// "Sessions" (no icon); the footer is the switcher key list
#[test]
fn headers_label_and_footer_are_as_specified() {
    let h = Harness::new("tui_switcher-chrome");
    search_config(&h);
    let app = h.project("app");
    managed(&h, &app, FIXED_NOW - 10);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(lines[0].starts_with("╭ Sessions "), "{:?}", lines[0]);
    assert!(!lines[0].contains('◉'), "{:?}", lines[0]);
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

// FR-ACT-01, FR-ACT-07, P-63: ctrl-x on the session the user is in switches the client to the most recent other
// session and kills this one; the list stays open without it
#[test]
fn kill_key_on_the_session_the_user_is_in_switches_to_another_session() {
    let mut h = Harness::new("tui_switcher-kill-current");
    search_config(&h);
    inside_tmux(&mut h);
    let app = h.project("app");
    let lib = h.project("lib");
    let name = managed(&h, &app, FIXED_NOW - 10);
    let lib_name = managed(&h, &lib, FIXED_NOW - 20);
    h.backend.set_current(&name);
    h.terminal.script(vec![ctrl('x'), esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    assert_eq!(live_names(&h), vec![lib_name.clone()]);
    assert_eq!(h.backend.current_session(), Ok(Some(lib_name)));
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

// FR-TUI-08, design 3.2, 3.5: a session whose directory is gone shows the (missing) cue in the Project column
#[test]
fn session_with_a_missing_directory_shows_the_cue_in_the_project_column() {
    let h = Harness::new("tui_switcher-missing-cue");
    search_config(&h);
    let gone = h.project("gone");
    let name = managed(&h, &gone, FIXED_NOW - 60);
    fs::remove_dir_all(&gone).expect("remove the directory");
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(line_with(&lines, &name).contains("(missing)"), "{lines:#?}");
}

// FR-PICK-05, FR-SESS-02: the session the user is in has no star; the switcher lists it in the same column as the others
#[test]
fn current_session_has_no_star() {
    let mut h = Harness::new("tui_switcher-current-star");
    search_config(&h);
    inside_tmux(&mut h);
    let a = h.project("alpha");
    let b = h.project("bravo");
    let alpha = managed(&h, &a, FIXED_NOW - 900);
    let bravo = managed(&h, &b, FIXED_NOW - 500);
    h.backend.set_current(&bravo);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(
        line_with(&lines, &bravo).starts_with(&format!("│ {bravo}")),
        "{lines:#?}"
    );
    assert!(
        line_with(&lines, &alpha).starts_with(&format!("│ {alpha}")),
        "{lines:#?}"
    );
    assert!(!lines.iter().any(|line| line.contains('*')), "{lines:#?}");
}

// design 3.2, 4.2 (P-59): an empty switcher shows only "esc close" in its footer and stays open
#[test]
fn empty_switcher_footer_is_esc_close() {
    let h = {
        let h = Harness::new("tui_switcher-empty-footer");
        search_config(&h);
        unmanaged(&h, "stray-1234");
        h.terminal.script(vec![esc()]);
        h
    };
    assert_eq!(session_switcher(&h.ctx()), Ok(Outcome::Cancelled));
    let lines = screen(&h);
    assert!(lines.iter().any(|line| line.contains("esc close")));
    assert!(!lines.iter().any(|line| line.contains("enter switch")));
}

// design 2.1, 2.2, 2.4 (S-42): at 120 cells the switcher has a Path column, and a Details section for the selected
// session: its path, its name, its windows and its profile
#[test]
fn switcher_at_120_cells_shows_the_path_column_and_details() {
    let h = Harness::new("tui_switcher-wide");
    h.terminal.set_size(120, 30);
    search_config(&h);
    let alpha = h.project("alpha");
    let name = managed(&h, &alpha, FIXED_NOW - 10);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(line_with(&lines, "Activity").contains("Path"), "{lines:#?}");
    let title = row_index(&lines, &format!("Details · {name}"));
    assert!(
        lines[title + 1].contains(&format!("Path      {alpha}")),
        "{:?}",
        lines[title + 1]
    );
    assert!(
        lines[title + 2].contains(&format!("Session   {name}")),
        "{:?}",
        lines[title + 2]
    );
    assert!(
        lines[title + 3].contains("Windows   "),
        "{:?}",
        lines[title + 3]
    );
    assert!(
        lines[title + 4].contains("Profile   "),
        "{:?}",
        lines[title + 4]
    );
}

/// Config with two search paths, `<root>/projects` and `<root>/work` (P-66).
fn search_config_two(h: &Harness) {
    let work = h.root().join("work");
    fs::create_dir_all(&work).expect("create work");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\", \"{}\"]}}",
        h.root().join("projects").display(),
        work.display()
    ));
}

/// A project folder `<root>/work/<name>`, the second search path's project (P-66); the path text.
fn work_project(h: &Harness, name: &str) -> String {
    let path = format!("{}/work/{name}", h.root().display());
    fs::create_dir_all(&path).expect("create work project");
    path
}

// P-66: two sessions in same-named projects show their labels, projects/api and work/api; a unique project keeps its name
#[test]
fn sessions_in_same_named_projects_show_distinct_labels() {
    let h = Harness::new("tui_switcher-p66-labels");
    h.terminal.set_size(240, 40);
    search_config_two(&h);
    let ours = h.project("api");
    let theirs = work_project(&h, "api");
    let lib = h.project("lib");
    let ours_name = managed(&h, &ours, FIXED_NOW - 10);
    let theirs_name = managed(&h, &theirs, FIXED_NOW - 20);
    let lib_name = managed(&h, &lib, FIXED_NOW - 30);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    let ours_row = line_with(&lines, &ours_name);
    assert_eq!(
        cell_after(ours_row, &ours_name),
        "projects/api",
        "{ours_row:?}"
    );
    let theirs_row = line_with(&lines, &theirs_name);
    assert_eq!(
        cell_after(theirs_row, &theirs_name),
        "work/api",
        "{theirs_row:?}"
    );
    let lib_row = line_with(&lines, &lib_name);
    assert_eq!(cell_after(lib_row, &lib_name), "lib", "{lib_row:?}");
}

// P-66: at a narrow width the label of a same-named project is cut in the middle, so its folder name still shows
#[test]
fn narrow_switcher_cuts_a_label_in_the_middle_and_keeps_its_folder() {
    let h = Harness::new("tui_switcher-p66-narrow");
    h.terminal.set_size(40, 30);
    let client = h.root().join("client-platform-team");
    fs::create_dir_all(&client).expect("create client folder");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\", \"{}\"]}}",
        h.root().join("projects").display(),
        client.display()
    ));
    let ours = h.project("api");
    let theirs = format!("{}/api", client.display());
    fs::create_dir_all(&theirs).expect("create api");
    managed(&h, &ours, FIXED_NOW - 10);
    let theirs_name = managed(&h, &theirs, FIXED_NOW - 20);
    h.terminal.script(vec![esc()]);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    let row = line_with(&lines, &theirs_name);
    let cell = cell_after(row, &theirs_name);
    assert!(
        cell.starts_with("client-") && cell.ends_with("/api") && cell.contains('…'),
        "cut label {cell:?} in {row:?}"
    );
}

/// Types `text` into the switcher filter, one character per key.
fn type_text(text: &str) -> Vec<Input> {
    text.chars().map(|c| key(KeyCode::Char(c))).collect()
}

// FR-TUI-02, S-54: typing a session name keeps only the session with that name; the other session is filtered out
#[test]
fn typing_a_session_name_keeps_only_that_session() {
    let h = Harness::new("tui_switcher-s54-session-name");
    h.terminal.set_size(240, 40);
    search_config(&h);
    let app = h.project("app");
    let lib = h.project("lib");
    let app_name = managed(&h, &app, FIXED_NOW - 10);
    let lib_name = managed(&h, &lib, FIXED_NOW - 20);
    let mut script = type_text(&app_name);
    script.push(esc());
    h.terminal.script(script);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains(&app_name)),
        "the typed session is kept: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains(&lib_name)),
        "the other session is filtered out: {lines:#?}"
    );
}

// FR-TUI-02, S-54: typing a project name does not match a session whose project is another one, even when its
// Project column shows that name only through the project it belongs to (session `app` in project `lib`)
#[test]
fn typing_a_project_name_does_not_match_a_session_of_another_project() {
    let h = Harness::new("tui_switcher-s54-project-name");
    h.terminal.set_size(240, 40);
    search_config(&h);
    let lib = h.project("lib");
    let nested = format!("{lib}/app");
    std::fs::create_dir_all(&nested).expect("create nested folder");
    let lib_name = managed(&h, &lib, FIXED_NOW - 10);
    let nested_name = managed(&h, &nested, FIXED_NOW - 20);
    let mut script = type_text("lib");
    script.push(esc());
    h.terminal.script(script);
    session_switcher(&h.ctx()).expect("switcher runs");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains(&lib_name)),
        "the session named lib is kept: {lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains(&nested_name)),
        "the session in project lib is not matched by its project name: {lines:#?}"
    );
}

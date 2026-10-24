//! TUI model tests for the project and history pickers (docs/testing.md level 3, picker test pattern). The
//! services in `app::pickers` run with `FakeBackend`, `FakeTerminalFactory` (scripted keys, `TestBackend`) and
//! `RealFs` on temp folders. Rows, labels, footers, the status line, enter, escape, the missing-directory rules
//! and the empty states are checked on the last drawn screen and on the outcome.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::ensure_session;
use ledge::app::pickers::{history_picker, project_picker};
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::{Profile, ProfileName};
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{LedgeError, Outcome};
use ledge::ports::backend::{NewSession, SessionBackend, OPT_PROFILE};
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;
use support::fake_backend::FakeBackend;

const PROJECT_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill";
const DEV_PROFILE: &str =
    r#"{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}"#;

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
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

/// Config with the one search path `<root>/projects`, which the harness creates empty.
fn search_config(h: &Harness) {
    let projects = h.root().join("projects");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"]}}",
        projects.display()
    ));
}

/// Launches the picker from `dir` (the current directory row, FR-PICK-04).
fn launch_from(h: &mut Harness, dir: &str) {
    h.cwd = AbsPath::clean(dir, &AbsPath::root());
}

/// The last screen drawn by the last picker, one string per row.
fn screen(h: &Harness) -> Vec<String> {
    h.terminal
        .last_record()
        .expect("a picker opened a terminal")
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

fn restores(h: &Harness) -> usize {
    h.terminal
        .last_record()
        .expect("a picker opened a terminal")
        .borrow()
        .restores
}

/// `history` file in the cache folder, with the given text.
fn write_history(h: &Harness, text: &str) {
    let cache = PathBuf::from(h.dirs.cache.as_str());
    fs::create_dir_all(&cache).expect("create cache folder");
    fs::write(cache.join("history"), text).expect("write history");
}

fn live(h: &Harness, dir: &str) {
    let profile = Profile::basic();
    ensure_session(&h.backend, &AbsPath::clean(dir, &AbsPath::root()), &profile)
        .expect("create session");
}

// FR-PICK-04: the launch directory's row comes first and is preselected, so enter opens it
#[test]
fn launch_directory_is_first_and_preselected() {
    let mut h = Harness::new("tui_project_picker-first");
    search_config(&h);
    h.project("app");
    let lib = h.project("lib");
    h.project("web");
    launch_from(&mut h, &lib);
    h.terminal.script(vec![enter()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&lib)));
}

// FR-PICK-04: a directory argument of the picker is the launch directory
#[test]
fn directory_argument_is_the_launch_directory() {
    let mut h = Harness::new("tui_project_picker-dir-arg");
    search_config(&h);
    h.project("app");
    let lib = h.project("lib");
    let projects = h.root().join("projects").display().to_string();
    launch_from(&mut h, &projects);
    h.terminal.script(vec![enter()]);
    let outcome = project_picker(&h.ctx(), Some(&lib), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&lib)));
}

// FR-PICK-05: a current directory that is not a project is an extra first row showing its path, marked "*"
#[test]
fn non_project_current_directory_is_an_extra_first_row_with_its_path() {
    let mut h = Harness::new("tui_project_picker-non-project");
    search_config(&h);
    h.terminal.set_size(220, 24);
    h.project("app");
    let projects = h.root().join("projects").display().to_string();
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let lines = screen(&h);
    let first = line_with(&lines, "*○");
    assert!(first.contains("/projects"), "{first}");
    assert!(line_with(&lines, "app").contains("○"));
}

// FR-PICK-06, FR-PICK-07: a live managed session shows its session name and profile and a filled marker;
// other projects have the empty marker. The border label and the footer are the project picker's (FR-TUI-04).
#[test]
fn live_session_shows_name_profile_and_filled_marker() {
    let mut h = Harness::new("tui_project_picker-live");
    search_config(&h);
    let app = h.project("app");
    let lib = h.project("lib");
    h.project("web");
    live(&h, &lib);
    launch_from(&mut h, &app);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    let lib_row = line_with(&lines, "lib");
    assert!(lib_row.contains("●"), "{lib_row}");
    assert!(lib_row.contains(&name_of(&lib)), "{lib_row}");
    assert!(lib_row.contains("basic"), "{lib_row}");
    assert!(line_with(&lines, "web").contains("○"));
    assert!(line_with(&lines, "*○").contains("app"));
    assert!(line_with(&lines, "▸ Projects & Submodules").contains("▸"));
    assert!(line_with(&lines, PROJECT_FOOTER).contains(PROJECT_FOOTER));
}

// FR-PICK-08: enter opens the chosen project through the open flow: its session is created and attached, and the
// project is recorded at the top of the history
#[test]
fn enter_opens_the_chosen_project_through_the_open_flow() {
    let mut h = Harness::new("tui_project_picker-open");
    search_config(&h);
    let app = h.project("app");
    let lib = h.project("lib");
    launch_from(&mut h, &app);
    h.terminal.script(vec![down(), enter()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&lib)));
    assert!(h.history_text().unwrap_or_default().starts_with(&lib));
}

// FR-PICK-09: a chosen directory that is gone shows its message with the picker open; nothing is opened
#[test]
fn missing_directory_on_enter_keeps_the_picker_open() {
    let mut h = Harness::new("tui_project_picker-missing");
    search_config(&h);
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    let gone = h.project("gone");
    h.terminal.remove_on_open(PathBuf::from(&gone));
    launch_from(&mut h, &app);
    h.terminal.script(vec![down(), enter(), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None);
    assert!(h.backend.list_sessions().unwrap_or_default().is_empty());
    let lines = screen(&h);
    assert!(
        line_with(&lines, "Missing directory: ").contains(&format!("Missing directory: {gone}")),
        "{lines:#?}"
    );
    assert_eq!(restores(&h), 1);
}

// FR-PICK-10: no search path gives the error that names the config file; no picker opens
#[test]
fn no_search_paths_is_an_error_naming_the_config_file() {
    let h = Harness::new("tui_project_picker-no-search");
    h.write_config(r#"{"search_paths": []}"#);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(
        outcome,
        Err(LedgeError::Fatal(format!(
            "No search paths defined in {}/config.json.",
            h.dirs.config.as_str()
        )))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// D-12: no projects at start closes with the info line and no window
#[test]
fn no_projects_closes_without_a_window() {
    let h = Harness::new("tui_project_picker-no-projects");
    search_config(&h);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(
        outcome,
        Ok(Outcome::Info(
            "No projects found in search paths.".to_string()
        ))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// FR-PICK-13: the border label names the profile that applies; the basic layout and --no-profile keep the default
#[test]
fn border_label_names_the_profile_that_applies() {
    let mut h = Harness::new("tui_project_picker-label");
    search_config(&h);
    let app = h.project("app");
    launch_from(&mut h, &app);
    let cases = [
        (ProfileOpt::Unset, "▸ Projects & Submodules"),
        (
            ProfileOpt::Named("dev".to_string()),
            "▸ Projects (Profile: dev)",
        ),
        (
            ProfileOpt::Named("basic".to_string()),
            "▸ Projects & Submodules",
        ),
        (ProfileOpt::None, "▸ Projects & Submodules"),
    ];
    for (option, label) in cases {
        h.terminal.script(vec![esc()]);
        project_picker(&h.ctx(), None, &option).expect("picker runs");
        assert!(
            line_with(&screen(&h), label).contains(label),
            "option {option:?}"
        );
    }
}

// FR-PICK-13: a default_profile from the config is the profile that applies to the open
#[test]
fn border_label_names_the_default_profile() {
    let mut h = Harness::new("tui_project_picker-label-default");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"], \"default_profile\": \"dev\"}}",
        h.root().join("projects").display()
    ));
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    assert!(line_with(&screen(&h), "Projects (Profile: dev)").contains("Projects (Profile: dev)"));
}

// FR-PICK-12: the --profile option applies to the session the open creates
#[test]
fn profile_option_applies_to_the_opened_session() {
    let mut h = Harness::new("tui_project_picker-profile");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    let lib = h.project("lib");
    launch_from(&mut h, &app);
    h.terminal.script(vec![down(), enter()]);
    let option = ProfileOpt::Named("dev".to_string());
    assert_eq!(project_picker(&h.ctx(), None, &option), Ok(Outcome::Done));
    assert_eq!(
        h.backend.window_names(&name_of(&lib)),
        vec!["editor".to_string(), "shell".to_string()]
    );
}

// FR-CMD-08: escape closes the picker without a choice: the outcome is Cancelled and nothing opens
#[test]
fn escape_cancels_without_opening_anything() {
    let mut h = Harness::new("tui_project_picker-cancel");
    search_config(&h);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None);
    assert_eq!(restores(&h), 1);
}

// Architecture 4, A-29: a failed session listing is an error with its detail
#[test]
fn failed_session_listing_is_an_error_with_its_detail() {
    let mut h = Harness::new("tui_project_picker-list-failure");
    search_config(&h);
    h.backend = FakeBackend::new().fail_on("list_sessions", "boom");
    let app = h.project("app");
    launch_from(&mut h, &app);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(
        outcome,
        Err(LedgeError::Fatal(
            "could not list sessions: boom".to_string()
        ))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// FR-HIST-05, FR-CMD-12: no history file prints the message and opens no window
#[test]
fn history_without_a_file_prints_the_message_and_stops() {
    let h = Harness::new("tui_history_picker-no-file");
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(
        outcome,
        Ok(Outcome::Info("No history cache found.".to_string()))
    );
    assert_eq!(h.terminal.opened(), 0);
}

// D-12, FR-CMD-12: an empty history file prints No history. and opens no window
#[test]
fn empty_history_file_prints_no_history_and_stops() {
    let h = Harness::new("tui_history_picker-empty");
    write_history(&h, "");
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Info("No history.".to_string())));
    assert_eq!(h.terminal.opened(), 0);
}

// FR-HIST-04, architecture 6.2: entries newest first, the history label, headers and footer; live sessions shown
#[test]
fn history_lists_entries_newest_first_with_the_footer() {
    let h = Harness::new("tui_history_picker-rows");
    h.terminal.set_size(220, 24);
    let a = h.project("a");
    let b = h.project("b");
    write_history(&h, &format!("{b}\n{a}\n"));
    live(&h, &b);
    h.terminal.script(vec![esc()]);
    history_picker(&h.ctx(), &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    assert!(line_with(&lines, "↺ History").contains("↺ History"));
    let header = line_with(&lines, "Session");
    assert!(
        header.contains("Project") && header.contains("Profile"),
        "{header}"
    );
    let first = line_with(&lines, &b);
    assert!(first.contains(&name_of(&b)), "{first}");
    assert!(line_with(&lines, PROJECT_FOOTER).contains(PROJECT_FOOTER));
    let row_of = |needle: &str| lines.iter().position(|line| line.contains(needle));
    assert!(row_of(&b).is_some() && row_of(&a).is_some());
    assert!(row_of(&b) < row_of(&a), "newest entry comes first");
}

// FR-HIST-04: enter opens the chosen history entry through the open flow
#[test]
fn history_enter_opens_the_chosen_entry() {
    let h = Harness::new("tui_history_picker-open");
    let a = h.project("a");
    let b = h.project("b");
    write_history(&h, &format!("{b}\n{a}\n"));
    h.terminal.script(vec![down(), enter()]);
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Done));
    assert_eq!(h.backend.attached_to(), Some(name_of(&a)));
}

// FR-CMD-08: escape in the history picker is a cancel
#[test]
fn history_escape_cancels() {
    let h = Harness::new("tui_history_picker-cancel");
    let a = h.project("a");
    write_history(&h, &format!("{a}\n"));
    h.terminal.script(vec![esc()]);
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(restores(&h), 1);
}

// FR-PICK-09, FR-TUI-08, FR-TUI-10, A-27: a missing history entry is marked, and enter on it reports the directory
// and keeps the picker open with its rows
#[test]
fn missing_history_entry_is_marked_and_keeps_the_picker_open() {
    let h = Harness::new("tui_history_picker-missing");
    h.terminal.set_size(220, 24);
    let a = h.project("a");
    let gone = format!("{}/projects/gone", h.root().display());
    write_history(&h, &format!("{gone}\n{a}\n"));
    h.terminal.script(vec![enter(), esc()]);
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.backend.attached_to(), None);
    let lines = screen(&h);
    assert!(line_with(&lines, &gone).contains("(missing)"));
    assert!(line_with(&lines, &a).contains(&a));
    assert!(
        line_with(&lines, "Missing directory: ").contains(&format!("Missing directory: {gone}"))
    );
    assert_eq!(restores(&h), 1);
}

// ---------------------------------------------------------------------------------------------
// S-15: session actions on the picker rows (ctrl-x kill, ctrl-r rebuild, ctrl-p profile then rebuild)
// ---------------------------------------------------------------------------------------------

const WEB_PROFILE: &str = r#"{"windows": [{"name": "server", "command": "npm start"}]}"#;
const TMUX_SET: &str = "/tmp/tmux-1000/default,1,0";

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

/// Names of the sessions on the fake server.
fn live_names(h: &Harness) -> Vec<String> {
    h.backend
        .list_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|session| session.name)
        .collect()
}

/// Window names of the session of `dir`.
fn windows_of(h: &Harness, dir: &str) -> Vec<String> {
    h.backend.window_names(&name_of(dir))
}

/// The recorded profile option (`@ledge_profile`) of the session of `dir`.
fn profile_of(h: &Harness, dir: &str) -> Option<String> {
    h.backend.option(&name_of(dir), OPT_PROFILE)
}

/// A managed session of `dir` built from the profile `name` with the given text.
fn live_with_profile(h: &Harness, dir: &str, name: &str, text: &str) {
    let profile_name = ProfileName::parse(name).expect("valid profile name");
    let profile = Profile::parse(&profile_name, text).expect("valid profile");
    ensure_session(&h.backend, &AbsPath::clean(dir, &AbsPath::root()), &profile)
        .expect("create session");
}

// FR-ACT-01: ctrl-x kills the managed session of the selected row, and the list is shown again without it
#[test]
fn kill_key_kills_the_selected_session_and_reopens_the_list() {
    let mut h = Harness::new("tui_actions-kill");
    search_config(&h);
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    let lib = h.project("lib");
    live(&h, &lib);
    launch_from(&mut h, &app);
    h.terminal.script(vec![down(), ctrl('x'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(!live_names(&h).contains(&name_of(&lib)));
    let lines = screen(&h);
    let lib_row = line_with(&lines, "lib");
    assert!(lib_row.contains("○"), "{lib_row}");
    assert!(!lib_row.contains(&name_of(&lib)), "{lib_row}");
    assert_eq!(restores(&h), 1);
}

// FR-ACT-01, FR-ACT-13: ctrl-x on a row without a managed session does nothing; an unmanaged session of the
// same name is left alone and the other session is untouched
#[test]
fn kill_key_on_a_row_without_a_managed_session_does_nothing() {
    let mut h = Harness::new("tui_actions-kill-noop");
    search_config(&h);
    let app = h.project("app");
    let lib = h.project("lib");
    live(&h, &lib);
    h.backend
        .create_session(&NewSession {
            name: name_of(&app),
            dir: app.clone(),
            window_name: "main".to_string(),
        })
        .expect("create unmanaged session");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let names = live_names(&h);
    assert!(names.contains(&name_of(&app)), "{names:?}");
    assert!(names.contains(&name_of(&lib)), "{names:?}");
}

// FR-ACT-07, D-17: ctrl-x on the session the user is in is refused; the status line says so and nothing is killed
#[test]
fn kill_key_refuses_the_session_the_user_is_in() {
    let mut h = Harness::new("tui_actions-kill-current");
    search_config(&h);
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    live(&h, &app);
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend
        .switch_client(&name_of(&app))
        .expect("switch client");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(live_names(&h), vec![name_of(&app)]);
    assert!(
        line_with(&screen(&h), "cannot kill the session you are in.")
            .contains("cannot kill the session you are in."),
    );
}

// FR-ACT-01: ctrl-x works in the history picker as well
#[test]
fn kill_key_works_in_the_history_picker() {
    let h = Harness::new("tui_actions-kill-history");
    let a = h.project("a");
    let b = h.project("b");
    write_history(&h, &format!("{b}\n{a}\n"));
    live(&h, &b);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = history_picker(&h.ctx(), &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(live_names(&h).is_empty());
}

// FR-ACT-02, FR-ACT-12: ctrl-r creates the missing session of the selected row with the --profile option
#[test]
fn rebuild_key_creates_the_missing_session_with_the_profile_option() {
    let mut h = Harness::new("tui_actions-rebuild-create");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('r'), esc()]);
    let option = ProfileOpt::Named("dev".to_string());
    assert_eq!(
        project_picker(&h.ctx(), None, &option),
        Ok(Outcome::Cancelled)
    );
    assert_eq!(
        windows_of(&h, &app),
        vec!["editor".to_string(), "shell".to_string()]
    );
    assert_eq!(profile_of(&h, &app), Some("dev".to_string()));
}

// FR-ACT-03, FR-ACT-12: ctrl-r on a live session kills it and builds it again from the --profile option
#[test]
fn rebuild_key_rebuilds_a_live_session_from_the_profile_option() {
    let mut h = Harness::new("tui_actions-rebuild-live");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    live(&h, &app);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('r'), esc()]);
    let option = ProfileOpt::Named("dev".to_string());
    assert_eq!(
        project_picker(&h.ctx(), None, &option),
        Ok(Outcome::Cancelled)
    );
    assert_eq!(
        windows_of(&h, &app),
        vec!["editor".to_string(), "shell".to_string()]
    );
}

// FR-PICK-09: ctrl-r on a directory that is gone shows its message, builds nothing and keeps the picker open
#[test]
fn rebuild_key_on_a_missing_directory_keeps_the_picker_open() {
    let mut h = Harness::new("tui_actions-rebuild-missing");
    search_config(&h);
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    let gone = h.project("gone");
    h.terminal.remove_on_open(PathBuf::from(&gone));
    launch_from(&mut h, &app);
    h.terminal.script(vec![down(), ctrl('r'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert!(live_names(&h).is_empty());
    assert!(line_with(&screen(&h), "Missing directory: ")
        .contains(&format!("Missing directory: {gone}")));
}

// FR-PROF-13, A-11: ctrl-p opens the profile picker with the current profile first, then the default, then the
// other profiles and basic by name; its border label and key footer are the profile picker's
#[test]
fn profile_key_opens_the_profile_picker_in_the_listed_order() {
    let mut h = Harness::new("tui_actions-profile-order");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"], \"default_profile\": \"web\"}}",
        h.root().join("projects").display()
    ));
    h.write_profile("dev", DEV_PROFILE);
    h.write_profile("web", WEB_PROFILE);
    h.write_profile("ops", WEB_PROFILE);
    let app = h.project("app");
    live_with_profile(&h, &app, "ops", WEB_PROFILE);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let records = h.terminal.records();
    assert_eq!(records.len(), 3, "parent, profile picker, parent again");
    let nested = records[1].borrow().lines.clone();
    let rows = ["[Current: ops]", "[Default: web]", "basic", "dev"];
    let positions: Vec<usize> = rows
        .iter()
        .map(|needle| {
            nested
                .iter()
                .position(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("no row {needle:?} in {nested:#?}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{nested:#?}"
    );
    assert!(line_with(&nested, "≡ Profiles").contains("≡ Profiles"));
    assert!(line_with(&nested, "enter choose · esc cancel").contains("enter choose · esc cancel"));
}

// FR-ACT-04, FR-ACT-12: ctrl-p, then a profile, rebuilds the selected session with the chosen profile
#[test]
fn profile_key_then_choice_rebuilds_the_session_with_that_profile() {
    let mut h = Harness::new("tui_actions-profile-rebuild");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    live(&h, &app);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), enter()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        windows_of(&h, &app),
        vec!["editor".to_string(), "shell".to_string()]
    );
    assert_eq!(profile_of(&h, &app), Some("dev".to_string()));
}

// A-08: escape in the nested profile picker returns to the parent with no change to the session
#[test]
fn escape_in_the_profile_picker_returns_to_the_parent_without_change() {
    let mut h = Harness::new("tui_actions-profile-escape");
    search_config(&h);
    h.write_profile("dev", DEV_PROFILE);
    let app = h.project("app");
    live(&h, &app);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(h.terminal.opened(), 3);
    assert_eq!(windows_of(&h, &app), vec!["Terminal".to_string()]);
    assert_eq!(live_names(&h), vec![name_of(&app)]);
}

// FR-PROF-11 (S-15 acceptance): a profile that fails to load shows its message in the status line of the parent,
// which stays open; the session keeps running
#[test]
fn profile_that_fails_to_load_reports_in_the_status_line() {
    let mut h = Harness::new("tui_actions-profile-broken");
    search_config(&h);
    h.write_profile("broken", "{");
    let app = h.project("app");
    live(&h, &app);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), enter()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let records = h.terminal.records();
    let parent = records[2].borrow().lines.clone();
    assert!(
        line_with(&parent, "profile 'broken' is invalid JSON")
            .contains("profile 'broken' is invalid JSON"),
        "{parent:#?}"
    );
    assert_eq!(windows_of(&h, &app), vec!["Terminal".to_string()]);
    assert_eq!(live_names(&h), vec![name_of(&app)]);
}

// FR-PICK-05: the extra row of a current directory under HOME shows the home folder as "~"
#[test]
fn non_project_current_directory_under_home_shows_tilde() {
    let mut h = Harness::new("tui_project_picker-home-tilde");
    search_config(&h);
    h.terminal.set_size(220, 24);
    h.project("app");
    let notes = format!("{}/home/notes", h.root().display());
    fs::create_dir_all(&notes).expect("create notes");
    launch_from(&mut h, &notes);
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    let first = line_with(&lines, "*○");
    assert!(first.contains("~/notes"), "{first}");
    assert!(!first.contains(&notes), "{first}");
}

// FR-HIST-04: a history entry under HOME shows the home folder as "~"; an entry outside HOME is shown in full
#[test]
fn history_entry_under_home_shows_tilde() {
    let h = Harness::new("tui_history_picker-home-tilde");
    h.terminal.set_size(220, 24);
    let under = format!("{}/home/code/app", h.root().display());
    fs::create_dir_all(&under).expect("create folder");
    let outside = h.project("a");
    write_history(&h, &format!("{under}\n{outside}\n"));
    h.terminal.script(vec![esc()]);
    assert_eq!(
        history_picker(&h.ctx(), &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    assert!(line_with(&lines, "~/code/app").contains("~/code/app"));
    assert!(
        !lines.iter().any(|line| line.contains(&under)),
        "no full path under HOME: {lines:#?}"
    );
    assert!(line_with(&lines, "/projects/a").contains(&outside));
}

// P-60, FR-CONF-09: an unknown theme and a colour token that is not a token warn on stderr before the picker opens;
// the picker still runs, with the default theme's colours on the screen
#[test]
fn theme_warnings_go_to_stderr_and_the_picker_still_opens() {
    let h = Harness::new("tui_project_picker-theme");
    let projects = h.root().join("projects");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"], \"theme\": \"neon\", \"theme_colors\": {{\"acent\": \"#112233\"}}}}",
        projects.display()
    ));
    h.project("app");
    h.terminal.script(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(
        h.console.err_lines(),
        vec![
            "Warning: theme 'neon' is unknown; using default.".to_string(),
            "Warning: theme_colors 'acent' is not a colour token; ignored.".to_string(),
        ]
    );
    assert!(line_with(&screen(&h), "Projects").contains("Projects"));
}

// FR-ACT-01, design 4.5 (P-59): ctrl-x on a row without a session says so and kills nothing
#[test]
fn kill_key_on_a_row_without_a_session_says_so() {
    let h = Harness::new("tui_project_picker-kill-no-session");
    search_config(&h);
    h.project("app");
    h.terminal.script(vec![ctrl('x'), esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("No session on this row.")));
}

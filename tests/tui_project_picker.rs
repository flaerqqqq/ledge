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
/// P-65: the project picker footer, with the ctrl-d key for custom projects.
const PROJECT_PICKER_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill · ^d remove";
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

// FR-PICK-05: a current directory that is not a project is an extra first row showing its path, with no star
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
    let first = line_with(&lines, "/projects");
    assert!(
        first.trim_start_matches('│').trim_start().starts_with('/'),
        "{first}"
    );
    assert!(!lines.iter().any(|line| line.contains('*')), "{lines:#?}");
}

// FR-PICK-06, FR-PICK-07, P-61: a live managed session shows its session name and profile in the session column; the
// project picker has no marker column, so no circle is drawn. The border label and the footer are the project
// picker's (FR-TUI-04).
#[test]
fn live_session_shows_its_name_and_profile_with_no_marker_column() {
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
    assert!(lib_row.contains(&name_of(&lib)), "{lib_row}");
    assert!(lib_row.contains("basic"), "{lib_row}");
    assert!(
        !lines
            .iter()
            .any(|line| line.contains('●') || line.contains('○')),
        "{lines:#?}"
    );
    assert!(line_with(&lines, "web").starts_with("│ web"), "{lines:#?}");
    assert!(line_with(&lines, "Projects & Submodules").starts_with("╭ Projects"));
    assert!(line_with(&lines, PROJECT_FOOTER).contains(PROJECT_FOOTER));
}

// FR-PICK-05, P-61: the header starts at the project column (no marker column before it); the current directory has
// no star, and the names start in the same column
#[test]
fn current_directory_has_no_mark_and_no_column_comes_first() {
    let mut h = Harness::new("tui_project_picker-current-mark");
    search_config(&h);
    let app = h.project("app");
    h.project("lib");
    launch_from(&mut h, &app);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    assert!(
        line_with(&lines, "Session").starts_with("│ Project"),
        "{lines:#?}"
    );
    assert!(line_with(&lines, "app").starts_with("│ app"), "{lines:#?}");
    assert!(line_with(&lines, "lib").starts_with("│ lib"), "{lines:#?}");
    assert!(!lines.iter().any(|line| line.contains('*')), "{lines:#?}");
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
        (ProfileOpt::Unset, "Projects & Submodules"),
        (
            ProfileOpt::Named("dev".to_string()),
            "Projects (Profile: dev)",
        ),
        (
            ProfileOpt::Named("basic".to_string()),
            "Projects & Submodules",
        ),
        (ProfileOpt::None, "Projects & Submodules"),
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
    assert!(line_with(&lines, "History").starts_with("╭ History "));
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
    assert!(!lib_row.contains(&name_of(&lib)), "{lib_row}");
    assert!(!lines.iter().any(|line| line.contains('●')), "{lines:#?}");
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

// FR-ACT-01, FR-ACT-07, P-63: ctrl-x on the session the user is in switches the client to the most recent other
// session, kills the row's session, and the list is shown again
#[test]
fn kill_key_on_the_session_the_user_is_in_switches_to_another_session() {
    let mut h = Harness::new("tui_actions-kill-current");
    search_config(&h);
    h.terminal.set_size(220, 24);
    let app = h.project("app");
    let lib = h.project("lib");
    live(&h, &app);
    live(&h, &lib);
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend
        .switch_client(&name_of(&app))
        .expect("switch client");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('x'), esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(live_names(&h), vec![name_of(&lib)]);
    assert_eq!(h.backend.current_session(), Ok(Some(name_of(&lib))));
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
    assert!(line_with(&nested, "Profiles").starts_with("╭ Profiles "));
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

const FIVE_PROFILE: &str = r#"{"windows": [{"name": "Code", "command": "nvim ."}, {"name": "Terminal"}, {"name": "AI", "command": "claude"}, {"name": "Run", "command": "npm run dev"}, {"name": "Infra", "command": "terraform plan"}]}"#;
const SEVEN_PROFILE: &str = r#"{"windows": [{"name": "w1"}, {"name": "w2"}, {"name": "w3"}, {"name": "w4"}, {"name": "w5"}, {"name": "w6"}, {"name": "w7"}]}"#;

/// The last screen of the nested profile picker (the second picker opened in a ctrl-p flow).
fn profile_picker_screen(h: &Harness) -> Vec<String> {
    h.terminal.records()[1].borrow().lines.clone()
}

/// HOME is the temp folder, so the profile folder under the config folder shows as `~/...` (FR-HIST-03).
fn home_is_the_temp_folder(h: &mut Harness) {
    let root = h.root().display().to_string();
    h.env = h.env.clone().with_var("HOME", &root);
}

// FR-PROF-13 (S-45): the preview of the highlighted profile shows its title, its file with ~ and the window count,
// then one line per window with its command; the footer is the profile picker's
#[test]
fn profile_preview_shows_title_source_count_and_windows_of_the_highlighted_profile() {
    let mut h = Harness::new("tui_profile-preview-dev");
    search_config(&h);
    home_is_the_temp_folder(&mut h);
    h.write_profile("dev", FIVE_PROFILE);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    let file = PathBuf::from(h.dirs.config.as_str())
        .join("profiles")
        .join("dev.json");
    let shown = format!(
        "~/{}",
        file.strip_prefix(h.root())
            .expect("profile under the temp folder")
            .display()
    );
    assert!(line_with(&screen, "Preview · dev").contains("Preview · dev"));
    assert!(
        line_with(&screen, &shown).contains(&format!("{shown} · 5 windows")),
        "{screen:#?}"
    );
    assert!(line_with(&screen, "nvim .").contains("Code"), "{screen:#?}");
    assert!(
        line_with(&screen, "Terminal").contains("Terminal"),
        "{screen:#?}"
    );
    assert!(line_with(&screen, "claude").contains("AI"), "{screen:#?}");
    assert!(
        line_with(&screen, "npm run dev").contains("Run"),
        "{screen:#?}"
    );
    assert!(
        line_with(&screen, "terraform plan").contains("Infra"),
        "{screen:#?}"
    );
    assert!(line_with(&screen, "enter choose · esc cancel").starts_with('╰'));
}

// FR-PROF-13 (S-45): the built-in basic profile has no file; its preview says so and shows its one window
#[test]
fn profile_preview_of_basic_is_built_in_with_one_window() {
    let mut h = Harness::new("tui_profile-preview-basic");
    search_config(&h);
    h.write_profile("dev", FIVE_PROFILE);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    assert!(line_with(&screen, "Preview · basic").contains("Preview · basic"));
    assert!(line_with(&screen, "built-in · 1 window").contains("built-in · 1 window"));
    assert!(
        line_with(&screen, "Terminal").contains("Terminal"),
        "{screen:#?}"
    );
}

// FR-PROF-13 (S-45): a profile whose file is invalid keeps its row; its preview shows the error text
#[test]
fn profile_preview_of_a_broken_file_shows_the_error_and_the_row_stays() {
    let mut h = Harness::new("tui_profile-preview-broken");
    search_config(&h);
    h.write_profile("broken", "{");
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    assert!(line_with(&screen, "Preview · broken").contains("Preview · broken"));
    assert!(
        line_with(&screen, "profile 'broken' is invalid JSON")
            .contains("profile 'broken' is invalid JSON"),
        "{screen:#?}"
    );
    assert!(line_with(&screen, "broken.json").contains("broken.json"));
}

// FR-PROF-13 (S-45): a broken profile row is still chosen by enter, and the failure is reported as before
#[test]
fn broken_profile_row_stays_selectable_and_enter_reports_its_error() {
    let mut h = Harness::new("tui_profile-preview-broken-enter");
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
    let parent = h.terminal.records()[2].borrow().lines.clone();
    assert!(line_with(&parent, "profile 'broken' is invalid JSON").contains("invalid JSON"));
    assert_eq!(windows_of(&h, &app), vec!["Terminal".to_string()]);
}

// FR-PROF-13 (S-45): more windows than the preview has lines show the first ones and "+ N more" last
#[test]
fn profile_preview_with_more_windows_than_lines_ends_with_plus_n_more() {
    let mut h = Harness::new("tui_profile-preview-more");
    search_config(&h);
    h.write_profile("dev", SEVEN_PROFILE);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    assert!(line_with(&screen, "7 windows").contains("7 windows"));
    let last_shown = row_index(&screen, "w4");
    assert_eq!(
        row_index(&screen, "+ 3 more"),
        last_shown + 1,
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains("w5")),
        "{screen:#?}"
    );
}

// FR-PROF-13 (S-45): at 17 rows only 10 list rows would remain with the preview, so the preview is dropped and the
// list keeps its rows
#[test]
fn profile_preview_is_dropped_when_fewer_than_three_list_rows_would_remain() {
    let mut h = Harness::new("tui_profile-preview-small");
    search_config(&h);
    h.write_profile("dev", FIVE_PROFILE);
    h.terminal.set_size(80, 17);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    assert!(
        !screen.iter().any(|line| line.contains("Preview ·")),
        "{screen:#?}"
    );
    assert!(line_with(&screen, "basic").contains("basic"));
    assert!(line_with(&screen, "dev").contains("dev"));
}

// FR-PROF-13 (S-45): at 18 rows three list rows remain, so the preview is drawn
#[test]
fn profile_preview_is_kept_when_three_list_rows_remain() {
    let mut h = Harness::new("tui_profile-preview-fits");
    search_config(&h);
    h.write_profile("dev", FIVE_PROFILE);
    h.terminal.set_size(80, 18);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    let screen = profile_picker_screen(&h);
    assert!(line_with(&screen, "Preview · basic").contains("Preview · basic"));
}

// FR-PROF-13 (S-45): enter on a previewed profile still rebuilds the session with that profile
#[test]
fn enter_on_the_previewed_profile_rebuilds_with_it() {
    let mut h = Harness::new("tui_profile-preview-enter");
    search_config(&h);
    h.write_profile("dev", FIVE_PROFILE);
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
        ["Code", "Terminal", "AI", "Run", "Infra"]
            .map(String::from)
            .to_vec()
    );
    assert_eq!(profile_of(&h, &app), Some("dev".to_string()));
}

// A-08 (S-45): escape on a previewed profile changes nothing, and the parent's footer is unchanged
#[test]
fn escape_on_the_previewed_profile_changes_nothing() {
    let mut h = Harness::new("tui_profile-preview-escape");
    search_config(&h);
    h.write_profile("dev", FIVE_PROFILE);
    let app = h.project("app");
    live(&h, &app);
    let profile_before = profile_of(&h, &app);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl('p')]);
    h.terminal.queue(vec![down(), esc()]);
    h.terminal.queue(vec![esc()]);
    let outcome = project_picker(&h.ctx(), None, &ProfileOpt::Unset);
    assert_eq!(outcome, Ok(Outcome::Cancelled));
    assert_eq!(windows_of(&h, &app), vec!["Terminal".to_string()]);
    assert_eq!(profile_of(&h, &app), profile_before);
    assert!(line_with(&screen(&h), PROJECT_FOOTER).starts_with('╰'));
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
    let first = line_with(&lines, "~/notes");
    assert!(
        first
            .trim_start_matches('│')
            .trim_start()
            .starts_with("~/notes"),
        "{first}"
    );
    assert!(!first.contains(&notes), "{first}");
    assert!(!lines.iter().any(|line| line.contains('*')), "{lines:#?}");
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

// design 2.1, 2.4 (S-42): at 120 cells the project picker has a Path column, and a Details section under the list for
// the selected row: its path, its session (none here) and its profile
#[test]
fn project_picker_at_120_cells_shows_the_path_column_and_details() {
    let mut h = Harness::new("tui_project_picker-wide");
    h.terminal.set_size(120, 30);
    search_config(&h);
    let app = h.project("app");
    launch_from(&mut h, &app);
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    assert!(line_with(&lines, "Profile").contains("Path"), "{lines:#?}");
    let title = row_index(&lines, "Details · app");
    assert!(
        lines[title + 1].contains("Path      "),
        "{:?}",
        lines[title + 1]
    );
    assert!(
        lines[title + 1].contains("projects/app"),
        "{:?}",
        lines[title + 1]
    );
    assert!(
        lines[title + 2].contains("Session   (no session)"),
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

// design 2.1 (S-42): at 80 cells the project picker has no Path column and no Details section
#[test]
fn project_picker_at_80_cells_has_no_path_column_or_details() {
    let h = Harness::new("tui_project_picker-narrow");
    h.terminal.set_size(80, 24);
    search_config(&h);
    h.project("app");
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    assert!(lines[3].contains("Project"), "{lines:#?}");
    assert!(!lines[3].contains("Path"), "{lines:#?}");
    assert!(lines.iter().all(|line| !line.contains("Details ·")));
}

// P-60, design 6.2 (S-42): theme zen is a known name in the config: no warning on stderr, and the picker opens
#[test]
fn theme_zen_in_config_is_accepted_without_a_warning() {
    let h = Harness::new("tui_project_picker-zen");
    let projects = h.root().join("projects");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}\"], \"theme\": \"zen\"}}",
        projects.display()
    ));
    h.project("app");
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    assert!(
        h.console.err_lines().is_empty(),
        "{:?}",
        h.console.err_lines()
    );
    assert!(line_with(&screen(&h), "Projects").contains("Projects"));
}

fn ctrl_d() -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
}

fn typed(text: &str) -> Vec<Input> {
    text.chars().map(|c| key(KeyCode::Char(c))).collect()
}

/// A custom project (P-65) at `<root>/scratch`, listed in the custom projects file; the path text.
fn custom_scratch(h: &Harness) -> String {
    let scratch = format!("{}/scratch", h.root().display());
    fs::create_dir_all(&scratch).expect("create scratch");
    h.write_custom_projects(&format!("{scratch}\n"));
    scratch
}

/// The `projects` folder of the harness (the search path), as text.
fn projects_dir(h: &Harness) -> String {
    h.root().join("projects").display().to_string()
}

/// Writes the bookmarks file text (P-65 tests).
fn write_bookmarks_text(h: &Harness, text: &str) {
    let cache = PathBuf::from(h.dirs.cache.as_str());
    fs::create_dir_all(&cache).expect("create cache folder");
    fs::write(cache.join("bookmarks"), text).expect("write bookmarks");
}

/// The bookmarks file text, or `None`.
fn bookmarks_text(h: &Harness) -> Option<String> {
    fs::read_to_string(PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")).ok()
}

/// Makes `LC_ALL` a UTF-8 locale, so the mark is the diamond (design UTF-8 check).
fn utf8(h: &mut Harness) {
    h.env = h.env.clone().with_var("LC_ALL", "en_US.UTF-8");
}

// P-65: a custom project shows the ◆ mark after its name in the project cell
#[test]
fn custom_project_row_shows_the_diamond_mark_after_its_name() {
    let mut h = Harness::new("tui_custom-mark");
    search_config(&h);
    h.project("app");
    custom_scratch(&h);
    utf8(&mut h);
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("scratch ◆")),
        "{lines:#?}"
    );
}

// P-65: without a UTF-8 locale the mark is the ASCII '#'
#[test]
fn custom_project_mark_is_ascii_when_the_locale_is_not_utf8() {
    let mut h = Harness::new("tui_custom-mark-ascii");
    search_config(&h);
    custom_scratch(&h);
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("scratch #")),
        "{lines:#?}"
    );
    assert!(!lines.iter().any(|line| line.contains('◆')));
}

// P-65: the search matches the plain name of a custom project, not the mark
#[test]
fn search_matches_the_plain_name_of_a_custom_project() {
    let mut h = Harness::new("tui_custom-search");
    search_config(&h);
    h.project("app");
    let scratch = custom_scratch(&h);
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    let mut keys = typed("scr");
    keys.push(enter());
    h.terminal.script(keys);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(h.backend.attached_to(), Some(name_of(&scratch)));
}

// P-65: the footer of the project picker names ctrl-d as remove
#[test]
fn project_picker_footer_shows_ctrl_d_remove() {
    let mut h = Harness::new("tui_custom-footer");
    search_config(&h);
    h.project("app");
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains(PROJECT_PICKER_FOOTER)));
}

// P-65: ctrl-d on a custom project row removes it from the list, deletes its bookmarks and kills its session
#[test]
fn ctrl_d_on_a_custom_row_removes_it_its_bookmarks_and_its_session() {
    let mut h = Harness::new("tui_custom-ctrl-d");
    search_config(&h);
    let scratch = custom_scratch(&h);
    let src = format!("{scratch}/src");
    fs::create_dir_all(&src).expect("create src");
    let other = format!("{}/projects/other", h.root().display());
    fs::create_dir_all(&other).expect("create other");
    write_bookmarks_text(&h, &format!("{src}\n{other}\n"));
    live(&h, &scratch);
    launch_from(&mut h, &scratch);
    h.terminal.script(vec![ctrl_d(), esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    assert_eq!(h.custom_projects_text(), Some(String::new()));
    assert_eq!(
        bookmarks_text(&h).as_deref(),
        Some(format!("{other}\n").as_str())
    );
    assert!(!h
        .backend
        .list_sessions()
        .expect("list")
        .iter()
        .any(|s| s.name == name_of(&scratch)));
}

// P-65: ctrl-d on a regular project row shows the one-line notice and changes nothing
#[test]
fn ctrl_d_on_a_regular_row_shows_a_notice_and_changes_nothing() {
    let mut h = Harness::new("tui_custom-ctrl-d-regular");
    search_config(&h);
    let app = h.project("app");
    custom_scratch(&h);
    launch_from(&mut h, &app);
    h.terminal.script(vec![ctrl_d(), esc()]);
    assert_eq!(
        project_picker(&h.ctx(), None, &ProfileOpt::Unset),
        Ok(Outcome::Cancelled)
    );
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("Only a custom project can be removed.")));
    assert!(h
        .custom_projects_text()
        .expect("file kept")
        .contains("scratch"));
}

// P-65: a custom project whose directory is gone stays listed, dimmed with (missing), and ctrl-d removes it
#[test]
fn missing_custom_project_is_listed_with_missing_and_removable() {
    let mut h = Harness::new("tui_custom-missing");
    search_config(&h);
    let gone = format!("{}/gone", h.root().display());
    h.write_custom_projects(&format!("{gone}\n"));
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    assert!(screen(&h)
        .iter()
        .any(|line| line.contains("gone #") && line.contains("(missing)")));

    let mut again = Harness::new("tui_custom-missing-remove");
    search_config(&again);
    again.write_custom_projects(&format!("{gone}\n"));
    launch_from(&mut again, &gone);
    again.terminal.script(vec![ctrl_d(), esc()]);
    project_picker(&again.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    assert_eq!(again.custom_projects_text(), Some(String::new()));
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

// P-66: both same-named project rows show their distinct labels
#[test]
fn same_named_projects_show_distinct_labels() {
    let mut h = Harness::new("tui_project_picker-p66-labels");
    search_config_two(&h);
    h.project("api");
    fs::create_dir_all(h.root().join("work").join("api")).expect("create work api");
    h.project("lib");
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("projects/api")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("work/api")),
        "{lines:#?}"
    );
    assert!(lines.iter().any(|line| line.contains("lib")), "{lines:#?}");
    assert!(
        !lines.iter().any(|line| line.contains(" api ")),
        "{lines:#?}"
    );
}

// P-66: the search matches the plain name, so typing api keeps both same-named rows on the screen
#[test]
fn search_by_plain_name_keeps_both_same_named_rows() {
    let mut h = Harness::new("tui_project_picker-p66-search");
    search_config_two(&h);
    h.project("api");
    fs::create_dir_all(h.root().join("work").join("api")).expect("create work api");
    h.project("lib");
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    let mut keys = typed("api");
    keys.push(esc());
    h.terminal.script(keys);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("projects/api")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("work/api")),
        "{lines:#?}"
    );
    assert!(!lines.iter().any(|line| line.contains("lib")), "{lines:#?}");
}

// P-66, P-65: a custom project with the same name keeps its mark after its label
#[test]
fn custom_project_with_a_colliding_name_keeps_its_mark_after_its_label() {
    let mut h = Harness::new("tui_project_picker-p66-custom");
    search_config_two(&h);
    h.project("api");
    let scratch = format!("{}/scratch/api", h.root().display());
    fs::create_dir_all(&scratch).expect("create scratch api");
    h.write_custom_projects(&format!("{scratch}\n"));
    utf8(&mut h);
    let projects = projects_dir(&h);
    launch_from(&mut h, &projects);
    h.terminal.script(vec![esc()]);
    project_picker(&h.ctx(), None, &ProfileOpt::Unset).expect("picker runs");
    let lines = screen(&h);
    assert!(
        lines.iter().any(|line| line.contains("scratch/api ◆")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("projects/api")),
        "{lines:#?}"
    );
}

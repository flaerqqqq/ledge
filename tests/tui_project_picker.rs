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
use ledge::domain::profile::Profile;
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{LedgeError, Outcome};
use ledge::ports::backend::SessionBackend;
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

//! Component tests for the open flow (`app::open`) over `FakeBackend`, `FakeEnv`, `FakeConsole` and `RealFs`
//! on temp folders (docs/testing.md level 2): directory checks (FR-OPEN-03), attach or switch (FR-OPEN-08),
//! the history written before the open and its warning (FR-OPEN-10, FR-OPEN-11), and the profile precedence
//! at open (FR-PROF-09..12, D-13). The real attach and switch need a tmux client and are manual (traceability).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::launch::{split_command, Editor};
use ledge::app::open::{open_dir, open_dir_with_editor};
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{LedgeError, Outcome};
use ledge::ports::backend::{SessionBackend, OPT_PROFILE};
use support::ctx::Harness;
use support::fake_backend::FakeBackend;
use support::fake_process::FakeProcess;

const HISTORY_WARNING: &str = "Warning: could not update history: ";

fn fatal(text: &str) -> LedgeError {
    LedgeError::Fatal(text.to_string())
}

/// The session name of a directory path, as the flow computes it (D-14).
fn name_of(dir: &str) -> String {
    session_name(&AbsPath::clean(dir, &AbsPath::root()))
}

fn window_names(h: &Harness, session: &str) -> Vec<String> {
    h.backend.window_names(session)
}

fn session_names(h: &Harness) -> Vec<String> {
    h.backend
        .list_sessions()
        .expect("list sessions")
        .into_iter()
        .map(|session| session.name)
        .collect()
}

/// `profiles/dev.json`: an editor with a command, and a shell.
const DEV_PROFILE: &str =
    r#"{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}"#;
/// `profiles/web.json`: one window named web.
const WEB_PROFILE: &str = r#"{"windows": [{"name": "web"}]}"#;

// FR-OPEN-03: a directory that does not exist gives the message, creates nothing and records nothing
#[test]
fn missing_directory_fails_with_message_and_changes_nothing() {
    let h = Harness::new("open_missing_directory");
    let result = open_dir(&h.ctx(), Some("/no/such/dir"), &ProfileOpt::Unset);
    assert_eq!(result, Err(fatal("Directory '/no/such/dir' not found.")));
    assert!(session_names(&h).is_empty());
    assert_eq!(h.history_text(), None);
}

// FR-OPEN-03: a path that is a file, not a folder, gives the same message
#[test]
fn file_instead_of_directory_fails_with_message() {
    let h = Harness::new("open_file_not_directory");
    let file = format!("{}/plain.txt", h.root().display());
    std::fs::write(&file, "text").expect("write file");
    let result = open_dir(&h.ctx(), Some(file.as_str()), &ProfileOpt::Unset);
    assert_eq!(
        result,
        Err(fatal(&format!("Directory '{file}' not found.")))
    );
}

// FR-OPEN-03, A-15: the argument is echoed as typed, not as the cleaned absolute path
#[test]
fn message_echoes_relative_argument_as_typed() {
    let h = Harness::new("open_relative_echo");
    let result = open_dir(&h.ctx(), Some("missing/../gone"), &ProfileOpt::Unset);
    assert_eq!(result, Err(fatal("Directory 'missing/../gone' not found.")));
}

// FR-OPEN-08: outside a tmux session the session is created and the terminal attaches to it
#[test]
fn outside_tmux_creates_session_and_attaches() {
    let h = Harness::new("open_outside_attach");
    let dir = h.project("app");
    let name = name_of(&dir);
    let result = open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset);
    assert_eq!(result, Ok(Outcome::Done));
    assert!(session_names(&h).contains(&name));
    assert_eq!(h.backend.attached_to(), Some(name));
    assert_eq!(h.backend.current_session(), Ok(None));
}

// FR-OPEN-08: an empty TMUX variable is not a session, so the flow attaches
#[test]
fn empty_tmux_variable_counts_as_outside() {
    let mut h = Harness::new("open_empty_tmux");
    h.env = h.env.clone().with_var("TMUX", "");
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(h.backend.attached_to(), Some(name));
}

// FR-OPEN-08: inside a tmux session (TMUX set) the current client is switched, not attached
#[test]
fn inside_tmux_switches_the_client() {
    let mut h = Harness::new("open_inside_switch");
    h.env = h.env.clone().with_var("TMUX", "/tmp/tmux-1000/default,1,0");
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(h.backend.current_session(), Ok(Some(name)));
    assert_eq!(h.backend.attached_to(), None);
}

// FR-OPEN-08, message "could not attach to '<name>': <detail>"
#[test]
fn attach_failure_gives_could_not_attach_message() {
    let mut h = Harness::new("open_attach_failure");
    h.backend = FakeBackend::new().fail_on("attach", "no server running");
    let dir = h.project("app");
    let name = name_of(&dir);
    let result = open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset);
    assert_eq!(
        result,
        Err(fatal(&format!(
            "could not attach to '{name}': no server running"
        )))
    );
}

// FR-OPEN-08, message "could not attach to '<name>': <detail>" for the switch inside a session
#[test]
fn switch_failure_gives_could_not_attach_message() {
    let mut h = Harness::new("open_switch_failure");
    h.env = h.env.clone().with_var("TMUX", "/tmp/tmux-1000/default,1,0");
    h.backend = FakeBackend::new().fail_on("switch_client", "can't find client");
    let dir = h.project("app");
    let name = name_of(&dir);
    let result = open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset);
    assert_eq!(
        result,
        Err(fatal(&format!(
            "could not attach to '{name}': can't find client"
        )))
    );
}

// FR-OPEN-10: the directory is in the history file before the session is opened, so a failed session
// build still leaves it recorded
#[test]
fn directory_is_recorded_before_the_session_opens() {
    let mut h = Harness::new("open_history_first");
    h.backend = FakeBackend::new().fail_on("create_session", "boom");
    let dir = h.project("app");
    let name = name_of(&dir);
    let result = open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset);
    assert_eq!(
        result,
        Err(fatal(&format!("could not create session '{name}': boom")))
    );
    assert_eq!(h.history_text(), Some(format!("{dir}\n")));
}

// FR-OPEN-10, FR-HIST-02: opening directories records them newest first, without duplicates
#[test]
fn each_open_moves_its_directory_to_the_top_of_history() {
    let h = Harness::new("open_history_order");
    let a = h.project("a");
    let b = h.project("b");
    for dir in [&a, &b, &a] {
        assert_eq!(
            open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
            Ok(Outcome::Done)
        );
    }
    assert_eq!(h.history_text(), Some(format!("{a}\n{b}\n")));
}

// FR-OPEN-11: when the history cannot be written the warning is printed and the session still opens
#[test]
fn history_failure_warns_and_still_opens() {
    let h = Harness::new("open_history_failure");
    h.break_cache_folder();
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    let errors = h.console.err_lines();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with(HISTORY_WARNING), "{errors:?}");
    assert_eq!(h.backend.attached_to(), Some(name));
}

// FR-PROF-09: --profile wins over the default profile
#[test]
fn profile_option_wins_over_default_profile() {
    let h = Harness::new("open_profile_over_default");
    h.write_config(r#"{"search_paths": [], "default_profile": "web"}"#);
    h.write_profile("dev", DEV_PROFILE);
    h.write_profile("web", WEB_PROFILE);
    let dir = h.project("app");
    let name = name_of(&dir);
    let option = ProfileOpt::Named("dev".to_string());
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &option),
        Ok(Outcome::Done)
    );
    assert_eq!(window_names(&h, &name), vec!["editor", "shell"]);
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("dev".to_string())
    );
}

// FR-PROF-10: --no-profile gives the single-window layout and ignores the default profile
#[test]
fn no_profile_ignores_profiles() {
    let h = Harness::new("open_no_profile");
    h.write_config(r#"{"search_paths": [], "default_profile": "web"}"#);
    h.write_profile("web", WEB_PROFILE);
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::None),
        Ok(Outcome::Done)
    );
    assert_eq!(window_names(&h, &name), vec!["Terminal"]);
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("basic".to_string())
    );
    assert!(h.console.err_lines().is_empty());
}

// FR-PROF-12, D-13: without an option a valid default profile applies, with no warning
#[test]
fn default_profile_applies_without_option() {
    let h = Harness::new("open_default_applies");
    h.write_config(r#"{"search_paths": [], "default_profile": "web"}"#);
    h.write_profile("web", WEB_PROFILE);
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(window_names(&h, &name), vec!["web"]);
    assert!(h.console.err_lines().is_empty());
}

// FR-PROF-11, message "profile '<name>' not found at <path>": a missing --profile is an error and nothing opens
#[test]
fn missing_named_profile_fails_with_profile_error() {
    let h = Harness::new("open_profile_missing");
    let dir = h.project("app");
    let option = ProfileOpt::Named("nope".to_string());
    let result = open_dir(&h.ctx(), Some(dir.as_str()), &option);
    let path = format!("{}/profiles/nope.json", h.dirs.config.as_str());
    assert_eq!(
        result,
        Err(fatal(&format!("profile 'nope' not found at {path}")))
    );
    assert!(session_names(&h).is_empty());
}

// FR-PROF-11, message "profile '<name>' is invalid (expected a 'windows' array)"
#[test]
fn invalid_named_profile_fails_with_profile_error() {
    let h = Harness::new("open_profile_invalid");
    h.write_profile("dev", r#"{"windows": []}"#);
    let dir = h.project("app");
    let option = ProfileOpt::Named("dev".to_string());
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &option),
        Err(fatal(
            "profile 'dev' is invalid (expected a 'windows' array)"
        ))
    );
    assert!(session_names(&h).is_empty());
}

// FR-PROF-12, message "default_profile '<name>' is missing or invalid; using single-window layout." (warning)
#[test]
fn missing_default_profile_warns_and_uses_single_window() {
    let h = Harness::new("open_default_missing");
    h.write_config(r#"{"search_paths": [], "default_profile": "gone"}"#);
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(
        h.console.err_lines(),
        vec!["Warning: default_profile 'gone' is missing or invalid; using single-window layout."]
    );
    assert_eq!(window_names(&h, &name), vec!["Terminal"]);
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("basic".to_string())
    );
}

// FR-PROF-12: a default profile that is not valid JSON gives the same warning and the single-window layout
#[test]
fn invalid_default_profile_warns_and_uses_single_window() {
    let h = Harness::new("open_default_invalid");
    h.write_config(r#"{"search_paths": [], "default_profile": "bad"}"#);
    h.write_profile("bad", "{ windows");
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(
        h.console.err_lines(),
        vec!["Warning: default_profile 'bad' is missing or invalid; using single-window layout."]
    );
    assert_eq!(window_names(&h, &name), vec!["Terminal"]);
}

/// A `--code`, `--idea` or `--editor` open of `dir` with no profile option.
fn open_with_editor(h: &Harness, dir: &str, editor: &Editor) -> Result<Outcome, LedgeError> {
    open_dir_with_editor(&h.ctx(), Some(dir), &ProfileOpt::Unset, Some(editor))
}

/// The detached spawns the fake process recorded: program and arguments.
fn detached_spawns(h: &Harness) -> Vec<(String, Vec<String>)> {
    h.process
        .calls()
        .into_iter()
        .filter(|call| call.detached)
        .map(|call| (call.program, call.args))
        .collect()
}

// FR-LAUNCH-01: the editor starts when the open creates the session, and the terminal still attaches
#[test]
fn editor_launches_when_open_creates_session() {
    let h = Harness::new("launch_created");
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    assert_eq!(
        detached_spawns(&h),
        vec![(
            "setsid".to_string(),
            vec!["-f".into(), "code".into(), dir.clone()]
        )]
    );
    assert_eq!(h.backend.attached_to(), Some(name));
    assert!(h.console.err_lines().is_empty());
}

// FR-LAUNCH-01: an open that reuses an existing session does not start an editor
#[test]
fn editor_not_launched_when_session_is_reused() {
    let h = Harness::new("launch_reused");
    let dir = h.project("app");
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    assert_eq!(detached_spawns(&h).len(), 1);
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    assert_eq!(detached_spawns(&h).len(), 1, "reuse must not launch");
}

// FR-LAUNCH-01: without an editor option nothing is started
#[test]
fn no_editor_option_starts_nothing() {
    let h = Harness::new("launch_none");
    let dir = h.project("app");
    assert_eq!(
        open_dir(&h.ctx(), Some(dir.as_str()), &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert!(detached_spawns(&h).is_empty());
}

// FR-LAUNCH-02: the idea option runs the program `idea` on the directory
#[test]
fn idea_option_runs_idea_on_directory() {
    let h = Harness::new("launch_idea");
    let dir = h.project("app");
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Idea),
        Ok(Outcome::Done)
    );
    assert_eq!(
        detached_spawns(&h),
        vec![(
            "setsid".to_string(),
            vec!["-f".into(), "idea".into(), dir.clone()]
        )]
    );
}

// FR-LAUNCH-02: the editor option is split on whitespace and the directory is appended last
#[test]
fn editor_command_is_split_on_whitespace_with_directory_last() {
    assert_eq!(
        split_command("  nvim   --wait\tfoo "),
        vec!["nvim", "--wait", "foo"]
    );
    let h = Harness::new("launch_command");
    let dir = h.project("app");
    let editor = Editor::Command(split_command("nvim   --wait"));
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &editor),
        Ok(Outcome::Done)
    );
    assert_eq!(
        detached_spawns(&h),
        vec![(
            "setsid".to_string(),
            vec!["-f".into(), "nvim".into(), "--wait".into(), dir.clone()]
        )]
    );
}

// FR-LAUNCH-03: with setsid present the editor is started as `setsid -f <cmd> <dir>`
#[test]
fn detached_start_uses_setsid_when_present() {
    let h = Harness::new("launch_setsid");
    let dir = h.project("app");
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    let spawns = detached_spawns(&h);
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].0, "setsid");
    assert_eq!(spawns[0].1.first().map(String::as_str), Some("-f"));
}

// FR-LAUNCH-03, A-18: without setsid the editor is started directly, with the warning
#[test]
fn detached_start_without_setsid_warns_and_starts_directly() {
    let mut h = Harness::new("launch_no_setsid");
    h.process = FakeProcess::new().absent(&["setsid"]);
    let dir = h.project("app");
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    assert_eq!(
        h.console.err_lines(),
        vec!["Warning: setsid is not installed; the editor is started without detaching."]
    );
    assert_eq!(
        detached_spawns(&h),
        vec![("code".to_string(), vec![dir.clone()])]
    );
}

// A-18: a missing editor is a warning, and the session still opens
#[test]
fn missing_editor_warns_and_session_still_opens() {
    let mut h = Harness::new("launch_missing_editor");
    h.process = FakeProcess::new().absent(&["code"]);
    let dir = h.project("app");
    let name = name_of(&dir);
    assert_eq!(
        open_with_editor(&h, dir.as_str(), &Editor::Code),
        Ok(Outcome::Done)
    );
    assert_eq!(
        h.console.err_lines(),
        vec!["Warning: could not start editor 'code': command not found"]
    );
    assert!(detached_spawns(&h).is_empty());
    assert_eq!(session_names(&h), vec![name.clone()]);
    assert_eq!(h.backend.attached_to(), Some(name));
}

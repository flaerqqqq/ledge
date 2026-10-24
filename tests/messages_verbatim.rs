//! Literal user-facing strings from requirements.md "Messages" and architecture 8.3 (docs/testing.md,
//! level 5). Each later slice adds its own literals here. A message change breaks this file on purpose.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::messages;

// FR-CMD-08, message "Cancelled." (stdout, exit 0)
#[test]
fn cancelled_text_is_verbatim() {
    assert_eq!(messages::cancelled(), "Cancelled.");
}

// FR-PLAT-02, architecture 9.1: no usable XDG value and no HOME
#[test]
fn home_not_set_text_is_verbatim() {
    assert_eq!(messages::home_not_set(), "HOME is not set.");
}

// FR-CONF-05, message "config is not valid JSON: <file>"
#[test]
fn config_not_valid_json_text_is_verbatim() {
    assert_eq!(
        messages::config_not_valid_json("/c/ledge/config.json"),
        "config is not valid JSON: /c/ledge/config.json"
    );
}

// FR-CONF-02, architecture 8.3 [new]: search_paths of the wrong type
#[test]
fn config_invalid_search_paths_text_is_verbatim() {
    assert_eq!(
        messages::config_invalid_search_paths("/c/ledge/config.json"),
        "config is invalid (expected 'search_paths' to be a list of strings): /c/ledge/config.json"
    );
}

// FR-CONF-02, architecture 9.2 (default_profile must be a string; see proposal P-9)
#[test]
fn config_invalid_default_profile_text_is_verbatim() {
    assert_eq!(
        messages::config_invalid_default_profile("/c/ledge/config.json"),
        "config is invalid (expected 'default_profile' to be a string): /c/ledge/config.json"
    );
}

// FR-CONF-01, D-05, architecture 8.3 [new]: config cannot be created
#[test]
fn could_not_create_config_text_is_verbatim() {
    assert_eq!(
        messages::could_not_create_config("/c/ledge", "Permission denied (os error 13)"),
        "could not create config '/c/ledge': Permission denied (os error 13)"
    );
}

// FR-CONF-02, architecture 8.3 [new] (proposal P-9): config cannot be read
#[test]
fn could_not_read_config_text_is_verbatim() {
    assert_eq!(
        messages::could_not_read_config("/c/ledge/config.json", "Is a directory (os error 21)"),
        "could not read config '/c/ledge/config.json': Is a directory (os error 21)"
    );
}

// FR-CONF-03, architecture 8.3 [new]: undefined variable in a search path
#[test]
fn config_undefined_variable_text_is_verbatim() {
    assert_eq!(
        messages::config_undefined_variable("$WORK/app", "WORK"),
        "config search path '$WORK/app' uses undefined variable 'WORK'."
    );
}

// FR-PROF-01, D-13: named profile missing
#[test]
fn profile_not_found_text_is_verbatim() {
    assert_eq!(
        messages::profile_not_found("dev", "/c/ledge/profiles/dev.json"),
        "profile 'dev' not found at /c/ledge/profiles/dev.json"
    );
}

// D-13: profile is not valid JSON
#[test]
fn profile_invalid_json_text_is_verbatim() {
    assert_eq!(
        messages::profile_invalid_json("dev", "expected value at line 1 column 1"),
        "profile 'dev' is invalid JSON: expected value at line 1 column 1"
    );
}

// FR-PROF-02, D-13: profile lacks a windows array
#[test]
fn profile_invalid_shape_text_is_verbatim() {
    assert_eq!(
        messages::profile_invalid_shape("dev"),
        "profile 'dev' is invalid (expected a 'windows' array)"
    );
}

// FR-PROF-01, architecture 8.3 [new] (proposal P-9): profile file cannot be read
#[test]
fn could_not_read_profile_text_is_verbatim() {
    assert_eq!(
        messages::could_not_read_profile(
            "dev",
            "/c/ledge/profiles/dev.json",
            "Is a directory (os error 21)"
        ),
        "could not read profile 'dev' at /c/ledge/profiles/dev.json: Is a directory (os error 21)"
    );
}

// FR-TUI-05, architecture 8.3 [new] (A-06): the terminal is below 40x8
#[test]
fn terminal_too_small_text_is_verbatim() {
    assert_eq!(
        messages::terminal_too_small(),
        "Terminal too small: need at least 40x8."
    );
}

// FR-TUI-01, architecture 8.3 [new]: a picker started without a tty
#[test]
fn terminal_required_text_is_verbatim() {
    assert_eq!(
        messages::terminal_required(),
        "a terminal is required for this picker."
    );
}

// FR-TUI-06, architecture 8.3 [new] (proposal P-16): the terminal cannot be drawn or read
#[test]
fn terminal_failed_text_is_verbatim() {
    assert_eq!(
        messages::terminal_failed("Broken pipe (os error 32)"),
        "could not use the terminal: Broken pipe (os error 32)"
    );
}

// FR-PLAT-01, architecture 8.3 [new]: tmux older than 3.2
#[test]
fn tmux_version_too_old_text_is_verbatim() {
    assert_eq!(
        messages::tmux_version_too_old("3.1"),
        "tmux 3.2 or later is required; found 3.1."
    );
}

// FR-OPEN-07, architecture 8.2: the session build failure with the backend detail
#[test]
fn could_not_create_session_text_is_verbatim() {
    assert_eq!(
        messages::could_not_create_session("my_app-d7d2", "duplicate session"),
        "could not create session 'my_app-d7d2': duplicate session"
    );
}

// FR-OPEN-07, D-14: a managed session of the name records another directory
#[test]
fn session_name_used_text_is_verbatim() {
    assert_eq!(
        messages::session_name_used("my_app-d7d2", "/home/u/other"),
        "could not create session 'my_app-d7d2': name already used by /home/u/other"
    );
}

// FR-CMD-10: tmux is missing
#[test]
fn tmux_not_installed_text_is_verbatim() {
    assert_eq!(
        messages::tmux_not_installed(),
        "Required tool 'tmux' is not installed."
    );
}

// FR-OPEN-03: the open target does not exist; the argument is echoed as typed
#[test]
fn directory_not_found_text_is_verbatim() {
    assert_eq!(
        messages::directory_not_found("../nope"),
        "Directory '../nope' not found."
    );
}

// FR-OPEN-04: two directories given to the open form
#[test]
fn open_takes_one_directory_text_is_verbatim() {
    assert_eq!(
        messages::open_takes_one_directory(),
        "project open takes at most one directory argument."
    );
}

// FR-OPEN-11, architecture 8.2: a warning line
#[test]
fn warning_prefix_is_verbatim() {
    assert_eq!(messages::warning("boom"), "Warning: boom");
}

// FR-OPEN-11, message "could not update history: <detail>" (warning)
#[test]
fn could_not_update_history_text_is_verbatim() {
    assert_eq!(
        messages::could_not_update_history("Permission denied (os error 13)"),
        "could not update history: Permission denied (os error 13)"
    );
}

// FR-PROF-12, message "default_profile '<name>' is missing or invalid; using single-window layout." (warning)
#[test]
fn default_profile_missing_text_is_verbatim() {
    assert_eq!(
        messages::default_profile_missing("web"),
        "default_profile 'web' is missing or invalid; using single-window layout."
    );
}

// FR-OPEN-08, message "could not attach to '<name>': <detail>"
#[test]
fn could_not_attach_text_is_verbatim() {
    assert_eq!(
        messages::could_not_attach("my_app-d7d2", "no server running"),
        "could not attach to 'my_app-d7d2': no server running"
    );
}

// FR-LAUNCH-03, A-18, architecture 8.3 [new]: setsid is missing when an editor is launched
#[test]
fn setsid_not_installed_text_is_verbatim() {
    assert_eq!(
        messages::setsid_not_installed(),
        "setsid is not installed; the editor is started without detaching."
    );
}

// FR-LAUNCH-02, A-18, architecture 8.3 [new]: the editor program cannot be started
#[test]
fn could_not_start_editor_text_is_verbatim() {
    assert_eq!(
        messages::could_not_start_editor("code", "command not found"),
        "could not start editor 'code': command not found"
    );
}

// FR-PICK-10, message "No search paths defined in <config file>." (error)
#[test]
fn no_search_paths_text_is_verbatim() {
    assert_eq!(
        messages::no_search_paths("/c/ledge/config.json"),
        "No search paths defined in /c/ledge/config.json."
    );
}

// D-12, FR-PICK-10: no projects at start (info)
#[test]
fn no_projects_found_text_is_verbatim() {
    assert_eq!(
        messages::no_projects_found(),
        "No projects found in search paths."
    );
}

// FR-PICK-09, message "Missing directory: <path>"
#[test]
fn missing_directory_text_is_verbatim() {
    assert_eq!(
        messages::missing_directory("/p/gone"),
        "Missing directory: /p/gone"
    );
}

// FR-HIST-05, message "No history cache found." (info)
#[test]
fn no_history_cache_text_is_verbatim() {
    assert_eq!(messages::no_history_cache(), "No history cache found.");
}

// D-12, FR-HIST-03, message "No history." (info)
#[test]
fn no_history_text_is_verbatim() {
    assert_eq!(messages::no_history(), "No history.");
}

// FR-PICK-13, border labels "Projects & Submodules" and "Projects (Profile: <name>)"
#[test]
fn projects_label_texts_are_verbatim() {
    assert_eq!(messages::projects_label(None), "Projects & Submodules");
    assert_eq!(
        messages::projects_label(Some("dev")),
        "Projects (Profile: dev)"
    );
}

// Architecture 6.2 (FR-HIST-04): history label, footer and column titles
#[test]
fn picker_chrome_texts_are_verbatim() {
    assert_eq!(messages::HISTORY_LABEL, "History");
    assert_eq!(
        messages::PICKER_FOOTER,
        "enter open · ^r rebuild · ^p profile · ^x kill"
    );
    assert_eq!(messages::HEADER_PROJECT, "Project");
    assert_eq!(messages::HEADER_SESSION, "Session");
    assert_eq!(messages::HEADER_PROFILE, "Profile");
}

// Proposal P-24 [new]: the history file cannot be read; tmux cannot list sessions
#[test]
fn new_picker_detail_messages_are_verbatim() {
    assert_eq!(
        messages::could_not_read_history("boom"),
        "could not read history: boom"
    );
    assert_eq!(
        messages::could_not_list_sessions("boom"),
        "could not list sessions: boom"
    );
}

// FR-ACT-13, FR-SESS-11, D-14 (error): the target is not a managed session
#[test]
fn not_a_ledge_session_text_is_verbatim() {
    assert_eq!(
        messages::not_a_ledge_session("app-1a2b"),
        "'app-1a2b' is not a ledge session."
    );
}

// FR-ACT-07, D-17 (error in the command form, status line in pickers)
#[test]
fn cannot_kill_current_session_text_is_verbatim() {
    assert_eq!(
        messages::cannot_kill_current_session(),
        "cannot kill the session you are in."
    );
}

// FR-PROF-15, D-04 (warning, the prefix comes from messages::warning)
#[test]
fn session_profile_missing_text_is_verbatim() {
    assert_eq!(
        messages::warning(&messages::session_profile_missing("dev")),
        "Warning: session profile 'dev' is missing or invalid; using the default profile."
    );
}

// Proposal P-26 [new]: a failed kill during a rebuild or kill
#[test]
fn could_not_kill_session_text_is_verbatim() {
    assert_eq!(
        messages::could_not_kill_session("app-1a2b", "can't find session"),
        "could not kill session 'app-1a2b': can't find session"
    );
}

// Proposal P-26 [new]: a failed rename of the rebuilt copy (D-17)
#[test]
fn could_not_rename_session_text_is_verbatim() {
    assert_eq!(
        messages::could_not_rename_session("app-1a2b-rebuild", "app-1a2b", "duplicate session"),
        "could not rename session 'app-1a2b-rebuild' to 'app-1a2b': duplicate session"
    );
}

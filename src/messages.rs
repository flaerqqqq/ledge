//! User-facing strings (architecture 8.3). One const or fn per text, verified literally in
//! tests/messages_verbatim.rs.

use crate::domain::age::Age;

/// FR-CMD-08, message "Cancelled." (stdout, exit 0).
pub fn cancelled() -> &'static str {
    "Cancelled."
}

/// FR-PLAT-02, architecture 9.1: no usable XDG value and no `HOME`.
pub fn home_not_set() -> &'static str {
    "HOME is not set."
}

/// FR-CONF-05, message "config is not valid JSON: <file>" (error).
pub fn config_not_valid_json(file: &str) -> String {
    format!("config is not valid JSON: {file}")
}

/// FR-CONF-02, architecture 8.3 [new]: `search_paths` is present but not a list of strings.
pub fn config_invalid_search_paths(file: &str) -> String {
    format!("config is invalid (expected 'search_paths' to be a list of strings): {file}")
}

/// FR-CONF-02, architecture 9.2: `default_profile` is present but not a string. Added by S-02
/// (proposal P-9); the architecture 8.3 table has no entry for it.
pub fn config_invalid_default_profile(file: &str) -> String {
    format!("config is invalid (expected 'default_profile' to be a string): {file}")
}

/// FR-CONF-01, D-05, architecture 8.3 [new]: the config folder or file cannot be created.
pub fn could_not_create_config(path: &str, detail: &str) -> String {
    format!("could not create config '{path}': {detail}")
}

/// FR-CONF-02, architecture 8.3 [new] (proposal P-9): the config file exists but cannot be read.
pub fn could_not_read_config(file: &str, detail: &str) -> String {
    format!("could not read config '{file}': {detail}")
}

/// FR-CONF-03, architecture 8.3 [new]: a search path uses a variable that is not set.
pub fn config_undefined_variable(entry: &str, var: &str) -> String {
    format!("config search path '{entry}' uses undefined variable '{var}'.")
}

/// FR-PROF-01, D-13: the named profile file does not exist (or the name is not valid).
pub fn profile_not_found(name: &str, path: &str) -> String {
    format!("profile '{name}' not found at {path}")
}

/// D-13: the profile file is not valid JSON.
pub fn profile_invalid_json(name: &str, detail: &str) -> String {
    format!("profile '{name}' is invalid JSON: {detail}")
}

/// FR-PROF-02, D-13: the profile is JSON but its shape is wrong.
pub fn profile_invalid_shape(name: &str) -> String {
    format!("profile '{name}' is invalid (expected a 'windows' array)")
}

/// FR-PROF-01, architecture 8.3 [new] (proposal P-9): the profile file exists but cannot be read.
pub fn could_not_read_profile(name: &str, path: &str, detail: &str) -> String {
    format!("could not read profile '{name}' at {path}: {detail}")
}

/// FR-TUI-05, architecture 8.3 [new] and A-06: the terminal is below 40x8 (status line, F-12).
pub fn terminal_too_small() -> &'static str {
    "Terminal too small: need at least 40x8."
}

/// FR-TUI-01, architecture 8.3 [new]: a picker is started and stdout is not a tty.
pub fn terminal_required() -> &'static str {
    "a terminal is required for this picker."
}

/// FR-TUI-06, architecture 8.3 [new] (proposal P-16): drawing or reading the terminal failed.
pub fn terminal_failed(detail: &str) -> String {
    format!("could not use the terminal: {detail}")
}

/// FR-PLAT-01, architecture 8.3 [new]: the installed tmux is older than 3.2. `found` is `<major>.<minor>`.
pub fn tmux_version_too_old(found: &str) -> String {
    format!("tmux 3.2 or later is required; found {found}.")
}

/// FR-OPEN-07, FR-PLAT-01, architecture 8.2: a session build failed. `detail` is the backend text.
pub fn could_not_create_session(name: &str, detail: &str) -> String {
    format!("could not create session '{name}': {detail}")
}

/// FR-OPEN-07, D-14: a managed session of the same name records another directory. The whole line is
/// `could not create session '<name>': name already used by <dir>`.
pub fn session_name_used(name: &str, dir: &str) -> String {
    could_not_create_session(name, &format!("name already used by {dir}"))
}

/// FR-CMD-10: tmux is not on PATH.
pub fn tmux_not_installed() -> &'static str {
    "Required tool 'tmux' is not installed."
}

/// Architecture 8.2: the prefix of a warning line on stderr. It does not change the exit code.
pub fn warning(text: &str) -> String {
    format!("Warning: {text}")
}

/// FR-OPEN-03: the open target is missing or not a directory. `dir` is the argument as typed (A-15).
pub fn directory_not_found(dir: &str) -> String {
    format!("Directory '{dir}' not found.")
}

/// FR-OPEN-04: two directories were given to the open form (parsed by S-17).
pub fn open_takes_one_directory() -> &'static str {
    "project open takes at most one directory argument."
}

/// FR-OPEN-11: the history file cannot be read or written; a warning, and the open continues.
pub fn could_not_update_history(detail: &str) -> String {
    format!("could not update history: {detail}")
}

/// FR-PROF-12: the default profile cannot be loaded at open; a warning, and the single-window layout is used.
pub fn default_profile_missing(name: &str) -> String {
    format!("default_profile '{name}' is missing or invalid; using single-window layout.")
}

/// FR-LAUNCH-02, A-18, architecture 8.3 [new]: the editor program cannot be started. `<detail>` is the
/// reason (`command not found` when the program is not on PATH).
pub fn could_not_start_editor(command: &str, detail: &str) -> String {
    format!("could not start editor '{command}': {detail}")
}

/// A-18: the detail of an editor that is not on PATH (proposal P-23).
pub fn command_not_found() -> &'static str {
    "command not found"
}

/// FR-LAUNCH-03, A-18, architecture 8.3 [new]: `setsid` is missing when an editor is launched; a warning.
pub fn setsid_not_installed() -> &'static str {
    "setsid is not installed; the editor is started without detaching."
}

/// FR-OPEN-08, FR-SESS-04: attaching to or switching to a session failed.
pub fn could_not_attach(name: &str, detail: &str) -> String {
    format!("could not attach to '{name}': {detail}")
}

/// FR-PICK-10, D-16: no search path is configured; `config_file` is the config file path.
pub fn no_search_paths(config_file: &str) -> String {
    format!("No search paths defined in {config_file}.")
}

/// D-12: the project picker has no projects at start; info, exit 0, no window.
pub fn no_projects_found() -> &'static str {
    "No projects found in search paths."
}

/// FR-PICK-09: the chosen directory no longer exists; the picker stays open.
pub fn missing_directory(path: &str) -> String {
    format!("Missing directory: {path}")
}

/// FR-HIST-05, D-16: the history picker is asked for and the history file does not exist; info.
pub fn no_history_cache() -> &'static str {
    "No history cache found."
}

/// D-12, FR-HIST-03: the history file is empty; info, exit 0.
pub fn no_history() -> &'static str {
    "No history."
}

/// FR-PICK-13: the border label of the project picker. `profile` is the name of a non-basic profile that
/// applies to the open, or `None` for the basic layout.
pub fn projects_label(profile: Option<&str>) -> String {
    match profile {
        Some(name) => format!("Projects (Profile: {name})"),
        None => "Projects & Submodules".to_string(),
    }
}

/// FR-HIST-04, architecture 6.2: the border label of the history picker.
pub const HISTORY_LABEL: &str = "History";

/// FR-TUI-04, architecture 6.2 and A-03: the footer of the project picker. The history picker uses the same
/// keys (proposal P-15).
pub const PICKER_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill";

/// Architecture 6.2: column titles of the project and history pickers.
pub const HEADER_PROJECT: &str = "Project";
pub const HEADER_SESSION: &str = "Session";
pub const HEADER_PROFILE: &str = "Profile";

/// Proposal P-24 [new]: the history file exists but cannot be read.
pub fn could_not_read_history(detail: &str) -> String {
    format!("could not read history: {detail}")
}

/// Proposal P-24 [new]: tmux could not list the sessions a picker shows.
pub fn could_not_list_sessions(detail: &str) -> String {
    format!("could not list sessions: {detail}")
}

/// FR-ACT-13, FR-SESS-11, D-14, D-16: the target is a session without the managed marker (error).
pub fn not_a_ledge_session(name: &str) -> String {
    format!("'{name}' is not a ledge session.")
}

/// FR-ACT-07, D-17, A-20: the session the user is in is refused for kill (error in the command form, status line in
/// pickers).
pub fn cannot_kill_current_session() -> &'static str {
    "cannot kill the session you are in."
}

/// FR-PROF-15, D-04: the recorded profile of a session cannot be loaded during a rebuild; a warning, and the default
/// profile is used.
pub fn session_profile_missing(name: &str) -> String {
    format!("session profile '{name}' is missing or invalid; using the default profile.")
}

/// Proposal P-26 [new]: tmux could not kill a session during a rebuild or kill.
pub fn could_not_kill_session(name: &str, detail: &str) -> String {
    format!("could not kill session '{name}': {detail}")
}

/// Proposal P-26 [new]: tmux could not rename the rebuilt copy back to the original name (D-17).
pub fn could_not_rename_session(old: &str, new: &str, detail: &str) -> String {
    format!("could not rename session '{old}' to '{new}': {detail}")
}

/// FR-PROF-13: the profile picker row label of the profile the session being rebuilt uses.
pub fn profile_current(name: &str) -> String {
    format!("[Current: {name}]")
}

/// FR-PROF-13: the profile picker row label of the config's default profile.
pub fn profile_default(name: &str) -> String {
    format!("[Default: {name}]")
}

/// FR-PROF-13: the border label of the profile picker (architecture 6.2).
pub const PROFILES_LABEL: &str = "Profiles";

/// Architecture 6.2 and A-03: the key footer of the profile picker.
pub const PROFILE_PICKER_FOOTER: &str = "enter choose · esc cancel";

/// FR-SESS-03, architecture 6.2: the border label of the session switcher.
pub const SESSIONS_LABEL: &str = "Sessions";

/// Architecture 6.2: the column title of the switcher's activity column.
pub const HEADER_ACTIVITY: &str = "Activity";

/// Architecture 6.2 (given): the key footer of the session switcher.
pub const SESSION_SWITCHER_FOOTER: &str = "enter switch · ^r rebuild · ^p profile · ^x kill";

/// D-08, D-12, FR-SESS-01: the switcher has no managed sessions; info, and the list stays open.
pub fn no_ledge_sessions() -> &'static str {
    "No ledge sessions."
}

/// FR-SESS-03, A-10: the activity age text of the switcher.
pub fn activity_age(age: Age) -> String {
    match age {
        Age::JustNow => "just now".to_string(),
        Age::Minutes(n) => format!("{n}m ago"),
        Age::Hours(n) => format!("{n}h ago"),
        Age::Days(n) => format!("{n}d ago"),
    }
}

/// FR-CMD-03: a group command has no sub-command named `word`. `group` is `project`, `history`, `bm` or `ss`.
pub fn unknown_group_command(group: &str, word: &str) -> String {
    format!("Unknown {group} command: {word} (run 'ledge {group}' for the list).")
}

/// FR-CMD-04: an option the command does not take, or an unknown one. `option` is the text as typed.
pub fn unknown_option(option: &str) -> String {
    format!("Unknown option: {option} (run 'ledge help' for usage).")
}

/// FR-CMD-05: `--profile` is the last word or is followed by another option.
pub fn profile_requires_name() -> &'static str {
    "--profile requires a profile name."
}

/// FR-CMD-06: `--editor` is the last word, followed by another option, or names no program.
pub fn editor_requires_command() -> &'static str {
    "--editor requires a command."
}

/// Architecture 2.3 rule 6: `--profile` and `--no-profile` given together.
pub fn profile_with_no_profile() -> &'static str {
    "--profile cannot be combined with --no-profile."
}

/// Architecture 2.3 rule 6, D-11: more than one of `--code`, `--idea`, `--editor`.
pub fn only_one_editor_option() -> &'static str {
    "only one of --code, --idea, --editor may be given."
}

/// FR-PICK-11: a directory given to `project pick`.
pub fn project_pick_takes_no_directory() -> &'static str {
    "project pick takes no directory argument."
}

/// FR-BM-14: a directory given to `bm pick`.
pub fn bm_pick_takes_no_directory() -> &'static str {
    "bm pick takes no directory argument; use 'ledge bm add DIR'."
}

/// FR-BM-07: two directories given to `bm add`.
pub fn bm_add_takes_at_most_one_directory() -> &'static str {
    "bm add takes at most one directory."
}

/// FR-TREE-09: two directories given to `bm tree`.
pub fn bm_tree_takes_at_most_one_directory() -> &'static str {
    "bm tree takes at most one directory."
}

/// FR-BM-10: `bm remove` without a directory.
pub fn bookmark_remove_requires_directory() -> &'static str {
    "bookmark remove requires a directory."
}

/// FR-TREE-10: `--session-scope` given together with a directory.
pub fn session_scope_with_directory() -> &'static str {
    "--session-scope cannot be combined with a directory."
}

/// FR-SESS-05: a directory given to the session switcher.
pub fn ss_takes_no_directory() -> &'static str {
    "ss takes no directory argument."
}

/// Architecture 8.3 [new]: a directory given to `ss tree` (same rule as FR-SESS-05).
pub fn ss_tree_takes_no_directory() -> &'static str {
    "ss tree takes no directory argument."
}

/// Architecture 2.3 rule 7 [new]: extra words for a command that takes none. `words` is the command, e.g. `history list`.
pub fn takes_no_arguments(words: &str) -> String {
    format!("'{words}' takes no arguments.")
}

/// FR-CMD-09, architecture A-01 (proposal P-2): a required program is missing. No command uses it yet.
pub fn required_program(name: &str) -> String {
    format!("{name} is required for this command.")
}

/// Proposal P-32 [new, not yet accepted]: a command that the binary does not build yet (its slice is later).
/// `words` is the command as typed, e.g. `history list`.
pub fn command_not_available(words: &str) -> String {
    format!("'{words}' is not available yet.")
}

/// Proposal P-33 [new, not yet accepted]: the working directory cannot be read at start.
pub fn could_not_read_working_directory(detail: &str) -> String {
    format!("could not read the working directory: {detail}")
}

/// Proposal P-33 [new, not yet accepted]: the interrupt handlers cannot be registered at start (FR-TUI-06).
pub fn could_not_register_signals(detail: &str) -> String {
    format!("could not register interrupt handlers: {detail}")
}

/// Proposal P-33 [new, not yet accepted]: `tmux -V` printed text with no version number.
pub fn could_not_read_tmux_version(detail: &str) -> String {
    format!("could not read the tmux version: {detail}")
}

/// FR-HIST-06, Messages: the history file was deleted by `history clean`; info, exit 0.
pub fn history_cleared() -> &'static str {
    "Project history cache clear successfully."
}

/// FR-BM-05, Messages: a bookmark was added; info, exit 0.
pub fn bookmarked(path: &str) -> String {
    format!("Bookmarked {path}")
}

/// FR-BM-06, Messages: the directory is already bookmarked and nothing changes; info, exit 0.
pub fn already_bookmarked(path: &str) -> String {
    format!("Already bookmarked: {path}")
}

/// FR-BM-09, Messages: a bookmark was removed; info, exit 0.
pub fn removed_bookmark(path: &str) -> String {
    format!("Removed bookmark {path}")
}

/// FR-BM-10, Messages: removing a directory that is not bookmarked; error.
pub fn not_a_bookmark(path: &str) -> String {
    format!("'{path}' is not a bookmark.")
}

/// FR-BM-08, Messages: `bm list` with no bookmarks; info, exit 0.
pub fn no_bookmarks() -> &'static str {
    "No bookmarks."
}

/// FR-BM-12, D-12, Messages: the bookmark picker has no bookmarks; info, exit 0, no window.
pub fn no_bookmarks_to_pick() -> &'static str {
    "No bookmarks. Use 'ledge bm add' to add one."
}

/// FR-BM-11, architecture 6.2: the border label of the bookmark picker.
pub const BOOKMARKS_LABEL: &str = "Bookmarks";

/// FR-BM-11, architecture 6.2: the column title of the bookmark picker's path column.
pub const HEADER_BOOKMARK: &str = "Bookmark";

/// FR-BM-13, architecture 6.2 (given): the key footer of the bookmark picker.
pub const BOOKMARK_PICKER_FOOTER: &str =
    "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark";

/// FR-BM-04, Messages: a project cannot be bookmarked; error.
pub fn bookmark_is_project(path: &str) -> String {
    format!("{path} is a project; open it with 'ledge project pick'.")
}

/// FR-BM-03, Messages: a directory outside every project cannot be bookmarked; error.
pub fn bookmark_outside_project(path: &str) -> String {
    format!("{path} is not inside a project. Bookmarks go inside a directory from search_paths.")
}

/// FR-BM-08, Messages: the status word of a bookmark row, "ok".
pub const BOOKMARK_STATUS_OK: &str = "ok";
/// FR-BM-08, Messages: the status word of a bookmark whose directory is gone.
pub const BOOKMARK_STATUS_MISSING: &str = "missing";
/// FR-BM-08, Messages: the status word of a bookmark outside every project.
pub const BOOKMARK_STATUS_OUTSIDE: &str = "outside project";

/// FR-BM-08, proposal P-35 [new, not yet accepted]: one row of `bm list`, the status word then the path,
/// the status padded to 16 columns.
pub fn bookmark_list_row(status: &str, path: &str) -> String {
    format!("{status:<16}{path}")
}

/// FR-EDIT-09, D-16: `--session-scope` used outside a tmux session; error.
pub fn session_scope_requires_session() -> &'static str {
    "--session-scope requires running inside a session."
}

/// FR-EDIT-10, Messages: the chosen directory is outside every project; error.
pub fn edit_outside_project(path: &str) -> String {
    format!("{path} is not inside a project from search_paths.")
}

/// FR-EDIT-01, architecture 6.2: the border label of the bookmark edit window.
pub fn edit_window_label(project: &str) -> String {
    format!("Bookmarks in {project}")
}

/// FR-EDIT-02, FR-EDIT-03, FR-EDIT-07, architecture 6.2 (given): the key footer of the bookmark edit window.
pub const EDIT_WINDOW_FOOTER: &str = "enter toggle bookmark · →/^l open · ←/^h close · esc done";

/// FR-EDIT-01, architecture 6.2: the column title of the directory tree in the bookmark edit window.
pub const HEADER_DIRECTORY: &str = "Directory";

/// FR-TREE-11, architecture 6.2: the border label of the bookmark tree window.
pub const BOOKMARK_TREE_LABEL: &str = "Bookmark tree";

/// FR-SESS-06, architecture 6.2: the border label of the session tree window.
pub const SESSION_TREE_LABEL: &str = "Session tree";

/// Screens and interactions (given): the key footer of the bookmark tree window.
pub const BOOKMARK_TREE_FOOTER: &str =
    "enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node";

/// Screens and interactions (given): the key footer of the session tree window.
pub const SESSION_TREE_FOOTER: &str =
    "enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node";

/// Proposal P-39 [new, not yet accepted]: the edit window state file exists but cannot be read; error.
pub fn could_not_read_edit_state(detail: &str) -> String {
    format!("could not read the bookmark edit state: {detail}")
}

/// Proposal P-39 [new, not yet accepted]: the edit window state file cannot be written; error.
pub fn could_not_save_edit_state(detail: &str) -> String {
    format!("could not save the bookmark edit state: {detail}")
}

/// Proposal P-35 [new, not yet accepted]: `history clean` cannot delete the history file; error.
pub fn could_not_clear_history(detail: &str) -> String {
    format!("could not clear history: {detail}")
}

/// Proposal P-35 [new, not yet accepted]: the bookmarks file exists but cannot be read; error.
pub fn could_not_read_bookmarks(detail: &str) -> String {
    format!("could not read bookmarks: {detail}")
}

/// Proposal P-35 [new, not yet accepted]: the bookmarks file cannot be written; error.
pub fn could_not_write_bookmarks(detail: &str) -> String {
    format!("could not write bookmarks: {detail}")
}

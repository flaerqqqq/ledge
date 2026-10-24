//! User-facing strings (architecture 8.3). One const or fn per text, verified literally in
//! tests/messages_verbatim.rs.

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

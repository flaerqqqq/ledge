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

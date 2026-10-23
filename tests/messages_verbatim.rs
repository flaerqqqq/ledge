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

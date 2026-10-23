//! Component tests for FR-PLAT-02 and FR-CONF-08: the XDG rules with the environment read through
//! `FakeEnv` (docs/testing.md level 2). The pure rules are unit-tested in `domain/location.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::domain::location::Dirs;
use ledge::error::{LedgeError, Result};
use ledge::ports::env::Env;
use support::fake_env::FakeEnv;

/// Reads the three variables the folders depend on and resolves them. Architecture 2 names no
/// production function for this step, so the mapping of names lives here.
fn dirs_from_env(env: &dyn Env) -> Result<Dirs> {
    Dirs::resolve(
        env.var("XDG_CONFIG_HOME").as_deref(),
        env.var("XDG_CACHE_HOME").as_deref(),
        env.var("HOME").as_deref(),
    )
}

// FR-PLAT-02: a bare environment (only HOME) uses the standard per-user folders
#[test]
fn bare_environment_uses_standard_user_folders() {
    let env = FakeEnv::new().with_var("HOME", "/home/u");
    let dirs = dirs_from_env(&env).expect("resolves");
    assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
    assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
}

// FR-CONF-08: absolute XDG values from the environment are honoured
#[test]
fn absolute_xdg_values_from_environment_are_honoured() {
    let env = FakeEnv::new()
        .with_var("HOME", "/home/u")
        .with_var("XDG_CONFIG_HOME", "/srv/cfg")
        .with_var("XDG_CACHE_HOME", "/srv/cache");
    let dirs = dirs_from_env(&env).expect("resolves");
    assert_eq!(dirs.config.as_str(), "/srv/cfg/ledge");
    assert_eq!(dirs.cache.as_str(), "/srv/cache/ledge");
}

// FR-CONF-08, A-17: an empty value is returned by the environment as set, and ledge ignores it
#[test]
fn empty_xdg_value_from_environment_falls_back_to_home() {
    let env = FakeEnv::new()
        .with_var("HOME", "/home/u")
        .with_var("XDG_CONFIG_HOME", "");
    assert_eq!(env.var("XDG_CONFIG_HOME"), Some(String::new()));
    let dirs = dirs_from_env(&env).expect("resolves");
    assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
}

// FR-CONF-08, A-17: a relative XDG value from the environment is ignored
#[test]
fn relative_xdg_value_from_environment_falls_back_to_home() {
    let env = FakeEnv::new()
        .with_var("HOME", "/home/u")
        .with_var("XDG_CACHE_HOME", "relative/cache");
    let dirs = dirs_from_env(&env).expect("resolves");
    assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
}

// FR-PLAT-02, architecture 9.1: no HOME and no XDG value gives `HOME is not set.`
#[test]
fn environment_without_home_or_xdg_is_an_error() {
    let env = FakeEnv::new();
    assert_eq!(
        dirs_from_env(&env),
        Err(LedgeError::Fatal("HOME is not set.".to_string()))
    );
}

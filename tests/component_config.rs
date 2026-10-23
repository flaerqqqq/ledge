//! Component tests for the configuration and profile files (docs/testing.md level 2): the store
//! services on `RealFs` over temp folders, with the environment from `FakeEnv`. Covers FR-CONF-01..06
//! and FR-PROF-01, 02, 06 through `app::store`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::fs::RealFs;
use ledge::app::store::{load_config, load_profile};
use ledge::domain::location::Dirs;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::error::{LedgeError, Result};
use ledge::ports::env::Env;
use std::fs;
use std::path::PathBuf;
use support::fake_env::FakeEnv;
use support::temp::TempTree;

/// The default configuration text, written out literally (architecture 9.2, D-03).
const DEFAULT_TEXT: &str = "{\n  \"search_paths\": [\"~/projects\"]\n}\n";

/// The environment of one test: HOME and the configuration folder under `tree`, plus extras.
fn env_in(tree: &TempTree, extra: &[(&str, &str)]) -> FakeEnv {
    let config_home = tree.path().join("xdg-config");
    let mut env = FakeEnv::new()
        .with_var("HOME", tree.path().join("home").to_str().expect("utf-8"))
        .with_var("XDG_CONFIG_HOME", config_home.to_str().expect("utf-8"))
        .with_cwd("/");
    for (name, value) in extra {
        env = env.with_var(name, value);
    }
    env
}

/// The folders resolved from the environment, as `main` does.
fn dirs_for(env: &dyn Env) -> Dirs {
    Dirs::resolve(
        env.var("XDG_CONFIG_HOME").as_deref(),
        env.var("XDG_CACHE_HOME").as_deref(),
        env.var("HOME").as_deref(),
    )
    .expect("dirs resolve")
}

/// The config folder of a test: `<XDG_CONFIG_HOME>/ledge`.
fn config_folder(tree: &TempTree) -> PathBuf {
    tree.path().join("xdg-config").join("ledge")
}

/// Writes `config.json` in the config folder, creating the folder.
fn write_config(tree: &TempTree, text: &str) {
    fs::create_dir_all(config_folder(tree)).expect("mkdir");
    fs::write(config_folder(tree).join("config.json"), text).expect("write config");
}

/// Writes `profiles/<name>.json`, creating the folder.
fn write_profile(tree: &TempTree, name: &str, text: &str) {
    let folder = config_folder(tree).join("profiles");
    fs::create_dir_all(&folder).expect("mkdir");
    fs::write(folder.join(format!("{name}.json")), text).expect("write profile");
}

/// Loads the configuration with the given environment.
fn load(env: &FakeEnv) -> Result<ledge::domain::config::Config> {
    let dirs = dirs_for(env);
    load_config(&RealFs, env, &dirs, &AbsPath::root())
}

// FR-CONF-01: on first run the missing config file is written with exactly the default content
#[test]
fn first_run_writes_default_config_with_exact_content() {
    let tree = TempTree::new("component_config-first-run");
    let env = env_in(&tree, &[]);
    load(&env).expect("loads");
    let text = fs::read_to_string(config_folder(&tree).join("config.json")).expect("config exists");
    assert_eq!(text, DEFAULT_TEXT);
}

// FR-CONF-01, D-03: the default search path is the literal "~/projects", and the folder is not created
#[test]
fn first_run_default_search_path_is_not_created_on_disk() {
    let tree = TempTree::new("component_config-default-path");
    let env = env_in(&tree, &[]);
    let config = load(&env).expect("loads");
    assert_eq!(config.search_paths.len(), 1);
    assert_eq!(
        config.search_paths[0].as_str(),
        tree.path().join("home/projects").to_str().expect("utf-8")
    );
    assert!(!tree.path().join("home").join("projects").exists());
}

// FR-CONF-06: the profiles folder is created when it is missing
#[test]
fn first_run_creates_profiles_folder() {
    let tree = TempTree::new("component_config-profiles-folder");
    let env = env_in(&tree, &[]);
    load(&env).expect("loads");
    assert!(config_folder(&tree).join("profiles").is_dir());
}

// D-05: a second run changes nothing: the file keeps its text and the folders stay
#[test]
fn second_run_changes_nothing() {
    let tree = TempTree::new("component_config-second-run");
    let env = env_in(&tree, &[]);
    let first = load(&env).expect("first run");
    let second = load(&env).expect("second run");
    assert_eq!(first, second);
    let text = fs::read_to_string(config_folder(&tree).join("config.json")).expect("config exists");
    assert_eq!(text, DEFAULT_TEXT);
    assert!(config_folder(&tree).join("profiles").is_dir());
}

// D-05: a config the user edited is never rewritten by a later run
#[test]
fn existing_config_is_never_rewritten() {
    let tree = TempTree::new("component_config-never-rewritten");
    let env = env_in(&tree, &[]);
    let edited = "{\"search_paths\": [\"/srv/mine\"], \"default_profile\": \"dev\"}";
    write_config(&tree, edited);
    load(&env).expect("loads");
    let text = fs::read_to_string(config_folder(&tree).join("config.json")).expect("config exists");
    assert_eq!(text, edited);
}

// FR-CONF-02: search paths and the default profile are read from the file
#[test]
fn search_paths_and_default_profile_are_read_from_file() {
    let tree = TempTree::new("component_config-read-values");
    let env = env_in(&tree, &[]);
    write_config(
        &tree,
        "{\"search_paths\": [\"/srv/code\", \"/home/x/work\"], \"default_profile\": \"dev\"}",
    );
    let config = load(&env).expect("loads");
    let paths: Vec<&str> = config.search_paths.iter().map(AbsPath::as_str).collect();
    assert_eq!(paths, vec!["/srv/code", "/home/x/work"]);
    assert_eq!(config.default_profile.as_deref(), Some("dev"));
}

// FR-CONF-03: "~", "$VAR" and "${VAR}" are expanded from the environment in the file
#[test]
fn search_paths_are_expanded_from_the_environment() {
    let tree = TempTree::new("component_config-expand");
    let env = env_in(&tree, &[("WORK", "/srv/w")]);
    write_config(
        &tree,
        "{\"search_paths\": [\"~/code\", \"$WORK/app\", \"${WORK}/lib\"]}",
    );
    let config = load(&env).expect("loads");
    let paths: Vec<&str> = config.search_paths.iter().map(AbsPath::as_str).collect();
    let home = tree.path().join("home");
    let home = home.to_str().expect("utf-8");
    assert_eq!(
        paths,
        vec![
            format!("{home}/code"),
            "/srv/w/app".into(),
            "/srv/w/lib".into()
        ]
    );
}

// FR-CONF-03, architecture 9.2: an undefined variable is an error that names the entry and variable
#[test]
fn undefined_variable_in_search_path_is_an_error() {
    let tree = TempTree::new("component_config-undefined");
    let env = env_in(&tree, &[]);
    write_config(&tree, "{\"search_paths\": [\"$NOPE/app\"]}");
    assert_eq!(
        load(&env),
        Err(LedgeError::Fatal(
            "config search path '$NOPE/app' uses undefined variable 'NOPE'.".to_string()
        ))
    );
}

// FR-CONF-04: empty entries are skipped and the rest are kept
#[test]
fn empty_search_path_entries_are_skipped() {
    let tree = TempTree::new("component_config-empty-entries");
    let env = env_in(&tree, &[]);
    write_config(&tree, "{\"search_paths\": [\"\", \"/srv/code\", \"\"]}");
    let config = load(&env).expect("loads");
    let paths: Vec<&str> = config.search_paths.iter().map(AbsPath::as_str).collect();
    assert_eq!(paths, vec!["/srv/code"]);
}

// FR-CONF-05: a config file that is not JSON fails with the file named in the message
#[test]
fn config_that_is_not_json_fails_with_file_in_message() {
    let tree = TempTree::new("component_config-not-json");
    let env = env_in(&tree, &[]);
    write_config(&tree, "{ \"search_paths\": ");
    let file = config_folder(&tree).join("config.json");
    assert_eq!(
        load(&env),
        Err(LedgeError::Fatal(format!(
            "config is not valid JSON: {}",
            file.display()
        )))
    );
}

// FR-CONF-06: a profiles folder that already exists is left alone
#[test]
fn existing_profiles_folder_is_kept() {
    let tree = TempTree::new("component_config-existing-profiles");
    let env = env_in(&tree, &[]);
    write_profile(
        &tree,
        "dev",
        "{\"windows\": [{\"name\": \"editor\", \"command\": \"nvim\"}]}",
    );
    load(&env).expect("loads");
    assert!(config_folder(&tree)
        .join("profiles")
        .join("dev.json")
        .is_file());
}

// FR-PROF-01: a profile is the file named by the profile in the profiles folder
#[test]
fn profile_is_read_from_named_file_in_profiles_folder() {
    let tree = TempTree::new("component_config-profile-read");
    let env = env_in(&tree, &[]);
    write_profile(
        &tree,
        "dev",
        "{\"windows\": [{\"name\": \"editor\", \"command\": \"nvim\"}, {\"name\": \"shell\"}]}",
    );
    let dirs = dirs_for(&env);
    let profile = load_profile(&RealFs, &dirs, "dev").expect("loads");
    assert_eq!(profile.name.as_str(), "dev");
    let names: Vec<&str> = profile.windows.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(names, vec!["editor", "shell"]);
    assert_eq!(profile.windows[0].command.as_deref(), Some("nvim"));
    assert_eq!(profile.windows[1].command, None);
}

// D-13, FR-PROF-01: a missing profile file gives the "not found" message with its path
#[test]
fn missing_profile_file_gives_not_found_message() {
    let tree = TempTree::new("component_config-profile-missing");
    let env = env_in(&tree, &[]);
    let dirs = dirs_for(&env);
    let path = config_folder(&tree).join("profiles").join("ghost.json");
    assert_eq!(
        load_profile(&RealFs, &dirs, "ghost"),
        Err(LedgeError::Fatal(format!(
            "profile 'ghost' not found at {}",
            path.display()
        )))
    );
}

// D-13: a name that does not match [A-Za-z0-9_-]+ gives the "not found" error and no file is read
#[test]
fn invalid_profile_name_gives_not_found_message() {
    let tree = TempTree::new("component_config-profile-bad-name");
    let env = env_in(&tree, &[]);
    let dirs = dirs_for(&env);
    let result = load_profile(&RealFs, &dirs, "../escape");
    assert!(
        matches!(result, Err(LedgeError::Fatal(text)) if text.starts_with("profile '../escape' not found at "))
    );
}

// D-13: a profile file that is not JSON gives the "is invalid JSON" message
#[test]
fn profile_that_is_not_json_gives_invalid_json_message() {
    let tree = TempTree::new("component_config-profile-not-json");
    let env = env_in(&tree, &[]);
    write_profile(&tree, "dev", "{ windows");
    let dirs = dirs_for(&env);
    let result = load_profile(&RealFs, &dirs, "dev");
    assert!(
        matches!(result, Err(LedgeError::Fatal(text)) if text.starts_with("profile 'dev' is invalid JSON: "))
    );
}

// FR-PROF-02: a profile without a windows list gives the shape message
#[test]
fn profile_without_windows_gives_shape_message() {
    let tree = TempTree::new("component_config-profile-no-windows");
    let env = env_in(&tree, &[]);
    write_profile(&tree, "dev", "{\"layout\": \"x\"}");
    let dirs = dirs_for(&env);
    assert_eq!(
        load_profile(&RealFs, &dirs, "dev"),
        Err(LedgeError::Fatal(
            "profile 'dev' is invalid (expected a 'windows' array)".to_string()
        ))
    );
}

// FR-PROF-06, D-13: "basic" is built in; a profiles/basic.json file is ignored
#[test]
fn basic_is_built_in_and_its_file_is_ignored() {
    let tree = TempTree::new("component_config-basic");
    let env = env_in(&tree, &[]);
    write_profile(
        &tree,
        "basic",
        "{\"windows\": [{\"name\": \"other\"}, {\"name\": \"more\"}]}",
    );
    let dirs = dirs_for(&env);
    assert_eq!(load_profile(&RealFs, &dirs, "basic"), Ok(Profile::basic()));
}

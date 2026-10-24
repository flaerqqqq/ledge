//! Component tests for `ledge doctor` (docs/testing.md level 2, FR-DOC-01..08, architecture 8.4). The config, profiles
//! and bookmarks are real files in a temp folder; tmux is `FakeBackend` and programs are `FakeProcess`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::doctor;
use ledge::error::Outcome;
use ledge::ports::backend::{NewSession, SessionBackend};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use support::ctx::Harness;
use support::fake_backend::FakeBackend;
use support::fake_process::FakeProcess;

/// The outcome as (exit code, stdout lines). `Info` is exit 0, `Report` carries its own code.
fn run_doctor(h: &Harness) -> (i32, Vec<String>) {
    let outcome = doctor::run(&h.ctx()).expect("doctor does not fail");
    match outcome {
        Outcome::Info(text) => (0, lines(&text)),
        Outcome::Report(text, code) => (code, lines(&text)),
        other => panic!("unexpected outcome {other:?}"),
    }
}

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

/// A harness with a valid config whose only search path is `<root>/projects`, which exists.
fn ready(name: &str) -> Harness {
    let h = Harness::new(name);
    let search = format!("{}/projects", h.root().display());
    h.write_config(&format!("{{\"search_paths\": [\"{search}\"]}}"));
    h
}

/// `<root>/<rel>`, created as a folder.
fn folder(h: &Harness, rel: &str) -> String {
    let path = format!("{}/{rel}", h.root().display());
    fs::create_dir_all(&path).expect("create folder");
    path
}

fn config_file(h: &Harness) -> PathBuf {
    PathBuf::from(h.dirs.config.as_str()).join("config.json")
}

fn write_bookmarks(h: &Harness, text: &str) {
    let folder = PathBuf::from(h.dirs.cache.as_str());
    fs::create_dir_all(&folder).expect("create cache folder");
    fs::write(folder.join("bookmarks"), text).expect("write bookmarks");
}

/// A managed session on the fake server, with the options ledge sets (FR-NAME-03).
fn managed_session(h: &Harness, name: &str, dir: &str, profile: Option<&str>) {
    h.backend
        .create_session(&NewSession {
            name: name.to_string(),
            dir: dir.to_string(),
            window_name: "main".to_string(),
        })
        .expect("create session");
    h.backend
        .set_option(name, "@ledge_managed", "1")
        .expect("set managed");
    if let Some(profile) = profile {
        h.backend
            .set_option(name, "@ledge_profile", profile)
            .expect("set profile");
    }
}

fn config_path_text(h: &Harness) -> String {
    config_file(h).display().to_string()
}

// FR-DOC-08, architecture 8.4: a clean installation prints one OK line per check, in the fixed order, and exits 0
#[test]
fn clean_installation_prints_every_check_in_order_and_exits_zero() {
    let h = ready("doctor-clean");
    let search = format!("{}/projects", h.root().display());
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        vec![
            "OK    tmux 3.4".to_string(),
            "OK    setsid".to_string(),
            "OK    code".to_string(),
            "OK    idea".to_string(),
            format!("OK    config file {}", config_path_text(&h)),
            format!("OK    search path {search}"),
            "OK    bookmarks (0)".to_string(),
            "OK    sessions (0)".to_string(),
        ]
    );
}

// FR-DOC-01: tmux is checked first; a version of 3.2 or later is OK
#[test]
fn tmux_version_three_two_is_ok() {
    let mut h = ready("doctor-tmux-32");
    h.backend = FakeBackend::new().with_version(3, 2);
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert_eq!(out.first().map(String::as_str), Some("OK    tmux 3.2"));
}

// FR-DOC-01: an installed tmux older than 3.2 is an ERR line (architecture 8.4), and the exit is 1
#[test]
fn tmux_older_than_three_two_is_an_error() {
    let mut h = ready("doctor-tmux-old");
    h.backend = FakeBackend::new().with_version(3, 1);
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert_eq!(
        out.first().map(String::as_str),
        Some("ERR   tmux 3.1 is older than 3.2")
    );
}

// FR-DOC-01: a missing tmux is an ERR line, not a failure; the other checks still print (A-29)
#[test]
fn missing_tmux_is_an_error_line_and_checks_continue() {
    let mut h = ready("doctor-tmux-missing");
    h.process = FakeProcess::new().absent(&["tmux"]);
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert_eq!(
        out.first().map(String::as_str),
        Some("ERR   tmux is not installed")
    );
    assert!(out.contains(&format!("OK    config file {}", config_path_text(&h))));
    assert_eq!(
        out.last().map(String::as_str),
        Some("WARN  sessions not checked (tmux is not installed)")
    );
}

// FR-DOC-01: setsid, code and idea are WARN lines when missing, and the exit stays 0 (architecture 8.4, A-12)
#[test]
fn missing_setsid_code_and_idea_are_warnings() {
    let mut h = ready("doctor-optional");
    h.process = FakeProcess::new().absent(&["setsid", "code", "idea"]);
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert_eq!(
        out[1],
        "WARN  setsid is not installed (editor launch will not detach)"
    );
    assert_eq!(out[2], "WARN  code is not installed (needed for --code)");
    assert_eq!(out[3], "WARN  idea is not installed (needed for --idea)");
}

// FR-DOC-02: a missing config file is an ERR line; doctor does not create it or the profiles folder
#[test]
fn missing_config_is_an_error_and_is_not_created() {
    let h = Harness::new("doctor-config-missing");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert!(out.contains(&format!(
        "ERR   config file {} is missing",
        config_path_text(&h)
    )));
    assert!(
        !config_file(&h).exists(),
        "doctor must not create the config"
    );
    assert!(!PathBuf::from(h.dirs.config.as_str())
        .join("profiles")
        .exists());
}

// FR-DOC-02: a config that is not JSON is an ERR line with the parse message
#[test]
fn config_that_is_not_json_is_an_error() {
    let h = Harness::new("doctor-config-json");
    h.write_config("{ nope");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert!(out.contains(&format!(
        "ERR   config is not valid JSON: {}",
        config_path_text(&h)
    )));
}

// FR-DOC-02: a config with search_paths of the wrong type is an ERR line with the invalid message
#[test]
fn config_with_wrong_search_paths_type_is_an_error() {
    let h = Harness::new("doctor-config-type");
    h.write_config("{\"search_paths\": 4}");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert!(out.contains(&format!(
        "ERR   config is invalid (expected 'search_paths' to be a list of strings): {}",
        config_path_text(&h)
    )));
}

// FR-DOC-03: no search path configured is a WARN line
#[test]
fn no_search_path_is_a_warning() {
    let h = Harness::new("doctor-no-search");
    h.write_config("{}");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&"WARN  no search path is configured".to_string()));
}

// FR-DOC-03: a search path that does not exist is a WARN line; the path is shown cleaned
#[test]
fn missing_search_path_is_a_warning() {
    let h = Harness::new("doctor-search-missing");
    let gone = format!("{}/gone", h.root().display());
    h.write_config(&format!("{{\"search_paths\": [\"{gone}\"]}}"));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&format!("WARN  search path {gone} does not exist")));
}

// FR-DOC-04: a bookmark whose directory is gone is a WARN line; a bookmark outside every project is a WARN line
#[test]
fn bookmarks_missing_or_outside_projects_are_warnings() {
    let h = ready("doctor-bookmarks-warn");
    let project = h.project("app");
    let outside = folder(&h, "elsewhere");
    let gone = format!("{project}/gone");
    write_bookmarks(&h, &format!("{gone}\n{outside}\n"));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&format!("WARN  bookmark {gone} is missing")));
    assert!(out.contains(&format!(
        "WARN  bookmark {outside} is outside every project"
    )));
    assert!(!out.iter().any(|line| line.starts_with("OK    bookmarks")));
}

// FR-DOC-04: bookmarks that all exist and lie inside projects give one OK line with the count
#[test]
fn good_bookmarks_give_one_ok_line_with_the_count() {
    let h = ready("doctor-bookmarks-ok");
    let app = h.project("app");
    let lib = h.project("lib");
    write_bookmarks(&h, &format!("{app}\n{lib}\n"));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&"OK    bookmarks (2)".to_string()));
}

// FR-DOC-05: a profile file without a windows list is a WARN line with the architecture 8.4 text
#[test]
fn profile_without_windows_is_a_warning() {
    let h = ready("doctor-profile-shape");
    h.write_profile("dev", "{\"name\": \"dev\"}");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(
        out.contains(&"WARN  profile 'dev' is invalid (expected a 'windows' array)".to_string())
    );
}

// FR-DOC-05: a valid profile file prints nothing
#[test]
fn valid_profile_prints_nothing() {
    let h = ready("doctor-profile-ok");
    h.write_profile("dev", "{\"windows\": [{\"name\": \"edit\"}]}");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(
        !out.iter().any(|line| !line.starts_with("OK")),
        "a valid profile adds no line: {out:?}"
    );
}

// FR-DOC-05: profiles/basic.json is reserved and is reported as ignored
#[test]
fn basic_profile_file_is_reported_as_reserved() {
    let h = ready("doctor-profile-basic");
    h.write_profile("basic", "{\"windows\": [{\"name\": \"x\"}]}");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(
        &"WARN  profile 'basic' is reserved; profiles/basic.json is ignored".to_string()
    ));
}

// FR-DOC-06: a default profile that is valid is an OK line
#[test]
fn valid_default_profile_is_ok() {
    let h = ready("doctor-default-ok");
    h.write_profile("dev", "{\"windows\": [{\"name\": \"edit\"}]}");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}/projects\"], \"default_profile\": \"dev\"}}",
        h.root().display()
    ));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&"OK    default profile dev".to_string()));
}

// FR-DOC-06: a default profile that does not exist is a WARN line that names it and gives the reason
#[test]
fn missing_default_profile_is_a_warning_with_reason() {
    let h = Harness::new("doctor-default-missing");
    h.write_config(&format!(
        "{{\"search_paths\": [\"{}/projects\"], \"default_profile\": \"ghost\"}}",
        h.root().display()
    ));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    let line = out
        .iter()
        .find(|line| line.starts_with("WARN  default_profile 'ghost'"))
        .expect("warning line for the default profile");
    assert!(line.starts_with(
        "WARN  default_profile 'ghost' is missing or invalid: profile 'ghost' not found at "
    ));
}

// FR-DOC-07: a managed session whose directory is missing is a WARN line
#[test]
fn managed_session_with_missing_directory_is_a_warning() {
    let h = ready("doctor-session-dir");
    let gone = format!("{}/gone", h.root().display());
    managed_session(&h, "gone-1a2b", &gone, None);
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(out.contains(&format!(
        "WARN  session 'gone-1a2b' directory is missing: {gone}"
    )));
}

// FR-DOC-07: a managed session whose profile no longer exists is a WARN line
#[test]
fn managed_session_with_missing_profile_is_a_warning() {
    let h = ready("doctor-session-profile");
    let project = h.project("app");
    managed_session(&h, "app-1a2b", &project, Some("ghost"));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(
        out.contains(&"WARN  session 'app-1a2b' profile 'ghost' is missing or invalid".to_string())
    );
}

// FR-DOC-07: a healthy managed session counts in the OK line; a session without the managed marker is not checked
#[test]
fn healthy_managed_sessions_count_and_unmanaged_ones_are_ignored() {
    let h = ready("doctor-session-ok");
    let project = h.project("app");
    managed_session(&h, "app-1a2b", &project, None);
    h.backend
        .create_session(&NewSession {
            name: "plain".to_string(),
            dir: format!("{}/gone", h.root().display()),
            window_name: "main".to_string(),
        })
        .expect("create plain session");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert_eq!(out.last().map(String::as_str), Some("OK    sessions (1)"));
}

// FR-DOC-07: a tmux server that fails to list sessions is an ERR line
#[test]
fn failing_session_listing_is_an_error_line() {
    let mut h = ready("doctor-session-fail");
    h.backend = FakeBackend::new().fail_on("list_sessions", "server exploded");
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 1);
    assert!(out.contains(&"ERR   could not list sessions: server exploded".to_string()));
}

// FR-DOC-08: every line starts with a six-character tag: OK, WARN or ERR padded with spaces, then the text
#[test]
fn every_line_has_a_padded_tag() {
    let mut h = Harness::new("doctor-tags");
    h.process = FakeProcess::new().absent(&["setsid", "tmux"]);
    let (_, out) = run_doctor(&h);
    assert!(!out.is_empty());
    for line in &out {
        let tag: String = line.chars().take(6).collect();
        assert!(
            ["OK    ", "WARN  ", "ERR   "].contains(&tag.as_str()),
            "line without a padded tag: {line:?}"
        );
    }
}

// FR-CMD-07, FR-DOC-08: the built binary prints the report on stdout and exits 1 when a check fails. A temp HOME and
// XDG folders are used, and PATH has no programs, so tmux is missing.
#[test]
fn binary_doctor_exits_one_with_error_line_when_tmux_is_missing() {
    let h = Harness::new("doctor-binary");
    let empty_path = folder(&h, "empty-bin");
    let output = Command::new(env!("CARGO_BIN_EXE_ledge"))
        .arg("doctor")
        .env_remove("TMUX")
        .env("PATH", &empty_path)
        .env("HOME", format!("{}/home", h.root().display()))
        .env(
            "XDG_CONFIG_HOME",
            format!("{}/xdg-config", h.root().display()),
        )
        .env(
            "XDG_CACHE_HOME",
            format!("{}/xdg-cache", h.root().display()),
        )
        .output()
        .expect("binary runs");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert!(
        stdout.starts_with("ERR   tmux is not installed\n"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("WARN  setsid is not installed"),
        "stdout: {stdout}"
    );
    assert!(output.stderr.is_empty());
    assert!(!Path::new(&format!(
        "{}/xdg-config/ledge/config.json",
        h.root().display()
    ))
    .exists());
}

// FR-DOC-04, FR-DOC-07: bookmark and session lines show a path under HOME as "~"; the search path line keeps its
// full path (a search path is a configured folder, not a project directory)
#[test]
fn bookmark_and_session_lines_show_home_paths_as_tilde() {
    let h = Harness::new("doctor-home-tilde");
    let code = format!("{}/home/code", h.root().display());
    let search = format!("{}/projects", h.root().display());
    h.write_config(&format!("{{\"search_paths\": [\"{code}\", \"{search}\"]}}"));
    let app = folder(&h, "home/code/app");
    let elsewhere = folder(&h, "home/elsewhere");
    write_bookmarks(&h, &format!("{app}/gone\n{elsewhere}\n"));
    managed_session(
        &h,
        "vanished-1a2b",
        &format!("{}/home/vanished", h.root().display()),
        None,
    );
    let (code_out, out) = run_doctor(&h);
    assert_eq!(code_out, 0);
    assert!(out.contains(&"WARN  bookmark ~/code/app/gone is missing".to_string()));
    assert!(out.contains(&"WARN  bookmark ~/elsewhere is outside every project".to_string()));
    assert!(
        out.contains(&"WARN  session 'vanished-1a2b' directory is missing: ~/vanished".to_string())
    );
    assert!(out.contains(&format!("OK    search path {code}")));
}

// P-65, FR-DOC-04: a bookmark inside a custom project lies inside a project, so it is not reported as outside
#[test]
fn bookmark_inside_a_custom_project_is_not_outside() {
    let h = ready("doctor-custom-inside");
    let scratch = folder(&h, "scratch");
    let src = folder(&h, "scratch/src");
    h.write_custom_projects(&format!("{scratch}\n"));
    write_bookmarks(&h, &format!("{src}\n"));
    let (code, out) = run_doctor(&h);
    assert_eq!(code, 0);
    assert!(!out
        .iter()
        .any(|line| line.contains("outside every project")));
}

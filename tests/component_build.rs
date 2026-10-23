//! Component tests for `app::build` over `FakeBackend` (docs/testing.md level 2): session creation from a profile,
//! the reuse rules at open (FR-OPEN-07, D-14), the windows and commands (FR-PROF-03..05), the recorded profile
//! and marker (FR-PROF-07, FR-NAME-03), profile use at creation only (FR-PROF-08), and the partial-session kill
//! (A-07).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::build::{ensure_session, Ensured};
use ledge::domain::path::AbsPath;
use ledge::domain::profile::{Profile, ProfileName, Window};
use ledge::error::LedgeError;
use ledge::ports::backend::{NewSession, SessionBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE};
use support::fake_backend::FakeBackend;

/// D-14 example: `/home/u/projects/my.app` is named `my_app-d7d2`.
const DIR: &str = "/home/u/projects/my.app";
const NAME: &str = "my_app-d7d2";

fn dir() -> AbsPath {
    AbsPath::clean(DIR, &AbsPath::root())
}

fn window(name: &str, command: Option<&str>) -> Window {
    Window {
        name: name.to_string(),
        command: command.map(str::to_string),
    }
}

fn profile(name: &str, windows: Vec<Window>) -> Profile {
    Profile {
        name: ProfileName::parse(name).expect("valid profile name"),
        windows,
    }
}

/// A three-window profile with a command in the first and last window, and a duplicate window name.
fn dev_profile() -> Profile {
    profile(
        "dev",
        vec![
            window("editor", Some("nvim")),
            window("shell", None),
            window("editor", Some("tail -f log")),
        ],
    )
}

fn created_error(detail: &str) -> LedgeError {
    LedgeError::Fatal(format!("could not create session '{NAME}': {detail}"))
}

// FR-OPEN-07: no session of the name exists, so the session is created with its windows
#[test]
fn creates_session_named_by_directory_when_none_exists() {
    let backend = FakeBackend::new();
    let outcome = ensure_session(&backend, &dir(), &dev_profile());
    assert_eq!(outcome, Ok(Ensured::Created));
    let names: Vec<String> = backend
        .list_sessions()
        .expect("list")
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec![NAME.to_string()]);
}

// FR-PROF-03: the windows are created in the order the profile lists them, first window first
#[test]
fn windows_are_created_in_profile_order() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(
        backend.window_names(NAME),
        vec!["editor", "shell", "editor"]
    );
}

// D-13: every window starts in the session directory
#[test]
fn every_window_starts_in_the_session_directory() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(backend.window_dirs(NAME), vec![DIR, DIR, DIR]);
}

// FR-PROF-04: a window with a command gets it typed then Enter; a window without one gets nothing typed
#[test]
fn commands_are_typed_in_their_own_window() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(
        backend.windows_with_typed(NAME),
        vec![
            ("editor".to_string(), vec!["nvim".to_string()]),
            ("shell".to_string(), vec![]),
            ("editor".to_string(), vec!["tail -f log".to_string()]),
        ]
    );
}

// FR-PROF-05: the first window is the session's first window and is selected after the build
#[test]
fn first_window_is_selected_after_build() {
    let backend = FakeBackend::new();
    let layout = profile(
        "dev",
        vec![
            window("editor", None),
            window("shell", None),
            window("logs", None),
        ],
    );
    ensure_session(&backend, &dir(), &layout).expect("built");
    assert_eq!(backend.window_names(NAME)[0], "editor");
    assert_eq!(backend.selected_window_name().as_deref(), Some("editor"));
}

// FR-PROF-07: the named profile is recorded on the session
#[test]
fn named_profile_is_recorded_on_the_session() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(backend.option(NAME, OPT_PROFILE).as_deref(), Some("dev"));
}

// FR-PROF-07: the single-window layout records "basic"
#[test]
fn basic_layout_records_basic() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &Profile::basic()).expect("built");
    assert_eq!(backend.option(NAME, OPT_PROFILE).as_deref(), Some("basic"));
    assert_eq!(backend.window_names(NAME), vec!["Terminal"]);
}

// FR-NAME-03: the session is marked managed and records its directory when it is created
#[test]
fn created_session_carries_managed_marker_and_directory() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(backend.option(NAME, OPT_MANAGED).as_deref(), Some("1"));
    assert_eq!(backend.option(NAME, OPT_DIR).as_deref(), Some(DIR));
}

// FR-OPEN-07: a session of the name that exists is not created again; no window is added
#[test]
fn existing_unmanaged_session_is_reused_without_building_or_marking() {
    let backend = FakeBackend::new();
    backend
        .create_session(&NewSession {
            name: NAME.to_string(),
            dir: DIR.to_string(),
            window_name: "work".to_string(),
        })
        .expect("pre-existing session");
    let outcome = ensure_session(&backend, &dir(), &dev_profile());
    assert_eq!(outcome, Ok(Ensured::Reused));
    assert_eq!(backend.window_names(NAME), vec!["work"]);
    assert_eq!(backend.option(NAME, OPT_MANAGED), None);
    assert_eq!(backend.option(NAME, OPT_PROFILE), None);
}

// FR-PROF-08: a profile applies only when the session is created; reopening with another profile changes nothing
#[test]
fn profile_applies_only_at_creation() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    let other = profile("other", vec![window("only", Some("htop"))]);
    let outcome = ensure_session(&backend, &dir(), &other);
    assert_eq!(outcome, Ok(Ensured::Reused));
    assert_eq!(backend.option(NAME, OPT_PROFILE).as_deref(), Some("dev"));
    assert_eq!(
        backend.window_names(NAME),
        vec!["editor", "shell", "editor"]
    );
    assert_eq!(backend.windows_with_typed(NAME).len(), 3);
}

// D-14: a managed session of the name that records another directory is refused, nothing is built
#[test]
fn managed_session_with_another_directory_is_refused() {
    let backend = FakeBackend::new();
    backend
        .create_session(&NewSession {
            name: NAME.to_string(),
            dir: "/elsewhere".to_string(),
            window_name: "work".to_string(),
        })
        .expect("pre-existing session");
    backend.set_option(NAME, OPT_MANAGED, "1").expect("mark");
    backend
        .set_option(NAME, OPT_DIR, "/elsewhere")
        .expect("dir");
    let outcome = ensure_session(&backend, &dir(), &dev_profile());
    assert_eq!(
        outcome,
        Err(LedgeError::Fatal(
            "could not create session 'my_app-d7d2': name already used by /elsewhere".to_string()
        ))
    );
    assert_eq!(backend.window_names(NAME), vec!["work"]);
}

// D-14, A-05: a managed session with no recorded directory is reused (no collision check)
#[test]
fn managed_session_without_recorded_directory_is_reused() {
    let backend = FakeBackend::new();
    backend
        .create_session(&NewSession {
            name: NAME.to_string(),
            dir: DIR.to_string(),
            window_name: "work".to_string(),
        })
        .expect("pre-existing session");
    backend.set_option(NAME, OPT_MANAGED, "1").expect("mark");
    assert_eq!(
        ensure_session(&backend, &dir(), &dev_profile()),
        Ok(Ensured::Reused)
    );
}

// FR-OPEN-07, D-14: a managed session whose recorded directory is the same is reused
#[test]
fn managed_session_with_same_directory_is_reused() {
    let backend = FakeBackend::new();
    ensure_session(&backend, &dir(), &dev_profile()).expect("built");
    assert_eq!(
        ensure_session(&backend, &dir(), &dev_profile()),
        Ok(Ensured::Reused)
    );
}

// A-07: a failed build kills the partial session, for every step after create_session
#[test]
fn failed_build_kills_partial_session() {
    for operation in ["set_option", "send_line", "add_window", "select_window"] {
        let backend = FakeBackend::new().fail_on(operation, "boom");
        let outcome = ensure_session(&backend, &dir(), &dev_profile());
        assert_eq!(outcome, Err(created_error("boom")), "{operation}");
        assert!(
            backend.list_sessions().expect("list").is_empty(),
            "partial session left after {operation}"
        );
    }
}

// FR-OPEN-07: a failure to create the session gives the build message with the backend detail
#[test]
fn failed_create_gives_build_message() {
    let backend = FakeBackend::new().fail_on("create_session", "server exploded");
    assert_eq!(
        ensure_session(&backend, &dir(), &dev_profile()),
        Err(created_error("server exploded"))
    );
    assert!(backend.list_sessions().expect("list").is_empty());
}

// FR-PROF-02: a profile with no windows is refused before any session is created
#[test]
fn profile_without_windows_creates_nothing() {
    let backend = FakeBackend::new();
    let empty = profile("dev", vec![]);
    assert_eq!(
        ensure_session(&backend, &dir(), &empty),
        Err(LedgeError::Fatal(
            "profile 'dev' is invalid (expected a 'windows' array)".to_string()
        ))
    );
    assert!(backend.list_sessions().expect("list").is_empty());
}

//! Integration tests against a real tmux (docs/testing.md level 4). Each test uses its own server name
//! `ledge-it-<pid>-<label>`, killed on drop, and removes TMUX from the test process so a tmux session the
//! developer runs these from is never reached. Run with `cargo test -- --ignored`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::process::SystemProcess;
use ledge::adapters::tmux::{TmuxBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE};
use ledge::domain::sessions::{managed, session_directory, tmux_version_supported};
use ledge::ports::backend::{NewSession, SessionBackend};
use std::process::Command;
use support::temp::TempTree;

/// Kills the isolated tmux server on drop, ignoring failures (testing.md, tmux isolation).
struct TestServer {
    name: String,
}

impl TestServer {
    fn new(label: &str) -> Self {
        // The test process must not see the developer's tmux session (testing.md).
        std::env::remove_var("TMUX");
        Self {
            name: format!("ledge-it-{}-{label}", std::process::id()),
        }
    }

    fn backend<'a>(&self, process: &'a SystemProcess) -> TmuxBackend<'a> {
        TmuxBackend::new(process, Some(self.name.clone()))
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.name, "kill-server"])
            .output();
    }
}

fn new_session(name: &str, dir: &str) -> NewSession {
    NewSession {
        name: name.to_string(),
        dir: dir.to_string(),
        window_name: "editor".to_string(),
    }
}

// FR-PLAT-01, FR-OPEN-09: with no server running the list is empty; the first session starts the server
#[test]
#[ignore = "requires tmux"]
fn first_session_starts_the_server() {
    let process = SystemProcess;
    let server = TestServer::new("first");
    let backend = server.backend(&process);
    assert_eq!(backend.list_sessions(), Ok(Vec::new()));

    let window = backend
        .create_session(&new_session("app-1a2b", "/"))
        .expect("create starts the server");
    assert!(window.starts_with('@'), "window id {window}");

    let listed = backend.list_sessions().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "app-1a2b");
    assert_eq!(listed[0].windows, 1);
}

// FR-PLAT-01: the installed tmux is 3.2 or later (the minimum of architecture 7.2)
#[test]
#[ignore = "requires tmux"]
fn installed_tmux_meets_the_minimum_version() {
    let process = SystemProcess;
    let server = TestServer::new("version");
    let version = server.backend(&process).version().expect("version");
    assert!(
        tmux_version_supported(version),
        "tmux {version:?} is too old"
    );
}

// FR-NAME-03: the managed marker, profile and directory are set and read back, with the windows added
#[test]
#[ignore = "requires tmux"]
fn managed_options_and_windows_are_created_and_read_back() {
    let process = SystemProcess;
    let server = TestServer::new("options");
    let backend = server.backend(&process);
    let tree = TempTree::new("options");
    let dir = tree.path().to_string_lossy().into_owned();

    backend
        .create_session(&new_session("app-1a2b", &dir))
        .expect("create");
    backend
        .set_option("app-1a2b", OPT_MANAGED, "1")
        .expect("managed");
    backend
        .set_option("app-1a2b", OPT_PROFILE, "dev")
        .expect("profile");
    backend.set_option("app-1a2b", OPT_DIR, &dir).expect("dir");
    let second = backend
        .add_window("app-1a2b", "server", &dir)
        .expect("window");
    backend.send_line(&second, "echo hi").expect("send");
    backend.select_window(&second).expect("select");

    let listed = backend.list_sessions().expect("list");
    assert_eq!(listed.len(), 1);
    let session = &listed[0];
    assert_eq!(session.windows, 2);
    assert!(session.managed);
    assert_eq!(session.profile.as_deref(), Some("dev"));
    assert_eq!(session_directory(session), Some(dir.as_str()));
}

// FR-NAME-04: a session without the marker is filtered out of every list
#[test]
#[ignore = "requires tmux"]
fn unmanaged_sessions_are_filtered_out() {
    let process = SystemProcess;
    let server = TestServer::new("filter");
    let backend = server.backend(&process);
    backend
        .create_session(&new_session("app-1a2b", "/"))
        .expect("managed create");
    backend
        .set_option("app-1a2b", OPT_MANAGED, "1")
        .expect("marker");
    backend
        .create_session(&new_session("scratch", "/"))
        .expect("unmanaged create");

    let all = backend.list_sessions().expect("list");
    assert_eq!(all.len(), 2, "both sessions are listed by the backend");
    let kept: Vec<String> = managed(all).iter().map(|s| s.info().name.clone()).collect();
    assert_eq!(kept, vec!["app-1a2b".to_string()]);
}

// FR-NAME-05: a session given the marker by hand is adopted; with no @ledge_dir it has no collision directory (A-05)
#[test]
#[ignore = "requires tmux"]
fn session_marked_by_hand_is_adopted() {
    let process = SystemProcess;
    let server = TestServer::new("adopt");
    let backend = server.backend(&process);
    backend
        .create_session(&new_session("legacy", "/"))
        .expect("create");
    assert!(managed(backend.list_sessions().expect("list")).is_empty());

    let status = Command::new("tmux")
        .args([
            "-L",
            &server.name,
            "set-option",
            "-t",
            "=legacy:",
            OPT_MANAGED,
            "1",
        ])
        .status()
        .expect("tmux runs");
    assert!(status.success());

    let adopted = managed(backend.list_sessions().expect("list"));
    assert_eq!(adopted.len(), 1);
    assert_eq!(adopted[0].info().name, "legacy");
    assert_eq!(adopted[0].info().dir, None);
}

// A-17: tmux reports the session_path of a session on the installed tmux (3.2 is checked by hand, see traceability)
#[test]
#[ignore = "requires tmux"]
fn session_path_is_reported() {
    let process = SystemProcess;
    let server = TestServer::new("path");
    let backend = server.backend(&process);
    let tree = TempTree::new("path");
    let dir = tree.path().to_string_lossy().into_owned();
    backend
        .create_session(&new_session("pathed", &dir))
        .expect("create");
    let listed = backend.list_sessions().expect("list");
    let path = listed[0].path.clone().expect("session_path is set");
    let expected = std::fs::canonicalize(tree.path()).expect("canonical");
    let actual = std::fs::canonicalize(&path).expect("canonical");
    assert_eq!(actual, expected);
}

// D-17, architecture 7.1: rename and kill act on the exact session name
#[test]
#[ignore = "requires tmux"]
fn rename_then_kill_removes_the_session() {
    let process = SystemProcess;
    let server = TestServer::new("kill");
    let backend = server.backend(&process);
    backend
        .create_session(&new_session("old", "/"))
        .expect("create");
    backend.rename_session("old", "new").expect("rename");
    let names: Vec<String> = backend
        .list_sessions()
        .expect("list")
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["new".to_string()]);
    backend.kill_session("new").expect("kill");
    assert!(backend.list_sessions().expect("list").is_empty());
}

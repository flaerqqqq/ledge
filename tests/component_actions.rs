//! Component tests for the kill and rebuild services (`app::actions`) over `FakeBackend`, `FakeEnv`, `FakeConsole`
//! and `RealFs` on temp folders (docs/testing.md level 2): managed-only kill and rebuild (FR-ACT-13, D-14), the
//! rebuild from the profile (FR-ACT-02, FR-ACT-03, FR-PROF-08), the profile precedence at rebuild (FR-ACT-12,
//! FR-PROF-10, FR-PROF-11, FR-PROF-14, FR-PROF-15, D-04), the rebuild and kill of the session the user is in
//! (FR-ACT-05, FR-ACT-07, D-17, A-07).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::actions::{delete_node_bookmarks, kill, kill_node, rebuild, rebuild_node};
use ledge::app::build::ensure_session;
use ledge::app::session_cmd;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::{Profile, ProfileName};
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{LedgeError, Outcome};
use ledge::ports::backend::{NewSession, SessionBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE};
use std::fs;
use std::path::PathBuf;
use support::ctx::Harness;
use support::fake_backend::FakeBackend;

const DEV: &str = r#"{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}"#;
const WEB: &str = r#"{"windows": [{"name": "server", "command": "npm start"}, {"name": "logs"}]}"#;
const TMUX_SET: &str = "/tmp/tmux-1000/default,1,0";

fn fatal(text: &str) -> LedgeError {
    LedgeError::Fatal(text.to_string())
}

/// The clean absolute directory of a project folder made under the temp root.
fn dir_of(h: &Harness, project: &str) -> AbsPath {
    AbsPath::clean(&h.project(project), &AbsPath::root())
}

fn profile_of(name: &str, text: &str) -> Profile {
    Profile::parse(&ProfileName::parse(name).expect("valid name"), text).expect("valid profile")
}

/// Builds the managed session of `dir` from `profile`, as the open flow does (FR-OPEN-07).
fn build_managed(h: &Harness, dir: &AbsPath, profile: &Profile) {
    ensure_session(&h.backend, dir, profile).expect("build session");
}

fn exists(h: &Harness, name: &str) -> bool {
    h.backend
        .list_sessions()
        .expect("list")
        .iter()
        .any(|session| session.name == name)
}

/// Window names with the typed lines of each window, in order.
fn windows(h: &Harness, name: &str) -> Vec<(String, Vec<String>)> {
    h.backend.windows_with_typed(name)
}

fn expected(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    rows.iter()
        .map(|(name, typed)| {
            (
                name.to_string(),
                typed.iter().map(|line| line.to_string()).collect(),
            )
        })
        .collect()
}

/// Makes the session `name` the one the user is in: TMUX is set and the client is switched to it.
fn be_inside(h: &mut Harness, name: &str) {
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend.switch_client(name).expect("switch client");
}

// FR-ACT-13: a kill removes a managed session
#[test]
fn kill_removes_a_managed_session() {
    let h = Harness::new("act_kill_managed");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    assert_eq!(kill(&h.ctx(), &name), Ok(Outcome::Done));
    assert!(!exists(&h, &name));
}

// FR-ACT-13, D-14: an unmanaged session is refused and left alone
#[test]
fn kill_refuses_an_unmanaged_session() {
    let h = Harness::new("act_kill_unmanaged");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    h.backend
        .create_session(&NewSession {
            name: name.clone(),
            dir: dir.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    assert_eq!(
        kill(&h.ctx(), &name),
        Err(fatal(&format!("'{name}' is not a ledge session.")))
    );
    assert!(exists(&h, &name));
}

// FR-ACT-07, FR-SESS-12, P-63: killing the session the user is in switches the client to the most recent other
// managed session first, then kills the original
#[test]
fn kill_of_the_current_session_switches_to_the_most_recent_other() {
    let mut h = Harness::new("act_kill_current_switch");
    let app = session_name(&dir_of(&h, "app"));
    let api = session_name(&dir_of(&h, "api"));
    let lib = session_name(&dir_of(&h, "lib"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "api"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "lib"), &Profile::basic());
    h.backend.set_activity(&api, 900);
    h.backend.set_activity(&lib, 100);
    be_inside(&mut h, &app);
    assert_eq!(kill(&h.ctx(), &app), Ok(Outcome::Done));
    assert!(!exists(&h, &app));
    assert_eq!(h.backend.current_session(), Ok(Some(api)));
    assert!(exists(&h, &lib));
    assert!(!h.backend.server_killed());
}

// FR-ACT-07, FR-SESS-12, P-63: the most recent other session is chosen among every session, not only managed ones
#[test]
fn kill_of_the_current_session_switches_to_the_most_recent_session_of_any_kind() {
    let mut h = Harness::new("act_kill_current_unmanaged");
    let app = session_name(&dir_of(&h, "app"));
    let lib = session_name(&dir_of(&h, "lib"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "lib"), &Profile::basic());
    h.backend
        .create_session(&NewSession {
            name: "scratch".to_string(),
            dir: h.cwd.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("unmanaged session");
    h.backend.set_activity("scratch", 500);
    h.backend.set_activity(&lib, 100);
    be_inside(&mut h, &app);
    assert_eq!(kill(&h.ctx(), &app), Ok(Outcome::Done));
    assert_eq!(h.backend.current_session(), Ok(Some("scratch".to_string())));
    assert!(exists(&h, &lib));
}

// FR-ACT-07, FR-SESS-12, P-63: killing the last session the user is in kills the tmux server, so tmux closes
#[test]
fn kill_of_the_last_session_kills_the_server() {
    let mut h = Harness::new("act_kill_last");
    let app = session_name(&dir_of(&h, "app"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    be_inside(&mut h, &app);
    assert_eq!(kill(&h.ctx(), &app), Ok(Outcome::Done));
    assert!(h.backend.server_killed());
    assert_eq!(h.backend.list_sessions(), Ok(Vec::new()));
}

// FR-ACT-07, P-63: a kill of another session leaves the client in its session and the server running
#[test]
fn kill_of_another_session_leaves_the_client_alone() {
    let mut h = Harness::new("act_kill_other");
    let app = session_name(&dir_of(&h, "app"));
    let api = session_name(&dir_of(&h, "api"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "api"), &Profile::basic());
    be_inside(&mut h, &app);
    assert_eq!(kill(&h.ctx(), &api), Ok(Outcome::Done));
    assert!(!exists(&h, &api));
    assert!(exists(&h, &app));
    assert_eq!(h.backend.current_session(), Ok(Some(app)));
    assert!(!h.backend.server_killed());
}

// FR-ACT-07, P-63: when the client cannot switch, the session the user is in is kept
#[test]
fn kill_of_the_current_session_keeps_it_when_the_switch_fails() {
    let mut h = Harness::new("act_kill_current_switch_fails");
    h.backend = FakeBackend::new().fail_on("switch_client", "no client");
    let app = session_name(&dir_of(&h, "app"));
    let api = session_name(&dir_of(&h, "api"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "api"), &Profile::basic());
    h.env = h.env.clone().with_var("TMUX", TMUX_SET);
    h.backend.set_current(&app);
    assert!(kill(&h.ctx(), &app).is_err());
    assert!(exists(&h, &app));
    assert!(exists(&h, &api));
}

// FR-ACT-13: a kill of a row with no session has nothing to do
#[test]
fn kill_of_a_missing_session_is_a_no_op() {
    let h = Harness::new("act_kill_missing");
    assert_eq!(kill(&h.ctx(), "nothing-0000"), Ok(Outcome::Done));
}

// Proposal P-26 [new]: a failed kill is a fatal error that names the session
#[test]
fn kill_failure_gives_could_not_kill_message() {
    let mut h = Harness::new("act_kill_failure");
    h.backend = FakeBackend::new().fail_on("kill_session", "server exited");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    assert_eq!(
        kill(&h.ctx(), &name),
        Err(fatal(&format!(
            "could not kill session '{name}': server exited"
        )))
    );
}

// FR-ACT-02, FR-PROF-03..05, FR-PROF-07: a rebuild of a missing session creates it from the profile
#[test]
fn rebuild_creates_a_missing_session_from_the_profile() {
    let h = Harness::new("act_rebuild_missing");
    h.write_profile("dev", DEV);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("dev".to_string())),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(h.backend.option(&name, OPT_MANAGED), Some("1".to_string()));
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("dev".to_string())
    );
    assert_eq!(
        h.backend.option(&name, OPT_DIR),
        Some(dir.as_str().to_string())
    );
}

// FR-ACT-03: a rebuild kills the running session and builds it again from its profile
#[test]
fn rebuild_kills_and_builds_again_from_the_profile() {
    let h = Harness::new("act_rebuild_again");
    h.write_profile("dev", DEV);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("web", WEB));
    assert_eq!(
        windows(&h, &name),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("dev".to_string())),
        Ok(Outcome::Done)
    );
    // The old windows and the command running in them are gone.
    assert_eq!(
        windows(&h, &name),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("dev".to_string())
    );
}

// FR-PROF-08: a rebuild applies the profile as it is now; an edit counts only at rebuild
#[test]
fn rebuild_applies_the_profile_file_as_it_is_now() {
    let h = Harness::new("act_rebuild_edited");
    h.write_profile("dev", DEV);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("dev", DEV));
    h.write_profile("dev", WEB);
    assert_eq!(
        windows(&h, &name),
        expected(&[("editor", &["nvim"]), ("shell", &[])]),
        "an edit has no effect before the rebuild"
    );
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
}

// FR-PROF-14: without a profile option the profile recorded on the session is reused, not the default
#[test]
fn rebuild_without_option_reuses_the_recorded_profile() {
    let h = Harness::new("act_rebuild_recorded");
    h.write_config(r#"{"search_paths": ["~/projects"], "default_profile": "web"}"#);
    h.write_profile("dev", DEV);
    h.write_profile("web", WEB);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("dev", DEV));
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("dev".to_string())
    );
    assert!(h.console.err_lines().is_empty());
}

// FR-ACT-12: a profile given on the rebuild takes effect over the recorded one
#[test]
fn rebuild_with_a_profile_option_overrides_the_recorded_profile() {
    let h = Harness::new("act_rebuild_option");
    h.write_profile("web", WEB);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("web".to_string())),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("web".to_string())
    );
}

// FR-PROF-10: --no-profile gives the single-window layout at rebuild, ignoring the default profile
#[test]
fn rebuild_with_no_profile_gives_the_single_window_layout() {
    let h = Harness::new("act_rebuild_none");
    h.write_config(r#"{"search_paths": ["~/projects"], "default_profile": "web"}"#);
    h.write_profile("web", WEB);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("web", WEB));
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::None),
        Ok(Outcome::Done)
    );
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("basic".to_string())
    );
}

// FR-PROF-11: a missing profile given on the rebuild is the profile's error, and the session keeps running
#[test]
fn rebuild_with_a_missing_requested_profile_fails_and_keeps_the_session() {
    let h = Harness::new("act_rebuild_requested_missing");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    let path = format!("{}/profiles/nope.json", h.dirs.config.as_str());
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("nope".to_string())),
        Err(fatal(&format!("profile 'nope' not found at {path}")))
    );
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
}

// FR-PROF-15, D-04: a recorded profile that cannot be loaded warns and the default profile is used
#[test]
fn missing_recorded_profile_warns_and_uses_the_default_profile() {
    let h = Harness::new("act_rebuild_fallback_default");
    h.write_config(r#"{"search_paths": ["~/projects"], "default_profile": "web"}"#);
    h.write_profile("web", WEB);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("gone", DEV));
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        h.console.err_lines(),
        vec![
            "Warning: session profile 'gone' is missing or invalid; using the default profile."
                .to_string()
        ]
    );
    assert!(h.console.out_lines().is_empty());
}

// FR-PROF-15, D-04: with no default profile the recorded profile falls back to the single window after the warning
#[test]
fn missing_recorded_profile_without_default_uses_the_single_window() {
    let h = Harness::new("act_rebuild_fallback_basic");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("gone", DEV));
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
    assert_eq!(
        h.console.err_lines(),
        vec![
            "Warning: session profile 'gone' is missing or invalid; using the default profile."
                .to_string()
        ]
    );
}

// FR-PROF-12, FR-PROF-15, D-04: when the default is missing too, a second warning is printed and the single window is used
#[test]
fn missing_recorded_and_default_profile_warn_twice_and_use_the_single_window() {
    let h = Harness::new("act_rebuild_fallback_twice");
    h.write_config(r#"{"search_paths": ["~/projects"], "default_profile": "ghost"}"#);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("gone", DEV));
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Ok(Outcome::Done)
    );
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
    assert_eq!(
        h.console.err_lines(),
        vec![
            "Warning: session profile 'gone' is missing or invalid; using the default profile."
                .to_string(),
            "Warning: default_profile 'ghost' is missing or invalid; using single-window layout."
                .to_string(),
        ]
    );
}

// FR-ACT-13, D-14: a rebuild refuses an unmanaged session and leaves it as it is
#[test]
fn rebuild_refuses_an_unmanaged_session() {
    let h = Harness::new("act_rebuild_unmanaged");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    h.backend
        .create_session(&NewSession {
            name: name.clone(),
            dir: dir.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Err(fatal(&format!("'{name}' is not a ledge session.")))
    );
    assert_eq!(h.backend.option(&name, OPT_MANAGED), None);
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
}

// D-14, A-24: a managed session of the name that records another directory is not rebuilt
#[test]
fn rebuild_refuses_a_session_recorded_for_another_directory() {
    let h = Harness::new("act_rebuild_other_dir");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    h.backend
        .create_session(&NewSession {
            name: name.clone(),
            dir: "/elsewhere".to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    h.backend.set_option(&name, OPT_MANAGED, "1").expect("mark");
    h.backend
        .set_option(&name, OPT_DIR, "/elsewhere")
        .expect("dir");
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Err(fatal(&format!(
            "could not create session '{name}': name already used by /elsewhere"
        )))
    );
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
}

// FR-ACT-05, D-17: rebuilding the session the user is in builds a copy, switches to it and gives it the original name
#[test]
fn rebuild_of_the_current_session_switches_to_a_copy_with_the_original_name() {
    let mut h = Harness::new("act_rebuild_current");
    h.write_profile("web", WEB);
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &profile_of("dev", DEV));
    be_inside(&mut h, &name);
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("web".to_string())),
        Ok(Outcome::Done)
    );
    assert!(!exists(&h, &format!("{name}-rebuild")));
    assert_eq!(
        windows(&h, &name),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        h.backend.option(&name, OPT_PROFILE),
        Some("web".to_string())
    );
    assert_eq!(h.backend.current_session(), Ok(Some(name.clone())));
    assert_eq!(h.backend.attached_to(), None);
}

// A-07, D-17: when the copy cannot be built the original stays and the error is returned
#[test]
fn rebuild_of_the_current_session_keeps_the_original_when_the_copy_fails() {
    let mut h = Harness::new("act_rebuild_current_fail");
    h.write_profile("web", WEB);
    h.backend = FakeBackend::new().fail_on("add_window", "no room");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    be_inside(&mut h, &name);
    let copy = format!("{name}-rebuild");
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Named("web".to_string())),
        Err(fatal(&format!(
            "could not create session '{copy}': no room"
        )))
    );
    assert!(!exists(&h, &copy));
    assert_eq!(windows(&h, &name), expected(&[("Terminal", &[])]));
    assert_eq!(h.backend.current_session(), Ok(Some(name.clone())));
}

// A-07, D-17: a copy name already in use is a build failure with the A-07 message
#[test]
fn rebuild_of_the_current_session_refuses_a_taken_copy_name() {
    let mut h = Harness::new("act_rebuild_current_taken");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    let copy = format!("{name}-rebuild");
    build_managed(&h, &dir, &profile_of("dev", DEV));
    h.backend
        .create_session(&NewSession {
            name: copy.clone(),
            dir: dir.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create copy");
    be_inside(&mut h, &name);
    assert_eq!(
        rebuild(&h.ctx(), &dir, &ProfileOpt::Unset),
        Err(fatal(&format!(
            "could not create session '{copy}': name already used"
        )))
    );
    assert_eq!(
        windows(&h, &name),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert!(exists(&h, &copy));
}

// FR-ACT-05, FR-ACT-07: a rebuild of another session while inside tmux is a plain rebuild; the current one is untouched
#[test]
fn rebuild_of_another_session_inside_tmux_is_a_plain_rebuild() {
    let mut h = Harness::new("act_rebuild_other_inside");
    h.write_profile("dev", DEV);
    let app = dir_of(&h, "app");
    let app_name = session_name(&app);
    let lib = dir_of(&h, "lib");
    let lib_name = session_name(&lib);
    build_managed(&h, &app, &Profile::basic());
    build_managed(&h, &lib, &Profile::basic());
    be_inside(&mut h, &lib_name);
    assert_eq!(
        rebuild(&h.ctx(), &app, &ProfileOpt::Named("dev".to_string())),
        Ok(Outcome::Done)
    );
    assert_eq!(
        windows(&h, &app_name),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(windows(&h, &lib_name), expected(&[("Terminal", &[])]));
    assert_eq!(h.backend.current_session(), Ok(Some(lib_name.clone())));
}

/// The profile file text of the fresh profile used after a profile file is edited (FR-PROF-08).
const FRESH: &str = r#"{"windows": [{"name": "fresh"}]}"#;

/// A managed session of `dir` with the profile `name` (`DEV`, `WEB` or `FRESH`), as the open flow records it.
fn managed_with(h: &Harness, dir: &AbsPath, name: &str, text: &str) {
    build_managed(h, dir, &profile_of(name, text));
}

/// The session names of the nodes of these tests: `app` holds `app/api`; `lib` is outside.
struct Node {
    app: AbsPath,
    api: AbsPath,
    lib: AbsPath,
}

/// Three managed sessions: `app` (dev), `app/api` (web) under the node, and `lib` (dev) outside it. The dev
/// profile file is then rewritten, so a rebuild shows whether it applied the current file (FR-PROF-08).
fn node_fixture(h: &mut Harness) -> Node {
    h.write_profile("dev", DEV);
    h.write_profile("web", WEB);
    let app = dir_of(h, "app");
    let api = AbsPath::clean(&format!("{}/api", app.as_str()), &AbsPath::root());
    let lib = dir_of(h, "lib");
    managed_with(h, &app, "dev", DEV);
    managed_with(h, &api, "web", WEB);
    managed_with(h, &lib, "dev", DEV);
    h.write_profile("dev", FRESH);
    Node { app, api, lib }
}

/// The bookmarks file in the cache folder.
fn bookmarks_file(h: &Harness) -> PathBuf {
    PathBuf::from(h.dirs.cache.as_str()).join("bookmarks")
}

// FR-ACT-08: alt-r rebuilds each session under the node with the profile recorded on it; the one outside is untouched
#[test]
fn node_rebuild_uses_each_recorded_profile_and_leaves_outside_sessions_alone() {
    let mut h = Harness::new("component_actions-node-rebuild");
    let n = node_fixture(&mut h);
    rebuild_node(&h.ctx(), &n.app, &ProfileOpt::Unset).expect("node rebuild");
    assert_eq!(
        windows(&h, &session_name(&n.app)),
        expected(&[("fresh", &[])])
    );
    assert_eq!(
        windows(&h, &session_name(&n.api)),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
    assert_eq!(
        windows(&h, &session_name(&n.lib)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
}

// FR-ACT-09: alt-p rebuilds every session under the node with the one profile chosen, and records it
#[test]
fn node_profile_rebuilds_every_session_under_the_node_with_the_chosen_profile() {
    let mut h = Harness::new("component_actions-node-profile");
    let n = node_fixture(&mut h);
    rebuild_node(&h.ctx(), &n.app, &ProfileOpt::Named("web".to_string())).expect("node profile");
    for dir in [&n.app, &n.api] {
        let name = session_name(dir);
        assert_eq!(
            windows(&h, &name),
            expected(&[("server", &["npm start"]), ("logs", &[])])
        );
        assert_eq!(h.backend.option(&name, OPT_PROFILE).as_deref(), Some("web"));
    }
    assert_eq!(
        windows(&h, &session_name(&n.lib)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
}

// FR-ACT-07: the session the user is in is not rebuilt by an alt-key action; the others under the node are
#[test]
fn node_rebuild_never_touches_the_session_the_user_is_in() {
    let mut h = Harness::new("component_actions-node-rebuild-current");
    let n = node_fixture(&mut h);
    be_inside(&mut h, &session_name(&n.app));
    rebuild_node(&h.ctx(), &n.app, &ProfileOpt::Unset).expect("node rebuild");
    assert_eq!(
        windows(&h, &session_name(&n.app)),
        expected(&[("editor", &["nvim"]), ("shell", &[])])
    );
    assert_eq!(
        windows(&h, &session_name(&n.api)),
        expected(&[("server", &["npm start"]), ("logs", &[])])
    );
}

// FR-ACT-06, FR-ACT-13: alt-x kills the managed sessions under the node; the one outside and unmanaged ones stay
#[test]
fn node_kill_removes_managed_sessions_under_the_node_only() {
    let mut h = Harness::new("component_actions-node-kill");
    let n = node_fixture(&mut h);
    let stray = format!("{}/stray", n.app.as_str());
    h.backend
        .create_session(&NewSession {
            name: "stray-0000".to_string(),
            dir: stray,
            window_name: "Terminal".to_string(),
        })
        .expect("unmanaged session");
    kill_node(&h.ctx(), &n.app).expect("node kill");
    assert!(!exists(&h, &session_name(&n.app)));
    assert!(!exists(&h, &session_name(&n.api)));
    assert!(exists(&h, &session_name(&n.lib)));
    assert!(exists(&h, "stray-0000"));
}

// FR-ACT-06, FR-ACT-07, P-63: alt-x on a node that holds the session the user is in kills it too, and the client
// goes to the most recent session outside the node
#[test]
fn node_kill_of_the_session_the_user_is_in_switches_to_the_one_outside() {
    let mut h = Harness::new("component_actions-node-kill-current");
    let n = node_fixture(&mut h);
    be_inside(&mut h, &session_name(&n.api));
    kill_node(&h.ctx(), &n.app).expect("node kill");
    assert!(!exists(&h, &session_name(&n.app)));
    assert!(!exists(&h, &session_name(&n.api)));
    assert!(exists(&h, &session_name(&n.lib)));
    assert_eq!(h.backend.current_session(), Ok(Some(session_name(&n.lib))));
}

// FR-ACT-10 (service part of alt-d): every bookmark under the node is deleted and the managed sessions of those
// bookmarks are killed; a bookmark outside the node and its session stay
#[test]
fn node_delete_removes_bookmarks_under_the_node_and_kills_their_sessions() {
    let mut h = Harness::new("component_actions-node-delete");
    let n = node_fixture(&mut h);
    let lib_child = format!("{}/x", n.lib.as_str());
    fs::create_dir_all(PathBuf::from(&lib_child)).expect("lib child");
    fs::create_dir_all(PathBuf::from(h.dirs.cache.as_str())).expect("cache folder");
    fs::write(
        bookmarks_file(&h),
        format!("{}\n{}\n{}\n", n.app.as_str(), n.api.as_str(), lib_child),
    )
    .expect("write bookmarks");
    delete_node_bookmarks(&h.ctx(), &n.app).expect("node delete");
    let text = fs::read_to_string(bookmarks_file(&h)).expect("bookmarks");
    assert_eq!(text.trim(), lib_child);
    assert!(!exists(&h, &session_name(&n.app)));
    assert!(!exists(&h, &session_name(&n.api)));
    assert!(exists(&h, &session_name(&n.lib)));
}

// FR-SESS-10: `ss kill` removes the managed session with the exact name and prints the FR-SESS-12 line
#[test]
fn ss_kill_by_exact_name_removes_the_managed_session() {
    let h = Harness::new("sess_kill_exact");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    let text = session_cmd::kill_target(&h.ctx(), &name);
    assert_eq!(text, Ok(Outcome::Info(format!("Killed session '{name}'."))));
    assert!(!exists(&h, &name));
}

// FR-SESS-10, A-20: a directory argument kills the managed session of that directory
#[test]
fn ss_kill_by_directory_removes_the_session_of_that_directory() {
    let h = Harness::new("sess_kill_dir");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    let typed = dir.as_str().to_string();
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), &typed),
        Ok(Outcome::Info(format!("Killed session '{name}'.")))
    );
    assert!(!exists(&h, &name));
}

// FR-SESS-10, A-20: a relative directory argument is cleaned against the working directory
#[test]
fn ss_kill_by_relative_directory_removes_the_session_of_that_directory() {
    let h = Harness::new("sess_kill_rel");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), "app"),
        Ok(Outcome::Info(format!("Killed session '{name}'.")))
    );
    assert!(!exists(&h, &name));
}

// FR-SESS-10, F-29: the exact managed name is tried first, so a session named like a directory is the one killed
#[test]
fn ss_kill_prefers_the_exact_name_over_a_directory_of_the_same_text() {
    let h = Harness::new("sess_kill_name_first");
    let dir = dir_of(&h, "app");
    let dir_name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    let other = AbsPath::clean(
        &format!("{}/elsewhere", h.root().display()),
        &AbsPath::root(),
    );
    h.backend
        .create_session(&NewSession {
            name: "app".to_string(),
            dir: other.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    h.backend
        .set_option("app", OPT_MANAGED, "1")
        .expect("mark managed");
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), "app"),
        Ok(Outcome::Info("Killed session 'app'.".to_string()))
    );
    assert!(!exists(&h, "app"));
    assert!(exists(&h, &dir_name));
}

// FR-SESS-11: a target that is neither a managed name nor the session of a directory
#[test]
fn ss_kill_of_an_unknown_target_is_not_a_ledge_session() {
    let h = Harness::new("sess_kill_unknown");
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), "nope"),
        Err(fatal("'nope' is not a ledge session."))
    );
}

// FR-SESS-11, FR-ACT-13: an unmanaged session is refused by name and left running
#[test]
fn ss_kill_of_an_unmanaged_session_is_refused_and_kept() {
    let h = Harness::new("sess_kill_unmanaged");
    h.backend
        .create_session(&NewSession {
            name: "plain".to_string(),
            dir: h.cwd.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), "plain"),
        Err(fatal("'plain' is not a ledge session."))
    );
    assert!(exists(&h, "plain"));
}

// FR-SESS-11: a directory whose session is not running is reported with the typed argument
#[test]
fn ss_kill_of_a_directory_without_a_session_is_not_a_ledge_session() {
    let h = Harness::new("sess_kill_dir_none");
    let dir = dir_of(&h, "app");
    let typed = dir.as_str().to_string();
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), &typed),
        Err(fatal(&format!("'{typed}' is not a ledge session.")))
    );
}

// FR-SESS-11, FR-ACT-13: the session of a directory that is unmanaged is refused and kept
#[test]
fn ss_kill_of_an_unmanaged_directory_session_is_refused_and_kept() {
    let h = Harness::new("sess_kill_dir_unmanaged");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    h.backend
        .create_session(&NewSession {
            name: name.clone(),
            dir: dir.as_str().to_string(),
            window_name: "Terminal".to_string(),
        })
        .expect("create");
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), "app"),
        Err(fatal("'app' is not a ledge session."))
    );
    assert!(exists(&h, &name));
}

// FR-SESS-10, FR-SESS-12, P-63: `ss kill` of the session the user is in switches to the most recent other one
#[test]
fn ss_kill_of_the_current_session_switches_to_the_most_recent_other() {
    let mut h = Harness::new("sess_kill_current");
    let app = session_name(&dir_of(&h, "app"));
    let api = session_name(&dir_of(&h, "api"));
    build_managed(&h, &dir_of(&h, "app"), &Profile::basic());
    build_managed(&h, &dir_of(&h, "api"), &Profile::basic());
    be_inside(&mut h, &app);
    assert_eq!(
        session_cmd::kill_target(&h.ctx(), &app),
        Ok(Outcome::Info(format!("Killed session '{app}'.")))
    );
    assert!(!exists(&h, &app));
    assert_eq!(h.backend.current_session(), Ok(Some(api)));
}

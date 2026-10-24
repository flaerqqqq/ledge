//! Component tests for the kill and rebuild services (`app::actions`) over `FakeBackend`, `FakeEnv`, `FakeConsole`
//! and `RealFs` on temp folders (docs/testing.md level 2): managed-only kill and rebuild (FR-ACT-13, D-14), the
//! rebuild from the profile (FR-ACT-02, FR-ACT-03, FR-PROF-08), the profile precedence at rebuild (FR-ACT-12,
//! FR-PROF-10, FR-PROF-11, FR-PROF-14, FR-PROF-15, D-04), the rebuild and kill of the session the user is in
//! (FR-ACT-05, FR-ACT-07, D-17, A-07).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::app::actions::{kill, rebuild};
use ledge::app::build::ensure_session;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::{Profile, ProfileName};
use ledge::domain::profile_rules::ProfileOpt;
use ledge::error::{LedgeError, Outcome};
use ledge::ports::backend::{NewSession, SessionBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE};
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

// FR-ACT-07, D-17: the session the user is in is refused for kill, with the D-17 text
#[test]
fn kill_refuses_the_session_the_user_is_in() {
    let mut h = Harness::new("act_kill_current");
    let dir = dir_of(&h, "app");
    let name = session_name(&dir);
    build_managed(&h, &dir, &Profile::basic());
    be_inside(&mut h, &name);
    assert_eq!(
        kill(&h.ctx(), &name),
        Err(fatal("cannot kill the session you are in."))
    );
    assert!(exists(&h, &name));
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

//! Component tests for `adapters::tmux::TmuxBackend` over `FakeProcess` (docs/testing.md level 2). They check the
//! exact argv of each operation (architecture 7.1), the `-L` rule, the error mapping (7.2), and the managed
//! filter (FR-NAME-04) on tmux output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::tmux::{TmuxBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE};
use ledge::domain::sessions::{managed, session_directory, tmux_version_supported};
use ledge::ports::backend::{BackendError, NewSession, SessionBackend};
use support::fake_env::FakeEnv;
use support::fake_process::FakeProcess;

/// The `list-sessions` format of architecture 7.1, with real tab characters.
const LIST_FORMAT: &str = "#{session_name}\t#{session_windows}\t#{session_attached}\t#{session_activity}\t#{@ledge_managed}\t#{@ledge_profile}\t#{@ledge_dir}\t#{session_path}";

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn with_server(list: &[&str]) -> Vec<String> {
    let mut argv = args(&["-L", "ledge-test"]);
    argv.extend(args(list));
    argv
}

// FR-PLAT-01, architecture 7.1: the version call takes no -L even when a server is named
#[test]
fn version_call_has_no_server_flag() {
    let process = FakeProcess::new().reply("-V", 0, "tmux 3.4\n", "");
    let backend = TmuxBackend::new(&process, Some("ledge-test".to_string()));
    assert_eq!(backend.version(), Ok((3, 4)));
    assert_eq!(process.calls()[0].args, args(&["-V"]));
    assert_eq!(process.calls()[0].program, "tmux");
}

// FR-PLAT-01, architecture 7.1: `tmux -V` output of a development build parses
#[test]
fn version_of_development_build_is_parsed() {
    let process = FakeProcess::new().reply("-V", 0, "tmux next-3.5\n", "");
    assert_eq!(TmuxBackend::new(&process, None).version(), Ok((3, 5)));
}

// FR-PLAT-01: 3.2 is the minimum (architecture 7.2)
#[test]
fn versions_below_3_2_are_not_supported() {
    assert!(tmux_version_supported((3, 2)));
    assert!(tmux_version_supported((3, 4)));
    assert!(!tmux_version_supported((3, 1)));
    assert!(!tmux_version_supported((2, 9)));
}

// FR-PLAT-01, architecture 7.1: -L is passed only when LEDGE_TMUX_SERVER names a server
#[test]
fn server_name_comes_from_environment_when_non_empty() {
    assert_eq!(TmuxBackend::server_from_env(&FakeEnv::new()), None);
    let empty = FakeEnv::new().with_var("LEDGE_TMUX_SERVER", "");
    assert_eq!(TmuxBackend::server_from_env(&empty), None);
    let set = FakeEnv::new().with_var("LEDGE_TMUX_SERVER", "ledge-test");
    assert_eq!(
        TmuxBackend::server_from_env(&set),
        Some("ledge-test".to_string())
    );
}

// FR-PICK-07, architecture 7.1: the list argv, with -L only when a server is named
#[test]
fn list_sessions_argv_matches_architecture() {
    let process = FakeProcess::new().reply("list-sessions", 0, "", "");
    TmuxBackend::new(&process, None)
        .list_sessions()
        .expect("list");
    assert_eq!(
        process.calls()[0].args,
        args(&["list-sessions", "-F", LIST_FORMAT])
    );

    let process = FakeProcess::new().reply("list-sessions", 0, "", "");
    TmuxBackend::new(&process, Some("ledge-test".to_string()))
        .list_sessions()
        .expect("list");
    assert_eq!(
        process.calls()[0].args,
        with_server(&["list-sessions", "-F", LIST_FORMAT])
    );
}

// FR-PICK-07: no server running is an empty list, so the markers are empty
#[test]
fn no_server_running_lists_nothing() {
    let stderr = "no server running on /tmp/tmux-1000/ledge-test";
    let process = FakeProcess::new().reply("list-sessions", 1, "", stderr);
    assert_eq!(
        TmuxBackend::new(&process, Some("ledge-test".to_string())).list_sessions(),
        Ok(Vec::new())
    );
}

// FR-PICK-07, architecture 7.2: other failures of list-sessions are errors
#[test]
fn other_list_failure_is_an_error_with_trimmed_stderr() {
    let process = FakeProcess::new().reply("list-sessions", 2, "", "  bad option  \n");
    assert_eq!(
        TmuxBackend::new(&process, None).list_sessions(),
        Err(BackendError::Failed {
            detail: "bad option".to_string()
        })
    );
}

// FR-NAME-04: only sessions with the managed marker survive the filter; the list order is kept
#[test]
fn only_managed_sessions_survive_the_filter() {
    let stdout = "app-1a2b\t1\t0\t0\t1\tdev\t/p/app\t/p/app\n\
                  scratch\t1\t0\t0\t\t\t\t/tmp\n\
                  lib-9f00\t2\t1\t0\t1\tbasic\t/p/lib\t/p/lib\n";
    let process = FakeProcess::new().reply("list-sessions", 0, stdout, "");
    let all = TmuxBackend::new(&process, None)
        .list_sessions()
        .expect("list");
    let kept: Vec<String> = managed(all).iter().map(|s| s.info().name.clone()).collect();
    assert_eq!(kept, vec!["app-1a2b".to_string(), "lib-9f00".to_string()]);
}

// A-37: @ledge_dir wins over session_path; session_path is the fallback
#[test]
fn session_directory_prefers_ledge_dir() {
    let stdout = "a\t1\t0\t0\t1\tdev\t/from/option\t/from/path\nb\t1\t0\t0\t1\t\t\t/from/path\n";
    let process = FakeProcess::new().reply("list-sessions", 0, stdout, "");
    let all = TmuxBackend::new(&process, None)
        .list_sessions()
        .expect("list");
    assert_eq!(session_directory(&all[0]), Some("/from/option"));
    assert_eq!(session_directory(&all[1]), Some("/from/path"));
}

// FR-NAME-03, architecture 7.1: create-session argv, and the printed window id is returned
#[test]
fn create_session_argv_and_first_window_id() {
    let process = FakeProcess::new().reply("new-session", 0, "@3\n", "");
    let new = NewSession {
        name: "app-1a2b".to_string(),
        dir: "/p/app".to_string(),
        window_name: "editor".to_string(),
    };
    let id = TmuxBackend::new(&process, Some("ledge-test".to_string()))
        .create_session(&new)
        .expect("create");
    assert_eq!(id, "@3");
    assert_eq!(
        process.calls()[0].args,
        with_server(&[
            "new-session",
            "-d",
            "-s",
            "app-1a2b",
            "-c",
            "/p/app",
            "-n",
            "editor",
            "-P",
            "-F",
            "#{window_id}",
        ])
    );
}

// FR-NAME-03, architecture 7.1: the three managed options are set on the exact session target
#[test]
fn set_option_argv_uses_exact_session_target() {
    let process = FakeProcess::new().reply("set-option", 0, "", "");
    let backend = TmuxBackend::new(&process, None);
    backend
        .set_option("app-1a2b", OPT_MANAGED, "1")
        .expect("managed");
    backend
        .set_option("app-1a2b", OPT_PROFILE, "dev")
        .expect("profile");
    backend
        .set_option("app-1a2b", OPT_DIR, "/p/app")
        .expect("dir");
    let calls = process.calls();
    assert_eq!(
        calls[0].args,
        args(&["set-option", "-t", "=app-1a2b:", "@ledge_managed", "1"])
    );
    assert_eq!(
        calls[1].args,
        args(&["set-option", "-t", "=app-1a2b:", "@ledge_profile", "dev"])
    );
    assert_eq!(
        calls[2].args,
        args(&["set-option", "-t", "=app-1a2b:", "@ledge_dir", "/p/app"])
    );
}

// FR-PROF-03: add-window argv, detached, with the window id returned
#[test]
fn add_window_argv_is_detached() {
    let process = FakeProcess::new().reply("new-window", 0, "@4\n", "");
    let id = TmuxBackend::new(&process, None)
        .add_window("app-1a2b", "server", "/p/app")
        .expect("add");
    assert_eq!(id, "@4");
    assert_eq!(
        process.calls()[0].args,
        args(&[
            "new-window",
            "-d",
            "-t",
            "=app-1a2b",
            "-n",
            "server",
            "-c",
            "/p/app",
            "-P",
            "-F",
            "#{window_id}",
        ])
    );
}

// FR-PROF-04: the command is typed literally, then Enter, both on the window id
#[test]
fn send_line_types_literal_text_then_enter() {
    let process = FakeProcess::new().reply("send-keys", 0, "", "");
    TmuxBackend::new(&process, None)
        .send_line("@4", "npm run dev")
        .expect("send");
    let calls = process.calls();
    assert_eq!(
        calls[0].args,
        args(&["send-keys", "-t", "@4", "-l", "--", "npm run dev"])
    );
    assert_eq!(calls[1].args, args(&["send-keys", "-t", "@4", "Enter"]));
}

// FR-PROF-05, architecture 7.1: select-window on the window id, switch-client and kill on the exact name
#[test]
fn select_switch_kill_and_rename_argv() {
    let process = FakeProcess::new();
    let backend = TmuxBackend::new(&process, None);
    backend.select_window("@3").expect("select");
    backend.switch_client("app-1a2b").expect("switch");
    backend.kill_session("app-1a2b").expect("kill");
    backend
        .rename_session("app-1a2b", "app-1a2b-rebuild")
        .expect("rename");
    let calls = process.calls();
    assert_eq!(calls[0].args, args(&["select-window", "-t", "@3"]));
    assert_eq!(calls[1].args, args(&["switch-client", "-t", "=app-1a2b"]));
    assert_eq!(calls[2].args, args(&["kill-session", "-t", "=app-1a2b"]));
    assert_eq!(
        calls[3].args,
        args(&["rename-session", "-t", "=app-1a2b", "app-1a2b-rebuild"])
    );
}

// FR-OPEN-08, architecture 7.1: attach inherits the terminal and names the exact session
#[test]
fn attach_inherits_terminal_with_exact_target() {
    let process = FakeProcess::new();
    TmuxBackend::new(&process, Some("ledge-test".to_string()))
        .attach("app-1a2b")
        .expect("attach");
    let call = &process.calls()[0];
    assert!(call.inherit);
    assert_eq!(
        call.args,
        with_server(&["attach-session", "-t", "=app-1a2b"])
    );
}

// FR-OPEN-08: attach that fails is a Failed error
#[test]
fn attach_failure_is_reported() {
    let process = FakeProcess::new().inherit_status(1);
    assert!(matches!(
        TmuxBackend::new(&process, None).attach("app-1a2b"),
        Err(BackendError::Failed { .. })
    ));
}

// FR-OPEN-08, architecture 7.1: the current session comes from display-message, empty is none
#[test]
fn current_session_reads_display_message() {
    let process = FakeProcess::new().reply("display-message", 0, "app-1a2b\n", "");
    assert_eq!(
        TmuxBackend::new(&process, None).current_session(),
        Ok(Some("app-1a2b".to_string()))
    );
    assert_eq!(
        process.calls()[0].args,
        args(&["display-message", "-p", "#{session_name}"])
    );
    let empty = FakeProcess::new().reply("display-message", 0, "\n", "");
    assert_eq!(TmuxBackend::new(&empty, None).current_session(), Ok(None));
}

// FR-PLAT-01, architecture 7.1: kill-server with -L when a server is named (tests only)
#[test]
fn kill_server_argv() {
    let process = FakeProcess::new();
    TmuxBackend::new(&process, Some("ledge-test".to_string()))
        .kill_server()
        .expect("kill");
    assert_eq!(process.calls()[0].args, with_server(&["kill-server"]));
}

// FR-CMD-10, architecture 7.2: a missing tmux binary is NotInstalled for every operation
#[test]
fn missing_binary_is_not_installed() {
    let process = FakeProcess::missing();
    let backend = TmuxBackend::new(&process, None);
    assert_eq!(backend.version(), Err(BackendError::NotInstalled));
    assert_eq!(backend.list_sessions(), Err(BackendError::NotInstalled));
    assert_eq!(
        backend.create_session(&NewSession {
            name: "app-1a2b".to_string(),
            dir: "/p/app".to_string(),
            window_name: "w".to_string(),
        }),
        Err(BackendError::NotInstalled)
    );
    assert_eq!(
        backend.kill_session("app-1a2b"),
        Err(BackendError::NotInstalled)
    );
}

// FR-PLAT-01: a failed create reports the trimmed stderr as the detail
#[test]
fn failed_create_reports_stderr_detail() {
    let process = FakeProcess::new().reply("new-session", 1, "", "duplicate session: app-1a2b\n");
    let new = NewSession {
        name: "app-1a2b".to_string(),
        dir: "/p/app".to_string(),
        window_name: "w".to_string(),
    };
    assert_eq!(
        TmuxBackend::new(&process, None).create_session(&new),
        Err(BackendError::Failed {
            detail: "duplicate session: app-1a2b".to_string()
        })
    );
}

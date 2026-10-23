//! `TmuxBackend`: the `SessionBackend` port over the tmux CLI (architecture 7.1 for every argv, 7.2 for the
//! error mapping, 7.3 for the managed marker). Every command goes through `ProcessRunner`.

use crate::domain::sessions::SessionInfo;
use crate::ports::backend::{BackendError, NewSession, SessionBackend};
use crate::ports::env::Env;
use crate::ports::process::{Captured, ProcessRunner};
use std::io;

/// FR-NAME-03, architecture 7.3: the option names live with the port so `app` can use them without the
/// adapter. Re-exported here for the tmux adapter tests.
pub use crate::ports::backend::{OPT_DIR, OPT_MANAGED, OPT_PROFILE};

/// Architecture 7.1: the `list-sessions` format, eight tab-separated fields in the order `parse_session_line`
/// reads them.
const LIST_FORMAT: &str = "#{session_name}\t#{session_windows}\t#{session_attached}\t#{session_activity}\t#{@ledge_managed}\t#{@ledge_profile}\t#{@ledge_dir}\t#{session_path}";

/// Architecture 7.2: `list-sessions` stderr texts that mean no server runs, which is an empty list.
const NO_SERVER_TEXTS: [&str; 3] = ["no server running", "error connecting to", "no sessions"];

/// FR-PLAT-01, architecture 7.1: the tmux backend. `server` is the `-L` socket name from `LEDGE_TMUX_SERVER`;
/// `None` uses the default server.
pub struct TmuxBackend<'a> {
    runner: &'a dyn ProcessRunner,
    server: Option<String>,
}

impl<'a> TmuxBackend<'a> {
    pub fn new(runner: &'a dyn ProcessRunner, server: Option<String>) -> Self {
        Self { runner, server }
    }

    /// Architecture 7.1: `LEDGE_TMUX_SERVER` when set and non-empty.
    pub fn server_from_env(env: &dyn Env) -> Option<String> {
        env.var("LEDGE_TMUX_SERVER").filter(|name| !name.is_empty())
    }

    /// `-L <server>` followed by the subcommand arguments (the version call passes no `-L`).
    fn argv(&self, args: &[&str]) -> Vec<String> {
        let mut argv = Vec::new();
        if let Some(server) = &self.server {
            argv.push("-L".to_string());
            argv.push(server.clone());
        }
        argv.extend(args.iter().map(|arg| arg.to_string()));
        argv
    }

    /// Runs tmux and returns its output whatever the status. A missing program is `NotInstalled`.
    fn run(&self, argv: Vec<String>) -> Result<Captured, BackendError> {
        self.runner.capture("tmux", &argv).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                BackendError::NotInstalled
            } else {
                BackendError::Failed {
                    detail: error.to_string(),
                }
            }
        })
    }

    /// Runs a tmux subcommand; a non-zero status is `Failed` with the trimmed stderr (architecture 7.2).
    /// Returns the trimmed stdout.
    fn run_checked(&self, args: &[&str]) -> Result<String, BackendError> {
        let captured = self.run(self.argv(args))?;
        checked(captured)
    }
}

/// Architecture 7.2: non-zero status is `Failed`; zero gives the trimmed stdout.
fn checked(captured: Captured) -> Result<String, BackendError> {
    if captured.status != 0 {
        return Err(BackendError::Failed {
            detail: captured.stderr.trim().to_string(),
        });
    }
    Ok(captured.stdout.trim().to_string())
}

/// FR-PLAT-01, architecture 7.1: parses `tmux -V` output (`tmux 3.4`, `tmux next-3.5`, `tmux 3.2a`) into
/// `(major, minor)`. `None` when no number pair is found.
pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    let token = text
        .split_whitespace()
        .last()?
        .trim_start_matches(|c: char| !c.is_ascii_digit());
    let (major, rest) = token.split_once('.')?;
    let minor: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    Some((major.parse().ok()?, minor.parse().ok()?))
}

/// FR-NAME-03..05, architecture 7.1: one `list-sessions` line in `LIST_FORMAT`. `None` when the line does not
/// have eight fields. Unset options come back empty and are read as `None`; the marker is managed when it is `1`.
pub fn parse_session_line(line: &str) -> Option<SessionInfo> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [name, windows, attached, activity, managed, profile, dir, path] = fields.as_slice() else {
        return None;
    };
    let non_empty = |text: &str| (!text.is_empty()).then(|| text.to_string());
    Some(SessionInfo {
        name: name.to_string(),
        windows: windows.parse().unwrap_or(0),
        attached: attached.parse::<u32>().unwrap_or(0) > 0,
        activity: activity.parse().unwrap_or(0),
        managed: *managed == "1",
        profile: non_empty(profile),
        dir: non_empty(dir),
        path: non_empty(path),
    })
}

impl SessionBackend for TmuxBackend<'_> {
    fn version(&self) -> Result<(u32, u32), BackendError> {
        // The version call takes no `-L`: the version belongs to the binary, not to a server.
        let text = checked(self.run(vec!["-V".to_string()])?)?;
        parse_version(&text).ok_or(BackendError::Failed { detail: text })
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>, BackendError> {
        let captured = self.run(self.argv(&["list-sessions", "-F", LIST_FORMAT]))?;
        if captured.status == 1
            && NO_SERVER_TEXTS
                .iter()
                .any(|text| captured.stderr.contains(text))
        {
            return Ok(Vec::new());
        }
        let stdout = checked(captured)?;
        Ok(stdout.lines().filter_map(parse_session_line).collect())
    }

    fn create_session(&self, new: &NewSession) -> Result<String, BackendError> {
        self.run_checked(&[
            "new-session",
            "-d",
            "-s",
            &new.name,
            "-c",
            &new.dir,
            "-n",
            &new.window_name,
            "-P",
            "-F",
            "#{window_id}",
        ])
    }

    fn set_option(&self, session: &str, option: &str, value: &str) -> Result<(), BackendError> {
        // FR-NAME-03: `set-option` rejects the bare `=name` target on tmux 3.5a ("no such session"); the
        // trailing colon keeps the exact-name match and names the session (proposal P-18).
        let target = format!("={session}:");
        self.run_checked(&["set-option", "-t", &target, option, value])
            .map(|_| ())
    }

    fn add_window(
        &self,
        session: &str,
        window_name: &str,
        dir: &str,
    ) -> Result<String, BackendError> {
        let target = format!("={session}");
        self.run_checked(&[
            "new-window",
            "-d",
            "-t",
            &target,
            "-n",
            window_name,
            "-c",
            dir,
            "-P",
            "-F",
            "#{window_id}",
        ])
    }

    fn send_line(&self, window: &str, command: &str) -> Result<(), BackendError> {
        self.run_checked(&["send-keys", "-t", window, "-l", "--", command])?;
        self.run_checked(&["send-keys", "-t", window, "Enter"])
            .map(|_| ())
    }

    fn select_window(&self, window: &str) -> Result<(), BackendError> {
        self.run_checked(&["select-window", "-t", window])
            .map(|_| ())
    }

    fn attach(&self, session: &str) -> Result<(), BackendError> {
        let target = format!("={session}");
        let status = self
            .runner
            .inherit("tmux", &self.argv(&["attach-session", "-t", &target]))
            .map_err(|error| BackendError::Failed {
                detail: error.to_string(),
            })?;
        exit_ok(status)
    }

    fn switch_client(&self, session: &str) -> Result<(), BackendError> {
        let target = format!("={session}");
        self.run_checked(&["switch-client", "-t", &target])
            .map(|_| ())
    }

    fn kill_session(&self, session: &str) -> Result<(), BackendError> {
        let target = format!("={session}");
        self.run_checked(&["kill-session", "-t", &target])
            .map(|_| ())
    }

    fn rename_session(&self, old: &str, new: &str) -> Result<(), BackendError> {
        let target = format!("={old}");
        self.run_checked(&["rename-session", "-t", &target, new])
            .map(|_| ())
    }

    fn current_session(&self) -> Result<Option<String>, BackendError> {
        let name = self.run_checked(&["display-message", "-p", "#{session_name}"])?;
        Ok((!name.is_empty()).then_some(name))
    }

    fn kill_server(&self) -> Result<(), BackendError> {
        self.run_checked(&["kill-server"]).map(|_| ())
    }
}

/// Architecture 7.1, `attach-session` inherits the terminal, so there is no stderr to report.
fn exit_ok(status: i32) -> Result<(), BackendError> {
    if status == 0 {
        Ok(())
    } else {
        Err(BackendError::Failed {
            detail: format!("tmux exited with status {status}"),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{parse_session_line, parse_version};

    // FR-PLAT-01: the version line of tmux releases, including the development form
    #[test]
    fn version_line_gives_major_and_minor() {
        assert_eq!(parse_version("tmux 3.4"), Some((3, 4)));
        assert_eq!(parse_version("tmux next-3.5"), Some((3, 5)));
        assert_eq!(parse_version("tmux 3.2a"), Some((3, 2)));
        assert_eq!(parse_version("tmux 2.9"), Some((2, 9)));
    }

    // FR-PLAT-01: text without a number pair is not a version
    #[test]
    fn text_without_version_gives_none() {
        assert_eq!(parse_version("tmux"), None);
        assert_eq!(parse_version(""), None);
    }

    // FR-NAME-03: a managed line with all three options set
    #[test]
    fn managed_line_reads_marker_profile_and_dir() {
        let line = "app-1a2b\t2\t1\t1700000000\t1\tdev\t/p/app\t/p/app";
        let info = parse_session_line(line).expect("eight fields");
        assert_eq!(info.name, "app-1a2b");
        assert_eq!(info.windows, 2);
        assert!(info.attached);
        assert_eq!(info.activity, 1_700_000_000);
        assert!(info.managed);
        assert_eq!(info.profile.as_deref(), Some("dev"));
        assert_eq!(info.dir.as_deref(), Some("/p/app"));
        assert_eq!(info.path.as_deref(), Some("/p/app"));
    }

    // FR-NAME-04: empty options read as unset and an unmarked session is not managed
    #[test]
    fn unset_options_are_unmanaged_and_none() {
        let line = "scratch\t1\t0\t0\t\t\t\t/tmp";
        let info = parse_session_line(line).expect("eight fields");
        assert!(!info.managed);
        assert!(!info.attached);
        assert_eq!(info.profile, None);
        assert_eq!(info.dir, None);
        assert_eq!(info.path.as_deref(), Some("/tmp"));
    }

    // FR-NAME-05: the marker is managed only when its value is 1
    #[test]
    fn marker_other_than_one_is_not_managed() {
        let line = "x\t1\t0\t0\t0\t\t\t";
        assert!(!parse_session_line(line).expect("eight fields").managed);
    }

    // FR-PICK-07: a line that does not have eight fields is skipped
    #[test]
    fn short_line_gives_none() {
        assert_eq!(parse_session_line("only\tthree\tfields"), None);
    }
}

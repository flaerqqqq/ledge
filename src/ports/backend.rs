//! Session backend port (architecture 4, 7.1, 7.2). Trait and data only; the real adapter is
//! `adapters::tmux::TmuxBackend`, the fake is `tests/support/fake_backend.rs`. Created by S-09.

pub use crate::domain::sessions::SessionInfo;

/// FR-PLAT-01, architecture 7.2: failures of the backend, mapped from the tmux CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// The program is not on `PATH` (FR-CMD-10; the caller prints the message).
    NotInstalled,
    /// Any other non-zero exit; `detail` is the trimmed stderr.
    Failed { detail: String },
}

/// FR-OPEN-07, architecture 4: the parameters of a new session. The first window is named `window_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSession {
    pub name: String,
    pub dir: String,
    pub window_name: String,
}

/// FR-PLAT-01, FR-OPEN-08, FR-OPEN-09, FR-NAME-03..05: sessions, windows and options of the tmux server.
pub trait SessionBackend {
    /// Installed tmux version as `(major, minor)`, parsed from `tmux -V`.
    fn version(&self) -> Result<(u32, u32), BackendError>;
    /// Every session on the server, in creation order. No server running gives an empty list.
    fn list_sessions(&self) -> Result<Vec<SessionInfo>, BackendError>;
    /// Creates a detached session and returns the id of its first window. Starts the server when none runs
    /// (FR-OPEN-09).
    fn create_session(&self, new: &NewSession) -> Result<String, BackendError>;
    /// FR-NAME-03: sets a session option (`@ledge_managed`, `@ledge_profile`, `@ledge_dir`).
    fn set_option(&self, session: &str, option: &str, value: &str) -> Result<(), BackendError>;
    /// FR-PROF-03: adds a detached window and returns its id.
    fn add_window(
        &self,
        session: &str,
        window_name: &str,
        dir: &str,
    ) -> Result<String, BackendError>;
    /// FR-PROF-04: types `command` literally into the window, then Enter.
    fn send_line(&self, window: &str, command: &str) -> Result<(), BackendError>;
    /// FR-PROF-05: makes the window current.
    fn select_window(&self, window: &str) -> Result<(), BackendError>;
    /// FR-OPEN-08: attaches the terminal to the session; returns when the client detaches.
    fn attach(&self, session: &str) -> Result<(), BackendError>;
    /// FR-OPEN-08, FR-SESS-04: switches the current client to the session.
    fn switch_client(&self, session: &str) -> Result<(), BackendError>;
    fn kill_session(&self, session: &str) -> Result<(), BackendError>;
    /// D-17: renames a session.
    fn rename_session(&self, old: &str, new: &str) -> Result<(), BackendError>;
    /// Name of the session the current client is in; `None` when there is none.
    fn current_session(&self) -> Result<Option<String>, BackendError>;
    /// Tests only: stops the server (architecture 4).
    fn kill_server(&self) -> Result<(), BackendError>;
}

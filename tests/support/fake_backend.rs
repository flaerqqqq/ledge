//! FakeBackend: in-memory `SessionBackend` (docs/testing.md "Rules for fakes"). It models what the code relies
//! on: the server exists when a session exists, `list_sessions` is in creation order, `kill_session` removes
//! the windows, options survive rename, and `create_session` on an existing name fails with `duplicate session`.
//! A scripted failure can be set for any operation by its trait method name.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::domain::sessions::SessionInfo;
use ledge::ports::backend::{BackendError, NewSession, SessionBackend};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Window {
    id: String,
    name: String,
    dir: String,
    typed: Vec<String>,
}

#[derive(Debug, Clone)]
struct Session {
    name: String,
    dir: String,
    windows: Vec<Window>,
    options: BTreeMap<String, String>,
    activity: i64,
    attached: bool,
}

#[derive(Default)]
pub struct FakeBackend {
    sessions: RefCell<Vec<Session>>,
    current: RefCell<Option<String>>,
    attached: RefCell<Option<String>>,
    selected: RefCell<Option<String>>,
    next_window: Cell<u32>,
    failures: RefCell<Vec<(String, String)>>,
    version: Cell<(u32, u32)>,
}

impl FakeBackend {
    pub fn new() -> Self {
        Self {
            version: Cell::new((3, 4)),
            ..Self::default()
        }
    }

    /// Scripted failure: the named operation (trait method name) fails with `detail`.
    pub fn fail_on(self, operation: &str, detail: &str) -> Self {
        self.failures
            .borrow_mut()
            .push((operation.to_string(), detail.to_string()));
        self
    }

    /// FR-SESS-03 (test setup): the last activity of a session, Unix seconds, as tmux `session_activity` reports it.
    pub fn set_activity(&self, session: &str, activity: i64) {
        let mut sessions = self.sessions.borrow_mut();
        if let Some(found) = sessions.iter_mut().find(|s| s.name == session) {
            found.activity = activity;
        }
    }

    /// FR-SESS-09 (test setup): marks a session as attached by a client, as `list-sessions` reports it.
    pub fn set_attached(&self, session: &str, attached: bool) {
        let mut sessions = self.sessions.borrow_mut();
        if let Some(found) = sessions.iter_mut().find(|s| s.name == session) {
            found.attached = attached;
        }
    }

    /// FR-SESS-02 (test setup): the session the current client is in, as `current_session` reports it.
    pub fn set_current(&self, session: &str) {
        *self.current.borrow_mut() = Some(session.to_string());
    }

    pub fn with_version(self, major: u32, minor: u32) -> Self {
        self.version.set((major, minor));
        self
    }

    /// Names of the windows of a session, in order (test inspection, not part of the port).
    pub fn window_names(&self, session: &str) -> Vec<String> {
        self.session(session)
            .map(|s| s.windows.iter().map(|w| w.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Lines typed into a window (test inspection).
    pub fn typed_in(&self, window: &str) -> Vec<String> {
        self.sessions
            .borrow()
            .iter()
            .flat_map(|s| s.windows.iter())
            .find(|w| w.id == window)
            .map(|w| w.typed.clone())
            .unwrap_or_default()
    }

    /// Option value of a session (test inspection).
    pub fn option(&self, session: &str, option: &str) -> Option<String> {
        self.session(session)
            .and_then(|s| s.options.get(option).cloned())
    }

    /// Name of the window made current by `select_window`, if any (test inspection).
    pub fn selected_window_name(&self) -> Option<String> {
        let selected = self.selected.borrow().clone()?;
        self.sessions
            .borrow()
            .iter()
            .flat_map(|s| s.windows.iter())
            .find(|w| w.id == selected)
            .map(|w| w.name.clone())
    }

    /// Windows of a session in order: name and the lines typed into each (test inspection).
    pub fn windows_with_typed(&self, session: &str) -> Vec<(String, Vec<String>)> {
        self.session(session)
            .map(|s| {
                s.windows
                    .iter()
                    .map(|w| (w.name.clone(), w.typed.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Directories the windows of a session were created in, in order (test inspection).
    pub fn window_dirs(&self, session: &str) -> Vec<String> {
        self.session(session)
            .map(|s| s.windows.iter().map(|w| w.dir.clone()).collect())
            .unwrap_or_default()
    }

    /// The session attached to by `attach`, if any (test inspection).
    pub fn attached_to(&self) -> Option<String> {
        self.attached.borrow().clone()
    }

    fn session(&self, name: &str) -> Option<Session> {
        self.sessions
            .borrow()
            .iter()
            .find(|s| s.name == name)
            .cloned()
    }

    fn check(&self, operation: &str) -> Result<(), BackendError> {
        match self
            .failures
            .borrow()
            .iter()
            .find(|(op, _)| op == operation)
        {
            Some((_, detail)) => Err(BackendError::Failed {
                detail: detail.clone(),
            }),
            None => Ok(()),
        }
    }

    fn new_window_id(&self) -> String {
        let next = self.next_window.get();
        self.next_window.set(next + 1);
        format!("@{next}")
    }

    fn with_session<T>(
        &self,
        name: &str,
        f: impl FnOnce(&mut Session) -> T,
    ) -> Result<T, BackendError> {
        let mut sessions = self.sessions.borrow_mut();
        match sessions.iter_mut().find(|s| s.name == name) {
            Some(session) => Ok(f(session)),
            None => Err(BackendError::Failed {
                detail: format!("can't find session: {name}"),
            }),
        }
    }
}

impl SessionBackend for FakeBackend {
    fn version(&self) -> Result<(u32, u32), BackendError> {
        self.check("version")?;
        Ok(self.version.get())
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>, BackendError> {
        self.check("list_sessions")?;
        Ok(self
            .sessions
            .borrow()
            .iter()
            .map(|s| SessionInfo {
                name: s.name.clone(),
                windows: s.windows.len() as u32,
                attached: s.attached,
                activity: s.activity,
                managed: s.options.get("@ledge_managed").map(String::as_str) == Some("1"),
                profile: s.options.get("@ledge_profile").cloned(),
                dir: s.options.get("@ledge_dir").cloned(),
                path: Some(s.dir.clone()),
            })
            .collect())
    }

    fn create_session(&self, new: &NewSession) -> Result<String, BackendError> {
        self.check("create_session")?;
        if self.session(&new.name).is_some() {
            return Err(BackendError::Failed {
                detail: "duplicate session".to_string(),
            });
        }
        let id = self.new_window_id();
        self.sessions.borrow_mut().push(Session {
            name: new.name.clone(),
            dir: new.dir.clone(),
            activity: 0,
            windows: vec![Window {
                id: id.clone(),
                name: new.window_name.clone(),
                dir: new.dir.clone(),
                typed: Vec::new(),
            }],
            options: BTreeMap::new(),
            attached: false,
        });
        Ok(id)
    }

    fn set_option(&self, session: &str, option: &str, value: &str) -> Result<(), BackendError> {
        self.check("set_option")?;
        self.with_session(session, |s| {
            s.options.insert(option.to_string(), value.to_string());
        })
    }

    fn add_window(
        &self,
        session: &str,
        window_name: &str,
        dir: &str,
    ) -> Result<String, BackendError> {
        self.check("add_window")?;
        let id = self.new_window_id();
        self.with_session(session, |s| {
            s.windows.push(Window {
                id: id.clone(),
                name: window_name.to_string(),
                dir: dir.to_string(),
                typed: Vec::new(),
            });
        })?;
        Ok(id)
    }

    fn send_line(&self, window: &str, command: &str) -> Result<(), BackendError> {
        self.check("send_line")?;
        let mut sessions = self.sessions.borrow_mut();
        for session in sessions.iter_mut() {
            if let Some(w) = session.windows.iter_mut().find(|w| w.id == window) {
                w.typed.push(command.to_string());
                return Ok(());
            }
        }
        Err(BackendError::Failed {
            detail: format!("can't find window: {window}"),
        })
    }

    fn select_window(&self, window: &str) -> Result<(), BackendError> {
        self.check("select_window")?;
        let known = self
            .sessions
            .borrow()
            .iter()
            .any(|s| s.windows.iter().any(|w| w.id == window));
        if known {
            *self.selected.borrow_mut() = Some(window.to_string());
            Ok(())
        } else {
            Err(BackendError::Failed {
                detail: format!("can't find window: {window}"),
            })
        }
    }

    fn attach(&self, session: &str) -> Result<(), BackendError> {
        self.check("attach")?;
        self.session(session).ok_or_else(|| BackendError::Failed {
            detail: format!("can't find session: {session}"),
        })?;
        *self.attached.borrow_mut() = Some(session.to_string());
        Ok(())
    }

    fn switch_client(&self, session: &str) -> Result<(), BackendError> {
        self.check("switch_client")?;
        self.session(session).ok_or_else(|| BackendError::Failed {
            detail: format!("can't find session: {session}"),
        })?;
        *self.current.borrow_mut() = Some(session.to_string());
        Ok(())
    }

    fn kill_session(&self, session: &str) -> Result<(), BackendError> {
        self.check("kill_session")?;
        let mut sessions = self.sessions.borrow_mut();
        let before = sessions.len();
        sessions.retain(|s| s.name != session);
        if sessions.len() == before {
            return Err(BackendError::Failed {
                detail: format!("can't find session: {session}"),
            });
        }
        Ok(())
    }

    fn rename_session(&self, old: &str, new: &str) -> Result<(), BackendError> {
        self.check("rename_session")?;
        if self.session(new).is_some() {
            return Err(BackendError::Failed {
                detail: "duplicate session".to_string(),
            });
        }
        self.with_session(old, |s| s.name = new.to_string())?;
        // tmux: a client attached to the session keeps it under its new name.
        for slot in [&self.current, &self.attached] {
            let mut slot = slot.borrow_mut();
            if slot.as_deref() == Some(old) {
                *slot = Some(new.to_string());
            }
        }
        Ok(())
    }

    fn current_session(&self) -> Result<Option<String>, BackendError> {
        self.check("current_session")?;
        Ok(self.current.borrow().clone())
    }

    fn kill_server(&self) -> Result<(), BackendError> {
        self.check("kill_server")?;
        self.sessions.borrow_mut().clear();
        Ok(())
    }
}

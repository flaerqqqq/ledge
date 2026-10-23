//! Building a session from a profile (architecture 6 and 7, FR-OPEN-07, FR-PROF-03..08, FR-NAME-03, D-14, A-07).
//! Uses only the `SessionBackend` port and the domain types.

use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::Profile;
use crate::error::{LedgeError, Result};
use crate::messages;
use crate::ports::backend::{
    BackendError, NewSession, SessionBackend, OPT_DIR, OPT_MANAGED, OPT_PROFILE,
};

/// What `ensure_session` did for the directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ensured {
    /// No session of the name existed; the profile was built.
    Created,
    /// A session of the name existed and was reused unchanged (the profile is not applied).
    Reused,
}

/// FR-OPEN-07, FR-PROF-08, D-14, A-05: the session of `dir` exists or is built from `profile`.
///
/// The session is created only when no session of the computed name exists. An existing managed session
/// whose `@ledge_dir` names another directory fails (`name already used by <dir>`). An existing managed
/// session without a recorded directory, and any unmanaged session of the name, is reused without change.
pub fn ensure_session(
    backend: &dyn SessionBackend,
    dir: &AbsPath,
    profile: &Profile,
) -> Result<Ensured> {
    let name = session_name(dir);
    let sessions = backend
        .list_sessions()
        .map_err(|error| backend_error(&name, error))?;
    if let Some(existing) = sessions.iter().find(|session| session.name == name) {
        if existing.managed {
            if let Some(recorded) = existing
                .dir
                .as_deref()
                .filter(|recorded| !recorded.is_empty())
            {
                if recorded != dir.as_str() {
                    return Err(LedgeError::Fatal(messages::session_name_used(
                        &name, recorded,
                    )));
                }
            }
        }
        return Ok(Ensured::Reused);
    }
    build_session(backend, &name, dir.as_str(), profile)?;
    Ok(Ensured::Created)
}

/// FR-OPEN-07, FR-PROF-03..07, FR-NAME-03: creates the session, marks it, types the commands and selects the
/// first window. A failure after `create_session` kills the partial session (A-07).
fn build_session(
    backend: &dyn SessionBackend,
    name: &str,
    dir: &str,
    profile: &Profile,
) -> Result<()> {
    let Some(first) = profile.windows.first() else {
        return Err(LedgeError::Fatal(messages::profile_invalid_shape(
            profile.name.as_str(),
        )));
    };
    let first_window = backend
        .create_session(&NewSession {
            name: name.to_string(),
            dir: dir.to_string(),
            window_name: first.name.clone(),
        })
        .map_err(|error| backend_error(name, error))?;
    if let Err(error) = build_windows(backend, name, dir, profile, &first_window) {
        // A-07: the partial session is removed; a failing kill does not hide the build error.
        let _ = backend.kill_session(name);
        return Err(backend_error(name, error));
    }
    Ok(())
}

/// FR-NAME-03 (options right after create, before windows are added), FR-PROF-03 (windows in order),
/// FR-PROF-04 (command typed then Enter), FR-PROF-05 (first window selected last).
fn build_windows(
    backend: &dyn SessionBackend,
    name: &str,
    dir: &str,
    profile: &Profile,
    first_window: &str,
) -> std::result::Result<(), BackendError> {
    backend.set_option(name, OPT_MANAGED, "1")?;
    backend.set_option(name, OPT_PROFILE, profile.name.as_str())?;
    backend.set_option(name, OPT_DIR, dir)?;
    if let Some(command) = profile
        .windows
        .first()
        .and_then(|window| window.command.as_deref())
    {
        backend.send_line(first_window, command)?;
    }
    for window in profile.windows.iter().skip(1) {
        let id = backend.add_window(name, &window.name, dir)?;
        if let Some(command) = window.command.as_deref() {
            backend.send_line(&id, command)?;
        }
    }
    backend.select_window(first_window)
}

/// FR-OPEN-07, architecture 7.2: a backend failure as the fatal text of the build.
fn backend_error(name: &str, error: BackendError) -> LedgeError {
    match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_create_session(name, &detail))
        }
    }
}

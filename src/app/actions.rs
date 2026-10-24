//! Kill and rebuild of managed sessions (architecture 2, 6.4, 7.2; FR-ACT-02, FR-ACT-03, FR-ACT-05, FR-ACT-07,
//! FR-ACT-12, FR-ACT-13, FR-PROF-14, FR-PROF-15, D-04, D-13, D-14, D-17, A-07). The pickers and the tree actions call
//! these services; the messages are the ones of `messages`. Only the ports in `Ctx` are used.

use crate::app::build::build_session;
use crate::app::open::{attach_error, inside_tmux};
use crate::app::pickers::backend_fatal;
use crate::app::{store, Ctx};
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::Profile;
use crate::domain::profile_rules::{rebuild_profile, ProfileOpt, RebuildProfile};
use crate::domain::sessions::SessionInfo;
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;

/// FR-ACT-07: true when an alt-key action must leave the session `name` alone because it is the session the user is
/// in. `current` is the session of the client, `None` outside tmux. The alt-key wiring is S-22.
pub fn never_touch_current(name: &str, current: Option<&str>) -> bool {
    current == Some(name)
}

/// FR-ACT-13, FR-ACT-07, D-14, D-17: kills the managed session `name`. A missing session is a no-op (the picker row
/// has nothing to kill). An unmanaged session is refused, and so is the session the user is in.
pub fn kill(ctx: &Ctx<'_>, name: &str) -> Result<Outcome> {
    let sessions = list_sessions(ctx)?;
    let Some(session) = find_session(&sessions, name) else {
        return Ok(Outcome::Done);
    };
    if !session.managed {
        return Err(LedgeError::Fatal(messages::not_a_ledge_session(name)));
    }
    if never_touch_current(name, current_session(ctx)?.as_deref()) {
        return Err(LedgeError::Fatal(
            messages::cannot_kill_current_session().to_string(),
        ));
    }
    ctx.backend
        .kill_session(name)
        .map_err(|error| kill_error(name, error))?;
    Ok(Outcome::Done)
}

/// FR-ACT-02, FR-ACT-03, FR-ACT-05, FR-ACT-12, FR-ACT-13, FR-PROF-14, FR-PROF-15, D-04, D-17, A-07: rebuilds the session
/// of `dir`. A missing session is built from the profile. A managed session is killed and built again. The session the
/// user is in is rebuilt as a copy that takes its name (`rebuild_current`). The profile is resolved before anything is
/// killed, so a bad `--profile` leaves the session running.
pub fn rebuild(ctx: &Ctx<'_>, dir: &AbsPath, option: &ProfileOpt) -> Result<Outcome> {
    let name = session_name(dir);
    let sessions = list_sessions(ctx)?;
    let existing = find_session(&sessions, &name);
    if let Some(session) = existing {
        if !session.managed {
            return Err(LedgeError::Fatal(messages::not_a_ledge_session(&name)));
        }
        if let Some(recorded) = session.dir.as_deref().filter(|d| !d.is_empty()) {
            if recorded != dir.as_str() {
                return Err(LedgeError::Fatal(messages::session_name_used(
                    &name, recorded,
                )));
            }
        }
    }
    let recorded = existing.and_then(|session| session.profile.as_deref());
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let profile = resolve_profile(ctx, option, recorded, config.default_profile.as_deref())?;
    if existing.is_some() {
        if never_touch_current(&name, current_session(ctx)?.as_deref()) {
            return rebuild_current(ctx, &name, dir, &profile);
        }
        ctx.backend
            .kill_session(&name)
            .map_err(|error| kill_error(&name, error))?;
    }
    build_session(ctx.backend, &name, dir.as_str(), &profile)?;
    Ok(Outcome::Done)
}

/// FR-ACT-05, D-17, A-07: builds `<name>-rebuild` from the profile, switches the client to it, kills the old session
/// and renames the copy to `name`. A failure before the switch leaves the original untouched; the copy is removed.
fn rebuild_current(ctx: &Ctx<'_>, name: &str, dir: &AbsPath, profile: &Profile) -> Result<Outcome> {
    let copy = format!("{name}-rebuild");
    let sessions = list_sessions(ctx)?;
    if find_session(&sessions, &copy).is_some() {
        return Err(LedgeError::Fatal(messages::could_not_create_session(
            &copy,
            "name already used",
        )));
    }
    build_session(ctx.backend, &copy, dir.as_str(), profile)?;
    if let Err(error) = ctx.backend.switch_client(&copy) {
        // A-07: the copy is removed; a failing kill does not hide the switch error.
        let _ = ctx.backend.kill_session(&copy);
        return Err(attach_error(&copy, error));
    }
    ctx.backend
        .kill_session(name)
        .map_err(|error| kill_error(name, error))?;
    ctx.backend
        .rename_session(&copy, name)
        .map_err(|error| rename_error(&copy, name, error))?;
    Ok(Outcome::Done)
}

/// FR-PROF-14, FR-PROF-15, FR-ACT-12, D-04, D-13: loads the profile a rebuild uses. A recorded profile that cannot be
/// loaded is the FR-PROF-15 warning and then the default profile applies. Each warning is printed once per rebuild.
fn resolve_profile(
    ctx: &Ctx<'_>,
    option: &ProfileOpt,
    recorded: Option<&str>,
    default_profile: Option<&str>,
) -> Result<Profile> {
    match rebuild_profile(option, recorded, default_profile) {
        RebuildProfile::Basic => Ok(Profile::basic()),
        RebuildProfile::Requested(name) => store::load_profile(ctx.fs, &ctx.dirs, &name),
        RebuildProfile::Recorded(name) => match store::load_profile(ctx.fs, &ctx.dirs, &name) {
            Ok(profile) => Ok(profile),
            Err(_) => {
                ctx.console
                    .err(&messages::warning(&messages::session_profile_missing(
                        &name,
                    )));
                Ok(default_or_basic(ctx, default_profile))
            }
        },
        RebuildProfile::Default(name) => Ok(default_or_basic(ctx, Some(&name))),
    }
}

/// FR-PROF-12, D-04: the default profile, or the single-window layout when it cannot be loaded (with a warning).
fn default_or_basic(ctx: &Ctx<'_>, default_profile: Option<&str>) -> Profile {
    let Some(name) = default_profile else {
        return Profile::basic();
    };
    match store::load_profile(ctx.fs, &ctx.dirs, name) {
        Ok(profile) => profile,
        Err(_) => {
            ctx.console
                .err(&messages::warning(&messages::default_profile_missing(name)));
            Profile::basic()
        }
    }
}

/// D-17: the session the client is in, read only inside tmux (`TMUX` set and not empty).
fn current_session(ctx: &Ctx<'_>) -> Result<Option<String>> {
    if !inside_tmux(ctx.env) {
        return Ok(None);
    }
    ctx.backend.current_session().map_err(backend_fatal)
}

fn list_sessions(ctx: &Ctx<'_>) -> Result<Vec<SessionInfo>> {
    ctx.backend.list_sessions().map_err(backend_fatal)
}

fn find_session<'s>(sessions: &'s [SessionInfo], name: &str) -> Option<&'s SessionInfo> {
    sessions.iter().find(|session| session.name == name)
}

/// Architecture 7.2: a failed kill. A missing tmux keeps its own message (FR-CMD-10).
fn kill_error(name: &str, error: BackendError) -> LedgeError {
    match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_kill_session(name, &detail))
        }
    }
}

/// D-17: a failed rename of the copy. A missing tmux keeps its own message (FR-CMD-10).
fn rename_error(old: &str, new: &str, error: BackendError) -> LedgeError {
    match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_rename_session(old, new, &detail))
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::never_touch_current;

    // FR-ACT-07: the session the user is in is never touched by an alt-key action
    #[test]
    fn current_session_is_protected() {
        assert!(never_touch_current("app-1a2b", Some("app-1a2b")));
    }

    // FR-ACT-07: another session is not protected
    #[test]
    fn other_session_is_not_protected() {
        assert!(!never_touch_current("app-1a2b", Some("lib-9f00")));
    }

    // FR-ACT-07: outside tmux there is no current session, so nothing is protected
    #[test]
    fn nothing_is_protected_outside_tmux() {
        assert!(!never_touch_current("app-1a2b", None));
    }
}

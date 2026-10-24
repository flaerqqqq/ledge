//! Kill and rebuild of managed sessions (architecture 2, 6.4, 7.2; FR-ACT-02, FR-ACT-03, FR-ACT-05, FR-ACT-07,
//! FR-ACT-12, FR-ACT-13, FR-PROF-14, FR-PROF-15, FR-CUST-02, D-04, D-13, D-14, D-17, A-07). The pickers and the tree actions call
//! these services; the messages are the ones of `messages`. Only the ports in `Ctx` are used.

use crate::app::bookmark_cmd::{read_bookmarks, write_bookmarks};
use crate::app::build::build_session;
use crate::app::open::{attach_error, inside_tmux};
use crate::app::pickers::backend_fatal;
use crate::app::{store, Ctx};
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::Profile;
use crate::domain::profile_rules::{rebuild_profile, ProfileOpt, RebuildProfile};
use crate::domain::sessions::{
    group_members, managed, most_recent_other, session_directory, ManagedSession, SessionInfo,
};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;

/// FR-ACT-07: true when an alt-key action must leave the session `name` alone because it is the session the user is
/// in. `current` is the session of the client, `None` outside tmux. The alt-key wiring is S-22.
pub fn never_touch_current(name: &str, current: Option<&str>) -> bool {
    current == Some(name)
}

/// FR-ACT-13, FR-ACT-07, FR-SESS-12, D-14, P-63: kills the managed session `name`. A missing session is a no-op (the
/// picker row has nothing to kill). An unmanaged session is refused. The session the user is in is killed too: the
/// client first switches to the most recent other tmux session (`kill_current`).
pub fn kill(ctx: &Ctx<'_>, name: &str) -> Result<Outcome> {
    let sessions = list_sessions(ctx)?;
    let Some(session) = find_session(&sessions, name) else {
        return Ok(Outcome::Done);
    };
    if !session.managed {
        return Err(LedgeError::Fatal(messages::not_a_ledge_session(name)));
    }
    if current_session(ctx)?.as_deref() == Some(name) {
        return kill_current(ctx, name, &sessions);
    }
    ctx.backend
        .kill_session(name)
        .map_err(|error| kill_error(name, error))?;
    Ok(Outcome::Done)
}

/// FR-SESS-12, FR-ACT-07, P-63: kills the session the client is in. When another tmux session remains (any kind, the
/// most recent first), the client switches to it and then `name` is killed; a failed switch keeps `name`. With no other
/// session the tmux server is killed, so tmux closes.
fn kill_current(ctx: &Ctx<'_>, name: &str, sessions: &[SessionInfo]) -> Result<Outcome> {
    match most_recent_other(sessions, name) {
        Some(next) => {
            ctx.backend
                .switch_client(&next.name)
                .map_err(|error| attach_error(&next.name, error))?;
            ctx.backend
                .kill_session(name)
                .map_err(|error| kill_error(name, error))?;
        }
        None => ctx
            .backend
            .kill_server()
            .map_err(|error| kill_error(name, error))?,
    }
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

/// FR-ACT-06, FR-ACT-08, FR-ACT-09, FR-ACT-07, A-38: rebuilds every managed session under `node` (the directory of the
/// tree row, FR-SCOPE-02). `option` is `Unset` for alt-r, so each session keeps the profile recorded on it; alt-p passes
/// the one profile chosen. The session the user is in is skipped. Every session is tried; the first failure is returned.
pub fn rebuild_node(ctx: &Ctx<'_>, node: &AbsPath, option: &ProfileOpt) -> Result<()> {
    let current = current_session(ctx)?;
    let mut first = None;
    for (name, dir) in node_sessions(ctx, node)? {
        if never_touch_current(&name, current.as_deref()) {
            continue;
        }
        if let Err(error) = rebuild(ctx, &dir, option) {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

/// FR-ACT-06, FR-ACT-07, P-63: kills every managed session under `node`. The session the user is in is killed last, so
/// the client switches to a session outside the node when one remains. Every session is tried; the first failure is
/// returned.
pub fn kill_node(ctx: &Ctx<'_>, node: &AbsPath) -> Result<()> {
    let current = current_session(ctx)?;
    let mut first = None;
    let mut current_in_node = false;
    for (name, _) in node_sessions(ctx, node)? {
        if never_touch_current(&name, current.as_deref()) {
            current_in_node = true;
            continue;
        }
        if let Err(error) = ctx
            .backend
            .kill_session(&name)
            .map_err(|error| kill_error(&name, error))
        {
            first.get_or_insert(error);
        }
    }
    if let Some(name) = current.as_deref().filter(|_| current_in_node) {
        if let Err(error) = kill(ctx, name) {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

/// FR-ACT-10 (alt-d), FR-ACT-07: deletes every bookmark under `node` and kills the managed sessions of those
/// directories, except the session the user is in. Nothing is written when no bookmark is under the node.
pub fn delete_node_bookmarks(ctx: &Ctx<'_>, node: &AbsPath) -> Result<()> {
    let mut bookmarks = read_bookmarks(ctx)?;
    let inside: Vec<AbsPath> = bookmarks
        .entries()
        .iter()
        .filter(|dir| dir.is_inside(node))
        .cloned()
        .collect();
    if inside.is_empty() {
        return Ok(());
    }
    for dir in &inside {
        bookmarks.remove(dir);
    }
    write_bookmarks(ctx, &bookmarks)?;
    let current = current_session(ctx)?;
    let live: Vec<String> = list_sessions(ctx)?
        .into_iter()
        .filter(|session| session.managed)
        .map(|session| session.name)
        .collect();
    let mut first = None;
    for dir in &inside {
        let name = session_name(dir);
        if !live.contains(&name) || never_touch_current(&name, current.as_deref()) {
            continue;
        }
        if let Err(error) = ctx
            .backend
            .kill_session(&name)
            .map_err(|error| kill_error(&name, error))
        {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

/// P-65, FR-CONF-07, FR-BM-13, P-67 FR-CUST-02: removes the custom project `dir`. Its bookmarks are deleted, its managed
/// sessions are killed (the session the user is in last, S-43), and the list entry goes last, so a failure before it
/// leaves the project listed. Returns `false` and changes nothing when `dir` is not a custom project.
pub fn remove_custom_project(ctx: &Ctx<'_>, dir: &AbsPath) -> Result<bool> {
    let mut custom = store::load_custom_projects(ctx.fs, &ctx.dirs).map_err(|error| {
        LedgeError::Fatal(messages::could_not_read_custom_projects(&error.to_string()))
    })?;
    if !custom.contains(dir) {
        return Ok(false);
    }
    delete_node_bookmarks(ctx, dir)?;
    kill_node(ctx, dir)?;
    custom.remove(dir);
    store::save_custom_projects(ctx.fs, &ctx.dirs, &custom).map_err(|error| {
        LedgeError::Fatal(messages::could_not_write_custom_projects(
            &error.to_string(),
        ))
    })?;
    Ok(true)
}

/// FR-SCOPE-02, A-37: the managed sessions whose directory is `node` or inside it, as (session name, directory). A
/// session whose name is not the name of its directory is left out: rebuild finds sessions by directory name (D-14).
fn node_sessions(ctx: &Ctx<'_>, node: &AbsPath) -> Result<Vec<(String, AbsPath)>> {
    let sessions: Vec<ManagedSession> = ctx
        .backend
        .list_sessions()
        .map(managed)
        .map_err(backend_fatal)?;
    let infos: Vec<&SessionInfo> = sessions.iter().map(ManagedSession::info).collect();
    let dirs: Vec<Option<AbsPath>> = infos
        .iter()
        .map(|info| session_directory(info).map(|text| AbsPath::clean(text, &ctx.cwd)))
        .collect();
    let members = group_members(node, &dirs)
        .into_iter()
        .filter_map(|index| {
            let info = infos.get(index)?;
            let dir = dirs.get(index)?.clone()?;
            (session_name(&dir) == info.name).then(|| (info.name.clone(), dir))
        })
        .collect();
    Ok(members)
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
pub(crate) fn current_session(ctx: &Ctx<'_>) -> Result<Option<String>> {
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

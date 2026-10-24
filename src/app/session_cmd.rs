//! Session commands (architecture 2): `ss list` (FR-SESS-09, A-34) and `ss kill NAME|DIR` (FR-SESS-10..13, A-20).
//! Only managed sessions are listed or killed (FR-NAME-04, FR-ACT-13). The kill itself is `app::actions::kill`, which
//! switches the client away from the session it is in first (FR-SESS-12, P-63). The table layout is here; the cell
//! widths come from `tui::columns`.

use crate::app::actions::kill;
use crate::app::pickers::backend_fatal;
use crate::app::{home_of, Ctx};
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::sessions::{managed, session_directory, ManagedSession, SessionInfo};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::tui::columns::width;

/// FR-SESS-09, A-34: the managed sessions as a table of name, windows, state and path, sorted by name, with a
/// header row. No managed session gives `No ledge sessions.` (info, exit 0).
pub fn list(ctx: &Ctx<'_>) -> Result<Outcome> {
    let sessions: Vec<ManagedSession> = ctx
        .backend
        .list_sessions()
        .map(managed)
        .map_err(backend_fatal)?;
    let mut infos: Vec<&SessionInfo> = sessions.iter().map(ManagedSession::info).collect();
    if infos.is_empty() {
        return Ok(Outcome::Info(messages::no_ledge_sessions().to_string()));
    }
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    let home = home_of(ctx);
    let rows: Vec<[String; 4]> = infos
        .iter()
        .map(|info| {
            [
                info.name.clone(),
                info.windows.to_string(),
                if info.attached {
                    messages::STATE_ATTACHED
                } else {
                    messages::STATE_DETACHED
                }
                .to_string(),
                // FR-HIST-03: a directory under HOME is shown with `~`.
                session_directory(info)
                    .map(|dir| AbsPath::clean(dir, &ctx.cwd).display_with_home(home.as_ref()))
                    .unwrap_or_default(),
            ]
        })
        .collect();
    Ok(Outcome::Info(table(&rows)))
}

/// FR-SESS-10, FR-SESS-11, FR-SESS-12, A-20: kills the managed session named exactly `target`, or else the managed
/// session of the directory `target` (cleaned against the working directory). Neither is a managed session gives
/// `'<target>' is not a ledge session.` (error). The current session is killed after the client switches (P-63).
pub fn kill_target(ctx: &Ctx<'_>, target: &str) -> Result<Outcome> {
    let sessions: Vec<ManagedSession> = ctx
        .backend
        .list_sessions()
        .map(managed)
        .map_err(backend_fatal)?;
    let Some(name) = resolve(ctx, &sessions, target) else {
        return Err(LedgeError::Fatal(messages::not_a_ledge_session(target)));
    };
    kill(ctx, &name)?;
    Ok(Outcome::Info(messages::killed_session(&name)))
}

/// F-29 (P-27 open item): the exact managed name first, then the session of the directory. `None` when neither is a
/// managed session.
fn resolve(ctx: &Ctx<'_>, sessions: &[ManagedSession], target: &str) -> Option<String> {
    let has = |name: &str| sessions.iter().any(|session| session.info().name == name);
    if has(target) {
        return Some(target.to_string());
    }
    let dir = AbsPath::clean(target, &ctx.cwd);
    let name = session_name(&dir);
    has(&name).then_some(name)
}

/// A-34: header row and rows, each column padded to its widest cell (display width) and separated by two spaces. The
/// last column is not padded.
fn table(rows: &[[String; 4]]) -> String {
    let header = [
        messages::HEADER_SESSION,
        messages::HEADER_WINDOWS,
        messages::HEADER_STATE,
        messages::HEADER_PATH,
    ]
    .map(str::to_string);
    let mut widths = [0usize; 3];
    for row in std::iter::once(&header).chain(rows.iter()) {
        for (slot, cell) in widths.iter_mut().zip(row.iter()) {
            *slot = (*slot).max(width(cell));
        }
    }
    std::iter::once(&header)
        .chain(rows.iter())
        .map(|row| line(row, &widths))
        .collect::<Vec<String>>()
        .join("\n")
}

fn line(cells: &[String; 4], widths: &[usize; 3]) -> String {
    let [session, windows, state, path] = cells;
    let [w_session, w_windows, w_state] = widths;
    format!(
        "{}  {}  {}  {}",
        pad(session, *w_session),
        pad(windows, *w_windows),
        pad(state, *w_state),
        path
    )
    .trim_end()
    .to_string()
}

fn pad(text: &str, to: usize) -> String {
    format!("{text}{}", " ".repeat(to.saturating_sub(width(text))))
}

//! Session records and group selection for session actions (FR-SCOPE-02, FR-NAME-04, A-37). Pure.
//! `SessionInfo` is the record of one `list-sessions` line; the port `ports::backend` re-exports it.

use super::path::AbsPath;

/// FR-NAME-03..05, architecture 7.1: one tmux session as listed by the backend. `managed` is the
/// `@ledge_managed` marker, `profile` and `dir` are the `@ledge_profile` and `@ledge_dir` options
/// (`None` when unset), `path` is tmux's `session_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub name: String,
    pub windows: u32,
    pub attached: bool,
    pub activity: i64,
    pub managed: bool,
    pub profile: Option<String>,
    pub dir: Option<String>,
    pub path: Option<String>,
}

/// FR-NAME-04: a session that carries the managed marker. Only this type reaches lists, pickers,
/// kill, labels and checks (architecture 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedSession(SessionInfo);

impl ManagedSession {
    pub fn info(&self) -> &SessionInfo {
        &self.0
    }
}

/// FR-NAME-04: keeps the sessions that carry the managed marker, in the order given.
pub fn managed(all: Vec<SessionInfo>) -> Vec<ManagedSession> {
    all.into_iter()
        .filter(|session| session.managed)
        .map(ManagedSession)
        .collect()
}

/// A-37: the session directory is `@ledge_dir` when set, else tmux's `session_path`. `None` when neither.
pub fn session_directory(session: &SessionInfo) -> Option<&str> {
    session
        .dir
        .as_deref()
        .or(session.path.as_deref())
        .filter(|dir| !dir.is_empty())
}

/// FR-SESS-02, A-10: the switcher order. The session the user is in (`current`, `None` outside tmux) comes first;
/// the rest by most recent activity; equal activity by session name.
pub fn switcher_order<'a>(
    sessions: &'a [ManagedSession],
    current: Option<&str>,
) -> Vec<&'a SessionInfo> {
    let mut infos: Vec<&SessionInfo> = sessions.iter().map(ManagedSession::info).collect();
    infos.sort_by(|a, b| {
        let a_current = current == Some(a.name.as_str());
        let b_current = current == Some(b.name.as_str());
        b_current
            .cmp(&a_current)
            .then(b.activity.cmp(&a.activity))
            .then(a.name.cmp(&b.name))
    });
    infos
}

/// FR-PLAT-01, architecture 7.2: tmux 3.2 is the minimum. `version` is `(major, minor)`.
pub fn tmux_version_supported(version: (u32, u32)) -> bool {
    version >= (3, 2)
}

/// FR-SCOPE-02: the indexes of the sessions whose directory is the group entry or lies inside it, by whole
/// components. A session with no directory (`None`) is in no group. The caller passes managed sessions only
/// (FR-NAME-04).
pub fn group_members(entry: &AbsPath, session_dirs: &[Option<AbsPath>]) -> Vec<usize> {
    session_dirs
        .iter()
        .enumerate()
        .filter(|(_, dir)| dir.as_ref().is_some_and(|d| d.is_inside(entry)))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{group_members, managed, switcher_order, ManagedSession, SessionInfo};
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    // FR-SCOPE-02: the group entry itself is a member
    #[test]
    fn session_in_the_entry_directory_is_a_member() {
        let dirs = vec![Some(abs("/p/app"))];
        assert_eq!(group_members(&abs("/p/app"), &dirs), vec![0]);
    }

    // FR-SCOPE-02: a session inside the entry is a member; one outside is not
    #[test]
    fn session_inside_the_entry_is_a_member_and_outside_is_not() {
        let dirs = vec![
            Some(abs("/p/app/src")),
            Some(abs("/p/lib")),
            Some(abs("/p/app")),
        ];
        assert_eq!(group_members(&abs("/p/app"), &dirs), vec![0, 2]);
    }

    // FR-SCOPE-02: whole components are compared, so /p/appx is not in the group /p/app
    #[test]
    fn name_prefix_is_not_membership() {
        let dirs = vec![Some(abs("/p/appx"))];
        assert!(group_members(&abs("/p/app"), &dirs).is_empty());
    }

    fn managed_session(name: &str, activity: i64) -> ManagedSession {
        let info = SessionInfo {
            name: name.to_string(),
            windows: 1,
            attached: false,
            activity,
            managed: true,
            profile: None,
            dir: None,
            path: None,
        };
        managed(vec![info]).remove(0)
    }

    // FR-SESS-02: the session the user is in comes first, even when another is more recent
    #[test]
    fn current_session_is_first_whatever_its_activity() {
        let sessions = vec![managed_session("a", 900), managed_session("b", 100)];
        let order: Vec<&str> = switcher_order(&sessions, Some("b"))
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(order, vec!["b", "a"]);
    }

    // FR-SESS-02: the rest by most recent activity
    #[test]
    fn rest_is_ordered_by_most_recent_activity() {
        let sessions = vec![
            managed_session("old", 100),
            managed_session("new", 900),
            managed_session("mid", 500),
        ];
        let order: Vec<&str> = switcher_order(&sessions, None)
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(order, vec!["new", "mid", "old"]);
    }

    // FR-SESS-02, A-10: equal activity is ordered by session name
    #[test]
    fn equal_activity_is_ordered_by_name() {
        let sessions = vec![managed_session("zeta", 500), managed_session("alpha", 500)];
        let order: Vec<&str> = switcher_order(&sessions, None)
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(order, vec!["alpha", "zeta"]);
    }

    // FR-SESS-02: outside tmux there is no current session, so the activity order alone applies
    #[test]
    fn outside_tmux_only_activity_orders() {
        let sessions = vec![managed_session("a", 1), managed_session("b", 2)];
        let order: Vec<&str> = switcher_order(&sessions, None)
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(order, vec!["b", "a"]);
    }

    // FR-SCOPE-02: a session with no directory is in no group
    #[test]
    fn session_without_directory_is_in_no_group() {
        let dirs = vec![None, Some(abs("/p/app"))];
        assert_eq!(group_members(&abs("/p/app"), &dirs), vec![1]);
    }
}

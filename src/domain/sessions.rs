//! Group selection for session actions (FR-SCOPE-02). Pure. S-04 covers only the directory step: the
//! managed filter and the session directory (`@ledge_dir`, then `session_path`, A-37) come with the
//! session slices.

use super::path::AbsPath;

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

    use super::group_members;
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

    // FR-SCOPE-02: a session with no directory is in no group
    #[test]
    fn session_without_directory_is_in_no_group() {
        let dirs = vec![None, Some(abs("/p/app"))];
        assert_eq!(group_members(&abs("/p/app"), &dirs), vec![1]);
    }
}

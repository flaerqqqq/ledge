//! Bookmark list: absolute directory paths in insertion order (architecture 5, 9.4), the add and remove rules and
//! the status of each row (FR-BM-02..10). Pure: no file access; the project list comes from the caller.

use super::path::{cache_lines_text, parse_cache_lines, AbsPath};
use super::projects::{project_containing, Project};

/// FR-BM-01: the bookmarked directories, in the order they were stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bookmarks {
    entries: Vec<AbsPath>,
}

/// FR-BM-08: the status word of a bookmark row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookmarkStatus {
    /// The directory exists and lies inside a project.
    Ok,
    /// The directory no longer exists.
    Missing,
    /// The directory exists but lies outside every project.
    OutsideProject,
}

/// FR-BM-02..04: why a directory cannot be bookmarked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// FR-BM-04: the directory is a project itself.
    IsProject,
    /// FR-BM-03: the directory is outside every project.
    OutsideProject,
}

impl Bookmarks {
    /// Reads the bookmarks file text. A-09 and D-06 rules are in `parse_cache_lines`; the file order is kept.
    pub fn parse(text: &str) -> Bookmarks {
        Bookmarks {
            entries: parse_cache_lines(text),
        }
    }

    /// The bookmarked directories in stored order.
    pub fn entries(&self) -> &[AbsPath] {
        &self.entries
    }

    /// FR-BM-01: the bookmarks file text, one path per line, in stored order.
    pub fn to_file_text(&self) -> String {
        cache_lines_text(&self.entries)
    }

    /// FR-BM-06: true when `dir` is already bookmarked.
    pub fn contains(&self, dir: &AbsPath) -> bool {
        self.entries.contains(dir)
    }

    /// FR-BM-01, FR-BM-05: appends `dir` after the stored entries. Returns `false` and changes nothing when the
    /// directory is already bookmarked (FR-BM-06).
    pub fn add(&mut self, dir: AbsPath) -> bool {
        if self.contains(&dir) {
            return false;
        }
        self.entries.push(dir);
        true
    }

    /// FR-BM-09: removes `dir`, keeping the order of the others. Returns `false` when it is not bookmarked
    /// (FR-BM-10).
    pub fn remove(&mut self, dir: &AbsPath) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry != dir);
        self.entries.len() != before
    }
}

/// FR-BM-02, FR-BM-03, FR-BM-04: a directory may be bookmarked only when it lies strictly inside a project. A
/// project itself is refused. "Inside" is by whole components (A-14), so `/p/apple` is not inside `/p/app`.
pub fn check_candidate(dir: &AbsPath, projects: &[Project]) -> Result<(), Refusal> {
    if projects.iter().any(|project| &project.path == dir) {
        return Err(Refusal::IsProject);
    }
    if project_containing(dir, projects).is_some() {
        Ok(())
    } else {
        Err(Refusal::OutsideProject)
    }
}

/// FR-BM-08: the status of a bookmark. A directory that is gone is `Missing` whatever its place; an existing one
/// is `OutsideProject` when no project contains it, and `Ok` otherwise. `exists` is the directory check of the caller.
pub fn status(path: &AbsPath, exists: bool, projects: &[Project]) -> BookmarkStatus {
    if !exists {
        BookmarkStatus::Missing
    } else if project_containing(path, projects).is_none() {
        BookmarkStatus::OutsideProject
    } else {
        BookmarkStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{check_candidate, status, BookmarkStatus, Bookmarks, Refusal};
    use crate::domain::path::AbsPath;
    use crate::domain::projects::Project;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn texts(bookmarks: &Bookmarks) -> Vec<&str> {
        bookmarks.entries().iter().map(AbsPath::as_str).collect()
    }

    /// A project `/p/app` under the search path `/p`.
    fn projects() -> Vec<Project> {
        vec![Project {
            path: abs("/p/app"),
            name: "app".to_string(),
        }]
    }

    // FR-BM-01: entries are read in the order they are stored, not sorted
    #[test]
    fn parse_keeps_stored_order() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a\n/p/m\n");
        assert_eq!(texts(&bookmarks), vec!["/p/z", "/p/a", "/p/m"]);
    }

    // FR-BM-01: one path per line, in stored order
    #[test]
    fn to_file_text_writes_one_path_per_line_in_stored_order() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a\n");
        assert_eq!(bookmarks.to_file_text(), "/p/z\n/p/a\n");
    }

    // FR-BM-01: text written by to_file_text reads back to the same list
    #[test]
    fn parse_of_file_text_round_trips() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a/\n");
        assert_eq!(Bookmarks::parse(&bookmarks.to_file_text()), bookmarks);
    }

    // A-09: relative lines are skipped
    #[test]
    fn parse_skips_relative_lines() {
        let bookmarks = Bookmarks::parse("app/src\n/p/a\n");
        assert_eq!(texts(&bookmarks), vec!["/p/a"]);
    }

    // D-06: duplicates keep the first occurrence, and the same path with a trailing slash is a duplicate
    #[test]
    fn parse_keeps_first_occurrence_of_duplicates() {
        let bookmarks = Bookmarks::parse("/p/b\n/p/a\n/p/b/\n");
        assert_eq!(texts(&bookmarks), vec!["/p/b", "/p/a"]);
    }

    // FR-BM-05: a new bookmark goes to the end of the stored order
    #[test]
    fn add_appends_in_insertion_order() {
        let mut bookmarks = Bookmarks::parse("/p/z\n");
        assert!(bookmarks.add(abs("/p/app/a")));
        assert!(bookmarks.add(abs("/p/app/b")));
        assert_eq!(texts(&bookmarks), vec!["/p/z", "/p/app/a", "/p/app/b"]);
    }

    // FR-BM-06: adding a bookmark that exists changes nothing and reports it
    #[test]
    fn add_of_existing_bookmark_changes_nothing() {
        let mut bookmarks = Bookmarks::parse("/p/app/a\n/p/app/b\n");
        assert!(!bookmarks.add(abs("/p/app/a/")));
        assert_eq!(texts(&bookmarks), vec!["/p/app/a", "/p/app/b"]);
    }

    // FR-BM-09: removing keeps the order of the others
    #[test]
    fn remove_keeps_order_of_the_rest() {
        let mut bookmarks = Bookmarks::parse("/p/app/a\n/p/app/b\n/p/app/c\n");
        assert!(bookmarks.remove(&abs("/p/app/b")));
        assert_eq!(texts(&bookmarks), vec!["/p/app/a", "/p/app/c"]);
    }

    // FR-BM-10: removing a directory that is not bookmarked reports it and changes nothing
    #[test]
    fn remove_of_unknown_bookmark_reports_false() {
        let mut bookmarks = Bookmarks::parse("/p/app/a\n");
        assert!(!bookmarks.remove(&abs("/p/app/x")));
        assert_eq!(texts(&bookmarks), vec!["/p/app/a"]);
    }

    // FR-BM-02: a directory strictly inside a project is accepted
    #[test]
    fn directory_inside_project_is_accepted() {
        assert_eq!(check_candidate(&abs("/p/app/src"), &projects()), Ok(()));
    }

    // FR-BM-04: a project itself is refused as a project
    #[test]
    fn project_itself_is_refused_as_project() {
        assert_eq!(
            check_candidate(&abs("/p/app/"), &projects()),
            Err(Refusal::IsProject)
        );
    }

    // FR-BM-03: a directory outside every project is refused
    #[test]
    fn directory_outside_projects_is_refused() {
        assert_eq!(
            check_candidate(&abs("/p/other/src"), &projects()),
            Err(Refusal::OutsideProject)
        );
        assert_eq!(
            check_candidate(&abs("/p"), &projects()),
            Err(Refusal::OutsideProject)
        );
    }

    // A-14: "inside" is by whole components, so a name that starts with the project name is outside it
    #[test]
    fn prefix_of_project_name_is_outside() {
        assert_eq!(
            check_candidate(&abs("/p/apple/src"), &projects()),
            Err(Refusal::OutsideProject)
        );
    }

    // FR-BM-08: an existing directory inside a project is "ok"
    #[test]
    fn existing_directory_inside_project_is_ok() {
        assert_eq!(
            status(&abs("/p/app/src"), true, &projects()),
            BookmarkStatus::Ok
        );
    }

    // FR-BM-08: a directory that no longer exists is "missing"
    #[test]
    fn gone_directory_is_missing() {
        assert_eq!(
            status(&abs("/p/app/src"), false, &projects()),
            BookmarkStatus::Missing
        );
    }

    // FR-BM-08: an existing directory outside every project is "outside project"
    #[test]
    fn existing_directory_outside_projects_is_outside_project() {
        assert_eq!(
            status(&abs("/q/x"), true, &projects()),
            BookmarkStatus::OutsideProject
        );
    }

    // FR-BM-08: a missing directory is reported as missing even when it is outside every project
    #[test]
    fn missing_takes_precedence_over_outside_project() {
        assert_eq!(
            status(&abs("/q/x"), false, &projects()),
            BookmarkStatus::Missing
        );
    }
}

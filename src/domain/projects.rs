//! Project listing and the current-directory row of the project picker (architecture 5, A-16).
//! Pure: the child names come from the caller (`app::load_projects`); no file system access.

use super::path::AbsPath;

/// architecture 5: a directory directly under a search path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub path: AbsPath,
    pub name: String,
}

/// FR-PICK-04, FR-PICK-05, A-16: one row of the project picker. `project` is false for a current
/// directory that is not a project; the picker shows its path instead of a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub path: AbsPath,
    pub project: bool,
    pub current: bool,
}

/// FR-PICK-01, FR-PICK-02, FR-PICK-03, FR-CONF-07: the projects of each search path, from the names of its
/// child directories. A name starting with "." is skipped. A path that two search paths give keeps its first
/// occurrence. The result is sorted by full path.
pub fn project_list(listings: &[(AbsPath, Vec<String>)]) -> Vec<Project> {
    let mut projects: Vec<Project> = Vec::new();
    for (base, names) in listings {
        for name in names {
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            let path = AbsPath::clean(name, base);
            if projects.iter().any(|known| known.path == path) {
                continue;
            }
            projects.push(Project {
                path,
                name: name.clone(),
            });
        }
    }
    projects.sort_by(|a, b| a.path.cmp(&b.path));
    projects
}

/// FR-PICK-04, FR-PICK-05, A-16: the rows of the project picker. The current directory is the first row,
/// and the picker preselects row 0. When it is a project it is listed once, first, and not again in sort
/// order. When it is not a project (also a directory inside a project), an extra first row shows its path.
/// The other rows keep the sort order of `projects`.
pub fn project_rows(cwd: &AbsPath, projects: &[Project]) -> Vec<ProjectRow> {
    let is_project = projects.iter().any(|p| &p.path == cwd);
    let mut rows = vec![ProjectRow {
        path: cwd.clone(),
        project: is_project,
        current: true,
    }];
    rows.extend(
        projects
            .iter()
            .filter(|p| &p.path != cwd)
            .map(|p| ProjectRow {
                path: p.path.clone(),
                project: true,
                current: false,
            }),
    );
    rows
}

/// FR-SCOPE-01: the project that contains `dir` (equal to it, or inside it by whole components). When
/// projects nest (overlapping search paths) the deepest one is the group.
pub fn project_containing<'a>(dir: &AbsPath, projects: &'a [Project]) -> Option<&'a Project> {
    projects
        .iter()
        .filter(|p| dir.is_inside(&p.path))
        .max_by_key(|p| p.path.as_str().len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{project_containing, project_list, project_rows, Project, ProjectRow};
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn names(text: &[&str]) -> Vec<String> {
        text.iter().map(|s| s.to_string()).collect()
    }

    fn project(path: &str) -> Project {
        let path = abs(path);
        let name = path.as_str().rsplit('/').next().unwrap_or("").to_string();
        Project { path, name }
    }

    fn paths(projects: &[Project]) -> Vec<&str> {
        projects.iter().map(|p| p.path.as_str()).collect()
    }

    // FR-PICK-01: every child directory of a search path is a project
    #[test]
    fn every_child_name_becomes_a_project_under_its_search_path() {
        let listings = vec![(abs("/home/u/projects"), names(&["app", "lib"]))];
        let projects = project_list(&listings);
        assert_eq!(
            paths(&projects),
            vec!["/home/u/projects/app", "/home/u/projects/lib"]
        );
        assert_eq!(projects[0].name, "app");
    }

    // FR-PICK-02: names starting with a dot are hidden
    #[test]
    fn dot_named_children_are_not_projects() {
        let listings = vec![(abs("/p"), names(&[".git", "app", ".cache"]))];
        assert_eq!(paths(&project_list(&listings)), vec!["/p/app"]);
    }

    // FR-PICK-03: projects are sorted by their full path, across search paths
    #[test]
    fn projects_are_sorted_by_full_path_across_search_paths() {
        let listings = vec![
            (abs("/z"), names(&["a"])),
            (abs("/a"), names(&["zeta", "beta"])),
        ];
        assert_eq!(
            paths(&project_list(&listings)),
            vec!["/a/beta", "/a/zeta", "/z/a"]
        );
    }

    // FR-PICK-03: sorting is by the full path, not by the basename
    #[test]
    fn sort_uses_full_path_not_basename() {
        let listings = vec![(abs("/p"), names(&["b", "a"])), (abs("/0"), names(&["z"]))];
        assert_eq!(
            paths(&project_list(&listings)),
            vec!["/0/z", "/p/a", "/p/b"]
        );
    }

    // FR-PICK-01: a path given by two search paths is listed once, first occurrence kept
    #[test]
    fn same_path_from_two_search_paths_is_listed_once() {
        let listings = vec![(abs("/p"), names(&["app"])), (abs("/p/"), names(&["app"]))];
        assert_eq!(paths(&project_list(&listings)), vec!["/p/app"]);
    }

    // FR-CONF-07: a directory is a project only as a direct child of a search path (the listing holds
    // child names only, so a nested name is never produced from the search path itself)
    #[test]
    fn only_names_of_the_search_path_itself_are_projects() {
        let listings = vec![(abs("/p"), names(&["app"]))];
        assert_eq!(paths(&project_list(&listings)), vec!["/p/app"]);
    }

    // FR-PICK-04: the current directory, when it is a project, is the first row and listed once
    #[test]
    fn current_project_is_first_and_not_repeated() {
        let projects = project_list(&[(abs("/p"), names(&["a", "b", "c"]))]);
        let rows = project_rows(&abs("/p/b"), &projects);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0],
            ProjectRow {
                path: abs("/p/b"),
                project: true,
                current: true
            }
        );
        let rest: Vec<&str> = rows[1..].iter().map(|r| r.path.as_str()).collect();
        assert_eq!(rest, vec!["/p/a", "/p/c"]);
        assert!(rows[1..].iter().all(|r| !r.current));
    }

    // FR-PICK-04: a current project that is already first in sort order is still listed once
    #[test]
    fn current_project_first_in_sort_order_is_listed_once() {
        let projects = project_list(&[(abs("/p"), names(&["a", "b"]))]);
        let rows = project_rows(&abs("/p/a"), &projects);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].current && rows[0].project);
        assert_eq!(rows[1].path.as_str(), "/p/b");
    }

    // FR-PICK-05: a current directory that is not a project is an extra first row showing its path
    #[test]
    fn non_project_current_directory_is_an_extra_first_row() {
        let projects = project_list(&[(abs("/p"), names(&["a"]))]);
        let rows = project_rows(&abs("/home/u"), &projects);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            ProjectRow {
                path: abs("/home/u"),
                project: false,
                current: true
            }
        );
        assert_eq!(rows[1].path.as_str(), "/p/a");
        assert!(rows[1].project && !rows[1].current);
    }

    // A-16: a current directory inside a project (not the project itself) is a non-project row
    #[test]
    fn directory_inside_a_project_is_a_non_project_current_row() {
        let projects = project_list(&[(abs("/p"), names(&["app"]))]);
        let rows = project_rows(&abs("/p/app/src"), &projects);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path.as_str(), "/p/app/src");
        assert!(!rows[0].project && rows[0].current);
        assert_eq!(rows[1].path.as_str(), "/p/app");
    }

    // A-16: with no projects, the current directory alone is the picker content
    #[test]
    fn no_projects_gives_only_the_current_directory_row() {
        let rows = project_rows(&abs("/p"), &[]);
        assert_eq!(
            rows,
            vec![ProjectRow {
                path: abs("/p"),
                project: false,
                current: true
            }]
        );
    }

    // FR-SCOPE-01: a directory belongs to the project it equals or lies inside by whole components
    #[test]
    fn project_containing_matches_equal_and_inside_by_components() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        assert_eq!(
            project_containing(&abs("/p/app"), &projects).map(|p| p.path.as_str()),
            Some("/p/app")
        );
        assert_eq!(
            project_containing(&abs("/p/app/src/main"), &projects).map(|p| p.path.as_str()),
            Some("/p/app")
        );
    }

    // FR-SCOPE-01: a name prefix is not containment
    #[test]
    fn project_containing_rejects_name_prefix() {
        let projects = vec![project("/p/app")];
        assert!(project_containing(&abs("/p/appx/y"), &projects).is_none());
        assert!(project_containing(&abs("/p/ap"), &projects).is_none());
    }

    // FR-SCOPE-01: a directory outside every project belongs to none
    #[test]
    fn project_containing_is_none_outside_every_project() {
        let projects = vec![project("/p/app")];
        assert!(project_containing(&abs("/home/u"), &projects).is_none());
    }

    // FR-SCOPE-01: with nested projects the deepest containing project is the group
    #[test]
    fn project_containing_prefers_the_deepest_project() {
        let projects = vec![project("/p"), project("/p/app")];
        assert_eq!(
            project_containing(&abs("/p/app/src"), &projects).map(|p| p.path.as_str()),
            Some("/p/app")
        );
    }
}

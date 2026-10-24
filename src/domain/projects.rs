//! Project listing and the current-directory row of the project picker (architecture 5, A-16).
//! Pure: the child names come from the caller (`app::load_projects`); no file system access.

use std::collections::BTreeMap;

use super::custom_projects::CustomProjects;
use super::path::AbsPath;

/// architecture 5: a directory directly under a search path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub path: AbsPath,
    /// The folder name (the last component of `path`).
    pub name: String,
    /// P-65: a custom project (opened from the command line, outside every search path) rather than a child of a
    /// search path.
    pub custom: bool,
    /// P-66: the name the project shows as. The folder name, or the last few path components when another project
    /// has the same folder name (`projects/api`, `work/api`). Set by `merge_projects`; `Project::new` gives the name.
    pub label: String,
}

impl Project {
    /// A project whose label is its name. `merge_projects` gives the labels of a list (P-66).
    pub fn new(path: AbsPath, name: String, custom: bool) -> Self {
        Self {
            path,
            label: name.clone(),
            name,
            custom,
        }
    }
}

/// FR-PICK-04, FR-PICK-05, A-16: one row of the project picker. `project` is false for a current
/// directory that is not a project; the picker shows its path instead of a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub path: AbsPath,
    pub project: bool,
    pub current: bool,
    /// P-65: the row is a custom project (the picker marks it).
    pub custom: bool,
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
            projects.push(Project::new(path, name.clone(), false));
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
    let cwd_project = projects.iter().find(|p| &p.path == cwd);
    let mut rows = vec![ProjectRow {
        path: cwd.clone(),
        project: cwd_project.is_some(),
        current: true,
        custom: cwd_project.is_some_and(|p| p.custom),
    }];
    rows.extend(
        projects
            .iter()
            .filter(|p| &p.path != cwd)
            .map(|p| ProjectRow {
                path: p.path.clone(),
                project: true,
                current: false,
                custom: p.custom,
            }),
    );
    rows
}

/// P-65, FR-PICK-01, FR-CONF-07: the search-path projects `listed` followed by the custom projects, sorted by full path.
/// A custom path that `listed` already holds is dropped, so the listed project wins. The name of a custom project is
/// its last component.
pub fn merge_projects(listed: &[Project], custom: &CustomProjects) -> Vec<Project> {
    let mut projects: Vec<Project> = listed.to_vec();
    for path in custom.entries() {
        if projects.iter().any(|known| &known.path == path) {
            continue;
        }
        projects.push(Project::new(path.clone(), basename_of(path), true));
    }
    projects.sort_by(|a, b| a.path.cmp(&b.path));
    with_unique_labels(&mut projects);
    projects
}

/// P-66, FR-PICK-06: the labels of a list. A project whose name no other project has keeps the name. Projects that
/// share a name get the last `k + 1` path components joined with `/` (`work/api`), with `k` raised until the labels
/// of that group are unique. Paths are unique, so the full paths always are.
fn with_unique_labels(projects: &mut [Project]) {
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, project) in projects.iter().enumerate() {
        groups.entry(project.name.as_str()).or_default().push(index);
    }
    let mut labelled: Vec<(usize, String)> = Vec::new();
    for indexes in groups.values().filter(|indexes| indexes.len() > 1) {
        let parts: Vec<Vec<&str>> = indexes
            .iter()
            .map(|&index| {
                projects[index]
                    .path
                    .as_str()
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .collect()
            })
            .collect();
        let longest = parts.iter().map(Vec::len).max().unwrap_or(0);
        let mut components = 2;
        let mut labels = labels_of_last(&parts, components);
        while components < longest && !all_distinct(&labels) {
            components += 1;
            labels = labels_of_last(&parts, components);
        }
        labelled.extend(indexes.iter().copied().zip(labels));
    }
    for (index, label) in labelled {
        projects[index].label = label;
    }
}

/// P-66: the last `count` components of each path, joined with `/`.
fn labels_of_last(parts: &[Vec<&str>], count: usize) -> Vec<String> {
    parts
        .iter()
        .map(|part| part[part.len().saturating_sub(count)..].join("/"))
        .collect()
}

/// P-66: whether no two of `labels` are equal.
fn all_distinct(labels: &[String]) -> bool {
    let mut sorted: Vec<&String> = labels.iter().collect();
    sorted.sort_unstable();
    sorted.windows(2).all(|pair| pair[0] != pair[1])
}

/// P-65: a directory is recorded as a custom project only when no project contains it. `projects` is the search-path
/// projects plus the custom list, so the check covers "is a project", "is inside a project" and "is already listed".
pub fn should_record(dir: &AbsPath, projects: &[Project]) -> bool {
    project_containing(dir, projects).is_none()
}

/// The last component of a cleaned absolute path (the name of a custom project).
fn basename_of(path: &AbsPath) -> String {
    path.as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
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

    use super::{
        merge_projects, project_containing, project_list, project_rows, should_record, Project,
        ProjectRow,
    };
    use crate::domain::custom_projects::CustomProjects;
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
        Project::new(path, name, false)
    }

    fn labels(projects: &[Project]) -> Vec<&str> {
        projects.iter().map(|p| p.label.as_str()).collect()
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
                current: true,
                custom: false,
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
                current: true,
                custom: false,
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
                current: true,
                custom: false,
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

    // P-65: a custom project is a project named by its last component, sorted by full path with the others
    #[test]
    fn merge_adds_custom_projects_sorted_with_the_listed_ones() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        let custom = CustomProjects::parse("/s/scratch\n");
        let merged = merge_projects(&listed, &custom);
        assert_eq!(paths(&merged), vec!["/p/app", "/s/scratch"]);
        assert_eq!(merged[1].name, "scratch");
        assert!(merged[1].custom);
        assert!(!merged[0].custom);
    }

    // P-65: a custom path that a search path already lists is not added twice; the listed project wins
    #[test]
    fn merge_drops_custom_path_that_is_already_listed() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        let custom = CustomProjects::parse("/p/app\n");
        let merged = merge_projects(&listed, &custom);
        assert_eq!(paths(&merged), vec!["/p/app"]);
        assert!(!merged[0].custom);
    }

    // P-65: the custom list has no duplicates to merge, and an empty list leaves the listed projects alone
    #[test]
    fn merge_with_empty_custom_list_is_the_listed_projects() {
        let listed = project_list(&[(abs("/p"), names(&["a", "b"]))]);
        assert_eq!(merge_projects(&listed, &CustomProjects::default()), listed);
    }

    // P-66: a project's label is its folder name until merge_projects gives it one
    #[test]
    fn project_new_sets_the_label_to_the_name() {
        let project = Project::new(abs("/p/app"), "app".to_string(), false);
        assert_eq!(project.label, "app");
        assert_eq!(project.name, "app");
        assert!(!project.custom);
    }

    // P-66: unique names keep their plain name as the label
    #[test]
    fn unique_names_keep_the_plain_name_as_label() {
        let listed = project_list(&[(abs("/p"), names(&["app", "lib"]))]);
        let merged = merge_projects(&listed, &CustomProjects::default());
        assert_eq!(labels(&merged), vec!["app", "lib"]);
    }

    // P-66: two projects with the same name each gain the parent folder: projects/api and work/api
    #[test]
    fn two_same_names_gain_one_parent_folder() {
        let listed = project_list(&[
            (abs("/home/u/projects"), names(&["api"])),
            (abs("/home/u/work"), names(&["api"])),
        ]);
        let merged = merge_projects(&listed, &CustomProjects::default());
        assert_eq!(
            paths(&merged),
            vec!["/home/u/projects/api", "/home/u/work/api"]
        );
        assert_eq!(labels(&merged), vec!["projects/api", "work/api"]);
        assert_eq!(merged[0].name, "api", "the name stays the folder name");
    }

    // P-66: a label that still collides at one parent takes a second: team/api of two parents becomes a/team/api
    // and b/team/api, while the unique c/api keeps the one parent it needs
    #[test]
    fn a_shared_parent_needs_a_second_level() {
        let listed = project_list(&[
            (abs("/a/team"), names(&["api"])),
            (abs("/b/team"), names(&["api"])),
            (abs("/c"), names(&["api"])),
        ]);
        let merged = merge_projects(&listed, &CustomProjects::default());
        assert_eq!(labels(&merged), vec!["a/team/api", "b/team/api", "c/api"]);
    }

    // P-66: a search-path project and a custom project with the same name are a collision too
    #[test]
    fn search_path_project_colliding_with_a_custom_project_gains_a_parent() {
        let listed = project_list(&[(abs("/p"), names(&["app", "lib"]))]);
        let custom = CustomProjects::parse("/s/app\n");
        let merged = merge_projects(&listed, &custom);
        assert_eq!(paths(&merged), vec!["/p/app", "/p/lib", "/s/app"]);
        assert_eq!(labels(&merged), vec!["p/app", "lib", "s/app"]);
        assert!(merged[2].custom && !merged[0].custom);
    }

    // P-66: the labels of a list are unique, and merging the same inputs again gives the same labels
    #[test]
    fn labels_are_unique_and_stable() {
        let listed = project_list(&[
            (abs("/a/team"), names(&["api", "web"])),
            (abs("/b/team"), names(&["api"])),
            (abs("/c"), names(&["api"])),
        ]);
        let custom = CustomProjects::parse("/d/web\n");
        let first = merge_projects(&listed, &custom);
        let second = merge_projects(&listed, &custom);
        assert_eq!(labels(&first), labels(&second));
        let mut seen = labels(&first);
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), first.len());
    }

    // P-65: a directory is recorded when no project contains it
    #[test]
    fn should_record_outside_every_project() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        assert!(should_record(&abs("/s/scratch"), &listed));
    }

    // P-65: a directory directly under a search path is a project, so it is not recorded
    #[test]
    fn should_not_record_a_project() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        assert!(!should_record(&abs("/p/app"), &listed));
    }

    // P-65: a subdirectory of a project is never recorded
    #[test]
    fn should_not_record_inside_a_project() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        assert!(!should_record(&abs("/p/app/src"), &listed));
    }

    // P-65: a search path itself is not a project, and nothing contains it, so it is recorded
    #[test]
    fn should_record_a_search_path_itself() {
        let listed = project_list(&[(abs("/p"), names(&["app"]))]);
        assert!(should_record(&abs("/p"), &listed));
    }

    // P-65: a directory inside a custom project is not recorded
    #[test]
    fn should_not_record_inside_a_custom_project() {
        let listed = merge_projects(&[], &CustomProjects::parse("/s/scratch\n"));
        assert!(!should_record(&abs("/s/scratch/sub"), &listed));
    }

    // P-65: a custom project is not recorded again
    #[test]
    fn should_not_record_a_listed_custom_project() {
        let listed = merge_projects(&[], &CustomProjects::parse("/s/scratch\n"));
        assert!(!should_record(&abs("/s/scratch"), &listed));
    }

    // P-65: the parent of a custom project is outside it, so it is recorded
    #[test]
    fn should_record_parent_of_a_custom_project() {
        let listed = merge_projects(&[], &CustomProjects::parse("/s/scratch\n"));
        assert!(should_record(&abs("/s"), &listed));
    }

    // P-65: the home folder is recorded like any other directory when no project contains it
    #[test]
    fn should_record_home_folder() {
        let listed = project_list(&[(abs("/home/u/projects"), names(&["app"]))]);
        assert!(should_record(&abs("/home/u"), &listed));
    }

    // P-65: the home folder recorded as a custom project makes the directories under it inside a project
    #[test]
    fn should_not_record_under_a_custom_home_folder() {
        let listed = merge_projects(&[], &CustomProjects::parse("/home/u\n"));
        assert!(!should_record(&abs("/home/u/notes"), &listed));
    }

    // P-65: a prefix of a project name is outside it
    #[test]
    fn should_record_name_prefix_of_a_project() {
        let listed = merge_projects(&[], &CustomProjects::parse("/s/scratch\n"));
        assert!(should_record(&abs("/s/scratchpad"), &listed));
    }
}

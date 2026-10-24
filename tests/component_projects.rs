//! Component tests for the project listing through `RealFs` on temp folders (docs/testing.md level 2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::adapters::fs::RealFs;
use ledge::app::load_projects;
use ledge::domain::custom_projects::CustomProjects;
use ledge::domain::path::AbsPath;
use std::fs;
use std::path::Path;
use support::temp::TempTree;

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

fn base_of(tree: &TempTree) -> AbsPath {
    abs(tree.path().to_str().expect("utf-8 temp path"))
}

fn folder(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create folder");
}

fn project_paths(search_paths: &[AbsPath]) -> Vec<String> {
    load_projects(&RealFs, search_paths, &CustomProjects::default())
        .iter()
        .map(|p| p.path.as_str().to_string())
        .collect()
}

// FR-PICK-01: every directory directly under a search path is a project; a plain file is not
#[test]
fn every_directory_under_search_path_is_a_project() {
    let tree = TempTree::new("component_projects-direct");
    folder(tree.path(), "app");
    folder(tree.path(), "lib");
    fs::write(tree.path().join("notes.txt"), "text").expect("write file");
    let base = base_of(&tree);
    assert_eq!(
        project_paths(std::slice::from_ref(&base)),
        vec![
            format!("{}/app", base.as_str()),
            format!("{}/lib", base.as_str())
        ]
    );
}

// FR-PICK-02: a directory whose name starts with a dot is hidden from the project list
#[test]
fn hidden_directories_are_not_projects() {
    let tree = TempTree::new("component_projects-hidden");
    folder(tree.path(), ".git");
    folder(tree.path(), "app");
    let base = base_of(&tree);
    assert_eq!(
        project_paths(std::slice::from_ref(&base)),
        vec![format!("{}/app", base.as_str())]
    );
}

// FR-CONF-07: only directories directly under a search path are projects; a nested one is not
#[test]
fn nested_directory_is_not_a_project() {
    let tree = TempTree::new("component_projects-nested");
    folder(tree.path(), "app/src/main");
    let base = base_of(&tree);
    assert_eq!(
        project_paths(std::slice::from_ref(&base)),
        vec![format!("{}/app", base.as_str())]
    );
}

// FR-PICK-03: projects from several search paths are sorted by full path
#[test]
fn projects_from_several_search_paths_are_sorted_by_full_path() {
    let tree = TempTree::new("component_projects-sorted");
    folder(tree.path(), "second/x");
    folder(tree.path(), "first/y");
    let base = base_of(&tree);
    let search = [
        abs(&format!("{}/second", base.as_str())),
        abs(&format!("{}/first", base.as_str())),
    ];
    assert_eq!(
        project_paths(&search),
        vec![
            format!("{}/first/y", base.as_str()),
            format!("{}/second/x", base.as_str())
        ]
    );
}

// FR-PICK-01: a search path that does not exist contributes no projects and does not fail
#[test]
fn missing_search_path_contributes_no_projects() {
    let tree = TempTree::new("component_projects-missing");
    folder(tree.path(), "app");
    let base = base_of(&tree);
    let search = [abs(&format!("{}/missing", base.as_str())), base.clone()];
    assert_eq!(
        project_paths(&search),
        vec![format!("{}/app", base.as_str())]
    );
}

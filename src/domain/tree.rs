//! Bookmark tree and session tree (architecture 2 and 5, A-13). Pure: the caller passes the project list
//! (`app::load_projects`), the bookmarks (`Bookmarks`) and the managed sessions with their directories
//! (A-37 is applied by the caller). Rows come out top-down; blocks are ordered by root path and inside a
//! block the nodes follow tree order (path components, not characters).

use std::collections::BTreeMap;

use super::bookmarks::Bookmarks;
use super::naming::session_name;
use super::path::AbsPath;
use super::projects::{project_containing, Project};

/// architecture 5: what a tree row stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// A project folder that is not itself a leaf (gray; openable only in the session tree).
    Root,
    /// A bookmarked directory (bookmark tree).
    Bookmark,
    /// A directory with a managed session (session tree), or a session outside every project.
    Session,
    /// A directory between a root and a leaf (gray, never openable).
    Connector,
}

/// architecture 5: one flattened row of a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow<K> {
    pub depth: usize,
    pub label: String,
    pub path: AbsPath,
    pub kind: K,
    pub openable: bool,
    /// The managed session on this row, if any (`SessionName` is a plain `String`, naming.rs).
    pub session: Option<String>,
    pub dimmed: bool,
}

/// FR-SESS-06: a managed session and the directory it is shown under (`@ledge_dir`, then `session_path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDir {
    pub name: String,
    pub dir: AbsPath,
}

/// FR-TREE-01..04, FR-TREE-11, FR-SCOPE-01, A-13: rows of the bookmark tree. One root per project that
/// holds a bookmark; a bookmark outside every project is left out (FR-TREE-04).
pub fn bookmark_tree(projects: &[Project], bookmarks: &Bookmarks) -> Vec<TreeRow<NodeKind>> {
    let leaves: Vec<(AbsPath, Leaf)> = bookmarks
        .entries()
        .iter()
        .map(|path| (path.clone(), Leaf::Bookmark))
        .collect();
    build(projects, &leaves, Which::Bookmarks)
}

/// FR-SESS-06, FR-SESS-07, FR-TREE-11, FR-SCOPE-01, A-13: rows of the session tree. One root per project
/// that holds a managed session; a session outside every project is its own root labelled with its path.
pub fn session_tree(projects: &[Project], sessions: &[SessionDir]) -> Vec<TreeRow<NodeKind>> {
    let leaves: Vec<(AbsPath, Leaf)> = sessions
        .iter()
        .map(|session| (session.dir.clone(), Leaf::Session(session.name.clone())))
        .collect();
    build(projects, &leaves, Which::Sessions)
}

/// Which tree is built. The two trees differ in the openable and label rules of a root without a leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    Bookmarks,
    Sessions,
}

/// What a leaf of the tree stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Leaf {
    Bookmark,
    Session(String),
}

/// Tree position of a directory: its path components. The `Ord` of a `Vec<String>` is the tree preorder.
type Key = Vec<String>;

/// The leaves of one project, by root key, with the project path.
type Groups = BTreeMap<Key, (AbsPath, Vec<(AbsPath, Leaf)>)>;

/// The rows of one block, keyed by the key of the block root, so blocks come out in tree order.
type Blocks = BTreeMap<Key, Vec<TreeRow<NodeKind>>>;

fn build(projects: &[Project], leaves: &[(AbsPath, Leaf)], which: Which) -> Vec<TreeRow<NodeKind>> {
    let mut groups: Groups = BTreeMap::new();
    let mut blocks: Blocks = BTreeMap::new();
    for (path, leaf) in leaves {
        match project_containing(path, projects) {
            Some(project) => groups
                .entry(key_of(&project.path))
                .or_insert_with(|| (project.path.clone(), Vec::new()))
                .1
                .push((path.clone(), leaf.clone())),
            // FR-TREE-04: a bookmark outside every project is left out. A session outside every
            // project is a root labelled with its path (A-13).
            None => {
                if let Leaf::Session(name) = leaf {
                    blocks.entry(key_of(path)).or_default().push(TreeRow {
                        depth: 0,
                        label: path.as_str().to_string(),
                        path: path.clone(),
                        kind: NodeKind::Session,
                        openable: true,
                        session: Some(name.clone()),
                        dimmed: false,
                    });
                }
            }
        }
    }
    for (key, (root, members)) in groups {
        blocks
            .entry(key)
            .or_default()
            .extend(group_rows(&root, &members, which));
    }
    blocks.into_values().flatten().collect()
}

/// The rows under one project root: every directory from the root down to a leaf is a node, a node
/// without a leaf is a connector (or the gray root), and each leaf is one row.
fn group_rows(root: &AbsPath, members: &[(AbsPath, Leaf)], which: Which) -> Vec<TreeRow<NodeKind>> {
    let root_key = key_of(root);
    let mut nodes: BTreeMap<Key, Vec<Leaf>> = BTreeMap::new();
    nodes.insert(root_key.clone(), Vec::new());
    for (path, leaf) in members {
        let key = key_of(path);
        for depth in (root_key.len() + 1)..key.len() {
            nodes
                .entry(key.iter().take(depth).cloned().collect())
                .or_default();
        }
        nodes.entry(key).or_default().push(leaf.clone());
    }

    let mut rows = Vec::new();
    for (key, leaves) in nodes {
        let path = path_of(&key);
        let depth = key.len().saturating_sub(root_key.len());
        let is_root = key == root_key;
        let base = key.last().cloned().unwrap_or_else(|| "/".to_string());
        if leaves.is_empty() {
            // FR-SESS-07, A-13: a gray project folder with no session of its own is labelled with the
            // session name it would get and is openable in the session tree; in the bookmark tree the
            // root is gray and not openable.
            let (label, openable) = match (is_root, which) {
                (true, Which::Sessions) => (session_name(&path), true),
                _ => (base, false),
            };
            rows.push(TreeRow {
                depth,
                label,
                path,
                kind: if is_root {
                    NodeKind::Root
                } else {
                    NodeKind::Connector
                },
                openable,
                session: None,
                dimmed: true,
            });
        } else {
            for leaf in leaves {
                let (kind, session) = match leaf {
                    Leaf::Bookmark => (NodeKind::Bookmark, None),
                    Leaf::Session(name) => (NodeKind::Session, Some(name)),
                };
                rows.push(TreeRow {
                    depth,
                    label: base.clone(),
                    path: path.clone(),
                    kind,
                    openable: true,
                    session,
                    dimmed: false,
                });
            }
        }
    }
    rows
}

fn key_of(path: &AbsPath) -> Key {
    path.as_str()
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn path_of(key: &[String]) -> AbsPath {
    AbsPath::clean(&format!("/{}", key.join("/")), &AbsPath::root())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{bookmark_tree, session_tree, NodeKind, SessionDir, TreeRow};
    use crate::domain::bookmarks::Bookmarks;
    use crate::domain::naming::session_name;
    use crate::domain::path::AbsPath;
    use crate::domain::projects::Project;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn project(text: &str) -> Project {
        let path = abs(text);
        let name = path.as_str().rsplit('/').next().unwrap_or("").to_string();
        Project { path, name }
    }

    fn session(name: &str, dir: &str) -> SessionDir {
        SessionDir {
            name: name.to_string(),
            dir: abs(dir),
        }
    }

    fn bookmarks(text: &str) -> Bookmarks {
        Bookmarks::parse(text)
    }

    /// (path, depth, kind, openable) of every row, top-down.
    fn shape(rows: &[TreeRow<NodeKind>]) -> Vec<(&str, usize, NodeKind, bool)> {
        rows.iter()
            .map(|r| (r.path.as_str(), r.depth, r.kind, r.openable))
            .collect()
    }

    // FR-TREE-01: one root per project that has a bookmark; a project without one is not shown
    #[test]
    fn one_root_per_project_that_has_a_bookmark() {
        let projects = vec![project("/p/app"), project("/p/lib"), project("/p/other")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/x\n/p/lib/y\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/x", 1, NodeKind::Bookmark, true),
                ("/p/lib", 0, NodeKind::Root, false),
                ("/p/lib/y", 1, NodeKind::Bookmark, true),
            ]
        );
    }

    // FR-TREE-01: a bookmark hangs under its nearest shown ancestor, and the directories between are shown
    #[test]
    fn bookmark_nests_under_nearest_shown_ancestor() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/a\n/p/app/a/b/c\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/a", 1, NodeKind::Bookmark, true),
                ("/p/app/a/b", 2, NodeKind::Connector, false),
                ("/p/app/a/b/c", 3, NodeKind::Bookmark, true),
            ]
        );
    }

    // FR-TREE-02: directories between the root and a bookmark are gray connectors
    #[test]
    fn directories_between_root_and_bookmark_are_gray_connectors() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/src/main/x\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/src", 1, NodeKind::Connector, false),
                ("/p/app/src/main", 2, NodeKind::Connector, false),
                ("/p/app/src/main/x", 3, NodeKind::Bookmark, true),
            ]
        );
        assert!(rows[1].dimmed && rows[2].dimmed);
    }

    // FR-TREE-03: a shared parent of two bookmarks is one gray entry, listed once
    #[test]
    fn shared_parent_of_two_bookmarks_is_one_gray_row() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/mods/a\n/p/app/mods/b\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/mods", 1, NodeKind::Connector, false),
                ("/p/app/mods/a", 2, NodeKind::Bookmark, true),
                ("/p/app/mods/b", 2, NodeKind::Bookmark, true),
            ]
        );
        assert!(rows[1].dimmed);
    }

    // FR-TREE-04: a bookmark outside every project is left out, and so is one that only shares a name prefix
    #[test]
    fn bookmark_outside_every_project_is_left_out() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/home/u/x\n/p/appx/z\n/p/app/y\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/y", 1, NodeKind::Bookmark, true),
            ]
        );
    }

    // FR-SCOPE-01: a bookmark belongs to the deepest project that contains it
    #[test]
    fn bookmark_is_grouped_under_the_deepest_project_containing_it() {
        let projects = vec![project("/p"), project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/src/x\n/p/k\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p", 0, NodeKind::Root, false),
                ("/p/k", 1, NodeKind::Bookmark, true),
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/src", 1, NodeKind::Connector, false),
                ("/p/app/src/x", 2, NodeKind::Bookmark, true),
            ]
        );
    }

    // FR-TREE-01: inside a root the rows follow tree order by path components, so "a-b" comes after "a/x"
    #[test]
    fn rows_follow_tree_order_by_path_components() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/a-b\n/p/app/a/x\n"));
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/a", 1, NodeKind::Connector, false),
                ("/p/app/a/x", 2, NodeKind::Bookmark, true),
                ("/p/app/a-b", 1, NodeKind::Bookmark, true),
            ]
        );
    }

    // FR-TREE-11: in the bookmark tree only bookmarks open; the gray root and connectors do not
    #[test]
    fn only_bookmarks_are_openable_in_bookmark_tree() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app/a/b\n"));
        assert!(!rows[0].openable && rows[0].dimmed);
        assert!(!rows[1].openable && rows[1].dimmed);
        assert!(rows[2].openable && !rows[2].dimmed);
    }

    // FR-TREE-01: a bookmark that is itself a project is that project's root and it is openable
    // (no spec text covers it; see docs/proposals.md)
    #[test]
    fn bookmark_equal_to_project_is_its_own_openable_root() {
        let projects = vec![project("/p/app")];
        let rows = bookmark_tree(&projects, &bookmarks("/p/app\n"));
        assert_eq!(shape(&rows), vec![("/p/app", 0, NodeKind::Bookmark, true)]);
        assert!(!rows[0].dimmed);
    }

    // FR-TREE-01: no bookmarks, no rows
    #[test]
    fn no_bookmarks_gives_no_rows() {
        let projects = vec![project("/p/app")];
        assert!(bookmark_tree(&projects, &bookmarks("")).is_empty());
    }

    // FR-SESS-06: a project with a managed session is a root; the project without one is not shown
    #[test]
    fn project_with_session_is_a_root_and_project_without_is_not_shown() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        let rows = session_tree(&projects, &[session("s1", "/p/app/src")]);
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, true),
                ("/p/app/src", 1, NodeKind::Session, true),
            ]
        );
    }

    // FR-SESS-07: a gray project folder with no session is labelled with the session name it would get
    #[test]
    fn gray_project_root_shows_the_session_name_it_would_get() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(&projects, &[session("s1", "/p/app/src")]);
        assert_eq!(rows[0].label, session_name(&abs("/p/app")));
        assert_eq!(rows[0].session, None);
        assert!(rows[0].dimmed && rows[0].openable);
    }

    // FR-SESS-06: a session on the project folder itself is a session row at depth 0
    #[test]
    fn session_at_project_root_is_a_session_row() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(&projects, &[session("s1", "/p/app")]);
        assert_eq!(shape(&rows), vec![("/p/app", 0, NodeKind::Session, true)]);
        assert_eq!(rows[0].label, "app");
        assert_eq!(rows[0].session.as_deref(), Some("s1"));
        assert!(!rows[0].dimmed);
    }

    // FR-SESS-06: connectors between the root and a session are gray and not openable
    #[test]
    fn connectors_between_root_and_session_are_gray_and_not_openable() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(&projects, &[session("s1", "/p/app/src/main/x")]);
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, true),
                ("/p/app/src", 1, NodeKind::Connector, false),
                ("/p/app/src/main", 2, NodeKind::Connector, false),
                ("/p/app/src/main/x", 3, NodeKind::Session, true),
            ]
        );
        assert!(rows[1].dimmed && rows[2].dimmed);
        assert_eq!(rows[1].session, None);
    }

    // FR-SESS-06: several sessions in one project share its root, each on its own row
    #[test]
    fn sessions_in_one_project_share_the_root() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(
            &projects,
            &[session("a", "/p/app/a"), session("b", "/p/app/b")],
        );
        assert_eq!(
            shape(&rows),
            vec![
                ("/p/app", 0, NodeKind::Root, true),
                ("/p/app/a", 1, NodeKind::Session, true),
                ("/p/app/b", 1, NodeKind::Session, true),
            ]
        );
    }

    // A-13, FR-SESS-06: a session outside every project is its own root labelled with its path
    #[test]
    fn session_outside_every_project_is_a_root_labelled_with_its_path() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(&projects, &[session("out", "/home/u/x")]);
        assert_eq!(
            shape(&rows),
            vec![("/home/u/x", 0, NodeKind::Session, true)]
        );
        assert_eq!(rows[0].label, "/home/u/x");
        assert_eq!(rows[0].session.as_deref(), Some("out"));
        assert!(!rows[0].dimmed);
    }

    // FR-SCOPE-01: a session is grouped under the deepest project that contains its directory
    #[test]
    fn session_is_grouped_under_the_deepest_project_containing_it() {
        let projects = vec![project("/p"), project("/p/app")];
        let rows = session_tree(
            &projects,
            &[session("a", "/p/app/src"), session("b", "/p/k")],
        );
        assert_eq!(
            shape(&rows),
            vec![
                ("/p", 0, NodeKind::Root, true),
                ("/p/k", 1, NodeKind::Session, true),
                ("/p/app", 0, NodeKind::Root, true),
                ("/p/app/src", 1, NodeKind::Session, true),
            ]
        );
    }

    // FR-SESS-06: with no managed sessions the session tree is empty, even when projects exist
    #[test]
    fn no_sessions_gives_no_rows() {
        let projects = vec![project("/p/app")];
        assert!(session_tree(&projects, &[]).is_empty());
    }
}

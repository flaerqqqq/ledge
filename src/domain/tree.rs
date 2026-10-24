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
    /// A project folder that is not itself a bookmark (bookmark tree: gray, not openable).
    Root,
    /// A bookmarked directory (bookmark tree).
    Bookmark,
    /// A directory with a managed session (session tree), or a session outside every project.
    Session,
    /// A directory between a root and a leaf (bookmark tree: gray, never openable).
    Connector,
    /// A project with sessions but no session of its own (session tree: gray, named by its would-be session).
    Placeholder,
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
    let mut groups: Groups<AbsPath> = BTreeMap::new();
    for path in bookmarks.entries() {
        // FR-TREE-04: a bookmark outside every project is left out.
        if let Some(project) = project_containing(path, projects) {
            groups
                .entry(key_of(&project.path))
                .or_insert_with(|| (project.path.clone(), Vec::new()))
                .1
                .push(path.clone());
        }
    }
    groups
        .into_values()
        .flat_map(|(root, members)| bookmark_rows(&root, &members))
        .collect()
}

/// FR-TREE-05..08: which part of the bookmark tree a window shows. `Directory` is the directory given to `bm tree`,
/// or the directory of the current session with `--session-scope` (FR-TREE-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeScope {
    All,
    Directory(AbsPath),
}

/// FR-TREE-05..08: the rows a window shows and the index of the row the cursor starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeView {
    pub rows: Vec<TreeRow<NodeKind>>,
    pub cursor: usize,
}

/// FR-TREE-05..08: the bookmark tree for `scope`. A directory shows only the tree of its project (the one that holds
/// a bookmark), with the cursor on the nearest shown node. A directory in no such tree shows only its root directory:
/// the root of its project, or the directory itself when it is outside every project (A-13, P-44).
pub fn bookmark_view(projects: &[Project], bookmarks: &Bookmarks, scope: &TreeScope) -> TreeView {
    let rows = bookmark_tree(projects, bookmarks);
    let dir = match scope {
        TreeScope::All => return TreeView { rows, cursor: 0 },
        TreeScope::Directory(dir) => dir,
    };
    let block =
        project_containing(dir, projects).and_then(|project| block_at(&rows, &project.path));
    match block {
        Some(block) => {
            let cursor = block
                .iter()
                .enumerate()
                .filter(|(_, row)| dir.is_inside(&row.path))
                .max_by_key(|(_, row)| row.depth)
                .map_or(0, |(index, _)| index);
            TreeView {
                rows: block,
                cursor,
            }
        }
        None => root_only(dir, projects),
    }
}

/// The rows of the block whose root row is at `root`: from that row up to the next depth-0 row.
fn block_at(rows: &[TreeRow<NodeKind>], root: &AbsPath) -> Option<Vec<TreeRow<NodeKind>>> {
    let start = rows
        .iter()
        .position(|row| row.depth == 0 && row.path == *root)?;
    let end = rows
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, row)| row.depth == 0)
        .map_or(rows.len(), |(index, _)| index);
    rows.get(start..end).map(<[TreeRow<NodeKind>]>::to_vec)
}

/// FR-TREE-07, A-13: the single gray row of a directory that no bookmark tree holds. Labelled with its project's
/// basename, or with its path outside every project.
fn root_only(dir: &AbsPath, projects: &[Project]) -> TreeView {
    let (path, label) = match project_containing(dir, projects) {
        Some(project) => (
            project.path.clone(),
            project
                .path
                .as_str()
                .rsplit('/')
                .find(|part| !part.is_empty())
                .unwrap_or("/")
                .to_string(),
        ),
        None => (dir.clone(), dir.as_str().to_string()),
    };
    TreeView {
        rows: vec![TreeRow {
            depth: 0,
            label,
            path,
            kind: NodeKind::Root,
            openable: false,
            session: None,
            dimmed: true,
        }],
        cursor: 0,
    }
}

/// FR-SESS-06, FR-SESS-07, P-54, FR-TREE-11, FR-SCOPE-01, A-13: rows of the session tree. Sessions nest by nearest
/// ancestor session; a project with sessions but no session of its own gets one gray placeholder row above them.
/// Folders without a session add no row. A session outside every project is its own root (A-13), and the sessions
/// outside every project nest by nearest ancestor session among themselves.
pub fn session_tree(projects: &[Project], sessions: &[SessionDir]) -> Vec<TreeRow<NodeKind>> {
    let mut groups: Groups<Member> = BTreeMap::new();
    let mut outside: Vec<Member> = Vec::new();
    for session in sessions {
        let member = (session.dir.clone(), session.name.clone());
        match project_containing(&session.dir, projects) {
            Some(project) => groups
                .entry(key_of(&project.path))
                .or_insert_with(|| (project.path.clone(), Vec::new()))
                .1
                .push(member),
            None => outside.push(member),
        }
    }
    let mut blocks: Blocks = BTreeMap::new();
    for (root, members) in groups.into_values() {
        push_blocks(&mut blocks, session_rows(Some(&root), members));
    }
    push_blocks(&mut blocks, session_rows(None, outside));
    blocks.into_values().flatten().collect()
}

/// A managed session: its directory and its name.
type Member = (AbsPath, String);

/// Tree position of a directory: its path components. The `Ord` of a `Vec<String>` is the tree preorder.
type Key = Vec<String>;

/// The members of one project, by root key, with the project path.
type Groups<T> = BTreeMap<Key, (AbsPath, Vec<T>)>;

/// The rows of one block, keyed by the key of the block root, so blocks come out in tree order.
type Blocks = BTreeMap<Key, Vec<TreeRow<NodeKind>>>;

fn push_blocks(blocks: &mut Blocks, rows: Vec<(Key, TreeRow<NodeKind>)>) {
    for (key, row) in rows {
        blocks.entry(key).or_default().push(row);
    }
}

/// FR-SESS-06, P-54: the rows of the sessions of one project (`root` given) or of the sessions outside every
/// project (`root` none), each with the key of its block root. Members sorted by path components are in tree
/// order, so a parent comes before its children, and a parent is the nearest earlier member that is an ancestor.
fn session_rows(root: Option<&AbsPath>, mut members: Vec<Member>) -> Vec<(Key, TreeRow<NodeKind>)> {
    members.sort_by_key(|(path, _)| key_of(path));
    // The project folder gets a placeholder row when none of its sessions is on the folder itself.
    let placeholder = root.filter(|root| !members.iter().any(|(path, _)| path == *root));
    let mut rows = Vec::new();
    let mut depths: Vec<usize> = Vec::new();
    let mut keys: Vec<Key> = Vec::new();
    if let Some(root) = placeholder {
        rows.push((
            key_of(root),
            TreeRow {
                depth: 0,
                label: session_name(root),
                path: root.clone(),
                kind: NodeKind::Placeholder,
                openable: true,
                session: None,
                dimmed: true,
            },
        ));
    }
    let top_depth = usize::from(placeholder.is_some());
    for (index, (path, name)) in members.iter().enumerate() {
        let parent = members[..index]
            .iter()
            .rposition(|(other, _)| other != path && path.is_inside(other));
        let (depth, key) = match parent {
            Some(parent) => (depths[parent] + 1, keys[parent].clone()),
            None => (top_depth, root.map_or_else(|| key_of(path), key_of)),
        };
        depths.push(depth);
        keys.push(key.clone());
        let label = match root {
            Some(_) => path
                .as_str()
                .rsplit('/')
                .find(|part| !part.is_empty())
                .unwrap_or("/")
                .to_string(),
            None => path.as_str().to_string(),
        };
        rows.push((
            key,
            TreeRow {
                depth,
                label,
                path: path.clone(),
                kind: NodeKind::Session,
                openable: true,
                session: Some(name.clone()),
                dimmed: false,
            },
        ));
    }
    rows
}

/// FR-TREE-01..03, FR-TREE-11, A-13: the rows under one project root of the bookmark tree. Every directory from the
/// root down to a bookmark is a node; a node without a bookmark is a gray connector (or the gray root).
fn bookmark_rows(root: &AbsPath, members: &[AbsPath]) -> Vec<TreeRow<NodeKind>> {
    let root_key = key_of(root);
    // Node key -> number of bookmarks on it.
    let mut nodes: BTreeMap<Key, usize> = BTreeMap::new();
    nodes.insert(root_key.clone(), 0);
    for path in members {
        let key = key_of(path);
        for depth in (root_key.len() + 1)..key.len() {
            nodes
                .entry(key.iter().take(depth).cloned().collect())
                .or_default();
        }
        *nodes.entry(key).or_default() += 1;
    }

    let mut rows = Vec::new();
    for (key, count) in nodes {
        let path = path_of(&key);
        let depth = key.len().saturating_sub(root_key.len());
        let is_root = key == root_key;
        let base = key.last().cloned().unwrap_or_else(|| "/".to_string());
        if count == 0 {
            rows.push(TreeRow {
                depth,
                label: base,
                path,
                kind: if is_root {
                    NodeKind::Root
                } else {
                    NodeKind::Connector
                },
                openable: false,
                session: None,
                dimmed: true,
            });
        } else {
            for _ in 0..count {
                rows.push(TreeRow {
                    depth,
                    label: base.clone(),
                    path: path.clone(),
                    kind: NodeKind::Bookmark,
                    openable: true,
                    session: None,
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

    use super::{
        bookmark_tree, bookmark_view, session_tree, NodeKind, SessionDir, TreeRow, TreeScope,
    };
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

    /// (path, depth, session) of every row top-down. A placeholder row has no session.
    fn session_shape(rows: &[TreeRow<NodeKind>]) -> Vec<(&str, usize, Option<&str>)> {
        rows.iter()
            .map(|r| (r.path.as_str(), r.depth, r.session.as_deref()))
            .collect()
    }

    // FR-SESS-06, P-54: a session is a child of the session with the nearest ancestor directory; with the
    // ledge project and its subfolder sessions the rows are at depths 0, 1 and 2
    #[test]
    fn session_nests_under_the_nearest_session_ancestor() {
        let projects = vec![project("/p/ledge")];
        let rows = session_tree(
            &projects,
            &[
                session("L", "/p/ledge"),
                session("D", "/p/ledge/docs"),
                session("A", "/p/ledge/docs/adr"),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/ledge", 0, Some("L")),
                ("/p/ledge/docs", 1, Some("D")),
                ("/p/ledge/docs/adr", 2, Some("A")),
            ]
        );
    }

    // FR-SESS-06, P-54: depth follows sessions, not folders: a folder with no session adds no row, so
    // ledge/docs/adr hangs under ledge at depth 1 when ledge/docs has no session
    #[test]
    fn folders_without_a_session_add_no_rows_and_no_depth() {
        let projects = vec![project("/p/ledge")];
        let rows = session_tree(
            &projects,
            &[session("L", "/p/ledge"), session("A", "/p/ledge/docs/adr")],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/ledge", 0, Some("L")),
                ("/p/ledge/docs/adr", 1, Some("A"))
            ]
        );
    }

    // FR-SESS-06, P-54: between a placeholder and the session under it there are folders only, so no
    // folder rows appear and the session is at depth 1
    #[test]
    fn placeholder_is_followed_by_sessions_with_no_folder_rows() {
        let projects = vec![project("/p/api")];
        let rows = session_tree(&projects, &[session("S", "/p/api/src/main/x")]);
        assert_eq!(
            session_shape(&rows),
            vec![("/p/api", 0, None), ("/p/api/src/main/x", 1, Some("S"))]
        );
    }

    // FR-SESS-06, P-54: a project with sessions but no session of its own gets one gray placeholder row at
    // depth 0, labelled with the session name it would get; its sessions hang under it
    #[test]
    fn project_without_its_own_session_gets_a_placeholder_over_its_sessions() {
        let projects = vec![project("/p/api")];
        let rows = session_tree(
            &projects,
            &[session("S1", "/p/api/server"), session("S2", "/p/api/web")],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/api", 0, None),
                ("/p/api/server", 1, Some("S1")),
                ("/p/api/web", 1, Some("S2")),
            ]
        );
        assert_eq!(rows[0].label, session_name(&abs("/p/api")));
        assert!(rows[0].openable, "enter opens the placeholder's project");
        assert!(rows[0].dimmed, "the placeholder is gray");
    }

    // FR-SESS-06, P-54: a project with no session of any kind is not listed; the project with sessions is
    #[test]
    fn project_without_sessions_is_not_listed() {
        let projects = vec![project("/p/api"), project("/p/notes")];
        let rows = session_tree(&projects, &[session("S", "/p/api/server")]);
        assert_eq!(
            session_shape(&rows),
            vec![("/p/api", 0, None), ("/p/api/server", 1, Some("S"))]
        );
    }

    // FR-SESS-06, P-54: a session on the project folder itself is a session row at depth 0, with no placeholder
    #[test]
    fn session_at_project_root_is_a_session_row() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(&projects, &[session("s1", "/p/app")]);
        assert_eq!(shape(&rows), vec![("/p/app", 0, NodeKind::Session, true)]);
        assert_eq!(rows[0].label, "app");
        assert_eq!(rows[0].session.as_deref(), Some("s1"));
        assert!(!rows[0].dimmed);
    }

    // FR-SESS-06, P-54: several sessions in one project without its own session share one placeholder
    #[test]
    fn sessions_in_one_project_share_one_placeholder() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(
            &projects,
            &[session("a", "/p/app/a"), session("b", "/p/app/b")],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/app", 0, None),
                ("/p/app/a", 1, Some("a")),
                ("/p/app/b", 1, Some("b")),
            ]
        );
    }

    // FR-SESS-06, P-54: a session outside every project is a depth 0 row labelled with its path (A-13)
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

    // FR-SESS-06, P-54: a session is grouped under the deepest project that contains its directory, and each
    // of those projects gets its own placeholder when it has no session of its own
    #[test]
    fn session_is_grouped_under_the_deepest_project_containing_it() {
        let projects = vec![project("/p"), project("/p/app")];
        let rows = session_tree(
            &projects,
            &[session("a", "/p/app/src"), session("b", "/p/k")],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p", 0, None),
                ("/p/k", 1, Some("b")),
                ("/p/app", 0, None),
                ("/p/app/src", 1, Some("a")),
            ]
        );
    }

    fn scope_dir(text: &str) -> TreeScope {
        TreeScope::Directory(abs(text))
    }

    // FR-TREE-08: with no directory and no session scope, every tree is shown and the cursor is on the first row
    #[test]
    fn all_trees_are_shown_when_no_scope_is_given() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        let view = bookmark_view(
            &projects,
            &bookmarks("/p/app/x\n/p/lib/y\n"),
            &TreeScope::All,
        );
        assert_eq!(
            shape(&view.rows),
            vec![
                ("/p/app", 0, NodeKind::Root, false),
                ("/p/app/x", 1, NodeKind::Bookmark, true),
                ("/p/lib", 0, NodeKind::Root, false),
                ("/p/lib/y", 1, NodeKind::Bookmark, true),
            ]
        );
        assert_eq!(view.cursor, 0);
    }

    // FR-TREE-05: a given directory shows only the tree that contains it
    #[test]
    fn directory_shows_only_the_tree_that_contains_it() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        let view = bookmark_view(
            &projects,
            &bookmarks("/p/app/x\n/p/lib/y\n"),
            &scope_dir("/p/lib/z"),
        );
        assert_eq!(
            shape(&view.rows),
            vec![
                ("/p/lib", 0, NodeKind::Root, false),
                ("/p/lib/y", 1, NodeKind::Bookmark, true),
            ]
        );
        assert_eq!(view.cursor, 0);
    }

    // FR-TREE-05: the cursor starts on the nearest node, the deepest shown directory that holds the given one
    #[test]
    fn cursor_starts_on_the_nearest_shown_node() {
        let projects = vec![project("/p/app")];
        let text = "/p/app/a/b/c\n/p/app/d\n";
        let node_at = |dir: &str| {
            let view = bookmark_view(&projects, &bookmarks(text), &scope_dir(dir));
            view.rows
                .get(view.cursor)
                .map(|row| row.path.as_str().to_string())
        };
        // a gray connector that is shown
        assert_eq!(node_at("/p/app/a/b/x/y").as_deref(), Some("/p/app/a/b"));
        // a directory that is not shown, between a connector and a bookmark
        assert_eq!(node_at("/p/app/a/q").as_deref(), Some("/p/app/a"));
        // a bookmark itself
        assert_eq!(node_at("/p/app/d").as_deref(), Some("/p/app/d"));
        // the project root
        assert_eq!(node_at("/p/app/z").as_deref(), Some("/p/app"));
    }

    // FR-TREE-07: a directory outside every tree shows only the root directory of its project
    #[test]
    fn directory_outside_every_tree_shows_only_its_root() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        let view = bookmark_view(
            &projects,
            &bookmarks("/p/app/x\n"),
            &scope_dir("/p/lib/z/w"),
        );
        assert_eq!(
            shape(&view.rows),
            vec![("/p/lib", 0, NodeKind::Root, false)]
        );
        assert_eq!(view.cursor, 0);
        assert!(view.rows[0].dimmed);
    }

    // FR-TREE-07: a directory outside every project is the only row, labelled with its path (A-13)
    #[test]
    fn directory_outside_every_project_shows_only_itself() {
        let projects = vec![project("/p/app")];
        let view = bookmark_view(&projects, &bookmarks("/p/app/x\n"), &scope_dir("/home/u/x"));
        assert_eq!(
            shape(&view.rows),
            vec![("/home/u/x", 0, NodeKind::Root, false)]
        );
        assert_eq!(view.rows[0].label, "/home/u/x");
        assert_eq!(view.cursor, 0);
    }

    // FR-SESS-06: with no managed sessions the session tree is empty, even when projects exist
    #[test]
    fn no_sessions_gives_no_rows() {
        let projects = vec![project("/p/app")];
        assert!(session_tree(&projects, &[]).is_empty());
    }
}

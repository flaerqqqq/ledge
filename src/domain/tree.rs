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

/// FR-SESS-06: a managed session and the directory it is shown under (`@ledge_dir`, then `session_path`), with its
/// last activity in Unix seconds (P-56: the siblings of the session tree are ordered by it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDir {
    pub name: String,
    pub dir: AbsPath,
    pub activity: i64,
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
        .flat_map(|(root, members)| {
            let label = root_label(&root, projects);
            bookmark_rows(&root, &label, &members)
        })
        .collect()
}

/// P-66: the label of the project at `root`, which names its root row. The last component of `root` when no project
/// has that path.
fn root_label(root: &AbsPath, projects: &[Project]) -> String {
    projects
        .iter()
        .find(|project| project.path == *root)
        .map_or_else(
            || {
                root.as_str()
                    .rsplit('/')
                    .find(|part| !part.is_empty())
                    .unwrap_or("/")
                    .to_string()
            },
            |project| project.label.clone(),
        )
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

/// FR-TREE-07, A-13, P-66: the single gray row of a directory that no bookmark tree holds. Labelled with its project's
/// label, or with its path outside every project.
fn root_only(dir: &AbsPath, projects: &[Project]) -> TreeView {
    let (path, label) = match project_containing(dir, projects) {
        Some(project) => (project.path.clone(), project.label.clone()),
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

/// FR-SESS-06, FR-SESS-07, P-54, P-56, FR-TREE-11, FR-SCOPE-01, A-13: rows of the session tree. Sessions nest by nearest
/// ancestor session; a project with sessions but no session of its own gets one gray placeholder row above them.
/// Folders without a session add no row. A session outside every project is its own root (A-13), and the sessions
/// outside every project nest by nearest ancestor session among themselves. P-56: sessions that share a parent are
/// listed most recent activity first, ties by name; the top-level projects (and the sessions outside every project)
/// are listed by their most recent session, ties by tree order.
pub fn session_tree(projects: &[Project], sessions: &[SessionDir]) -> Vec<TreeRow<NodeKind>> {
    let mut groups: Groups<SessionDir> = BTreeMap::new();
    let mut outside: Vec<SessionDir> = Vec::new();
    for session in sessions {
        match project_containing(&session.dir, projects) {
            Some(project) => groups
                .entry(key_of(&project.path))
                .or_insert_with(|| (project.path.clone(), Vec::new()))
                .1
                .push(session.clone()),
            None => outside.push(session.clone()),
        }
    }
    let mut units: Vec<Unit> = groups
        .into_values()
        .map(|(root, members)| project_unit(&root, members))
        .collect();
    units.extend(outside_units(outside));
    units.sort_by(|a, b| b.activity.cmp(&a.activity).then_with(|| a.key.cmp(&b.key)));
    units.into_iter().flat_map(|unit| unit.rows).collect()
}

/// A managed session with the sessions nested under it (P-54), siblings in the order of P-56.
struct Node {
    session: SessionDir,
    children: Vec<Node>,
}

/// One top-level part of the session tree: a project with its sessions, or a session outside every project with the
/// sessions under it. `activity` is the most recent activity of its sessions and `key` its tree position.
struct Unit {
    activity: i64,
    key: Key,
    rows: Vec<TreeRow<NodeKind>>,
}

/// Tree position of a directory: its path components. The `Ord` of a `Vec<String>` is the tree preorder.
type Key = Vec<String>;

/// The members of one project, by root key, with the project path.
type Groups<T> = BTreeMap<Key, (AbsPath, Vec<T>)>;

/// FR-SESS-06, P-54, P-56: the unit of one project: the placeholder (when no session is on the folder itself) and the
/// sessions of the project in their nested order.
fn project_unit(root: &AbsPath, members: Vec<SessionDir>) -> Unit {
    let activity = members
        .iter()
        .map(|session| session.activity)
        .max()
        .unwrap_or_default();
    // The project folder gets a placeholder row when none of its sessions is on the folder itself.
    let placeholder = !members.iter().any(|session| session.dir == *root);
    let mut rows = Vec::new();
    if placeholder {
        rows.push(TreeRow {
            depth: 0,
            label: session_name(root),
            path: root.clone(),
            kind: NodeKind::Placeholder,
            openable: true,
            session: None,
            dimmed: true,
        });
    }
    let top_depth = usize::from(placeholder);
    for node in forest(members) {
        push_rows(&node, top_depth, true, &mut rows);
    }
    Unit {
        activity,
        key: key_of(root),
        rows,
    }
}

/// FR-SESS-06, P-56: each session outside every project is a unit of its own, with the sessions nested under it.
fn outside_units(members: Vec<SessionDir>) -> Vec<Unit> {
    forest(members)
        .into_iter()
        .map(|node| {
            let mut rows = Vec::new();
            push_rows(&node, 0, false, &mut rows);
            Unit {
                activity: node.session.activity,
                key: key_of(&node.session.dir),
                rows,
            }
        })
        .collect()
}

/// FR-SESS-06, P-54, P-56: the sessions nested by nearest ancestor directory. Sessions sorted by path components come
/// in tree order, so a parent is the nearest earlier session whose directory is an ancestor. The top-level sessions
/// and the children of each session are ordered most recent activity first, ties by name.
fn forest(mut members: Vec<SessionDir>) -> Vec<Node> {
    members.sort_by_key(|session| key_of(&session.dir));
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); members.len()];
    let mut top: Vec<usize> = Vec::new();
    for index in 0..members.len() {
        let dir = &members[index].dir;
        let parent = members[..index]
            .iter()
            .rposition(|other| other.dir != *dir && dir.is_inside(&other.dir));
        match parent {
            Some(parent) => children[parent].push(index),
            None => top.push(index),
        }
    }
    let mut nodes: Vec<Node> = top
        .into_iter()
        .map(|index| node_at(index, &members, &children))
        .collect();
    order_siblings(&mut nodes);
    nodes
}

fn node_at(index: usize, members: &[SessionDir], children: &[Vec<usize>]) -> Node {
    let mut nodes: Vec<Node> = children[index]
        .iter()
        .map(|&child| node_at(child, members, children))
        .collect();
    order_siblings(&mut nodes);
    Node {
        session: members[index].clone(),
        children: nodes,
    }
}

/// P-56: most recent activity first, ties by session name.
fn order_siblings(nodes: &mut [Node]) {
    nodes.sort_by(|a, b| {
        b.session
            .activity
            .cmp(&a.session.activity)
            .then_with(|| a.session.name.cmp(&b.session.name))
    });
}

/// FR-SESS-06, P-54: the rows of a node and the nodes under it, depth first. A project's session is labelled with its
/// folder name; a session outside every project with its path (A-13).
fn push_rows(node: &Node, depth: usize, in_project: bool, rows: &mut Vec<TreeRow<NodeKind>>) {
    let path = &node.session.dir;
    let label = if in_project {
        path.as_str()
            .rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or("/")
            .to_string()
    } else {
        path.as_str().to_string()
    };
    rows.push(TreeRow {
        depth,
        label,
        path: path.clone(),
        kind: NodeKind::Session,
        openable: true,
        session: Some(node.session.name.clone()),
        dimmed: false,
    });
    for child in &node.children {
        push_rows(child, depth + 1, in_project, rows);
    }
}

/// FR-TREE-01..03, FR-TREE-11, A-13: the rows under one project root of the bookmark tree. Every directory from the
/// root down to a bookmark is a node; a node without a bookmark is a gray connector (or the gray root). P-66: the root
/// row is labelled `root_label`, the other rows with their folder name.
fn bookmark_rows(root: &AbsPath, root_label: &str, members: &[AbsPath]) -> Vec<TreeRow<NodeKind>> {
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
        let label = if is_root {
            root_label.to_string()
        } else {
            base
        };
        if count == 0 {
            rows.push(TreeRow {
                depth,
                label,
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
                    label: label.clone(),
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
    use crate::domain::custom_projects::CustomProjects;
    use crate::domain::naming::session_name;
    use crate::domain::path::AbsPath;
    use crate::domain::projects::{merge_projects, project_list, Project};

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn project(text: &str) -> Project {
        let path = abs(text);
        let name = path.as_str().rsplit('/').next().unwrap_or("").to_string();
        Project::new(path, name, false)
    }

    /// The projects of two search paths that hold a folder of the same name (`projects/api`, `work/api`), labelled
    /// as `merge_projects` labels them.
    fn colliding_projects() -> Vec<Project> {
        let listed = project_list(&[
            (abs("/home/u/projects"), vec!["api".to_string()]),
            (abs("/home/u/work"), vec!["api".to_string()]),
        ]);
        merge_projects(&listed, &CustomProjects::default())
    }

    // P-66: a bookmark tree root is labelled with the project label, so two same-named roots are told apart
    #[test]
    fn bookmark_tree_roots_use_the_project_label() {
        let projects = colliding_projects();
        let rows = bookmark_tree(
            &projects,
            &bookmarks("/home/u/projects/api/src\n/home/u/work/api/src\n"),
        );
        let roots: Vec<(&str, NodeKind)> = rows
            .iter()
            .filter(|row| row.depth == 0)
            .map(|row| (row.label.as_str(), row.kind))
            .collect();
        assert_eq!(
            roots,
            vec![
                ("projects/api", NodeKind::Root),
                ("work/api", NodeKind::Root)
            ]
        );
        assert_eq!(
            rows[1].label, "src",
            "a folder below the root keeps its folder name"
        );
    }

    // P-66: a project root with a bookmark of its own is labelled with the project label too
    #[test]
    fn a_bookmarked_root_is_labelled_with_the_project_label() {
        let projects = colliding_projects();
        let rows = bookmark_tree(&projects, &bookmarks("/home/u/work/api\n"));
        assert_eq!(rows[0].label, "work/api");
        assert_eq!(rows[0].kind, NodeKind::Bookmark);
    }

    // P-66: the gray root of a directory that no tree holds is labelled with the project label
    #[test]
    fn root_only_view_is_labelled_with_the_project_label() {
        let projects = colliding_projects();
        let view = bookmark_view(
            &projects,
            &bookmarks(""),
            &scope_dir("/home/u/work/api/src"),
        );
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].label, "work/api");
        assert_eq!(view.rows[0].path.as_str(), "/home/u/work/api");
    }

    // P-66: a session row of a project shows nothing but its folder name, and a placeholder keeps its session name
    #[test]
    fn session_rows_keep_their_labels_with_colliding_projects() {
        let projects = colliding_projects();
        let rows = session_tree(&projects, &[session("S", "/home/u/work/api/src")]);
        assert_eq!(rows[0].label, session_name(&abs("/home/u/work/api")));
        assert_eq!(rows[1].label, "src");
    }

    fn session(name: &str, dir: &str) -> SessionDir {
        session_at(name, dir, 0)
    }

    fn session_at(name: &str, dir: &str, activity: i64) -> SessionDir {
        SessionDir {
            name: name.to_string(),
            dir: abs(dir),
            activity,
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

    // FR-SESS-06, P-56: sessions that share a parent are listed most recent activity first
    #[test]
    fn siblings_are_listed_most_recent_first() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(
            &projects,
            &[
                session_at("a", "/p/app/a", 100),
                session_at("b", "/p/app/b", 300),
                session_at("c", "/p/app/c", 200),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/app", 0, None),
                ("/p/app/b", 1, Some("b")),
                ("/p/app/c", 1, Some("c")),
                ("/p/app/a", 1, Some("a")),
            ]
        );
    }

    // FR-SESS-06, P-56: siblings with equal activity are ordered by session name
    #[test]
    fn equal_activity_siblings_are_ordered_by_name() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(
            &projects,
            &[
                session_at("zulu", "/p/app/a", 100),
                session_at("alpha", "/p/app/b", 100),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/app", 0, None),
                ("/p/app/b", 1, Some("alpha")),
                ("/p/app/a", 1, Some("zulu")),
            ]
        );
    }

    // FR-SESS-06, P-54, P-56: the nesting is the same whatever the activity; only the order of siblings changes
    #[test]
    fn nesting_is_kept_and_children_are_ordered_by_activity() {
        let projects = vec![project("/p/ledge")];
        let rows = session_tree(
            &projects,
            &[
                session_at("L", "/p/ledge", 10),
                session_at("D", "/p/ledge/docs", 20),
                session_at("A", "/p/ledge/docs/adr", 50),
                session_at("S", "/p/ledge/src", 900),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/ledge", 0, Some("L")),
                ("/p/ledge/src", 1, Some("S")),
                ("/p/ledge/docs", 1, Some("D")),
                ("/p/ledge/docs/adr", 2, Some("A")),
            ]
        );
    }

    // FR-SESS-06, P-56: top-level projects are listed by their most recent session, first; each keeps its nesting
    #[test]
    fn top_level_projects_are_listed_by_their_most_recent_session() {
        let projects = vec![project("/p/app"), project("/p/lib")];
        let rows = session_tree(
            &projects,
            &[
                session_at("a", "/p/app/x", 100),
                session_at("l", "/p/lib", 500),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![
                ("/p/lib", 0, Some("l")),
                ("/p/app", 0, None),
                ("/p/app/x", 1, Some("a")),
            ]
        );
    }

    // FR-SESS-06, P-56: sessions outside every project are top-level rows, most recent first
    #[test]
    fn sessions_outside_every_project_are_ordered_by_activity() {
        let projects = vec![project("/p/app")];
        let rows = session_tree(
            &projects,
            &[
                session_at("old", "/home/u/x", 10),
                session_at("new", "/home/u/y", 99),
            ],
        );
        assert_eq!(
            session_shape(&rows),
            vec![("/home/u/y", 0, Some("new")), ("/home/u/x", 0, Some("old"))]
        );
    }
}

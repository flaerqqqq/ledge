//! Bookmark edit window state and listing (architecture 2, 5, 9.4): the folders expanded in the window (FR-EDIT-03,
//! FR-EDIT-05, D-09) and the rows the window shows for a project (FR-EDIT-01, FR-EDIT-04, FR-EDIT-06, FR-EDIT-11).
//! Pure: the child folder names come from the caller through a closure, so no file access happens here.

use super::bookmarks::Bookmarks;
use super::path::{cache_lines_text, parse_cache_lines, AbsPath};

/// FR-EDIT-05, D-09: the folders expanded in the edit window, kept globally in `bm-edit-open`. Paths are absolute,
/// so entries of different projects cannot collide.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExpandedFolders {
    entries: Vec<AbsPath>,
}

/// FR-EDIT-04: the marker shown before a folder name. A folder without subfolders has no marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// No subfolders.
    Leaf,
    /// FR-EDIT-04: subfolders, not expanded: `+`.
    Collapsed,
    /// FR-EDIT-04: subfolders, expanded: `-`.
    Expanded,
}

impl Marker {
    /// FR-EDIT-04: the marker text (empty for a leaf).
    pub fn text(self) -> &'static str {
        match self {
            Marker::Leaf => "",
            Marker::Collapsed => "+",
            Marker::Expanded => "-",
        }
    }
}

/// One row of the edit window, top-down (FR-EDIT-01). `depth` is 0 for the project root row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRow {
    pub depth: usize,
    pub path: AbsPath,
    pub name: String,
    pub marker: Marker,
    /// FR-EDIT-06: the directory is bookmarked (shown in the marker column).
    pub bookmarked: bool,
    /// The project root row; enter does nothing on it (D-15).
    pub root: bool,
    /// S-44 (Subfolders column): the number of direct visible subfolders of the folder. Subfolders of the subfolders are
    /// not counted, and hidden folders are not counted (FR-EDIT-01).
    pub subfolders: usize,
}

impl ExpandedFolders {
    /// Reads the `bm-edit-open` text: one path per line, cleaned and deduplicated like the other cache files (D-06).
    pub fn parse(text: &str) -> ExpandedFolders {
        ExpandedFolders {
            entries: parse_cache_lines(text),
        }
    }

    /// FR-EDIT-05: the file text, one path per line.
    pub fn to_file_text(&self) -> String {
        cache_lines_text(&self.entries)
    }

    /// The expanded folders in stored order.
    pub fn entries(&self) -> &[AbsPath] {
        &self.entries
    }

    /// FR-EDIT-03: true when `dir` is expanded.
    pub fn is_expanded(&self, dir: &AbsPath) -> bool {
        self.entries.contains(dir)
    }

    /// FR-EDIT-03: marks `dir` expanded. Returns `false` when it already was.
    pub fn expand(&mut self, dir: AbsPath) -> bool {
        if self.is_expanded(&dir) {
            return false;
        }
        self.entries.push(dir);
        true
    }

    /// FR-EDIT-03, D-09: removes only the entry of `dir`. The expanded state of its descendants is kept for the next
    /// time the folder opens. Returns `false` when `dir` was not expanded.
    pub fn collapse(&mut self, dir: &AbsPath) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry != dir);
        self.entries.len() != before
    }

    /// D-09: the same state without the folders for which `is_dir` is false. Used when the file is written.
    pub fn pruned(&self, is_dir: &dyn Fn(&AbsPath) -> bool) -> ExpandedFolders {
        ExpandedFolders {
            entries: self
                .entries
                .iter()
                .filter(|dir| is_dir(dir))
                .cloned()
                .collect(),
        }
    }
}

/// FR-EDIT-01, FR-EDIT-06, FR-EDIT-11, FR-EDIT-04: the rows the edit window shows for the project at `root`.
///
/// The root row comes first and is always expanded. Below it, the visible subfolders of each expanded folder are
/// listed in name order, depth first. Folders whose names start with "." are skipped. A project with no visible
/// subfolders gives no rows at all, so the window does not open (FR-EDIT-11).
pub fn visible_rows(
    root: &AbsPath,
    expanded: &ExpandedFolders,
    bookmarks: &Bookmarks,
    children_of: &dyn Fn(&AbsPath) -> Vec<String>,
) -> Vec<EditRow> {
    let top = subdirectories(root, children_of);
    if top.is_empty() {
        return Vec::new();
    }
    let name = root
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    let mut rows = vec![EditRow {
        depth: 0,
        path: root.clone(),
        name,
        marker: Marker::Expanded,
        bookmarked: bookmarks.contains(root),
        root: true,
        subfolders: top.len(),
    }];
    push_children(&mut rows, top, 1, expanded, bookmarks, children_of);
    rows
}

/// Appends `children` and, for each expanded one, its own visible children below it.
fn push_children(
    rows: &mut Vec<EditRow>,
    children: Vec<(AbsPath, String)>,
    depth: usize,
    expanded: &ExpandedFolders,
    bookmarks: &Bookmarks,
    children_of: &dyn Fn(&AbsPath) -> Vec<String>,
) {
    for (path, name) in children {
        let below = subdirectories(&path, children_of);
        let subfolders = below.len();
        let open = !below.is_empty() && expanded.is_expanded(&path);
        let marker = match (below.is_empty(), open) {
            (true, _) => Marker::Leaf,
            (false, true) => Marker::Expanded,
            (false, false) => Marker::Collapsed,
        };
        let bookmarked = bookmarks.contains(&path);
        rows.push(EditRow {
            depth,
            path,
            name,
            marker,
            bookmarked,
            root: false,
            subfolders,
        });
        if open {
            push_children(rows, below, depth + 1, expanded, bookmarks, children_of);
        }
    }
}

/// FR-EDIT-01: the visible subfolders of `dir` (names not starting with "."), sorted by name, with their paths.
fn subdirectories(
    dir: &AbsPath,
    children_of: &dyn Fn(&AbsPath) -> Vec<String>,
) -> Vec<(AbsPath, String)> {
    let mut names: Vec<String> = children_of(dir)
        .into_iter()
        .filter(|name| !name.is_empty() && !name.starts_with('.'))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| (AbsPath::clean(&name, dir), name))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{visible_rows, EditRow, ExpandedFolders, Marker};
    use crate::domain::bookmarks::Bookmarks;
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    /// A folder listing: each directory path with the names of its child folders.
    fn listing(entries: &[(&str, &[&str])]) -> impl Fn(&AbsPath) -> Vec<String> {
        let map: Vec<(String, Vec<String>)> = entries
            .iter()
            .map(|(dir, names)| {
                (
                    dir.to_string(),
                    names.iter().map(|name| name.to_string()).collect(),
                )
            })
            .collect();
        move |dir: &AbsPath| {
            map.iter()
                .find(|(path, _)| path == dir.as_str())
                .map(|(_, names)| names.clone())
                .unwrap_or_default()
        }
    }

    /// The project `/p/app`: `src` (with `main`), `docs`, and the hidden `.git`.
    fn project_listing() -> impl Fn(&AbsPath) -> Vec<String> {
        listing(&[
            ("/p/app", &["src", ".git", "docs"]),
            ("/p/app/src", &["main", ".cache"]),
            ("/p/app/src/main", &[]),
            ("/p/app/docs", &[]),
        ])
    }

    fn expanded(paths: &[&str]) -> ExpandedFolders {
        let mut state = ExpandedFolders::default();
        for path in paths {
            state.expand(abs(path));
        }
        state
    }

    /// (name, marker text, depth) of every row, for compact assertions.
    fn shape(rows: &[EditRow]) -> Vec<(String, &'static str, usize)> {
        rows.iter()
            .map(|row| (row.name.clone(), row.marker.text(), row.depth))
            .collect()
    }

    // FR-EDIT-01: the subfolders of the project are listed in name order; hidden folders are skipped
    #[test]
    fn subfolders_are_listed_in_name_order_and_hidden_ones_skipped() {
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&[]),
            &Bookmarks::default(),
            &project_listing(),
        );
        assert_eq!(
            shape(&rows),
            vec![
                ("app".to_string(), "-", 0),
                ("docs".to_string(), "", 1),
                ("src".to_string(), "+", 1),
            ]
        );
    }

    // FR-EDIT-01: a folder's children are shown only while the folder is expanded
    #[test]
    fn children_appear_only_under_an_expanded_folder() {
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&["/p/app/src"]),
            &Bookmarks::default(),
            &project_listing(),
        );
        assert_eq!(
            shape(&rows),
            vec![
                ("app".to_string(), "-", 0),
                ("docs".to_string(), "", 1),
                ("src".to_string(), "-", 1),
                ("main".to_string(), "", 2),
            ]
        );
    }

    // FR-EDIT-03: expanding adds the folder; collapsing removes only its entry and keeps its descendants
    #[test]
    fn collapse_removes_only_that_folder_entry() {
        let mut state = expanded(&["/p/app", "/p/app/src", "/p/app/src/main"]);
        assert!(state.collapse(&abs("/p/app/src")));
        assert!(!state.is_expanded(&abs("/p/app/src")));
        assert!(state.is_expanded(&abs("/p/app/src/main")));
        assert!(!state.collapse(&abs("/p/app/src")));
    }

    // FR-EDIT-03: expanding a folder twice keeps one entry
    #[test]
    fn expand_twice_keeps_one_entry() {
        let mut state = ExpandedFolders::default();
        assert!(state.expand(abs("/p/app")));
        assert!(!state.expand(abs("/p/app")));
        assert_eq!(state.entries().len(), 1);
    }

    // FR-EDIT-04: a folder with subfolders is "+" when collapsed and "-" when expanded; a leaf has no marker
    #[test]
    fn markers_are_plus_minus_or_nothing() {
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&["/p/app/src"]),
            &Bookmarks::default(),
            &project_listing(),
        );
        let markers: Vec<Marker> = rows.iter().map(|row| row.marker).collect();
        assert_eq!(
            markers,
            vec![
                Marker::Expanded,
                Marker::Leaf,
                Marker::Expanded,
                Marker::Leaf
            ]
        );
        assert_eq!(Marker::Collapsed.text(), "+");
        assert_eq!(Marker::Expanded.text(), "-");
        assert_eq!(Marker::Leaf.text(), "");
    }

    // FR-EDIT-05: the file text has one path per line and reads back to the same state
    #[test]
    fn file_text_round_trips() {
        let state = expanded(&["/p/app", "/p/app/src"]);
        assert_eq!(state.to_file_text(), "/p/app\n/p/app/src\n");
        assert_eq!(ExpandedFolders::parse(&state.to_file_text()), state);
    }

    // A-09, D-06: relative lines are skipped, trailing slashes are cleaned and repeats keep the first entry
    #[test]
    fn parse_cleans_and_deduplicates() {
        let state = ExpandedFolders::parse("app/src\n/p/app/\n/p/app\n/p/app/src\n");
        let texts: Vec<&str> = state.entries().iter().map(AbsPath::as_str).collect();
        assert_eq!(texts, vec!["/p/app", "/p/app/src"]);
    }

    // D-09: pruning keeps only the folders that still exist, in stored order
    #[test]
    fn pruned_drops_folders_that_no_longer_exist() {
        let state = expanded(&["/p/app", "/p/gone", "/p/app/src"]);
        let live = state.pruned(&|dir: &AbsPath| dir.as_str() != "/p/gone");
        let texts: Vec<&str> = live.entries().iter().map(AbsPath::as_str).collect();
        assert_eq!(texts, vec!["/p/app", "/p/app/src"]);
    }

    // FR-EDIT-06: a bookmarked folder carries the bookmark flag; the others do not
    #[test]
    fn bookmarked_rows_carry_the_flag() {
        let bookmarks = Bookmarks::parse("/p/app/docs\n");
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&[]),
            &bookmarks,
            &project_listing(),
        );
        let flags: Vec<(&str, bool)> = rows
            .iter()
            .map(|row| (row.name.as_str(), row.bookmarked))
            .collect();
        assert_eq!(flags, vec![("app", false), ("docs", true), ("src", false)]);
    }

    // FR-EDIT-11: a project without subfolders gives no rows, so no window opens
    #[test]
    fn project_without_subfolders_gives_no_rows() {
        let listing = listing(&[("/p/app", &[])]);
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&[]),
            &Bookmarks::default(),
            &listing,
        );
        assert!(rows.is_empty());
    }

    // FR-EDIT-11: a project whose only subfolders are hidden also gives no rows
    #[test]
    fn project_with_only_hidden_subfolders_gives_no_rows() {
        let listing = listing(&[("/p/app", &[".git", ".cache"])]);
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&[]),
            &Bookmarks::default(),
            &listing,
        );
        assert!(rows.is_empty());
    }

    // FR-EDIT-01: the project root row is the first row, flagged as the root
    #[test]
    fn first_row_is_the_project_root() {
        let rows = visible_rows(
            &abs("/p/app"),
            &expanded(&[]),
            &Bookmarks::default(),
            &project_listing(),
        );
        assert!(rows[0].root);
        assert_eq!(rows[0].path, abs("/p/app"));
        assert!(rows[1..].iter().all(|row| !row.root));
    }
}

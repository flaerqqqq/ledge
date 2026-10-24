//! Bookmark commands (architecture 2, 2.3): `bm add [DIR]`, `bm remove DIR`, `bm list` (FR-BM-02..10) and the bookmark
//! edit window `bm edit [DIR]` (FR-EDIT-02..11). The rules are in `domain::bookmarks` and `domain::bm_edit`, the project
//! list in `app::load_projects`, the file work in `app::store`.

use crate::app::actions::current_session;
use crate::app::pickers::{backend_fatal, bookmark_tree_picker, run_in_terminal};
use crate::app::{load_projects, store, Ctx};
use crate::domain::bm_edit::{visible_rows, EditRow, ExpandedFolders, Marker};
use crate::domain::bookmarks::{check_candidate, status, BookmarkStatus, Bookmarks, Refusal};
use crate::domain::path::AbsPath;
use crate::domain::profile_rules::ProfileOpt;
use crate::domain::projects::{project_containing, Project};
use crate::domain::sessions::session_directory;
use crate::domain::tree::TreeScope;
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::tui::columns::ColumnSpec;
use crate::tui::engine::{PickerHandler, PickerResult, Step};
use crate::tui::keys::Action;
use crate::tui::picker::{Chrome, PickerModel, PickerRow};
use crate::tui::render;
use std::path::Path;

/// FR-EDIT-06: the bookmark marker shown in the marker column of a bookmarked row.
const BOOKMARK_MARK: &str = "★";

/// FR-BM-02..07: bookmarks the directory `dir` (the working directory when `None`). The argument is cleaned against the
/// working directory (D-06). A missing directory gives `Directory '<dir>' not found.` with the argument as typed (A-19).
/// A bookmark already present changes nothing (FR-BM-06). The project rules refuse a project itself (FR-BM-04) and a
/// directory outside every project (FR-BM-03).
pub fn add(ctx: &Ctx<'_>, dir: Option<&str>) -> Result<Outcome> {
    let typed = dir.unwrap_or(ctx.cwd.as_str());
    let path = AbsPath::clean(typed, &ctx.cwd);
    if !ctx.fs.is_dir(Path::new(path.as_str())) {
        return Err(LedgeError::Fatal(messages::directory_not_found(typed)));
    }
    let mut bookmarks = read_bookmarks(ctx)?;
    if bookmarks.contains(&path) {
        return Ok(Outcome::Info(messages::already_bookmarked(path.as_str())));
    }
    let projects = project_list(ctx)?;
    match check_candidate(&path, &projects) {
        Ok(()) => {}
        Err(Refusal::IsProject) => {
            return Err(LedgeError::Fatal(messages::bookmark_is_project(
                path.as_str(),
            )))
        }
        Err(Refusal::OutsideProject) => {
            return Err(LedgeError::Fatal(messages::bookmark_outside_project(
                path.as_str(),
            )))
        }
    }
    bookmarks.add(path.clone());
    write_bookmarks(ctx, &bookmarks)?;
    Ok(Outcome::Info(messages::bookmarked(path.as_str())))
}

/// FR-BM-09, FR-BM-10: removes the bookmark of `dir`. The directory need not exist (D-06). A directory that is not
/// bookmarked gives `'<path>' is not a bookmark.`.
pub fn remove(ctx: &Ctx<'_>, dir: &str) -> Result<Outcome> {
    let path = AbsPath::clean(dir, &ctx.cwd);
    let mut bookmarks = read_bookmarks(ctx)?;
    if !bookmarks.remove(&path) {
        return Err(LedgeError::Fatal(messages::not_a_bookmark(path.as_str())));
    }
    write_bookmarks(ctx, &bookmarks)?;
    Ok(Outcome::Info(messages::removed_bookmark(path.as_str())))
}

/// FR-BM-08: one bookmark per line in stored order, each with its status word (`ok`, `missing` or `outside project`).
/// An empty list prints `No bookmarks.`.
pub fn list(ctx: &Ctx<'_>) -> Result<Outcome> {
    let bookmarks = read_bookmarks(ctx)?;
    if bookmarks.entries().is_empty() {
        return Ok(Outcome::Info(messages::no_bookmarks().to_string()));
    }
    let projects = project_list(ctx)?;
    let rows: Vec<String> = bookmarks
        .entries()
        .iter()
        .map(|path| {
            let exists = ctx.fs.is_dir(Path::new(path.as_str()));
            let word = match status(path, exists, &projects) {
                BookmarkStatus::Ok => messages::BOOKMARK_STATUS_OK,
                BookmarkStatus::Missing => messages::BOOKMARK_STATUS_MISSING,
                BookmarkStatus::OutsideProject => messages::BOOKMARK_STATUS_OUTSIDE,
            };
            messages::bookmark_list_row(word, path.as_str())
        })
        .collect();
    Ok(Outcome::Info(rows.join("\n")))
}

/// FR-EDIT-08, FR-EDIT-09, FR-EDIT-10, FR-EDIT-11: opens the bookmark edit window for the project that contains `dir`
/// (the working directory when `None`), or the directory of the current session with `session_scope`. A directory
/// outside every project is an error. A project without subfolders opens nothing (exit 0, no output). Escape closes
/// the window with no output (footer "esc done").
pub fn edit(ctx: &Ctx<'_>, dir: Option<&str>, session_scope: bool) -> Result<Outcome> {
    let target = if session_scope {
        session_dir(ctx)?
    } else {
        AbsPath::clean(dir.unwrap_or(ctx.cwd.as_str()), &ctx.cwd)
    };
    let projects = project_list(ctx)?;
    let Some(project) = project_containing(&target, &projects) else {
        return Err(LedgeError::Fatal(messages::edit_outside_project(
            target.as_str(),
        )));
    };
    let mut handler = EditHandler {
        ctx,
        project: project.path.clone(),
        expanded: read_expanded(ctx)?,
        bookmarks: read_bookmarks(ctx)?,
    };
    let rows = handler.rows();
    if rows.is_empty() {
        return Ok(Outcome::Done);
    }
    let model = PickerModel::new(rows, edit_chrome(&project.name));
    match run_in_terminal(ctx, model, &mut handler)? {
        PickerResult::Closed(outcome) => Ok(outcome),
        PickerResult::Cancelled
        | PickerResult::Chosen(_)
        | PickerResult::Profile(_)
        | PickerResult::ProfileNode(_) => Ok(Outcome::Done),
    }
}

/// FR-TREE-05..08: `bm tree [DIR] [--session-scope]`. A directory shows the tree that contains it; `--session-scope`
/// uses the directory of the current session (FR-TREE-06); neither shows every tree. The directory is cleaned against
/// the working directory and is not required to exist (as `bm edit`).
pub fn tree(
    ctx: &Ctx<'_>,
    dir: Option<&str>,
    session_scope: bool,
    option: &ProfileOpt,
) -> Result<Outcome> {
    let scope = if session_scope {
        TreeScope::Directory(session_dir(ctx)?)
    } else {
        match dir {
            Some(dir) => TreeScope::Directory(AbsPath::clean(dir, &ctx.cwd)),
            None => TreeScope::All,
        }
    };
    bookmark_tree_picker(ctx, option, &scope)
}

/// FR-EDIT-08, FR-EDIT-09: the directory of the session the client is in. Outside tmux, or when that session has no
/// directory, the refusal is `--session-scope requires running inside a session.` (D-16).
fn session_dir(ctx: &Ctx<'_>) -> Result<AbsPath> {
    let refusal = || LedgeError::Fatal(messages::session_scope_requires_session().to_string());
    let Some(name) = current_session(ctx)? else {
        return Err(refusal());
    };
    let sessions = ctx.backend.list_sessions().map_err(backend_fatal)?;
    sessions
        .iter()
        .find(|info| info.name == name)
        .and_then(session_directory)
        .map(|dir| AbsPath::clean(dir, &ctx.cwd))
        .ok_or_else(refusal)
}

/// FR-EDIT-05: the expanded folders of the last run; a missing file gives none expanded.
fn read_expanded(ctx: &Ctx<'_>) -> Result<ExpandedFolders> {
    store::load_expanded(ctx.fs, &ctx.dirs)
        .map_err(|error| LedgeError::Fatal(messages::could_not_read_edit_state(&error.to_string())))
}

/// Enter, left and right of the edit window (architecture 6.3, D-15). The state is the expanded folders and the
/// bookmarks; each change is written at once so the next run starts from it.
struct EditHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
    project: AbsPath,
    expanded: ExpandedFolders,
    bookmarks: Bookmarks,
}

impl EditHandler<'_, '_> {
    /// FR-EDIT-01, FR-EDIT-06: the rows of the window for the current state.
    fn rows(&self) -> Vec<PickerRow<EditRow>> {
        let fs = self.ctx.fs;
        let children = |dir: &AbsPath| fs.list_dirs(Path::new(dir.as_str())).unwrap_or_default();
        visible_rows(&self.project, &self.expanded, &self.bookmarks, &children)
            .iter()
            .map(edit_row)
            .collect()
    }

    /// FR-EDIT-02: adds the bookmark of the row, or removes it when it is bookmarked.
    fn toggle(&mut self, row: &EditRow) -> Result<()> {
        if self.bookmarks.contains(&row.path) {
            self.bookmarks.remove(&row.path);
        } else {
            self.bookmarks.add(row.path.clone());
        }
        write_bookmarks(self.ctx, &self.bookmarks)
    }

    /// FR-EDIT-05, D-09: writes the expanded folders.
    fn save_expanded(&self) -> std::result::Result<(), LedgeError> {
        store::save_expanded(self.ctx.fs, &self.ctx.dirs, &self.expanded).map_err(|error| {
            LedgeError::Fatal(messages::could_not_save_edit_state(&error.to_string()))
        })
    }

    /// FR-EDIT-03: expands a folder and keeps the window open.
    fn expand(&mut self, row: &EditRow) -> Step {
        self.expanded.expand(row.path.clone());
        self.after_state_change()
    }

    /// FR-EDIT-03: collapses a folder; only its own entry is removed (D-09).
    fn collapse(&mut self, row: &EditRow) -> Step {
        self.expanded.collapse(&row.path);
        self.after_state_change()
    }

    fn after_state_change(&self) -> Step {
        match self.save_expanded() {
            Ok(()) => Step::Continue {
                status: None,
                reload: true,
            },
            Err(error) => failed(error),
        }
    }
}

impl PickerHandler<EditRow> for EditHandler<'_, '_> {
    /// FR-EDIT-02, D-15: enter toggles the bookmark of the row without closing; the project root does nothing.
    fn choose(&mut self, row: &EditRow) -> Step {
        if row.root {
            return no_action();
        }
        match self.toggle(row) {
            Ok(()) => Step::Continue {
                status: None,
                reload: true,
            },
            Err(error) => failed(error),
        }
    }

    /// FR-EDIT-03, D-15: right expands a collapsed folder; left collapses an expanded one, and otherwise moves to the
    /// parent row. The project root is always expanded, so left does nothing on it.
    fn act(&mut self, action: Action, row: Option<&EditRow>) -> Step {
        let Some(row) = row else {
            return no_action();
        };
        match action {
            Action::Right if row.marker == Marker::Collapsed => self.expand(row),
            Action::Left if row.root => no_action(),
            Action::Left if row.marker == Marker::Expanded => self.collapse(row),
            Action::Left => Step::ToParent,
            _ => no_action(),
        }
    }

    fn reload(&mut self) -> Result<Vec<PickerRow<EditRow>>> {
        Ok(self.rows())
    }
}

/// FR-EDIT-01, FR-EDIT-06: one row of the window. Column 0 is the bookmark marker, column 1 the directory tree: the
/// indent comes from the depth, then `+` or `-` (FR-EDIT-04) and the folder name.
fn edit_row(row: &EditRow) -> PickerRow<EditRow> {
    let mark = if row.bookmarked { BOOKMARK_MARK } else { "" };
    let marker = if row.marker.text().is_empty() {
        " "
    } else {
        row.marker.text()
    };
    // FR-EDIT-13: the search matches the folder name.
    let mut picker_row = PickerRow::new(
        vec![mark.to_string(), format!("{marker} {}", row.name)],
        &row.name,
        row.clone(),
    );
    picker_row.depth = row.depth;
    picker_row.marked = row.bookmarked;
    picker_row
}

/// Architecture 6.2: the bookmark edit window. No filter (D-15), so no search column and no prompt row.
fn edit_chrome(project: &str) -> Chrome {
    let mut chrome = Chrome::new(
        &messages::edit_window_label(project),
        render::icon::BOOKMARKS,
        vec![
            ColumnSpec::fixed("", 2),
            ColumnSpec::text(messages::HEADER_DIRECTORY),
        ],
        messages::EDIT_WINDOW_FOOTER,
    );
    chrome.search_column = None;
    chrome.tree_column = Some(1);
    chrome
}

/// Keys with no effect in the edit window.
fn no_action() -> Step {
    Step::Continue {
        status: None,
        reload: false,
    }
}

/// A failed change is shown in the status line and the window stays open.
fn failed(error: LedgeError) -> Step {
    Step::Continue {
        status: Some(error.to_string()),
        reload: false,
    }
}

pub(crate) fn read_bookmarks(ctx: &Ctx<'_>) -> Result<Bookmarks> {
    store::load_bookmarks(ctx.fs, &ctx.dirs)
        .map_err(|error| LedgeError::Fatal(messages::could_not_read_bookmarks(&error.to_string())))
}

pub(crate) fn write_bookmarks(ctx: &Ctx<'_>, bookmarks: &Bookmarks) -> Result<()> {
    store::save_bookmarks(ctx.fs, &ctx.dirs, bookmarks)
        .map_err(|error| LedgeError::Fatal(messages::could_not_write_bookmarks(&error.to_string())))
}

/// FR-PICK-01, FR-CONF-07: the projects of the configured search paths. Reads the config as the open flow does.
fn project_list(ctx: &Ctx<'_>) -> Result<Vec<Project>> {
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    Ok(load_projects(ctx.fs, &config.search_paths))
}

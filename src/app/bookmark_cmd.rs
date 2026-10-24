//! Bookmark commands (architecture 2, 2.3): `bm add [DIR]`, `bm remove DIR` and `bm list` (FR-BM-02..10). The rules
//! are in `domain::bookmarks`, the project list in `app::load_projects`, the file work in `app::store`.

use crate::app::{load_projects, store, Ctx};
use crate::domain::bookmarks::{check_candidate, status, BookmarkStatus, Bookmarks, Refusal};
use crate::domain::path::AbsPath;
use crate::domain::projects::Project;
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use std::path::Path;

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

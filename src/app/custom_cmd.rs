//! Custom project commands (proposal P-67, FR-CUST-01..03): `custom add [DIR]`, `custom remove DIR` and `custom list`.
//! The list is the custom projects file (P-65, `store`); the rules are `domain::projects` (`should_record`) and
//! `domain::custom_projects`. The removal is `actions::remove_custom_project`, the same as ctrl-d in the project picker.
//! The window `custom pick` (FR-CUST-04) is `pickers::custom_picker`.

use crate::app::actions::remove_custom_project;
use crate::app::{home_of, load_projects, store, Ctx};
use crate::domain::custom_projects::CustomProjects;
use crate::domain::path::AbsPath;
use crate::domain::projects::should_record;
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use std::path::Path;

/// FR-CUST-01: records `dir` (the working directory when `None`) as a custom project. The argument is cleaned against
/// the working directory (D-06). A directory that does not exist gives `Directory '<dir>' not found.`. A directory that
/// is already listed is an info. A project (the same rule as the command-line open, `should_record`) is refused: the
/// project of its own path with the project message, and a directory inside a project or a custom project with the
/// inside message.
pub fn add(ctx: &Ctx<'_>, dir: Option<&str>) -> Result<Outcome> {
    let typed = dir.unwrap_or(ctx.cwd.as_str());
    let path = AbsPath::clean(typed, &ctx.cwd);
    if !ctx.fs.is_dir(Path::new(path.as_str())) {
        return Err(LedgeError::Fatal(messages::directory_not_found(typed)));
    }
    let text = shown(ctx, &path);
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let mut custom = read_custom(ctx)?;
    if custom.contains(&path) {
        return Ok(Outcome::Info(messages::custom_already_listed(&text)));
    }
    let projects = load_projects(ctx.fs, &config.search_paths, &custom);
    if !should_record(&path, &projects) {
        let is_project = projects.iter().any(|project| project.path == path);
        return Err(LedgeError::Fatal(if is_project {
            messages::bookmark_is_project(&text)
        } else {
            messages::custom_inside_project(&text)
        }));
    }
    custom.add(path);
    write_custom(ctx, &custom)?;
    Ok(Outcome::Info(messages::custom_added(&text)))
}

/// FR-CUST-02: removes the custom project `dir` (cleaned as `add` does, and not required to exist). Its bookmarks are
/// deleted and its managed sessions killed (`actions::remove_custom_project`). A directory that is not listed is an
/// error.
pub fn remove(ctx: &Ctx<'_>, dir: &str) -> Result<Outcome> {
    let path = AbsPath::clean(dir, &ctx.cwd);
    let text = shown(ctx, &path);
    if !remove_custom_project(ctx, &path)? {
        return Err(LedgeError::Fatal(messages::custom_not_listed(&text)));
    }
    Ok(Outcome::Info(messages::custom_removed(&text)))
}

/// FR-CUST-03: the custom project paths, one per line in stored order, with `~` for the home folder. An empty list
/// prints `No custom projects.`.
pub fn list(ctx: &Ctx<'_>) -> Result<Outcome> {
    let custom = read_custom(ctx)?;
    if custom.entries().is_empty() {
        return Ok(Outcome::Info(messages::NO_CUSTOM_PROJECTS.to_string()));
    }
    let rows: Vec<String> = custom
        .entries()
        .iter()
        .map(|path| shown(ctx, path))
        .collect();
    Ok(Outcome::Info(rows.join("\n")))
}

/// FR-HIST-03: a stored or cleaned path as the messages show it: a path under HOME starts with `~`.
fn shown(ctx: &Ctx<'_>, path: &AbsPath) -> String {
    path.display_with_home(home_of(ctx).as_ref())
}

/// P-65: the custom projects file. Unlike the picker's quiet read, a file that cannot be read is an error here.
fn read_custom(ctx: &Ctx<'_>) -> Result<CustomProjects> {
    store::load_custom_projects(ctx.fs, &ctx.dirs).map_err(|error| {
        LedgeError::Fatal(messages::could_not_read_custom_projects(&error.to_string()))
    })
}

/// P-65: writes the custom projects file. A failure is an error.
fn write_custom(ctx: &Ctx<'_>, custom: &CustomProjects) -> Result<()> {
    store::save_custom_projects(ctx.fs, &ctx.dirs, custom).map_err(|error| {
        LedgeError::Fatal(messages::could_not_write_custom_projects(
            &error.to_string(),
        ))
    })
}

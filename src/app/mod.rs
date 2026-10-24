//! Use cases (architecture 2). `Ctx` bundles the ports the open flow needs (architecture 4; the other fields
//! arrive with the slices that use them). The store and build services and the project listing helper
//! live beside it.
use crate::domain::custom_projects::CustomProjects;
use crate::domain::location::Dirs;
use crate::domain::path::AbsPath;
use crate::domain::projects::{merge_projects, project_list, Project};
use crate::messages;
use crate::ports::backend::SessionBackend;
use crate::ports::clock::Clock;
use crate::ports::console::Console;
use crate::ports::env::Env;
use crate::ports::fs::FileSystem;
use crate::ports::process::ProcessRunner;
use crate::ports::signals::Signals;
use crate::tui::terminal::TerminalFactory;
use std::path::Path;

pub mod actions;
pub mod bookmark_cmd;
pub mod build;
pub mod custom_cmd;
pub mod doctor;
pub mod history_cmd;
pub mod launch;
pub mod open;
pub mod pickers;
pub mod session_cmd;
pub mod store;

/// Architecture 4, S-11: the ports and folders a command runs with. Built once in `main` (or in a test
/// harness) and passed by reference; no command names an adapter.
pub struct Ctx<'a> {
    pub env: &'a dyn Env,
    pub fs: &'a dyn FileSystem,
    /// FR-SESS-03: the time the activity age text is computed from.
    pub clock: &'a dyn Clock,
    pub backend: &'a dyn SessionBackend,
    pub console: &'a dyn Console,
    /// External programs; the editor launch (FR-LAUNCH-*) is its first user (P-21).
    pub process: &'a dyn ProcessRunner,
    /// Resolved once from `Env` (FR-PLAT-02, FR-CONF-08).
    pub dirs: Dirs,
    /// Opens the picker screen (FR-TUI-01); the picker services are its first user (proposal P-24).
    pub terminal: &'a dyn TerminalFactory,
    /// Interrupt flags the picker loop polls (FR-TUI-06); first user: the picker services (P-24).
    pub signals: &'a dyn Signals,
    /// The working directory, cleaned and absolute.
    pub cwd: AbsPath,
}

/// FR-HIST-03: the home folder cleaned from HOME in the environment, the value `AbsPath::display_with_home` takes.
/// `None` when HOME is not set.
pub fn home_of(ctx: &Ctx<'_>) -> Option<AbsPath> {
    ctx.env
        .var("HOME")
        .map(|home| AbsPath::clean(&home, &AbsPath::root()))
}

/// FR-PICK-01, FR-CONF-07, P-65: the projects of the search paths, read through `fs`, followed by the custom projects
/// `custom`. A search path that cannot be read (missing, not a folder) contributes no projects and no message
/// (architecture A-42).
pub fn load_projects(
    fs: &dyn FileSystem,
    search_paths: &[AbsPath],
    custom: &CustomProjects,
) -> Vec<Project> {
    let listings: Vec<(AbsPath, Vec<String>)> = search_paths
        .iter()
        .map(|base| {
            let names = fs.list_dirs(Path::new(base.as_str())).unwrap_or_default();
            (base.clone(), names)
        })
        .collect();
    merge_projects(&project_list(&listings), custom)
}

/// P-65: the custom projects of the cache file. A file that cannot be read is a warning on stderr, and the list is
/// empty, so the projects of the search paths still load.
pub fn custom_projects(ctx: &Ctx<'_>) -> CustomProjects {
    store::load_custom_projects(ctx.fs, &ctx.dirs).unwrap_or_else(|error| {
        let text = messages::could_not_read_custom_projects(&error.to_string());
        ctx.console.err(&messages::warning(&text));
        CustomProjects::default()
    })
}

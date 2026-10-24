//! Use cases (architecture 2). `Ctx` bundles the ports the open flow needs (architecture 4; the other fields
//! arrive with the slices that use them). The store and build services and the project listing helper
//! live beside it.
use crate::domain::location::Dirs;
use crate::domain::path::AbsPath;
use crate::domain::projects::{project_list, Project};
use crate::ports::backend::SessionBackend;
use crate::ports::console::Console;
use crate::ports::env::Env;
use crate::ports::fs::FileSystem;
use std::path::Path;

pub mod build;
pub mod open;
pub mod store;

/// Architecture 4, S-11: the ports and folders a command runs with. Built once in `main` (or in a test
/// harness) and passed by reference; no command names an adapter.
pub struct Ctx<'a> {
    pub env: &'a dyn Env,
    pub fs: &'a dyn FileSystem,
    pub backend: &'a dyn SessionBackend,
    pub console: &'a dyn Console,
    /// Resolved once from `Env` (FR-PLAT-02, FR-CONF-08).
    pub dirs: Dirs,
    /// The working directory, cleaned and absolute.
    pub cwd: AbsPath,
}

/// FR-PICK-01, FR-CONF-07: the projects of the search paths, read through `fs`. A search path that cannot be
/// read (missing, not a folder) contributes no projects and no message (architecture A-42).
pub fn load_projects(fs: &dyn FileSystem, search_paths: &[AbsPath]) -> Vec<Project> {
    let listings: Vec<(AbsPath, Vec<String>)> = search_paths
        .iter()
        .map(|base| {
            let names = fs.list_dirs(Path::new(base.as_str())).unwrap_or_default();
            (base.clone(), names)
        })
        .collect();
    project_list(&listings)
}

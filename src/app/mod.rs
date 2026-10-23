//! Use cases (architecture 2). S-02 adds `store`; S-04 adds the project listing helper. `Ctx` arrives with the
//! slices that need every port.
use crate::domain::path::AbsPath;
use crate::domain::projects::{project_list, Project};
use crate::ports::fs::FileSystem;
use std::path::Path;

pub mod build;
pub mod store;

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

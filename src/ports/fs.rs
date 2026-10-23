//! File system port (architecture 4 and 9.3). Trait only; the real adapter is `adapters::fs::RealFs`.
//! Tests use `RealFs` on temp folders, so there is no fake.

use std::io;
use std::path::Path;

/// D-05, FR-CONF-01: every file ledge reads or writes goes through this trait.
pub trait FileSystem {
    /// Text of a file; `None` when the file does not exist.
    fn read_text(&self, path: &Path) -> io::Result<Option<String>>;
    /// D-05: writes the complete new text to a temp file in the same folder, then renames it over
    /// `path`. Creates the parent folder when it is missing (architecture 9.3).
    fn write_atomic(&self, path: &Path, text: &str) -> io::Result<()>;
    /// D-05: writes `text` only when `path` does not exist yet. Returns `false` (and changes
    /// nothing) when the file already exists.
    fn create_new(&self, path: &Path, text: &str) -> io::Result<bool>;
    /// Creates `path` and any missing parent folders; succeeds when it already exists as a folder.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    /// Follows symbolic links (D-06, F-24).
    fn is_dir(&self, path: &Path) -> bool;
    /// `true` when anything exists at `path`, following symbolic links.
    fn exists(&self, path: &Path) -> bool;
    /// Names of the directories directly inside `path`, sorted by name.
    fn list_dirs(&self, path: &Path) -> io::Result<Vec<String>>;
    /// Names of the files directly inside `path`, sorted by name.
    fn list_files(&self, path: &Path) -> io::Result<Vec<String>>;
    /// Removes the file at `path`; an error when it does not exist.
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

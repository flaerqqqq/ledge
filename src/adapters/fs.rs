//! `RealFs`: the `FileSystem` port on `std::fs` (architecture 4, 9.3).

use crate::ports::fs::FileSystem;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// D-05: real file system adapter.
pub struct RealFs;

/// Writes `text` to an open file and syncs it to disk. The file is closed when this returns.
fn write_and_sync(mut file: File, text: &str) -> io::Result<()> {
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

/// Architecture 9.3, steps 2 to 4: write and sync the temp file, then rename it over the target.
fn write_then_rename(tmp: &Path, target: &Path, text: &str) -> io::Result<()> {
    write_and_sync(File::create(tmp)?, text)?;
    fs::rename(tmp, target)
}

/// Names of the entries of `path` that satisfy `keep`, sorted. Non-UTF-8 names are skipped
/// (conventions, Structure). Symlinks are followed by `keep` (D-06).
fn names_where(path: &Path, keep: fn(&Path) -> bool) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !keep(&entry.path()) {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

impl FileSystem for RealFs {
    fn read_text(&self, path: &Path) -> io::Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn write_atomic(&self, path: &Path, text: &str) -> io::Result<()> {
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
        let dir = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        fs::create_dir_all(dir)?;
        // The pid keeps two ledge processes from sharing one temp file (architecture 9.3).
        let mut tmp_name = OsString::from(".");
        tmp_name.push(name);
        tmp_name.push(format!(".tmp-{}", std::process::id()));
        let tmp: PathBuf = dir.join(tmp_name);
        let result = write_then_rename(&tmp, path, text);
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    fn create_new(&self, path: &Path, text: &str) -> io::Result<bool> {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(file) => {
                if let Err(error) = write_and_sync(file, text) {
                    // D-05: a partly written config must not stay, so remove it. The removal result
                    // is ignored on purpose: the original write error is more useful to the caller
                    // than a secondary removal error, and the caller gets it through the return value.
                    let _ = fs::remove_file(path);
                    return Err(error);
                }
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_dirs(&self, path: &Path) -> io::Result<Vec<String>> {
        names_where(path, |p| p.is_dir())
    }

    fn list_files(&self, path: &Path) -> io::Result<Vec<String>> {
        names_where(path, |p| p.is_file())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }
}

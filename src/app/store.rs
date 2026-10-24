//! Configuration, profile, history and bookmark files: load, create on first run, read a named profile,
//! read and write the cache files (architecture 9.2 to 9.4, D-05). Reads and writes go through the ports;
//! the parsing is in `domain::config`, `domain::profile`, `domain::history` and `domain::bookmarks`.

use crate::domain::bookmarks::Bookmarks;
use crate::domain::config::{self, Config, DEFAULT_CONFIG_TEXT};
use crate::domain::history::History;
use crate::domain::location::Dirs;
use crate::domain::path::AbsPath;
use crate::domain::profile::{Profile, ProfileName};
use crate::error::{LedgeError, Result};
use crate::messages;
use crate::ports::env::Env;
use crate::ports::fs::FileSystem;
use std::io;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "config.json";
const PROFILES_FOLDER: &str = "profiles";
const HISTORY_FILE: &str = "history";
const BOOKMARKS_FILE: &str = "bookmarks";

/// FR-CONF-01, FR-CONF-06, FR-CONF-02, D-05, architecture 9.2: loads `config.json` from the config
/// folder, creating the file with the default content when it is missing and the profiles folder
/// when it is missing. A file that exists is never rewritten, so a second run changes nothing.
pub fn load_config(
    fs: &dyn FileSystem,
    env: &dyn Env,
    dirs: &Dirs,
    cwd: &AbsPath,
) -> Result<Config> {
    let config_dir = PathBuf::from(dirs.config.as_str());
    let file = config_dir.join(CONFIG_FILE);
    let file_text = file.display().to_string();

    fs.create_dir_all(&config_dir).map_err(|error| {
        fatal(messages::could_not_create_config(
            &config_dir.display().to_string(),
            &error.to_string(),
        ))
    })?;

    let text = match fs.read_text(&file) {
        Ok(Some(text)) => text,
        Ok(None) => {
            // D-03: the default search path is written literally; its folder is not created.
            fs.create_new(&file, DEFAULT_CONFIG_TEXT).map_err(|error| {
                fatal(messages::could_not_create_config(
                    &file_text,
                    &error.to_string(),
                ))
            })?;
            DEFAULT_CONFIG_TEXT.to_string()
        }
        Err(error) => {
            return Err(fatal(messages::could_not_read_config(
                &file_text,
                &error.to_string(),
            )))
        }
    };

    let profiles = config_dir.join(PROFILES_FOLDER);
    fs.create_dir_all(&profiles).map_err(|error| {
        fatal(messages::could_not_create_config(
            &profiles.display().to_string(),
            &error.to_string(),
        ))
    })?;

    let lookup = |name: &str| env.var(name);
    let home = env.var("HOME");
    config::parse(&text, &file_text, &lookup, home.as_deref(), cwd)
}

/// FR-PROF-01, FR-PROF-02, FR-PROF-06, D-13: reads the profile `name` from the profiles folder.
/// `basic` is built in, so a `profiles/basic.json` file is never read. A name that is not valid
/// gives the "not found" error without touching the disk.
pub fn load_profile(fs: &dyn FileSystem, dirs: &Dirs, name: &str) -> Result<Profile> {
    let profiles = PathBuf::from(dirs.config.as_str()).join(PROFILES_FOLDER);
    let path = profiles.join(format!("{name}.json"));
    let Some(profile_name) = ProfileName::parse(name) else {
        return Err(not_found(name, &path));
    };
    if profile_name.is_basic() {
        return Ok(Profile::basic());
    }
    match fs.read_text(&path) {
        Ok(Some(text)) => Profile::parse(&profile_name, &text),
        Ok(None) => Err(not_found(name, &path)),
        Err(error) => Err(fatal(messages::could_not_read_profile(
            name,
            &path.display().to_string(),
            &error.to_string(),
        ))),
    }
}

/// FR-PROF-13: the names of the profile files in the profiles folder, without the `.json` suffix. A missing folder
/// gives none. Names are not checked here; the profile rules skip invalid ones.
pub fn profile_files(fs: &dyn FileSystem, dirs: &Dirs) -> Vec<String> {
    let profiles = PathBuf::from(dirs.config.as_str()).join(PROFILES_FOLDER);
    fs.list_files(&profiles)
        .unwrap_or_default()
        .iter()
        .filter_map(|file| file.strip_suffix(".json"))
        .map(str::to_string)
        .collect()
}

/// FR-HIST-01, FR-HIST-02, architecture 9.4: reads the history file from the cache folder. A missing
/// file gives an empty history.
pub fn load_history(fs: &dyn FileSystem, dirs: &Dirs) -> io::Result<History> {
    let text = fs.read_text(&cache_file(dirs, HISTORY_FILE))?;
    Ok(History::parse(text.as_deref().unwrap_or("")))
}

/// FR-HIST-01, D-05: writes the whole history to the cache folder through an atomic write. The caller
/// wraps an error with the message that names the situation.
pub fn save_history(fs: &dyn FileSystem, dirs: &Dirs, history: &History) -> io::Result<()> {
    fs.write_atomic(&cache_file(dirs, HISTORY_FILE), &history.to_file_text())
}

/// FR-BM-01, architecture 9.4: reads the bookmarks file from the cache folder. A missing file gives an
/// empty list.
pub fn load_bookmarks(fs: &dyn FileSystem, dirs: &Dirs) -> io::Result<Bookmarks> {
    let text = fs.read_text(&cache_file(dirs, BOOKMARKS_FILE))?;
    Ok(Bookmarks::parse(text.as_deref().unwrap_or("")))
}

/// FR-BM-01, D-05: writes the whole bookmark list to the cache folder through an atomic write.
pub fn save_bookmarks(fs: &dyn FileSystem, dirs: &Dirs, bookmarks: &Bookmarks) -> io::Result<()> {
    fs.write_atomic(&cache_file(dirs, BOOKMARKS_FILE), &bookmarks.to_file_text())
}

/// Architecture 9.1: the config file, `<config>/config.json`.
pub fn config_path(dirs: &Dirs) -> PathBuf {
    PathBuf::from(dirs.config.as_str()).join(CONFIG_FILE)
}

/// Architecture 9.1: the history file, `<cache>/history`. FR-HIST-05 checks that it exists.
pub fn history_path(dirs: &Dirs) -> PathBuf {
    cache_file(dirs, HISTORY_FILE)
}

/// Architecture 9.1: the path of a file in the cache folder.
fn cache_file(dirs: &Dirs, name: &str) -> PathBuf {
    PathBuf::from(dirs.cache.as_str()).join(name)
}

fn not_found(name: &str, path: &Path) -> LedgeError {
    fatal(messages::profile_not_found(
        name,
        &path.display().to_string(),
    ))
}

fn fatal(text: String) -> LedgeError {
    LedgeError::Fatal(text)
}

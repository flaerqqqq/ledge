//! Configuration and profile files: load, create on first run, read a named profile (architecture 9.2,
//! D-05). Reads and creates go through the ports; the parsing is in `domain::config` and
//! `domain::profile`.

use crate::domain::config::{self, Config, DEFAULT_CONFIG_TEXT};
use crate::domain::location::Dirs;
use crate::domain::path::AbsPath;
use crate::domain::profile::{Profile, ProfileName};
use crate::error::{LedgeError, Result};
use crate::messages;
use crate::ports::env::Env;
use crate::ports::fs::FileSystem;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "config.json";
const PROFILES_FOLDER: &str = "profiles";

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

fn not_found(name: &str, path: &Path) -> LedgeError {
    fatal(messages::profile_not_found(
        name,
        &path.display().to_string(),
    ))
}

fn fatal(text: String) -> LedgeError {
    LedgeError::Fatal(text)
}

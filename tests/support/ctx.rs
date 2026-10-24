//! Harness: builds `app::Ctx` from the fakes, the real file adapter and a `TempTree` (docs/testing.md
//! "Test layout"). The XDG folders, HOME and the working directory all live under the temp folder, so no
//! test reads or writes the real home folder.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::fake_backend::FakeBackend;
use super::fake_clock::FakeClock;
use super::fake_console::FakeConsole;
use super::fake_env::FakeEnv;
use super::fake_process::FakeProcess;
use super::fake_signals::FakeSignals;
use super::fake_terminal::FakeTerminalFactory;
use super::temp::TempTree;
use ledge::adapters::fs::RealFs;
use ledge::app::Ctx;
use ledge::domain::location::Dirs;
use ledge::domain::path::AbsPath;
use std::fs;
use std::path::{Path, PathBuf};

/// The clock of every harness: Unix seconds of 2026-10-24 00:00 UTC, so activity ages are exact in tests.
pub const FIXED_NOW: i64 = 1_792_800_000;

/// The fixtures of one test. The fields are public so a test can change one before it runs the flow
/// (for example `h.env = h.env.clone().with_var("TMUX", "...")` or `h.backend = FakeBackend::new().fail_on(..)`).
pub struct Harness {
    tree: TempTree,
    pub env: FakeEnv,
    pub fs: RealFs,
    pub clock: FakeClock,
    pub backend: FakeBackend,
    pub console: FakeConsole,
    pub process: FakeProcess,
    pub terminal: FakeTerminalFactory,
    pub signals: FakeSignals,
    pub dirs: Dirs,
    pub cwd: AbsPath,
    root: String,
}

impl Harness {
    /// A fresh temp folder `name` with `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `HOME` and the working
    /// directory set under it. The working directory is `<root>/projects`, which exists.
    pub fn new(name: &str) -> Harness {
        let tree = TempTree::new(name);
        let root = tree.path().display().to_string();
        let config = format!("{root}/xdg-config");
        let cache = format!("{root}/xdg-cache");
        let home = format!("{root}/home");
        let projects = format!("{root}/projects");
        fs::create_dir_all(&projects).expect("create projects folder");
        let env = FakeEnv::new()
            .with_var("XDG_CONFIG_HOME", &config)
            .with_var("XDG_CACHE_HOME", &cache)
            .with_var("HOME", &home)
            .with_cwd(&projects);
        let dirs = Dirs::resolve(Some(&config), Some(&cache), Some(&home)).expect("dirs");
        Harness {
            tree,
            env,
            fs: RealFs,
            clock: FakeClock::at(FIXED_NOW),
            backend: FakeBackend::new(),
            console: FakeConsole::new(),
            process: FakeProcess::new(),
            terminal: FakeTerminalFactory::new(),
            signals: FakeSignals::none(),
            dirs,
            cwd: AbsPath::clean(&projects, &AbsPath::root()),
            root,
        }
    }

    /// The application context over these fixtures.
    pub fn ctx(&self) -> Ctx<'_> {
        Ctx {
            env: &self.env,
            fs: &self.fs,
            clock: &self.clock,
            backend: &self.backend,
            console: &self.console,
            process: &self.process,
            terminal: &self.terminal,
            signals: &self.signals,
            dirs: self.dirs.clone(),
            cwd: self.cwd.clone(),
        }
    }

    /// The temp folder of the test.
    pub fn root(&self) -> &Path {
        self.tree.path()
    }

    /// Creates `<root>/projects/<name>` and returns its path text.
    pub fn project(&self, name: &str) -> String {
        let path = format!("{}/projects/{name}", self.root);
        fs::create_dir_all(&path).expect("create project folder");
        path
    }

    /// Writes `config.json` in the config folder.
    pub fn write_config(&self, text: &str) {
        self.write_under(
            &PathBuf::from(self.dirs.config.as_str()).join("config.json"),
            text,
        );
    }

    /// Writes `profiles/<name>.json` in the config folder.
    pub fn write_profile(&self, name: &str, text: &str) {
        let path = PathBuf::from(self.dirs.config.as_str())
            .join("profiles")
            .join(format!("{name}.json"));
        self.write_under(&path, text);
    }

    /// The history file text, or `None` when there is no file.
    pub fn history_text(&self) -> Option<String> {
        fs::read_to_string(PathBuf::from(self.dirs.cache.as_str()).join("history")).ok()
    }

    /// The custom projects file text (P-65), or `None` when there is no file.
    pub fn custom_projects_text(&self) -> Option<String> {
        fs::read_to_string(PathBuf::from(self.dirs.cache.as_str()).join("custom-projects")).ok()
    }

    /// Writes the custom projects file (P-65) in the cache folder, as a hand-edited file would be.
    pub fn write_custom_projects(&self, text: &str) {
        self.write_under(
            &PathBuf::from(self.dirs.cache.as_str()).join("custom-projects"),
            text,
        );
    }

    /// Makes the cache folder unusable: a regular file sits where the folder should be, so neither the
    /// history read nor its write can succeed (FR-OPEN-11).
    pub fn break_cache_folder(&self) {
        let parent = PathBuf::from(self.dirs.cache.as_str())
            .parent()
            .map(Path::to_path_buf)
            .expect("cache folder has a parent");
        fs::create_dir_all(&parent).expect("create cache parent");
        fs::write(self.dirs.cache.as_str(), "not a folder").expect("write blocking file");
    }

    fn write_under(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("file has a parent")).expect("create folder");
        fs::write(path, text).expect("write file");
    }
}

//! FakeEnv: map-backed `Env` (docs/testing.md "Test layout"). No real environment is read.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::ports::env::Env;
use std::collections::BTreeMap;
use std::io;

/// A map of variables and a fixed working directory.
#[derive(Debug, Clone)]
pub struct FakeEnv {
    vars: BTreeMap<String, String>,
    cwd: String,
}

impl FakeEnv {
    pub fn new() -> Self {
        FakeEnv {
            vars: BTreeMap::new(),
            cwd: "/".to_string(),
        }
    }

    /// Sets a variable; an empty value is stored as set, as the real environment does.
    pub fn with_var(mut self, name: &str, value: &str) -> Self {
        self.vars.insert(name.to_string(), value.to_string());
        self
    }

    pub fn with_cwd(mut self, cwd: &str) -> Self {
        self.cwd = cwd.to_string();
        self
    }
}

impl Default for FakeEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl Env for FakeEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }

    fn cwd(&self) -> io::Result<String> {
        Ok(self.cwd.clone())
    }
}

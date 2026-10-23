//! `RealEnv`: the `Env` port on `std::env` (architecture 4).

use crate::ports::env::Env;
use std::io;

/// FR-PLAT-02, FR-CONF-08: reads the process environment and working directory.
pub struct RealEnv;

impl Env for RealEnv {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn cwd(&self) -> io::Result<String> {
        std::env::current_dir()?
            .into_os_string()
            .into_string()
            .map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "working directory is not UTF-8")
            })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::RealEnv;
    use crate::ports::env::Env;
    use std::path::Path;

    // FR-CONF-08: the working directory is reported as an absolute path
    #[test]
    fn cwd_is_absolute() {
        let cwd = RealEnv.cwd().expect("cwd is readable");
        assert!(Path::new(&cwd).is_absolute(), "{cwd} is not absolute");
    }
}

//! `SystemProcess`: the `ProcessRunner` port on `std::process` and the `PATH` (architecture 4).

use crate::ports::process::{Captured, ProcessRunner};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// FR-PLAT-01, FR-CMD-10: runs real programs.
pub struct SystemProcess;

impl ProcessRunner for SystemProcess {
    fn find_program(&self, program: &str) -> bool {
        let Some(path) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&path).any(|dir| is_executable(&dir.join(program)))
    }

    fn capture(&self, program: &str, args: &[String]) -> io::Result<Captured> {
        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()?;
        Ok(Captured {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn inherit(&self, program: &str, args: &[String]) -> io::Result<i32> {
        let status = Command::new(program).args(args).status()?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// `true` for a regular file with an execute bit set.
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

//! Process port (architecture 4, section 7.1): runs external programs. Trait only; the real adapter is
//! `adapters::process::SystemProcess`, the fake is `tests/support/fake_process.rs`. Created by S-09 (P-6).

use std::io;

/// What a captured program produced. `status` is -1 when the program ended without an exit code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Captured {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// FR-PLAT-01, FR-CMD-10: external programs (tmux, and later setsid and the editors).
pub trait ProcessRunner {
    /// `true` when `program` is found on `PATH` as an executable file.
    fn find_program(&self, program: &str) -> bool;
    /// Runs `program` with `args`, waits, and returns its status and captured output. Stdin is closed.
    /// A program that cannot be started gives an error of kind `NotFound`.
    fn capture(&self, program: &str, args: &[String]) -> io::Result<Captured>;
    /// Runs `program` with the terminal inherited (for example `tmux attach-session`), waits, and
    /// returns the exit status (-1 when there is none).
    fn inherit(&self, program: &str, args: &[String]) -> io::Result<i32>;
}

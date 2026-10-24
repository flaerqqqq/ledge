//! FakeProcess: scripted `ProcessRunner` (docs/testing.md "Test layout"). It records every call and returns
//! the scripted reply for the tmux subcommand (the first argument after an optional `-L <server>`). An
//! unscripted call returns status 0 and empty output. Programs listed with `absent` are not on PATH and
//! fail to start, as if missing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::ports::process::{Captured, ProcessRunner};
use std::cell::RefCell;
use std::io;

/// One recorded call: the program, its arguments, whether it inherited the terminal, and whether it was
/// a detached spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub program: String,
    pub args: Vec<String>,
    pub inherit: bool,
    pub detached: bool,
}

pub struct FakeProcess {
    replies: RefCell<Vec<(String, Captured)>>,
    inherit_status: i32,
    missing: bool,
    absent: Vec<String>,
    calls: RefCell<Vec<Call>>,
}

impl FakeProcess {
    pub fn new() -> Self {
        Self {
            replies: RefCell::new(Vec::new()),
            inherit_status: 0,
            missing: false,
            absent: Vec::new(),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Every program is missing: `find_program` is false and `capture` fails with `NotFound`.
    pub fn missing() -> Self {
        Self {
            missing: true,
            ..Self::new()
        }
    }

    /// The named programs are missing: `find_program` is false and starting them fails with `NotFound`.
    pub fn absent(mut self, programs: &[&str]) -> Self {
        self.absent = programs.iter().map(|program| program.to_string()).collect();
        self
    }

    /// The reply for the tmux subcommand `subcommand` (the first argument after `-L <server>`).
    pub fn reply(self, subcommand: &str, status: i32, stdout: &str, stderr: &str) -> Self {
        self.replies.borrow_mut().push((
            subcommand.to_string(),
            Captured {
                status,
                stdout: stdout.to_string(),
                stderr: stderr.to_string(),
            },
        ));
        self
    }

    /// The status `inherit` returns.
    pub fn inherit_status(mut self, status: i32) -> Self {
        self.inherit_status = status;
        self
    }

    /// Every call so far, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }
}

impl Default for FakeProcess {
    fn default() -> Self {
        Self::new()
    }
}

/// The subcommand of a tmux argv: the first argument that is not `-L` or its value.
pub fn subcommand(args: &[String]) -> String {
    let rest = match args {
        [flag, _server, rest @ ..] if flag == "-L" => rest,
        all => all,
    };
    rest.first().cloned().unwrap_or_default()
}

impl FakeProcess {
    /// `true` when starting `program` must fail: every program when `missing`, or the named absent ones.
    fn is_missing(&self, program: &str) -> bool {
        self.missing || self.absent.iter().any(|name| name == program)
    }
}

impl ProcessRunner for FakeProcess {
    fn find_program(&self, program: &str) -> bool {
        !self.is_missing(program)
    }

    fn capture(&self, program: &str, args: &[String]) -> io::Result<Captured> {
        self.calls.borrow_mut().push(Call {
            program: program.to_string(),
            args: args.to_vec(),
            inherit: false,
            detached: false,
        });
        if self.is_missing(program) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
        let sub = subcommand(args);
        let replies = self.replies.borrow();
        Ok(replies
            .iter()
            .find(|(name, _)| *name == sub)
            .map(|(_, reply)| reply.clone())
            .unwrap_or_default())
    }

    fn inherit(&self, program: &str, args: &[String]) -> io::Result<i32> {
        self.calls.borrow_mut().push(Call {
            program: program.to_string(),
            args: args.to_vec(),
            inherit: true,
            detached: false,
        });
        if self.is_missing(program) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
        Ok(self.inherit_status)
    }

    fn spawn_detached(&self, program: &str, args: &[String]) -> io::Result<()> {
        self.calls.borrow_mut().push(Call {
            program: program.to_string(),
            args: args.to_vec(),
            inherit: false,
            detached: true,
        });
        if self.is_missing(program) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
        Ok(())
    }
}

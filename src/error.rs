//! Error and outcome types (architecture 8.1).
use std::fmt;

/// FR-CMD-07: every error exits 1 and prints `Error: <text>` on stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgeError {
    Fatal(String),
}

impl fmt::Display for LedgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgeError::Fatal(text) => f.write_str(text),
        }
    }
}

/// Result of a command that did not fail (architecture 8.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Info(String),
    Cancelled,
    Interrupted(i32),
    /// FR-DOC-08: a report printed on stdout (no prefix) that exits with the given code (doctor, proposal P-47).
    Report(String, i32),
}

pub type Result<T> = std::result::Result<T, LedgeError>;

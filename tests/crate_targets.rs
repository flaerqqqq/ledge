//! S-00 acceptance: the package builds as a library (`ledge`) and a binary (`ledge`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::error::{LedgeError, Outcome};
use std::path::Path;

// FR-CMD-07 (types the exit mapping is built on are reachable from the library)
#[test]
fn library_exposes_error_and_outcome_types() {
    let outcomes = [
        Outcome::Done,
        Outcome::Info(String::new()),
        Outcome::Cancelled,
        Outcome::Interrupted(130),
    ];
    assert_eq!(outcomes.len(), 4);
    let error = LedgeError::Fatal("text".to_string());
    assert_eq!(error, LedgeError::Fatal("text".to_string()));
}

// S-00 acceptance: the binary target is built alongside the library
#[test]
fn binary_target_is_built() {
    let bin = Path::new(env!("CARGO_BIN_EXE_ledge"));
    assert!(bin.is_file(), "binary missing at {}", bin.display());
}

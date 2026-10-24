//! FakeConsole: records stdout and stderr lines (docs/testing.md "Test layout"). Lines are stored without
//! the newline, as the caller passed them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::ports::console::Console;
use std::cell::RefCell;

#[derive(Default)]
pub struct FakeConsole {
    out: RefCell<Vec<String>>,
    err: RefCell<Vec<String>>,
}

impl FakeConsole {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stdout lines so far, in order.
    pub fn out_lines(&self) -> Vec<String> {
        self.out.borrow().clone()
    }

    /// Stderr lines so far, in order.
    pub fn err_lines(&self) -> Vec<String> {
        self.err.borrow().clone()
    }
}

impl Console for FakeConsole {
    fn out(&self, text: &str) {
        self.out.borrow_mut().push(text.to_string());
    }

    fn err(&self, text: &str) {
        self.err.borrow_mut().push(text.to_string());
    }
}

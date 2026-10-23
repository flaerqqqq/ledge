//! FakeSignals: scripted `Signals` (docs/testing.md "Test layout"). It reports one interrupt on a
//! chosen call of `take_interrupt`, then nothing. The engine calls it on every poll, so the call
//! number is the poll number.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::ports::signals::Signals;
use std::cell::Cell;

pub struct FakeSignals {
    interrupt_on_call: Option<usize>,
    code: i32,
    calls: Cell<usize>,
}

impl FakeSignals {
    /// No signal arrives.
    pub fn none() -> Self {
        Self {
            interrupt_on_call: None,
            code: 0,
            calls: Cell::new(0),
        }
    }

    /// The `call`-th check (1-based) reports an interrupt with exit `code`.
    pub fn interrupt_on_call(call: usize, code: i32) -> Self {
        Self {
            interrupt_on_call: Some(call),
            code,
            calls: Cell::new(0),
        }
    }
}

impl Signals for FakeSignals {
    fn take_interrupt(&self) -> Option<i32> {
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if self.interrupt_on_call == Some(call) {
            Some(self.code)
        } else {
            None
        }
    }
}

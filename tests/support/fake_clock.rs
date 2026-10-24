//! FakeClock: settable `Clock` (docs/testing.md "Test layout"). The time is fixed until a test sets it, so the
//! activity age text is deterministic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::ports::clock::Clock;
use std::cell::Cell;

pub struct FakeClock {
    now: Cell<i64>,
}

impl FakeClock {
    /// A clock that reads `now` (Unix seconds) until it is set again.
    pub fn at(now: i64) -> Self {
        Self {
            now: Cell::new(now),
        }
    }

    pub fn set(&self, now: i64) {
        self.now.set(now);
    }
}

impl Clock for FakeClock {
    fn now_secs(&self) -> i64 {
        self.now.get()
    }
}

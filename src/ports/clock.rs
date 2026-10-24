//! Clock port (architecture 4): the current time in Unix seconds, used for the activity age text (FR-SESS-03).
//! The adapter is `adapters::clock::SystemClock`, the fake is `FakeClock` in tests/support.

/// FR-SESS-03: the current time as Unix seconds.
pub trait Clock {
    fn now_secs(&self) -> i64;
}

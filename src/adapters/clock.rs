//! SystemClock: the real `Clock` port (architecture 4). Reads the system time once per call.

use crate::ports::clock::Clock;
use std::time::{SystemTime, UNIX_EPOCH};

/// FR-SESS-03: the system time in Unix seconds. A clock before 1970 reads as 0.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }
}

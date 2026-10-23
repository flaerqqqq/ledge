//! SignalFlags: SIGINT, SIGHUP and SIGTERM as flags (architecture 6.5, FR-TUI-06). The OS handler only
//! stores the signal number; the engine reads it on its next poll and returns the interrupt path.

use crate::ports::signals::Signals;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::flag;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// FR-TUI-06: the signals that lead to the interrupt path.
const INTERRUPTS: [i32; 3] = [SIGINT, SIGHUP, SIGTERM];

pub struct SignalFlags {
    received: Arc<AtomicUsize>,
}

impl SignalFlags {
    /// Registers SIGINT, SIGHUP and SIGTERM. Fails when the OS refuses a registration.
    pub fn new() -> io::Result<Self> {
        let received = Arc::new(AtomicUsize::new(0));
        for signal in INTERRUPTS {
            flag::register_usize(signal, Arc::clone(&received), signal as usize)?;
        }
        Ok(Self { received })
    }
}

impl Signals for SignalFlags {
    fn take_interrupt(&self) -> Option<i32> {
        let signal = self.received.swap(0, Ordering::SeqCst);
        exit_code(signal)
    }
}

/// FR-TUI-06, architecture 8.2: the exit code of a received signal is 128 plus its number
/// (SIGINT 130, SIGHUP 129, SIGTERM 143). `0` (none received) and other signals give `None`.
pub fn exit_code(signal: usize) -> Option<i32> {
    let signal = i32::try_from(signal).ok()?;
    INTERRUPTS.contains(&signal).then_some(128 + signal)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::exit_code;

    // FR-TUI-06, architecture 8.2: SIGINT 130, SIGHUP 129, SIGTERM 143
    #[test]
    fn interrupt_signals_map_to_their_exit_codes() {
        assert_eq!(exit_code(2), Some(130));
        assert_eq!(exit_code(1), Some(129));
        assert_eq!(exit_code(15), Some(143));
    }

    // FR-TUI-06: nothing received, and signals that do not interrupt a picker
    #[test]
    fn nothing_received_and_other_signals_are_not_interrupts() {
        assert_eq!(exit_code(0), None);
        assert_eq!(exit_code(10), None);
    }
}

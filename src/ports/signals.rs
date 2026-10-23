//! Signals port (architecture 4, section 6.5): a flag the engine polls. The adapter is
//! `adapters::signals::SignalFlags`; the fake is `FakeSignals` in tests/support.

/// FR-TUI-06: the exit code of a signal received since the last call, once. `None` when none arrived.
pub trait Signals {
    fn take_interrupt(&self) -> Option<i32>;
}

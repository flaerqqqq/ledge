//! Adapters implementing the ports (architecture 2). S-01 adds `env` and `fs`; S-08 adds `signals`
//! (SignalFlags) and `terminal` (CrosstermTerminal, the terminal factory); S-09 adds `process` (SystemProcess)
//! and `tmux` (TmuxBackend).

pub mod clock;
pub mod console;
pub mod env;
pub mod fs;
pub mod process;
pub mod signals;
pub mod terminal;
pub mod tmux;

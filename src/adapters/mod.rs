//! Adapters implementing the ports (architecture 2). S-01 adds `env` and `fs`; S-08 adds `signals`
//! (SignalFlags) and `terminal` (CrosstermTerminal, the terminal factory).

pub mod env;
pub mod fs;
pub mod signals;
pub mod terminal;

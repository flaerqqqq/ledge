//! ledge library target. S-00 skeleton: the architecture section 2 module tree is declared as empty
//! modules; only `error` and `messages` hold code so far.
#![forbid(unsafe_code)]

pub mod adapters;
pub mod app;
pub mod cli;
pub mod domain;
pub mod error;
pub mod messages;
pub mod ports;
pub mod tui;

//! Shared test fakes and helpers (docs/testing.md "Test layout"). S-01 adds `fake_env` and `temp`;
//! S-07 adds `fake_terminal`; the other fakes arrive with the slices that first use them.
#![allow(dead_code)]

pub mod fake_env;
pub mod fake_terminal;
pub mod temp;

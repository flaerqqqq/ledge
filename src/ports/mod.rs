//! Port traits (architecture 2 and 4). Trait definitions only, no behaviour.
//! S-01 adds `env` and `fs`; the other ports arrive with the slices that first use them.

pub mod env;
pub mod fs;

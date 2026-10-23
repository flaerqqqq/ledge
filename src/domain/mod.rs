//! Pure domain logic, no I/O (architecture 2). S-01 adds the path, naming and location modules; S-02 adds
//! config and profile; S-03 adds history and bookmarks.

pub mod bookmarks;
pub mod config;
pub mod history;
pub mod location;
pub mod naming;
pub mod path;
pub mod profile;

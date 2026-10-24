//! Terminal UI (architecture 2, 6). `fuzzy` (S-06), the headless picker model, keys, columns and
//! rendering (S-07), and the engine loop (S-08). The crossterm adapter lives in `adapters`.

pub mod columns;
pub mod engine;
pub mod fuzzy;
pub mod keys;
pub mod picker;
pub mod render;
pub mod terminal;
pub mod theme;

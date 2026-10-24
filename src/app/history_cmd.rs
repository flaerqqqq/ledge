//! History commands (architecture 2, 2.3): `history list` and `history clean` (FR-HIST-03, FR-HIST-06..08). The file
//! work is in `app::store`; the text is in `messages`. Bookmarks are never read or written here (FR-HIST-08).

use crate::app::{home_of, store, Ctx};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;

/// FR-HIST-03: one history entry per line, newest first, with the home folder shown as `~`. An empty or missing
/// history prints `No history.`.
pub fn list(ctx: &Ctx<'_>) -> Result<Outcome> {
    let history = store::load_history(ctx.fs, &ctx.dirs)
        .map_err(|error| LedgeError::Fatal(messages::could_not_read_history(&error.to_string())))?;
    if history.entries().is_empty() {
        return Ok(Outcome::Info(messages::no_history().to_string()));
    }
    let home = home_of(ctx);
    let rows: Vec<String> = history
        .entries()
        .iter()
        .map(|entry| entry.display_with_home(home.as_ref()))
        .collect();
    Ok(Outcome::Info(rows.join("\n")))
}

/// FR-HIST-06, FR-HIST-07, FR-HIST-08: deletes the history file and prints the success text. With no file it prints
/// `No history cache found.` and changes nothing. The bookmarks file is not touched.
pub fn clean(ctx: &Ctx<'_>) -> Result<Outcome> {
    let path = store::history_path(&ctx.dirs);
    if !ctx.fs.exists(&path) {
        return Ok(Outcome::Info(messages::no_history_cache().to_string()));
    }
    ctx.fs.remove_file(&path).map_err(|error| {
        LedgeError::Fatal(messages::could_not_clear_history(&error.to_string()))
    })?;
    Ok(Outcome::Info(messages::history_cleared().to_string()))
}

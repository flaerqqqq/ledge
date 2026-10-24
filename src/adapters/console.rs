//! `StdConsole`: the `Console` port over the process's stdout and stderr (architecture 4).

use crate::ports::console::Console;
use std::io::Write;

/// FR-OPEN-11, architecture 8.2: each line goes to the stream the caller names; a write failure is ignored, as
/// there is no other stream to report it on.
pub struct StdConsole;

impl Console for StdConsole {
    fn out(&self, text: &str) {
        let _ = writeln!(std::io::stdout().lock(), "{text}");
    }

    fn err(&self, text: &str) {
        let _ = writeln!(std::io::stderr().lock(), "{text}");
    }
}

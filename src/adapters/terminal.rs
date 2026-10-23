//! CrosstermTerminal (architecture 6.5, FR-TUI-01, FR-TUI-06): raw mode, the alternate screen and a
//! hidden cursor while a picker runs. `Drop` and the panic hook restore the terminal. This is the only
//! module that writes terminal control sequences. Crossterm is reached through `ratatui::crossterm`.

use crate::messages;
use crate::tui::terminal::{Input, TerminalFactory, TerminalSession};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::crossterm::ExecutableCommand;
use ratatui::{Frame, Terminal};
use std::io::{self, IsTerminal, Stdout};
use std::time::Duration;

/// FR-TUI-01: a picker needs a terminal; `Err` carries the message when stdout is not a tty.
pub fn require_tty(stdout_is_tty: bool) -> Result<(), String> {
    if stdout_is_tty {
        Ok(())
    } else {
        Err(messages::terminal_required().to_string())
    }
}

/// Opens `CrosstermTerminal` on the process's stdout.
pub struct CrosstermFactory;

impl TerminalFactory for CrosstermFactory {
    fn open(&self) -> Result<Box<dyn TerminalSession>, String> {
        require_tty(io::stdout().is_terminal())?;
        enable_raw_mode().map_err(|error| messages::terminal_failed(&error.to_string()))?;
        if let Err(error) = enter_screen() {
            restore();
            return Err(messages::terminal_failed(&error.to_string()));
        }
        match Terminal::new(CrosstermBackend::new(io::stdout())) {
            Ok(terminal) => Ok(Box::new(CrosstermTerminal { terminal })),
            Err(error) => {
                restore();
                Err(messages::terminal_failed(&error.to_string()))
            }
        }
    }
}

fn enter_screen() -> io::Result<()> {
    let mut out = io::stdout();
    out.execute(EnterAlternateScreen)?.execute(Hide)?;
    Ok(())
}

/// FR-TUI-06, architecture 6.5: leaves the alternate screen, shows the cursor and disables raw mode.
/// Errors are ignored: this runs on the way out, when nothing is left to report them to.
fn restore() {
    let mut out = io::stdout();
    let _ = out.execute(Show);
    let _ = out.execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// FR-TUI-06, architecture 6.5: a panic restores the terminal first, then the previous hook runs, so
/// the panic message is readable. Only acts while raw mode is on (a picker is open).
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if is_raw_mode_enabled().unwrap_or(false) {
            restore();
        }
        previous(info);
    }));
}

/// A picker screen on the real terminal. Restores in `Drop` (FR-TUI-06).
pub struct CrosstermTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Drop for CrosstermTerminal {
    fn drop(&mut self) {
        restore();
    }
}

impl TerminalSession for CrosstermTerminal {
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<Input>> {
        if !event::poll(timeout)? {
            return Ok(None);
        }
        Ok(match event::read()? {
            Event::Key(key) => Some(Input::Key(key)),
            Event::Resize(width, height) => Some(Input::Resize(width, height)),
            _ => None,
        })
    }

    fn draw(&mut self, draw: &mut dyn FnMut(&mut Frame)) -> io::Result<()> {
        self.terminal.draw(|frame| draw(frame)).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::require_tty;

    // FR-TUI-01: a picker started without a tty fails with the terminal message
    #[test]
    fn picker_without_a_tty_is_refused_with_the_message() {
        assert_eq!(
            require_tty(false),
            Err("a terminal is required for this picker.".to_string())
        );
        assert_eq!(require_tty(true), Ok(()));
    }
}

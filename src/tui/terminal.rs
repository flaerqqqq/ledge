//! Terminal screen port (architecture 4, 6.4, 6.5). The traits live in the tui layer; the crossterm
//! adapter (S-08) and `FakeTerminal` (tests/support) implement them. No I/O is done here.

use ratatui::crossterm::event::KeyEvent;
use ratatui::Frame;
use std::io;
use std::time::Duration;

/// One input event. Keys are crossterm events, re-exported by ratatui so no second crossterm
/// dependency is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(KeyEvent),
    Resize(u16, u16),
}

/// An open terminal screen. The adapter restores the terminal in `Drop` (FR-TUI-06, section 6.5).
pub trait TerminalSession {
    /// Waits up to `timeout` for one input event. `None` when nothing arrived.
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<Input>>;
    /// Draws one frame. The closure gets the whole screen.
    fn draw(&mut self, draw: &mut dyn FnMut(&mut Frame)) -> io::Result<()>;
}

/// Opens a `TerminalSession`. Fails with the picker's terminal message when stdout is not a tty
/// (architecture 4, section 8.3).
pub trait TerminalFactory {
    fn open(&self) -> Result<Box<dyn TerminalSession>, String>;
}

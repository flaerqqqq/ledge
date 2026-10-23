//! FakeTerminal: a `TerminalSession` that replays scripted inputs and draws into ratatui's
//! `TestBackend` (docs/testing.md). After the last scripted input, `poll` returns `None`; after 1000
//! such idle polls it fails, so a test that never closes its picker fails instead of hanging. A
//! scripted resize also resizes the test backend, as a real terminal would.
//!
//! `record()` returns a handle that outlives the session. The session is moved into the engine, so
//! the handle is how a test sees the restore (`Drop`, FR-TUI-06) and the last drawn screen.

use ledge::tui::terminal::{Input, TerminalSession};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::{Frame, Terminal};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::rc::Rc;
use std::time::Duration;

/// Idle polls allowed after the scripted inputs run out.
const IDLE_POLL_LIMIT: usize = 1000;

/// What a test can read after the session is gone.
#[derive(Debug, Default)]
pub struct Record {
    /// Times the session was dropped (restore, FR-TUI-06). Must be exactly 1 after a picker run.
    pub restores: usize,
    /// Frames drawn so far.
    pub draws: usize,
    /// The last drawn screen, one string per row.
    pub lines: Vec<String>,
}

pub struct FakeTerminal {
    terminal: Terminal<TestBackend>,
    inputs: VecDeque<Input>,
    idle_polls: usize,
    record: Rc<RefCell<Record>>,
}

impl FakeTerminal {
    pub fn new(width: u16, height: u16, inputs: Vec<Input>) -> io::Result<Self> {
        Ok(Self {
            terminal: Terminal::new(TestBackend::new(width, height))?,
            inputs: inputs.into(),
            idle_polls: 0,
            record: Rc::new(RefCell::new(Record::default())),
        })
    }

    /// Shared handle to the restore count and the last screen.
    pub fn record(&self) -> Rc<RefCell<Record>> {
        Rc::clone(&self.record)
    }

    /// The last drawn buffer.
    pub fn buffer(&self) -> &Buffer {
        self.terminal.backend().buffer()
    }

    /// One string per screen row, the cell symbols joined.
    pub fn lines(&self) -> Vec<String> {
        let buffer = self.buffer();
        let area = buffer.area;
        (area.top()..area.bottom())
            .map(|y| {
                (area.left()..area.right())
                    .filter_map(|x| buffer.cell((x, y)))
                    .map(Cell::symbol)
                    .collect::<String>()
            })
            .collect()
    }

    /// The cell at (`x`, `y`) of the last drawn buffer.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.buffer().cell((x, y))
    }

    /// Number of frames drawn so far.
    pub fn draws(&self) -> usize {
        self.record.borrow().draws
    }
}

impl Drop for FakeTerminal {
    fn drop(&mut self) {
        self.record.borrow_mut().restores += 1;
    }
}

impl TerminalSession for FakeTerminal {
    fn poll(&mut self, _timeout: Duration) -> io::Result<Option<Input>> {
        match self.inputs.pop_front() {
            Some(input) => {
                if let Input::Resize(width, height) = input {
                    self.terminal.backend_mut().resize(width, height);
                }
                Ok(Some(input))
            }
            None => {
                self.idle_polls += 1;
                if self.idle_polls > IDLE_POLL_LIMIT {
                    Err(io::Error::other("no input: the picker never closed"))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn draw(&mut self, draw: &mut dyn FnMut(&mut Frame)) -> io::Result<()> {
        self.terminal.draw(|frame| draw(frame)).map(|_| ())?;
        let lines = self.lines();
        let mut record = self.record.borrow_mut();
        record.draws += 1;
        record.lines = lines;
        Ok(())
    }
}

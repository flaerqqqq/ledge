//! FakeTerminal: a `TerminalSession` that replays scripted inputs and draws into ratatui's
//! `TestBackend` (docs/testing.md). After the last scripted input, `poll` returns `None`; after 1000
//! such idle polls it fails, so a test that never closes its picker fails instead of hanging. A
//! scripted resize also resizes the test backend, as a real terminal would.
//!
//! `record()` returns a handle that outlives the session. The session is moved into the engine, so
//! the handle is how a test sees the restore (`Drop`, FR-TUI-06) and the last drawn screen.

use ledge::tui::terminal::{Input, TerminalFactory, TerminalSession};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::{Frame, Terminal};
use std::cell::{Cell as StdCell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::PathBuf;
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

/// FakeTerminalFactory: the `TerminalFactory` the picker services use in tests. `open` builds a `FakeTerminal`
/// from the scripted inputs and keeps its record for the test. `remove_on_open` deletes a folder when the
/// picker opens, after its rows were read, to model a directory that vanishes while the list is open (FR-PICK-09).
pub struct FakeTerminalFactory {
    /// One script per picker that opens, in order (a nested picker takes the next one).
    scripts: RefCell<VecDeque<Vec<Input>>>,
    width: StdCell<u16>,
    height: StdCell<u16>,
    opened: StdCell<usize>,
    records: RefCell<Vec<Rc<RefCell<Record>>>>,
    vanish: RefCell<Vec<PathBuf>>,
}

impl FakeTerminalFactory {
    /// An 80x24 screen with no scripted input.
    pub fn new() -> Self {
        Self {
            scripts: RefCell::new(VecDeque::new()),
            width: StdCell::new(80),
            height: StdCell::new(24),
            opened: StdCell::new(0),
            records: RefCell::new(Vec::new()),
            vanish: RefCell::new(Vec::new()),
        }
    }

    /// The size of the next screen (long paths and status lines need more than 80 cells).
    pub fn set_size(&self, width: u16, height: u16) {
        self.width.set(width);
        self.height.set(height);
    }

    /// The inputs the next picker reads, in order. Replaces any queued scripts.
    pub fn script(&self, inputs: Vec<Input>) {
        let mut scripts = self.scripts.borrow_mut();
        scripts.clear();
        scripts.push_back(inputs);
    }

    /// Adds the inputs of a later picker: the first picker takes the first script, the next one (for
    /// example a nested profile picker) takes the next.
    pub fn queue(&self, inputs: Vec<Input>) {
        self.scripts.borrow_mut().push_back(inputs);
    }

    /// Deletes `path` when the next picker opens.
    pub fn remove_on_open(&self, path: PathBuf) {
        self.vanish.borrow_mut().push(path);
    }

    /// How many times a picker opened a terminal. Zero when a command stops before the picker.
    pub fn opened(&self) -> usize {
        self.opened.get()
    }

    /// The record of the last terminal opened: restores, draws and the last screen.
    pub fn last_record(&self) -> Option<Rc<RefCell<Record>>> {
        self.records.borrow().last().cloned()
    }

    /// The records of every terminal opened, in order (the screen of each picker of a flow).
    pub fn records(&self) -> Vec<Rc<RefCell<Record>>> {
        self.records.borrow().clone()
    }
}

impl TerminalFactory for FakeTerminalFactory {
    fn open(&self) -> Result<Box<dyn TerminalSession>, String> {
        self.opened.set(self.opened.get() + 1);
        for path in self.vanish.borrow().iter() {
            let _ = fs::remove_dir_all(path);
        }
        let inputs = self.scripts.borrow_mut().pop_front().unwrap_or_default();
        let term = FakeTerminal::new(self.width.get(), self.height.get(), inputs)
            .map_err(|e| e.to_string())?;
        self.records.borrow_mut().push(term.record());
        Ok(Box::new(term))
    }
}

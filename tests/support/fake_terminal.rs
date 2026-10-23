//! FakeTerminal: a `TerminalSession` that replays scripted inputs and draws into ratatui's
//! `TestBackend` (docs/testing.md). After the last scripted input, `poll` returns `None`. A scripted
//! resize also resizes the test backend, as a real terminal would.

use ledge::tui::terminal::{Input, TerminalSession};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::{Frame, Terminal};
use std::collections::VecDeque;
use std::io;
use std::time::Duration;

pub struct FakeTerminal {
    terminal: Terminal<TestBackend>,
    inputs: VecDeque<Input>,
    draws: usize,
}

impl FakeTerminal {
    pub fn new(width: u16, height: u16, inputs: Vec<Input>) -> io::Result<Self> {
        Ok(Self {
            terminal: Terminal::new(TestBackend::new(width, height))?,
            inputs: inputs.into(),
            draws: 0,
        })
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
        self.draws
    }
}

impl TerminalSession for FakeTerminal {
    fn poll(&mut self, _timeout: Duration) -> io::Result<Option<Input>> {
        let input = self.inputs.pop_front();
        if let Some(Input::Resize(width, height)) = input {
            self.terminal.backend_mut().resize(width, height);
        }
        Ok(input)
    }

    fn draw(&mut self, draw: &mut dyn FnMut(&mut Frame)) -> io::Result<()> {
        self.draws += 1;
        self.terminal.draw(|frame| draw(frame)).map(|_| ())
    }
}

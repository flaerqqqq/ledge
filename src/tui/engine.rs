//! Picker engine (architecture 6.4, 6.5): the loop that draws a `PickerModel`, reads input, calls the
//! caller's `PickerHandler`, and returns a result. The terminal is restored when `run_picker` returns,
//! on every path, because the session is dropped there (FR-TUI-06).

use crate::error::{LedgeError, Outcome};
use crate::messages;
use crate::ports::signals::Signals;
use crate::tui::keys::{self, Action};
use crate::tui::picker::{self, Effect, PickerModel, PickerRow};
use crate::tui::render;
use crate::tui::terminal::TerminalSession;
use crate::tui::theme::Theme;
use ratatui::Frame;
use std::io;
use std::time::Duration;

/// Poll interval of the loop; signals are checked on every poll (architecture 6.4).
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The caller's answer to a key or a choice (architecture 6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Keep the picker open. `status` replaces the status line; `reload` asks for `reload()`.
    Continue {
        status: Option<String>,
        reload: bool,
    },
    /// Keep the picker open. `message` is an error and replaces the status line in the error style (design 4.5);
    /// `reload` asks for `reload()`.
    Failed { message: String, reload: bool },
    /// Return the selected row to the caller (enter). Used after the terminal is restored.
    Accept,
    /// Return this outcome to the caller.
    Close(Outcome),
    /// FR-EDIT-03: left arrow on a folder that is not expanded moves the selection to its parent row.
    ToParent,
    /// ctrl-p on a row (S-15, FR-ACT-04): the picker returns the row as `PickerResult::Profile`, so the caller can
    /// run the profile picker for it and open this picker again. Only meaningful from `act`.
    Profile,
    /// alt-p on a tree node (S-22, FR-ACT-09): the picker returns the node row as `PickerResult::ProfileNode`, so the
    /// caller asks one profile and rebuilds every session under the node. Only meaningful from `act`.
    ProfileNode,
}

/// What the picker returned. `Closed` carries the outcome of an interrupt or a handler `Close`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerResult<T> {
    Chosen(T),
    Cancelled,
    Closed(Outcome),
    /// ctrl-p on this row: the caller picks a profile for it and then reopens the picker.
    Profile(T),
    /// alt-p on this node row: the caller picks one profile for every session under it, then reopens the picker.
    ProfileNode(T),
}

/// The caller's side of a picker: what enter, actions and reloads do (architecture 6.4).
pub trait PickerHandler<T: Clone> {
    /// Enter on a row.
    fn choose(&mut self, row: &T) -> Step;
    /// Ctrl and alt actions. `row` is `None` when the list is empty.
    fn act(&mut self, action: Action, row: Option<&T>) -> Step;
    /// Rows again after an action with `reload: true`. Filter and position are kept.
    fn reload(&mut self) -> Result<Vec<PickerRow<T>>, LedgeError>;
}

/// FR-TUI-01, FR-TUI-06: runs one picker until it chooses, cancels, closes or is interrupted.
/// `term` is dropped before this returns, so the terminal is restored on success, cancel, interrupt
/// and error alike. `color` comes from `render::color_enabled` (FR-TUI-09); `theme` gives the colours (design 6).
pub fn run_picker<T: Clone>(
    mut model: PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    mut term: Box<dyn TerminalSession>,
    signals: &dyn Signals,
    theme: &Theme,
    color: bool,
) -> Result<PickerResult<T>, LedgeError> {
    let result = event_loop(&mut model, handler, term.as_mut(), signals, theme, color);
    drop(term);
    result
}

fn event_loop<T: Clone>(
    model: &mut PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    term: &mut dyn TerminalSession,
    signals: &dyn Signals,
    theme: &Theme,
    color: bool,
) -> Result<PickerResult<T>, LedgeError> {
    loop {
        if let Some(code) = signals.take_interrupt() {
            return Ok(PickerResult::Closed(Outcome::Interrupted(code)));
        }
        draw(model, term, theme, color).map_err(terminal_error)?;
        let Some(input) = term.poll(POLL_INTERVAL).map_err(terminal_error)? else {
            continue;
        };
        // ctrl-c always closes the picker, even with the edit window's search bar open (FR-EDIT-13): escape only
        // closes the bar, and both keys are the same action in the map.
        if keys::is_interrupt_key(&input) {
            return Ok(PickerResult::Cancelled);
        }
        // A resize needs no action: the next draw takes the new size (FR-TUI-05).
        let Some(action) = keys::map(&input) else {
            continue;
        };
        if let Some(result) = handle(model, handler, action)? {
            return Ok(result);
        }
    }
}

/// Applies one action. `Some` ends the picker.
fn handle<T: Clone>(
    model: &mut PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    action: Action,
) -> Result<Option<PickerResult<T>>, LedgeError> {
    match model.apply(action) {
        Effect::Handled => Ok(None),
        Effect::Cancel => Ok(Some(PickerResult::Cancelled)),
        Effect::Choose => {
            let Some(row) = model.selected_row() else {
                return Ok(Some(PickerResult::Cancelled));
            };
            let payload = row.payload.clone();
            let step = handler.choose(&payload);
            finish(model, handler, step, payload)
        }
        Effect::Act(action) => {
            let payload = model.selected_row().map(|row| row.payload.clone());
            let step = handler.act(action, payload.as_ref());
            match (step, payload) {
                (Step::Accept, Some(payload)) => Ok(Some(PickerResult::Chosen(payload))),
                (Step::Accept, None) => Ok(Some(PickerResult::Cancelled)),
                (Step::Profile, Some(payload)) => Ok(Some(PickerResult::Profile(payload))),
                (Step::Profile, None) => Ok(None),
                (Step::ProfileNode, Some(payload)) => Ok(Some(PickerResult::ProfileNode(payload))),
                (Step::ProfileNode, None) => Ok(None),
                (other, _) => apply_step(model, handler, other),
            }
        }
    }
}

/// After `choose`: `Accept` returns the chosen payload, anything else is applied as usual.
fn finish<T: Clone>(
    model: &mut PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    step: Step,
    payload: T,
) -> Result<Option<PickerResult<T>>, LedgeError> {
    match step {
        Step::Accept => Ok(Some(PickerResult::Chosen(payload))),
        other => apply_step(model, handler, other),
    }
}

fn apply_step<T: Clone>(
    model: &mut PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    step: Step,
) -> Result<Option<PickerResult<T>>, LedgeError> {
    match step {
        Step::Accept => Ok(Some(PickerResult::Cancelled)),
        Step::Close(outcome) => Ok(Some(PickerResult::Closed(outcome))),
        // Only `act` returns Profile, and `handle` answers it with the row; here there is no row to hand back.
        Step::Profile | Step::ProfileNode => Ok(None),
        Step::ToParent => {
            model.select_parent();
            Ok(None)
        }
        Step::Continue { status, reload } => {
            model.set_status(status);
            if reload {
                let rows = handler.reload()?;
                model.reload(rows);
            }
            Ok(None)
        }
        Step::Failed { message, reload } => {
            model.set_error(message);
            if reload {
                let rows = handler.reload()?;
                model.reload(rows);
            }
            Ok(None)
        }
    }
}

fn draw<T>(
    model: &mut PickerModel<T>,
    term: &mut dyn TerminalSession,
    theme: &Theme,
    color: bool,
) -> io::Result<()> {
    term.draw(&mut |frame: &mut Frame| {
        let area = frame.area();
        // The edit window has no prompt row unless its search bar is open or a match is kept (architecture 6.2),
        // so its list gets one more row without it.
        let no_prompt = usize::from(!model.has_prompt());
        model.set_list_height(picker::list_height(area.height) + no_prompt);
        render::draw(model, area, frame, theme, color);
    })
}

/// Wraps a terminal failure with the message that names it (architecture 8.3).
fn terminal_error(error: io::Error) -> LedgeError {
    LedgeError::Fatal(messages::terminal_failed(&error.to_string()))
}

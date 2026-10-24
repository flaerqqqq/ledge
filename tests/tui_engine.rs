//! TUI model tests (docs/testing.md level 3): keys, layout, resize, dimming and cues, driven through
//! `FakeTerminal` (ratatui `TestBackend`). `drive` is the model part of the loop without the engine
//! (S-07). The engine tests at the end run `run_picker` itself (S-08): restore, interrupts, errors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::error::{LedgeError, Outcome};
use ledge::tui::columns::ColumnSpec;
use ledge::tui::engine::{run_picker, PickerHandler, PickerResult, Step};
use ledge::tui::keys::{self, Action};
use ledge::tui::picker::{self, Chrome, Effect, PickerModel, PickerRow};
use ledge::tui::render::{self, icon};
use ledge::tui::terminal::{Input, TerminalSession};
use ledge::tui::theme::Theme;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use ratatui::Frame;
use std::time::Duration;
use support::fake_signals::FakeSignals;
use support::fake_terminal::FakeTerminal;

const PROJECT_FOOTER: &str = "enter open · ^r rebuild · ^p profile · ^x kill";

fn key(code: KeyCode) -> Input {
    Input::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn ctrl(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}

fn alt(c: char) -> Input {
    Input::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT))
}

fn typed(text: &str) -> Vec<Input> {
    text.chars().map(|c| key(KeyCode::Char(c))).collect()
}

/// The model part of the picker loop. Returns the effects of the keys in order.
fn drive(model: &mut PickerModel<usize>, term: &mut FakeTerminal, color: bool) -> Vec<Effect> {
    drive_themed(model, term, &Theme::default(), color)
}

/// `drive` with a theme (FR-CONF-02, design 6).
fn drive_themed(
    model: &mut PickerModel<usize>,
    term: &mut FakeTerminal,
    theme: &Theme,
    color: bool,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    let height = term.buffer().area.height;
    model.set_list_height(picker::list_height(height));
    draw_themed(model, term, theme, color);
    let mut polls = 0;
    while let Some(input) = term.poll(Duration::from_millis(100)).unwrap() {
        polls += 1;
        assert!(polls < 1000, "the scripted run did not end");
        match &input {
            Input::Resize(_, height) => model.set_list_height(picker::list_height(*height)),
            Input::Key(_) => {
                if let Some(action) = keys::map(&input) {
                    effects.push(model.apply(action));
                }
            }
        }
        draw_themed(model, term, theme, color);
    }
    effects
}

fn draw_themed(model: &PickerModel<usize>, term: &mut FakeTerminal, theme: &Theme, color: bool) {
    term.draw(&mut |frame: &mut Frame| {
        render::draw(model, frame.area(), frame, theme, color);
    })
    .unwrap();
}

fn projects_chrome() -> Chrome {
    Chrome::new(
        "Projects & Submodules",
        icon::PROJECTS,
        vec![
            ColumnSpec::fixed("", 2),
            ColumnSpec::path("Project"),
            ColumnSpec::text("Session"),
            ColumnSpec::text("Profile"),
        ],
        PROJECT_FOOTER,
    )
}

fn name_chrome(footer: &str) -> Chrome {
    Chrome::new(
        "Names",
        icon::PROJECTS,
        vec![ColumnSpec::text("Name"), ColumnSpec::text("Session")],
        footer,
    )
}

fn named_rows(count: usize) -> Vec<PickerRow<usize>> {
    (0..count)
        .map(|i| {
            PickerRow::new(
                vec![format!("row{i:02}"), format!("row{i:02}-1a2b")],
                &format!("row{i:02}"),
                i,
            )
        })
        .collect()
}

/// The first list line whose first cell is reverse video (the selected row).
fn selected_line(term: &FakeTerminal) -> Option<String> {
    let area = term.buffer().area;
    (area.top()..area.bottom())
        .find(|&y| {
            term.cell(1, y)
                .is_some_and(|cell| cell.modifier.contains(Modifier::REVERSED))
        })
        .and_then(|y| term.lines().get(usize::from(y)).cloned())
}

fn line_containing(term: &FakeTerminal, needle: &str) -> Option<(u16, String)> {
    term.lines()
        .into_iter()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .and_then(|(y, line)| u16::try_from(y).ok().map(|y| (y, line)))
}

// FR-TUI-04: header row, border label with icon, footer, and the "> " prompt
#[test]
fn chrome_shows_label_prompt_header_status_and_footer() {
    let rows = vec![
        PickerRow::new(
            vec![
                "*".into(),
                "/home/u/dev/alpha".into(),
                "alpha-1a2b".into(),
                "basic".into(),
            ],
            "alpha",
            0,
        ),
        PickerRow::new(
            vec!["●".into(), "/home/u/dev/beta".into(), "".into(), "".into()],
            "beta",
            1,
        ),
    ];
    let mut model = PickerModel::new(rows, projects_chrome());
    model.set_status(Some("cannot kill the session you are in.".into()));
    let mut term = FakeTerminal::new(80, 20, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let lines = term.lines();

    assert!(
        lines[0].contains("▸ Projects & Submodules"),
        "{:?}",
        lines[0]
    );
    assert!(lines[1].contains("> "), "prompt row: {:?}", lines[1]);
    assert!(lines[2].contains("Project"), "header: {:?}", lines[2]);
    assert!(lines[2].contains("Session"), "header: {:?}", lines[2]);
    assert!(lines[2].contains("Profile"), "header: {:?}", lines[2]);
    assert!(lines[3].contains("/home/u/dev/alpha"), "{:?}", lines[3]);
    assert!(lines[17].contains("cannot kill the session you are in."));
    assert!(
        lines[18].contains(PROJECT_FOOTER),
        "footer: {:?}",
        lines[18]
    );
}

// FR-TUI-04, FR-TUI-02: the prompt shows the filter typed so far
#[test]
fn prompt_shows_the_typed_filter() {
    let rows = named_rows(5);
    let mut model = PickerModel::new(rows, name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, typed("row0")).unwrap();
    drive(&mut model, &mut term, false);
    assert!(term.lines()[1].contains("> row0"), "{:?}", term.lines()[1]);
}

// FR-TUI-03: arrow and page keys, home and end move the highlighted row
#[test]
fn arrow_page_home_and_end_move_the_selection() {
    let run = |inputs: Vec<Input>| {
        let mut model = PickerModel::new(named_rows(30), name_chrome("enter open"));
        let mut term = FakeTerminal::new(40, 20, inputs).unwrap();
        drive(&mut model, &mut term, true);
        (model.selected_position(), selected_line(&term))
    };

    let (position, line) = run(vec![key(KeyCode::End)]);
    assert_eq!(position, 29);
    assert!(line.unwrap().contains("row29"));

    let (position, _) = run(vec![key(KeyCode::Home)]);
    assert_eq!(position, 0);

    // The list is 14 rows high at 40x20, so a page is 13 rows.
    let (position, line) = run(vec![key(KeyCode::PageDown)]);
    assert_eq!(position, 13);
    assert!(line.unwrap().contains("row13"));

    let (position, _) = run(vec![key(KeyCode::PageDown), key(KeyCode::PageDown)]);
    assert_eq!(position, 26);

    let (position, _) = run(vec![
        key(KeyCode::Down),
        key(KeyCode::Down),
        key(KeyCode::Up),
    ]);
    assert_eq!(position, 1);

    let (position, _) = run(vec![key(KeyCode::Up)]);
    assert_eq!(position, 0, "no wrap-around at the top");
}

// FR-TUI-05: the selected row stays visible when the terminal is resized to 40x8 and back
#[test]
fn resize_keeps_the_selected_row_visible() {
    let mut model = PickerModel::new(named_rows(30), name_chrome("enter open"));
    let mut term =
        FakeTerminal::new(80, 24, vec![key(KeyCode::End), Input::Resize(40, 8)]).unwrap();
    drive(&mut model, &mut term, true);
    assert_eq!(term.buffer().area.width, 40);
    assert!(
        selected_line(&term).unwrap().contains("row29"),
        "selected row not visible at 40x8"
    );
    // At 40x8 the list has two rows: the selection is the bottom one.
    assert!(line_containing(&term, "row28").is_some());

    let mut model = PickerModel::new(named_rows(30), name_chrome("enter open"));
    let mut term =
        FakeTerminal::new(80, 24, vec![key(KeyCode::End), Input::Resize(120, 40)]).unwrap();
    drive(&mut model, &mut term, true);
    assert!(selected_line(&term).unwrap().contains("row29"));
}

// FR-TUI-05, A-06: below 40x8 only the too-small line is drawn; keys still cancel
#[test]
fn below_minimum_size_draws_only_the_too_small_line() {
    for (width, height) in [(39, 8), (40, 7)] {
        let mut model = PickerModel::new(named_rows(5), name_chrome("enter open"));
        let mut term = FakeTerminal::new(width, height, vec![key(KeyCode::Esc)]).unwrap();
        let effects = drive(&mut model, &mut term, true);
        assert_eq!(
            term.lines()[0].trim_end(),
            "Terminal too small: need at least 40x8.",
            "size {width}x{height}"
        );
        assert!(term.lines()[1..].iter().all(|line| line.trim().is_empty()));
        assert_eq!(effects, vec![Effect::Cancel]);
    }
}

// FR-TUI-07: the same key does the same action in every picker
#[test]
fn same_key_same_action_in_every_picker() {
    let pickers = [
        ("project", projects_chrome()),
        (
            "history",
            name_chrome("enter open · ^r rebuild · ^p profile · ^x kill"),
        ),
        (
            "switcher",
            name_chrome("enter switch · ^r rebuild · ^p profile · ^x kill"),
        ),
        (
            "bookmarks",
            name_chrome("enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark"),
        ),
        (
            "session tree",
            name_chrome("enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node"),
        ),
        (
            "bookmark tree",
            name_chrome("enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node"),
        ),
        ("profiles", name_chrome("enter choose · esc cancel")),
    ];
    let expected = [
        (ctrl('r'), Action::Rebuild),
        (ctrl('p'), Action::Profile),
        (ctrl('x'), Action::Kill),
        (ctrl('d'), Action::DeleteBookmark),
        (alt('r'), Action::RebuildNode),
        (alt('p'), Action::ProfileNode),
        (alt('x'), Action::KillNode),
        (alt('d'), Action::DeleteNode),
    ];
    for (name, chrome) in pickers {
        for (input, action) in &expected {
            let mut model = PickerModel::new(named_rows(2), chrome.clone());
            let mapped = keys::map(input).unwrap();
            assert_eq!(mapped, *action, "{name}");
            assert_eq!(model.apply(mapped), Effect::Act(*action), "{name}");
        }
        let mut model = PickerModel::new(named_rows(2), chrome.clone());
        assert_eq!(model.apply(Action::Choose), Effect::Choose, "{name}");
        assert_eq!(model.apply(Action::Cancel), Effect::Cancel, "{name}");
    }
}

// FR-TUI-08, A-04: a missing directory shows the ` (missing)` cue, and is dimmed with color
#[test]
fn missing_directory_has_the_cue_and_is_dimmed_with_color() {
    let mut chrome = name_chrome("enter open");
    chrome.missing_column = Some(0);
    let mut gone = PickerRow::new(vec!["/x/gone".into(), "gone-0000".into()], "gone", 0);
    gone.dimmed = true;
    let rows = vec![
        gone,
        PickerRow::new(vec!["/x/here".into(), "".into()], "here", 1),
    ];

    let mut model = PickerModel::new(rows.clone(), chrome.clone());
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, line) = line_containing(&term, "(missing)").unwrap();
    assert!(line.contains("/x/gone (missing)"), "{line:?}");
    assert!(term.cell(2, y).unwrap().modifier.contains(Modifier::DIM));
    let (y, _) = line_containing(&term, "/x/here").unwrap();
    assert!(!term.cell(2, y).unwrap().modifier.contains(Modifier::DIM));

    let mut model = PickerModel::new(rows, chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(missing)").unwrap();
    assert!(line.contains("/x/gone (missing)"));
    assert!(!term.cell(2, y).unwrap().modifier.contains(Modifier::DIM));
}

// FR-TUI-09, A-04: the selected row is reverse video, so it shows without color
#[test]
fn selected_row_is_reverse_video_without_color() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, vec![key(KeyCode::Down)]).unwrap();
    drive(&mut model, &mut term, false);
    assert!(selected_line(&term).unwrap().contains("row01"));
}

// FR-TUI-09, A-04: a gray connector is wrapped in parentheses, and gray with color
#[test]
fn connector_is_parenthesised_and_gray_with_color() {
    let chrome = Chrome::new(
        "Bookmark tree",
        icon::BOOKMARKS,
        vec![ColumnSpec::text("Directory"), ColumnSpec::text("Session")],
        "enter open",
    );
    let mut connector = PickerRow::new(vec!["work".into(), "".into()], "work", 0);
    connector.connector = true;
    connector.depth = 1;

    let mut model = PickerModel::new(vec![connector.clone()], chrome.clone());
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, line) = line_containing(&term, "(work)").unwrap();
    assert!(line.contains("  (work)"), "{line:?}");
    assert_eq!(term.cell(2, y).unwrap().fg, Color::DarkGray);

    let mut model = PickerModel::new(vec![connector], chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(work)").unwrap();
    assert!(line.contains("  (work)"));
    assert_ne!(term.cell(2, y).unwrap().fg, Color::DarkGray);
}

// FR-TUI-02, FR-TUI-09: matched characters are bold and underlined
#[test]
fn matched_characters_are_bold_and_underlined() {
    let rows = vec![
        PickerRow::new(vec!["ledge".into(), "".into()], "ledge", 0),
        PickerRow::new(vec!["other".into(), "".into()], "other", 1),
    ];
    let mut model = PickerModel::new(rows, name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, typed("le")).unwrap();
    drive(&mut model, &mut term, false);
    let (y, _) = line_containing(&term, "ledge").unwrap();
    for x in [2, 3] {
        let modifier = term.cell(x, y).unwrap().modifier;
        assert!(
            modifier.contains(Modifier::BOLD | Modifier::UNDERLINED),
            "x={x}"
        );
    }
    assert!(!term
        .cell(4, y)
        .unwrap()
        .modifier
        .contains(Modifier::UNDERLINED));
}

// FR-TUI-10: a picker with rows stays open after an action; one with no rows closes unless it stays
// open (session switcher, session tree)
#[test]
fn picker_stays_open_with_rows_and_closes_when_empty_unless_staying_open() {
    let mut closing = PickerModel::new(named_rows(2), name_chrome("enter open"));
    closing.reload(named_rows(1));
    assert!(!closing.should_close());
    closing.reload(Vec::new());
    assert!(closing.should_close());

    let mut staying_chrome = name_chrome("enter switch");
    staying_chrome.stay_open_when_empty = true;
    let mut staying = PickerModel::new(named_rows(1), staying_chrome);
    staying.reload(Vec::new());
    assert!(!staying.should_close());
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut staying, &mut term, false);
    assert!(term.lines()[0].contains("▸ Names"));
}

// FR-TUI-09, design 9.2a: the selected row has the bar in column 0, and reverse video runs over the whole row
// and its gaps (no colour)
#[test]
fn selection_bar_and_reverse_video_cover_the_whole_row() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, vec![key(KeyCode::Down)]).unwrap();
    drive(&mut model, &mut term, false);
    let (y, _) = line_containing(&term, "row01-1a2b").unwrap();
    assert_eq!(term.cell(1, y).unwrap().symbol(), "▌");
    // The gap between Name (x 2..=6) and Session is x 7; the row runs to the inner edge (x 58).
    for x in 1..=58 {
        assert!(
            term.cell(x, y)
                .unwrap()
                .modifier
                .contains(Modifier::REVERSED),
            "x={x}"
        );
    }
    let (other, _) = line_containing(&term, "row00-1a2b").unwrap();
    assert_eq!(term.cell(1, other).unwrap().symbol(), " ");
    assert!(!term
        .cell(7, other)
        .unwrap()
        .modifier
        .contains(Modifier::REVERSED));
}

// FR-TUI-09, design 6.1, 9.2a: with ink the selected row uses selection_bg over the whole row, gaps included
#[test]
fn selection_gap_takes_the_theme_selection_background() {
    let ink = Theme::builtin("ink").unwrap();
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, vec![key(KeyCode::Down)]).unwrap();
    drive_themed(&mut model, &mut term, &ink, true);
    let (y, _) = line_containing(&term, "row01-1a2b").unwrap();
    let selection_bg = Color::Rgb(0x39, 0x3b, 0x44);
    for x in [1, 2, 6, 7, 8, 18, 58] {
        let cell = term.cell(x, y).unwrap();
        assert_eq!(cell.bg, selection_bg, "x={x}");
        assert!(!cell.modifier.contains(Modifier::REVERSED), "x={x}");
    }
    assert_eq!(term.cell(1, y).unwrap().fg, Color::Rgb(0x7f, 0xb4, 0xca));
}

// FR-TUI-04, design 4.2: at 60 cells the footer drops whole key pairs from the end, never half a pair
#[test]
fn footer_drops_whole_key_pairs_from_the_end_at_60_cells() {
    let footer = "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark";
    let mut model = PickerModel::new(named_rows(2), name_chrome(footer));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let footer = term.lines()[10]
        .trim_matches(|c: char| c == '│' || c == ' ')
        .to_string();
    assert_eq!(footer, "enter open · ^r rebuild · ^p profile · ^x kill");
}

// FR-NAME-01, design 3.2: a long session name is cut in the middle and keeps its four-character hash
#[test]
fn long_session_name_is_cut_in_the_middle_keeping_the_hash() {
    let name = "api-gateway-service-long-session-name-91bc";
    let chrome = Chrome::new(
        "Sessions",
        icon::SESSIONS,
        vec![ColumnSpec::name("Session")],
        "enter switch",
    );
    let mut model = PickerModel::new(vec![PickerRow::new(vec![name.into()], name, 0)], chrome);
    let mut term = FakeTerminal::new(40, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    // 40 cells, 38 inside the border, 1 for the selection bar: 37 cells, the hash is 5 of them.
    let expected = format!("▌{}…-91bc", &name[..31]);
    let (_, line) = line_containing(&term, "…-91bc").unwrap();
    assert!(line.contains(&expected), "{line:?}");
}

// FR-TUI-09, design 8.4: an error in the status row is bold and keeps its text
#[test]
fn error_status_is_bold_and_keeps_its_text() {
    let mut model = PickerModel::new(named_rows(2), name_chrome("enter open"));
    model.set_error("could not kill session 'x'.".into());
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, line) = line_containing(&term, "could not kill session").unwrap();
    assert_eq!(y, 9, "status row");
    assert!(line.contains("could not kill session 'x'."));
    assert!(term.cell(1, y).unwrap().modifier.contains(Modifier::BOLD));
}

// ---------------------------------------------------------------------------------------------
// Engine (S-08): run_picker drives the loop, restores the terminal on every return path (FR-TUI-06)
// ---------------------------------------------------------------------------------------------

/// Records what the engine asks of the handler and answers with scripted steps.
struct Recorder {
    acts: usize,
    chooses: Vec<usize>,
    act_step: Step,
    reload_error: Option<String>,
    rows: usize,
}

impl Recorder {
    fn new(rows: usize) -> Self {
        Self {
            acts: 0,
            chooses: Vec::new(),
            act_step: Step::Continue {
                status: None,
                reload: false,
            },
            reload_error: None,
            rows,
        }
    }
}

impl PickerHandler<usize> for Recorder {
    fn choose(&mut self, row: &usize) -> Step {
        self.chooses.push(*row);
        Step::Accept
    }

    fn act(&mut self, _action: Action, _row: Option<&usize>) -> Step {
        self.acts += 1;
        self.act_step.clone()
    }

    fn reload(&mut self) -> Result<Vec<ledge::tui::picker::PickerRow<usize>>, LedgeError> {
        match &self.reload_error {
            Some(text) => Err(LedgeError::Fatal(text.clone())),
            None => Ok(named_rows(self.rows)),
        }
    }
}

fn engine_model(rows: usize) -> PickerModel<usize> {
    let chrome = Chrome::new(
        "Projects & Submodules",
        icon::PROJECTS,
        vec![ColumnSpec::text("Name"), ColumnSpec::text("Session")],
        "enter open",
    );
    PickerModel::new(named_rows(rows), chrome)
}

/// Runs the engine on a 80x24 FakeTerminal with the scripted inputs. Returns the result, the
/// handler, and the restore record of the terminal.
fn run_engine(
    rows: usize,
    inputs: Vec<Input>,
    handler: &mut Recorder,
    signals: &FakeSignals,
) -> (
    Result<PickerResult<usize>, LedgeError>,
    std::rc::Rc<std::cell::RefCell<support::fake_terminal::Record>>,
) {
    let term = FakeTerminal::new(80, 24, inputs).unwrap();
    let record = term.record();
    let result = run_picker(
        engine_model(rows),
        handler,
        Box::new(term),
        signals,
        &Theme::default(),
        false,
    );
    (result, record)
}

// FR-TUI-01: the engine draws the picker (label, header, rows) before it returns
#[test]
fn engine_draws_the_picker_before_returning() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(
        3,
        vec![key(KeyCode::Esc)],
        &mut handler,
        &FakeSignals::none(),
    );
    assert_eq!(result.unwrap(), PickerResult::Cancelled);
    let screen = record.borrow().lines.join("\n");
    assert!(screen.contains("Projects & Submodules"), "{screen}");
    assert!(screen.contains("row00"), "{screen}");
    assert!(screen.contains("row02"), "{screen}");
}

// FR-TUI-06: enter returns the selected row after the terminal is restored
#[test]
fn enter_chooses_the_selected_row_and_restores_once() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(
        3,
        vec![key(KeyCode::Down), key(KeyCode::Enter)],
        &mut handler,
        &FakeSignals::none(),
    );
    assert_eq!(result.unwrap(), PickerResult::Chosen(1));
    assert_eq!(handler.chooses, vec![1]);
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-06: escape cancels and the terminal is restored
#[test]
fn escape_cancels_and_restores_once() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(
        3,
        vec![key(KeyCode::Esc)],
        &mut handler,
        &FakeSignals::none(),
    );
    assert_eq!(result.unwrap(), PickerResult::Cancelled);
    assert_eq!(record.borrow().restores, 1);
}

// A-08: ctrl-c is a cancel (a key in raw mode), not an interrupt; exit is a normal return
#[test]
fn ctrl_c_is_a_cancel_and_restores_once() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(3, vec![ctrl('c')], &mut handler, &FakeSignals::none());
    assert_eq!(result.unwrap(), PickerResult::Cancelled);
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-06: a handler error returns through the drop path, so the terminal is restored
#[test]
fn handler_error_returns_error_and_restores_once() {
    let mut handler = Recorder::new(3);
    handler.act_step = Step::Continue {
        status: None,
        reload: true,
    };
    handler.reload_error = Some("boom".to_string());
    let (result, record) = run_engine(3, vec![ctrl('r')], &mut handler, &FakeSignals::none());
    assert_eq!(result.unwrap_err(), LedgeError::Fatal("boom".to_string()));
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-06: a terminal read failure returns an error and the terminal is restored
#[test]
fn terminal_failure_returns_error_and_restores_once() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(3, Vec::new(), &mut handler, &FakeSignals::none());
    assert!(result.is_err(), "a picker that never closes must fail");
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-06: an interrupt returns Closed(Interrupted(code)) after the restore, and no later key is read
// (SIGINT 130, SIGHUP 129, SIGTERM 143: architecture 8.2)
#[test]
fn interrupt_returns_its_exit_code_after_restore() {
    for code in [130, 129, 143] {
        let mut handler = Recorder::new(3);
        handler.act_step = Step::Continue {
            status: None,
            reload: false,
        };
        let inputs = vec![ctrl('r'); 5];
        let (result, record) = run_engine(
            3,
            inputs,
            &mut handler,
            &FakeSignals::interrupt_on_call(3, code),
        );
        assert_eq!(
            result.unwrap(),
            PickerResult::Closed(Outcome::Interrupted(code)),
            "exit code {code}"
        );
        assert_eq!(handler.acts, 2, "keys after the interrupt are not read");
        assert_eq!(record.borrow().restores, 1, "restore for code {code}");
    }
}

// FR-TUI-06: an interrupt before any key is read still restores
#[test]
fn interrupt_before_any_key_restores_once() {
    let mut handler = Recorder::new(3);
    let (result, record) = run_engine(
        3,
        vec![key(KeyCode::Enter)],
        &mut handler,
        &FakeSignals::interrupt_on_call(1, 130),
    );
    assert_eq!(
        result.unwrap(),
        PickerResult::Closed(Outcome::Interrupted(130))
    );
    assert!(handler.chooses.is_empty());
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-06: a handler step that closes the picker returns its outcome and restores
#[test]
fn close_step_returns_the_outcome_and_restores_once() {
    let mut handler = Recorder::new(3);
    handler.act_step = Step::Close(Outcome::Info("done".to_string()));
    let (result, record) = run_engine(3, vec![ctrl('x')], &mut handler, &FakeSignals::none());
    assert_eq!(
        result.unwrap(),
        PickerResult::Closed(Outcome::Info("done".to_string()))
    );
    assert_eq!(record.borrow().restores, 1);
}

// FR-TUI-05: a resize keeps the selected row visible inside the engine loop (40x8 gives 2 list rows)
#[test]
fn resize_keeps_the_selected_row_visible_in_the_engine() {
    let mut handler = Recorder::new(20);
    let term = FakeTerminal::new(
        80,
        24,
        vec![key(KeyCode::End), Input::Resize(40, 8), key(KeyCode::Esc)],
    )
    .unwrap();
    let record = term.record();
    let result = run_picker(
        engine_model(20),
        &mut handler,
        Box::new(term),
        &FakeSignals::none(),
        &Theme::default(),
        false,
    );
    assert_eq!(result.unwrap(), PickerResult::Cancelled);
    let screen = record.borrow().lines.join("\n");
    assert!(screen.contains("row19"), "{screen}");
    assert_eq!(record.borrow().restores, 1);
}

// FR-ACT-04, D-12: a Profile step returns the row to the caller as a profile request; the picker is not closed
// by the engine's own rules and nothing else runs
#[test]
fn profile_step_returns_the_row_as_a_profile_request() {
    let mut handler = Recorder::new(3);
    handler.act_step = Step::Profile;
    let (result, record) = run_engine(3, vec![ctrl('p')], &mut handler, &FakeSignals::none());
    assert_eq!(result.unwrap(), PickerResult::Profile(0));
    assert_eq!(record.borrow().restores, 1);
}

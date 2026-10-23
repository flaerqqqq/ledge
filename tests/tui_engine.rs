//! TUI model tests (docs/testing.md level 3, S-07 part): keys, layout, resize, dimming and cues,
//! driven through `FakeTerminal` (ratatui `TestBackend`). `run_picker` (S-08) is not here yet, so
//! `drive` below is the same loop without the engine: keys go through `keys::map` and the model,
//! resizes set the list height, and every input is followed by a draw.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::tui::columns::ColumnSpec;
use ledge::tui::keys::{self, Action};
use ledge::tui::picker::{self, Chrome, Effect, PickerModel, PickerRow};
use ledge::tui::render::{self, icon};
use ledge::tui::terminal::{Input, TerminalSession};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use ratatui::Frame;
use std::time::Duration;
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
    let mut effects = Vec::new();
    let height = term.buffer().area.height;
    model.set_list_height(picker::list_height(height));
    draw(model, term, color);
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
        draw(model, term, color);
    }
    effects
}

fn draw(model: &PickerModel<usize>, term: &mut FakeTerminal, color: bool) {
    term.draw(&mut |frame: &mut Frame| {
        render::draw(model, frame.area(), frame, color);
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
    assert!(term.cell(1, y).unwrap().modifier.contains(Modifier::DIM));
    let (y, _) = line_containing(&term, "/x/here").unwrap();
    assert!(!term.cell(1, y).unwrap().modifier.contains(Modifier::DIM));

    let mut model = PickerModel::new(rows, chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(missing)").unwrap();
    assert!(line.contains("/x/gone (missing)"));
    assert!(!term.cell(1, y).unwrap().modifier.contains(Modifier::DIM));
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
    assert_eq!(term.cell(1, y).unwrap().fg, Color::DarkGray);

    let mut model = PickerModel::new(vec![connector], chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(work)").unwrap();
    assert!(line.contains("  (work)"));
    assert_ne!(term.cell(1, y).unwrap().fg, Color::DarkGray);
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
    for x in [1, 2] {
        let modifier = term.cell(x, y).unwrap().modifier;
        assert!(
            modifier.contains(Modifier::BOLD | Modifier::UNDERLINED),
            "x={x}"
        );
    }
    assert!(!term
        .cell(3, y)
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

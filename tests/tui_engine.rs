//! TUI model tests (docs/testing.md level 3): keys, layout, resize, dimming and cues, driven through
//! `FakeTerminal` (ratatui `TestBackend`). `drive` is the model part of the loop without the engine
//! (S-07). The engine tests at the end run `run_picker` itself (S-08): restore, interrupts, errors.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::error::{LedgeError, Outcome};
use ledge::tui::columns::ColumnSpec;
use ledge::tui::engine::{run_picker, PickerHandler, PickerResult, Step};
use ledge::tui::keys::{self, Action};
use ledge::tui::picker::{
    Chrome, Details, Effect, PickerModel, PickerRow, Preview, PreviewWindow, SessionCell,
};
use ledge::tui::render;
use ledge::tui::terminal::{Input, TerminalSession};
use ledge::tui::theme::{parse_hex, resolve, ColourLevel, Theme, Token};
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
    let area = term.buffer().area;
    model.set_list_height(model.list_rows(area.height, area.width));
    draw_themed(model, term, theme, color);
    let mut polls = 0;
    while let Some(input) = term.poll(Duration::from_millis(100)).unwrap() {
        polls += 1;
        assert!(polls < 1000, "the scripted run did not end");
        match &input {
            Input::Resize(width, height) => model.set_list_height(model.list_rows(*height, *width)),
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

// FR-TUI-04: header row, border label, footer, and the "> " prompt
#[test]
fn chrome_shows_label_prompt_header_status_and_footer() {
    let rows = vec![
        PickerRow::new(
            vec![
                "".into(),
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
        lines[0].starts_with("╭ Projects & Submodules "),
        "{:?}",
        lines[0]
    );
    assert!(lines[1].contains("> "), "prompt row: {:?}", lines[1]);
    assert!(lines[2].starts_with("│───"), "separator: {:?}", lines[2]);
    assert!(lines[3].contains("Project"), "header: {:?}", lines[3]);
    assert!(lines[3].contains("Session"), "header: {:?}", lines[3]);
    assert!(lines[3].contains("Profile"), "header: {:?}", lines[3]);
    assert!(lines[4].starts_with("│───"), "header rule: {:?}", lines[4]);
    assert!(lines[5].contains("/home/u/dev/alpha"), "{:?}", lines[5]);
    assert!(lines[18].contains("cannot kill the session you are in."));
    assert!(
        lines[19].contains(PROJECT_FOOTER),
        "footer: {:?}",
        lines[19]
    );
}

// FR-TUI-04, FR-TUI-09: the window label has no icon and takes the footer key colour (text), not the accent
#[test]
fn label_has_no_icon_and_takes_the_footer_key_colour() {
    let mut model = PickerModel::new(named_rows(3), projects_chrome());
    let mut term = FakeTerminal::new(80, 20, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let lines = term.lines();
    assert!(!lines[0].contains('▸'), "{:?}", lines[0]);
    // The label starts after the corner and one blank; the footer key `enter` starts after the corner and one blank.
    let label_fg = term.cell(2, 0).unwrap().fg;
    let key_fg = term.cell(2, 19).unwrap().fg;
    assert_eq!(lines[19].chars().nth(2), Some('e'), "{:?}", lines[19]);
    assert_eq!(label_fg, key_fg);
    assert_ne!(label_fg, Color::Cyan, "the label is not the accent");
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

    // The list is 13 rows high at 40x20 (prompt, separator, header, header rule and status take 7), so a page is 12.
    let (position, line) = run(vec![key(KeyCode::PageDown)]);
    assert_eq!(position, 12);
    assert!(line.unwrap().contains("row12"));

    let (position, _) = run(vec![key(KeyCode::PageDown), key(KeyCode::PageDown)]);
    assert_eq!(position, 24);

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
    // At 40x8 the list has one row: the selection is the only visible row.
    assert!(line_containing(&term, "row28").is_none());

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
    gone.missing = true;
    let rows = vec![
        gone,
        PickerRow::new(vec!["/x/here".into(), "".into()], "here", 1),
    ];

    let mut model = PickerModel::new(rows.clone(), chrome.clone());
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, line) = line_containing(&term, "(missing)").unwrap();
    assert!(line.contains("/x/gone (missing)"), "{line:?}");
    assert!(term.cell(3, y).unwrap().modifier.contains(Modifier::DIM));
    let (y, _) = line_containing(&term, "/x/here").unwrap();
    assert!(!term.cell(3, y).unwrap().modifier.contains(Modifier::DIM));

    let mut model = PickerModel::new(rows, chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(missing)").unwrap();
    assert!(line.contains("/x/gone (missing)"));
    assert!(!term.cell(3, y).unwrap().modifier.contains(Modifier::DIM));
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
    assert_eq!(term.cell(3, y).unwrap().fg, Color::DarkGray);

    let mut model = PickerModel::new(vec![connector], chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (y, line) = line_containing(&term, "(work)").unwrap();
    assert!(line.contains("  (work)"));
    assert_ne!(term.cell(3, y).unwrap().fg, Color::DarkGray);
}

/// Column of the first cell of `text` on the row that contains it.
fn cell_x(line: &str, text: &str) -> u16 {
    let at = line.find(text).expect("the text is on the row");
    u16::try_from(line[..at].chars().count()).expect("the column fits")
}

// design 6.1, 9 step 4b: a live session name takes session_active and the "(no session)" cell takes session_none
#[test]
fn session_cells_take_their_row_state_token() {
    let mut chrome = name_chrome("enter open");
    chrome.session_column = Some(1);
    let mut live = PickerRow::new(vec!["app".into(), "app-1a2b".into()], "app", 0);
    live.session_cell = SessionCell::Live;
    let mut none = PickerRow::new(vec!["lib".into(), "(no session)".into()], "lib", 1);
    none.session_cell = SessionCell::NoSession;
    let mut model = PickerModel::new(vec![live, none], chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, line) = line_containing(&term, "app-1a2b").unwrap();
    assert_eq!(
        term.cell(cell_x(&line, "app-1a2b"), y).unwrap().fg,
        Color::Green
    );
    let (y, line) = line_containing(&term, "(no session)").unwrap();
    assert_eq!(
        term.cell(cell_x(&line, "(no session)"), y).unwrap().fg,
        Color::DarkGray
    );
}

// design 3.6: the lead (the tree branch glyphs) is drawn before the cell text, and the (missing) cue needs the row's
// missing flag, not only its dimmed state (a gray placeholder is not missing). No row gets a star (FR-PICK-05)
#[test]
fn lead_is_drawn_before_the_text_and_the_cue_needs_the_missing_flag() {
    let mut chrome = Chrome::new(
        "Session tree",
        vec![ColumnSpec::name("Session"), ColumnSpec::text("Project")],
        "enter open",
    );
    chrome.missing_column = Some(1);
    let mut child = PickerRow::new(vec!["api-91bc".into(), "api".into()], "api", 0);
    child.lead = Some("\u{2514}\u{2500}".into());
    let mut gray = PickerRow::new(vec!["web (no session)".into(), "web".into()], "web", 1);
    gray.dimmed = true;
    let mut gone = PickerRow::new(vec!["old-0c4e".into(), "/x/old".into()], "old", 2);
    gone.missing = true;
    let mut model = PickerModel::new(vec![child, gray, gone], chrome);
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let (_, child_line) = line_containing(&term, "\u{2514}\u{2500}api-91bc").unwrap();
    assert!(
        child_line.starts_with("│ \u{2514}\u{2500}api-91bc"),
        "{child_line:?}"
    );
    assert!(!term.lines().iter().any(|line| line.contains('*')));
    assert!(line_containing(&term, "/x/old (missing)").is_some());
    assert!(line_containing(&term, "web (no session)")
        .unwrap()
        .1
        .find("(missing)")
        .is_none());
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
    // The name starts at x 2 (the left margin is x 1).
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
    assert!(term.lines()[0].starts_with("╭ Names "));
}

// FR-TUI-09, design 3.1: the selected row has no bar and no gap in column 0; reverse video runs over the whole row,
// its gaps included (no colour), and no bar is drawn anywhere
#[test]
fn selected_row_is_reverse_video_across_the_whole_row_with_no_bar() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, vec![key(KeyCode::Down)]).unwrap();
    drive(&mut model, &mut term, false);
    assert!(!term.lines().iter().any(|line| line.contains('▌')));
    let (y, _) = line_containing(&term, "row01-1a2b").unwrap();
    // Reverse video runs from the left margin (x 1) to the right margin (x 58), gaps included.
    assert_eq!(term.cell(2, y).unwrap().symbol(), "r");
    for x in 1..=58 {
        assert!(
            term.cell(x, y)
                .unwrap()
                .modifier
                .contains(Modifier::REVERSED),
            "x={x}"
        );
    }
    // The gap between Name (x 2..=6) and Session is x 7..=8. The row above is not selected: no reverse video there.
    let (other, _) = line_containing(&term, "row00-1a2b").unwrap();
    assert_eq!(term.cell(2, other).unwrap().symbol(), "r");
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
    // Left margin, Name, the gap, Session, its padding, and the right margin.
    for x in [1, 2, 6, 7, 8, 9, 19, 20, 58] {
        let cell = term.cell(x, y).unwrap();
        assert_eq!(cell.bg, selection_bg, "x={x}");
        assert!(!cell.modifier.contains(Modifier::REVERSED), "x={x}");
    }
}

// FR-TUI-04, design 4.2: at 60 cells the footer drops whole key pairs from the end, never half a pair
#[test]
fn footer_drops_whole_key_pairs_from_the_end_at_60_cells() {
    let footer = "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark";
    let mut model = PickerModel::new(named_rows(2), name_chrome(footer));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    // The footer is in the bottom edge, between the corner and the one-cell padding (line 11 at 60x12).
    let footer = term.lines()[11]
        .trim_start_matches('╰')
        .trim_end_matches(|c: char| "─╯ ".contains(c))
        .trim()
        .to_string();
    assert_eq!(footer, "enter open · ^r rebuild · ^p profile · ^x kill");
}

// FR-NAME-01, design 3.2: a long session name is cut in the middle and keeps its four-character hash
#[test]
fn long_session_name_is_cut_in_the_middle_keeping_the_hash() {
    let name = "api-gateway-service-long-session-name-91bc";
    let chrome = Chrome::new(
        "Sessions",
        vec![ColumnSpec::name("Session")],
        "enter switch",
    );
    let mut model = PickerModel::new(vec![PickerRow::new(vec![name.into()], name, 0)], chrome);
    let mut term = FakeTerminal::new(40, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    // 40 cells, 38 inside the border, 2 for the margins: 36 cells, the hash is 5 of them.
    let expected = format!("{}…-91bc", &name[..30]);
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
    assert_eq!(y, 10, "status row");
    assert!(line.contains("could not kill session 'x'."));
    // The message is on the right of the strip (design 2.1): bold from its first letter.
    let at = cell_x(&line, "could not");
    assert!(term.cell(at, y).unwrap().modifier.contains(Modifier::BOLD));
}

// design 3.1, 3.3: the frame is rounded; the window label is in the top edge and the footer keys in the bottom edge
#[test]
fn frame_is_rounded_with_label_on_top_and_footer_on_bottom() {
    let mut model = PickerModel::new(named_rows(3), projects_chrome());
    let mut term = FakeTerminal::new(80, 20, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    assert!(
        lines[0].starts_with("╭ Projects & Submodules "),
        "{:?}",
        lines[0]
    );
    assert!(lines[0].ends_with('╮'), "{:?}", lines[0]);
    assert!(
        lines[19].starts_with(&format!("╰ {PROJECT_FOOTER} ")),
        "footer in the bottom edge: {:?}",
        lines[19]
    );
    assert!(lines[19].ends_with('╯'), "{:?}", lines[19]);
    assert_eq!(term.cell(0, 0).unwrap().symbol(), "╭");
    assert_eq!(term.cell(79, 0).unwrap().symbol(), "╮");
    assert_eq!(term.cell(0, 19).unwrap().symbol(), "╰");
    assert_eq!(term.cell(79, 19).unwrap().symbol(), "╯");
    for y in 1..19 {
        assert_eq!(term.cell(0, y).unwrap().symbol(), "│", "y={y}");
        assert_eq!(term.cell(79, y).unwrap().symbol(), "│", "y={y}");
    }
    // The status row above the bottom border holds no footer keys.
    assert!(!lines[18].contains(PROJECT_FOOTER), "{:?}", lines[18]);
}

// design 3.1, 3.2: two cells between columns, in the header row and in the list
#[test]
fn columns_are_two_cells_apart_in_the_header_and_the_rows() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    let header = &lines[3];
    let (y, row) = line_containing(&term, "row00-1a2b").unwrap();
    // The name column is 5 cells wide (the widest name), so Session starts 5 + 2 cells after Name.
    let name_at = cell_x(&row, "row00");
    let session_at = cell_x(&row, "row00-1a2b");
    assert_eq!(session_at - name_at, 5 + 2, "{row:?}");
    assert_eq!(cell_x(header, "Name"), name_at, "{header:?}");
    assert_eq!(cell_x(header, "Session"), session_at, "{header:?}");
    for x in name_at + 5..session_at {
        assert_eq!(term.cell(x, y).unwrap().symbol(), " ", "x={x}");
    }
}

// design 3.1: a one-cell margin inside the frame on the left and the right, in the prompt and the list
#[test]
fn one_cell_margin_inside_the_frame_on_both_sides() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    assert!(lines[1].starts_with("│ > "), "{:?}", lines[1]);
    assert!(lines[1].ends_with(" │"), "{:?}", lines[1]);
    let (y, _) = line_containing(&term, "row00-1a2b").unwrap();
    assert_eq!(term.cell(1, y).unwrap().symbol(), " ", "left margin");
    assert_eq!(term.cell(58, y).unwrap().symbol(), " ", "right margin");
    assert_eq!(term.cell(59, y).unwrap().symbol(), "│");
}

// design 3.1, P-61: the header row is bold; a dim rule of ─ runs under the prompt row and under the header, across
// the inner width
#[test]
fn header_row_is_bold_and_has_dim_rules_under_the_prompt_and_under_it() {
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    assert!(term.cell(3, 3).unwrap().modifier.contains(Modifier::BOLD));
    for y in [2, 4] {
        assert_eq!(term.cell(0, y).unwrap().symbol(), "│", "y={y}");
        for x in 1..=58 {
            let cell = term.cell(x, y).unwrap();
            assert_eq!(cell.symbol(), "─", "x={x} y={y}");
            assert!(cell.modifier.contains(Modifier::DIM), "x={x} y={y}");
        }
        assert_eq!(term.cell(59, y).unwrap().symbol(), "│", "y={y}");
    }
}

// design 3.1, 4.5: the status line sits above the bottom border; information is dim, an error is bold
#[test]
fn status_line_sits_above_the_bottom_border_and_is_dim_unless_error() {
    let mut model = PickerModel::new(named_rows(2), name_chrome("enter open"));
    model.set_status(Some("Killed session 'x'.".into()));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive(&mut model, &mut term, true);
    let (y, _) = line_containing(&term, "Killed session").unwrap();
    assert_eq!(y, 10, "status row");
    assert_eq!(term.cell(2, y).unwrap().fg, Color::DarkGray);
    assert!(!term.cell(2, y).unwrap().modifier.contains(Modifier::BOLD));
    assert!(term.lines()[11].starts_with('╰'), "{:?}", term.lines()[11]);
}

// design 6.2, option B 2.3 (S-42): the zen theme has the Kanso zen values, and surface is its own token
#[test]
fn zen_theme_has_the_kanso_zen_values() {
    let zen = Theme::builtin("zen").unwrap();
    let expected = [
        (Token::Background, "#090E13"),
        (Token::Surface, "#1C1E25"),
        (Token::Border, "#393B44"),
        (Token::SelectionBg, "#393B44"),
        (Token::SelectionFg, "#C5C9C7"),
        (Token::Text, "#C5C9C7"),
        (Token::TextMuted, "#909398"),
        (Token::Accent, "#7FB4CA"),
        (Token::Success, "#98BB6C"),
        (Token::Warning, "#DCA561"),
        (Token::Error, "#E46876"),
    ];
    for (token, hex) in expected {
        assert_eq!(zen.color(token), parse_hex(hex).unwrap(), "{token:?}");
    }
    // 256-colour terminals: the nearest xterm index of each value (design 7, themes.md)
    let indexed = zen.at_level(ColourLevel::Indexed);
    assert_eq!(indexed.color(Token::Surface), Color::Indexed(234));
    assert_eq!(indexed.color(Token::Error), Color::Indexed(168));
    assert_eq!(indexed.color(Token::TextMuted), Color::Indexed(246));
    assert_eq!(indexed.color(Token::SelectionBg), Color::Indexed(237));
}

// FR-CONF-02, P-60: the config accepts the theme name zen; no warning, and the picker draws with the zen values
#[test]
fn zen_is_a_known_theme_name() {
    let resolved = resolve(Some("zen"), &[], ColourLevel::TrueColour);
    assert!(resolved.warnings.is_empty(), "{:?}", resolved.warnings);
    assert_eq!(resolved.theme, Theme::builtin("zen").unwrap());
}

// design 2.4, 2.3 (S-42): the column header is bold text_muted on the surface band, across the whole row
#[test]
fn column_header_sits_on_the_surface_band_with_zen() {
    let zen = Theme::builtin("zen").unwrap();
    let mut model = PickerModel::new(named_rows(3), name_chrome("enter open"));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive_themed(&mut model, &mut term, &zen, true);
    let surface = Color::Rgb(0x1c, 0x1e, 0x25);
    assert!(term.lines()[3].contains("Name") && term.lines()[3].contains("Session"));
    for x in 1..=58 {
        assert_eq!(term.cell(x, 3).unwrap().bg, surface, "x={x}");
    }
    let name = term.cell(2, 3).unwrap();
    assert_eq!(name.fg, parse_hex("#909398").unwrap());
    assert!(name.modifier.contains(Modifier::BOLD));
}

// design 2.1, 2.4 (S-42): the status strip is the row above the bottom edge, on the surface band; the state count is on
// the left and the last message on the right
#[test]
fn status_strip_is_above_the_bottom_edge_with_count_left_and_message_right() {
    let zen = Theme::builtin("zen").unwrap();
    let mut model = PickerModel::new(named_rows(2), name_chrome("enter open"));
    model.set_status(Some("Killed session 'x'.".into()));
    let mut term = FakeTerminal::new(60, 12, Vec::new()).unwrap();
    drive_themed(&mut model, &mut term, &zen, true);
    let lines = term.lines();
    assert!(lines[11].starts_with('╰'), "{:?}", lines[11]);
    let status = &lines[10];
    assert!(status.starts_with("│ 2 of 2 "), "{status:?}");
    assert!(
        status
            .trim_end()
            .trim_end_matches('│')
            .trim_end()
            .ends_with("Killed session 'x'."),
        "{status:?}"
    );
    for x in 1..=58 {
        assert_eq!(
            term.cell(x, 10).unwrap().bg,
            Color::Rgb(0x1c, 0x1e, 0x25),
            "x={x}"
        );
    }
}

/// The option B rows at 120 cells: a project with its path, session and profile.
fn wide_project_rows() -> Vec<PickerRow<usize>> {
    let mut ledge = PickerRow::new(
        vec![
            "ledge".into(),
            "ledge-3f2a".into(),
            "dev".into(),
            "~/dev/projects/ledge".into(),
        ],
        "ledge",
        0,
    );
    ledge.details = Some(Details {
        title: "ledge".into(),
        path: "~/dev/projects/ledge".into(),
        session: "ledge-3f2a, live".into(),
        windows: "3".into(),
        profile: "dev".into(),
    });
    let mut api = PickerRow::new(
        vec![
            "api-gateway".into(),
            "none".into(),
            "".into(),
            "~/dev/projects/api-gateway".into(),
        ],
        "api-gateway",
        1,
    );
    api.details = Some(Details {
        title: "api-gateway".into(),
        path: "~/dev/projects/api-gateway".into(),
        session: "none".into(),
        windows: String::new(),
        profile: String::new(),
    });
    vec![ledge, api]
}

fn wide_chrome() -> Chrome {
    Chrome::new(
        "Projects & Submodules",
        vec![
            ColumnSpec::path("Project"),
            ColumnSpec::name("Session"),
            ColumnSpec::text("Profile"),
            ColumnSpec::wide_path("Path"),
        ],
        PROJECT_FOOTER,
    )
}

// design 2.1, 2.4 (S-42): at 120 cells the list has a Path column, and a Details section sits under the list with the
// selected row's path, session facts and profile, between rules; the status strip is still above the bottom edge
#[test]
fn path_column_and_details_section_show_at_120_cells() {
    let mut model = PickerModel::new(wide_project_rows(), wide_chrome());
    let mut term = FakeTerminal::new(120, 24, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    let header = &lines[3];
    assert!(header.contains("Path"), "{header:?}");
    assert!(
        cell_x(header, "Path") > cell_x(header, "Profile"),
        "{header:?}"
    );
    let (_, row) = line_containing(&term, "ledge-3f2a").unwrap();
    assert!(row.contains("~/dev/projects/ledge"), "{row:?}");

    // list 5..=14 (10 rows at 24 lines), then the details rule at 15, the title at 16, four facts, the rule at 21
    let (title_y, title) = line_containing(&term, "Details · ledge").unwrap();
    assert_eq!(title_y, 16, "{title:?}");
    assert_eq!(
        term.cell(0, 15).unwrap().symbol(),
        "│",
        "rule above details"
    );
    assert_eq!(term.cell(0, 21).unwrap().symbol(), "│", "rule above status");
    for x in 1..=118 {
        assert_eq!(term.cell(x, 15).unwrap().symbol(), "─", "x={x}");
        assert_eq!(term.cell(x, 21).unwrap().symbol(), "─", "x={x}");
    }
    assert!(
        lines[17].contains("Path      ~/dev/projects/ledge"),
        "{:?}",
        lines[17]
    );
    assert!(
        lines[18].contains("Session   ledge-3f2a, live"),
        "{:?}",
        lines[18]
    );
    assert!(lines[19].contains("Windows   3"), "{:?}", lines[19]);
    assert!(lines[20].contains("Profile   dev"), "{:?}", lines[20]);
    assert!(lines[22].contains("2 of 2"), "{:?}", lines[22]);
    assert!(lines[23].starts_with('╰'), "{:?}", lines[23]);

    // Moving the selection changes the details
    let mut model = PickerModel::new(wide_project_rows(), wide_chrome());
    let mut term = FakeTerminal::new(120, 24, vec![key(KeyCode::Down)]).unwrap();
    drive(&mut model, &mut term, false);
    assert!(line_containing(&term, "Details · api-gateway").is_some());
    assert!(line_containing(&term, "Session   none").is_some());
}

/// Profile picker rows with a preview: a profile with two windows, and a broken one with its error.
fn profile_preview_rows() -> Vec<PickerRow<usize>> {
    let mut dev = PickerRow::new(vec!["dev".into()], "dev", 0);
    dev.preview = Some(Preview {
        title: "dev".into(),
        source: "~/cfg/profiles/dev.json".into(),
        windows: vec![
            PreviewWindow {
                name: "editor".into(),
                command: "nvim .".into(),
            },
            PreviewWindow {
                name: "shell".into(),
                command: String::new(),
            },
        ],
        error: None,
    });
    let mut broken = PickerRow::new(vec!["broken".into()], "broken", 1);
    broken.preview = Some(Preview {
        title: "broken".into(),
        source: "~/cfg/profiles/broken.json".into(),
        windows: Vec::new(),
        error: Some("profile 'broken' is invalid JSON: EOF".into()),
    });
    vec![dev, broken]
}

fn profile_preview_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        "Profiles",
        vec![ColumnSpec::text("Profile")],
        "enter choose · esc cancel",
    );
    chrome.preview = true;
    chrome
}

// FR-PROF-13 (S-45): the preview sits between the list and the status strip: a rule, the title, the source with the
// window count, one line per window with its command; the error of a broken profile is in the error token
#[test]
fn preview_draws_rule_title_source_and_windows_above_the_status_strip() {
    let mut model = PickerModel::new(profile_preview_rows(), profile_preview_chrome());
    let mut term = FakeTerminal::new(80, 24, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    // list 5..=13 (nine rows at 24 lines), the rule at 14, the title at 15, the source at 16, windows from 17
    assert_eq!(term.cell(1, 14).unwrap().symbol(), "─", "{lines:#?}");
    assert!(lines[15].contains("Preview · dev"), "{lines:#?}");
    assert!(
        lines[16].contains("~/cfg/profiles/dev.json · 2 windows"),
        "{lines:#?}"
    );
    assert!(
        lines[17].contains("editor") && lines[17].contains("nvim ."),
        "{lines:#?}"
    );
    assert!(lines[18].contains("shell"), "{lines:#?}");
    assert!(lines[22].contains("2 of 2"), "{lines:#?}");
    assert!(lines[23].starts_with('╰'), "{lines:#?}");

    let mut model = PickerModel::new(profile_preview_rows(), profile_preview_chrome());
    let mut term = FakeTerminal::new(80, 24, vec![key(KeyCode::Down)]).unwrap();
    drive(&mut model, &mut term, true);
    let lines = term.lines();
    assert!(lines[15].contains("Preview · broken"), "{lines:#?}");
    assert!(
        lines[16].contains("~/cfg/profiles/broken.json"),
        "{lines:#?}"
    );
    assert!(!lines[16].contains("windows"), "{lines:#?}");
    assert!(
        lines[17].contains("profile 'broken' is invalid JSON: EOF"),
        "{lines:#?}"
    );
    let theme = Theme::default();
    assert_eq!(term.cell(2, 17).unwrap().fg, theme.color(Token::Error));
}

// design 2.1 (S-42): below 120 cells there is no Path column and no Details section; the list gets its rows back
#[test]
fn path_column_and_details_are_left_out_below_120_cells() {
    let mut model = PickerModel::new(wide_project_rows(), wide_chrome());
    let mut term = FakeTerminal::new(119, 24, Vec::new()).unwrap();
    drive(&mut model, &mut term, false);
    let lines = term.lines();
    assert!(!lines[3].contains("Path"), "{:?}", lines[3]);
    assert!(lines.iter().all(|line| !line.contains("Details ·")));
    assert!(lines.iter().all(|line| !line.contains("~/dev/projects")));
    assert!(lines[5].contains("ledge-3f2a"), "{:?}", lines[5]);
    assert!(lines[22].contains("2 of 2"), "{:?}", lines[22]);
    assert!(lines[23].starts_with('╰'), "{:?}", lines[23]);
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

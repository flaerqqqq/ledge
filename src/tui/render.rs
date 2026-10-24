//! Picker rendering (architecture 6.2, design 3 and 6). `draw` is pure: it reads a `PickerModel` and writes into a
//! ratatui frame, so tests use `TestBackend` (tests/tui_engine.rs). Colours come only from the theme's tokens
//! (`tui::theme`). Colours are off when `NO_COLOR` is set or `TERM` is `dumb` (A-04); the text cues, reverse video and
//! bold are drawn in both cases.

use crate::messages;
use crate::tui::columns::{self, ColumnSpec, Cut, GAP};
use crate::tui::fuzzy;
use crate::tui::picker::{
    Chrome, PickerModel, PickerRow, SessionCell, DETAILS_ROWS, PREVIEW_ROWS, WIDE_WIDTH,
};
use crate::tui::theme::{Theme, Token};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

/// Blank cells inside the frame on the left and on the right of every line (design 3.1).
const MARGIN: usize = 1;
/// The rule under the prompt row and the rule under the column titles (design 3.1, P-61): one dim line across the
/// inner width.
const RULE: &str = "─";

/// Cells of a list line that the columns take: the inner width less both margins.
fn content_width(full: usize) -> usize {
    full.saturating_sub(2 * MARGIN)
}

/// Smallest terminal the picker draws in (A-06). Below it only the too-small line is shown.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 8;

/// FR-TUI-09, A-04: colors are on unless `NO_COLOR` is set and non-empty, or `TERM` is `dumb`.
/// The values are passed in so the rule is a pure function.
pub fn color_enabled(no_color: Option<&str>, term: Option<&str>) -> bool {
    let no_color_set = no_color.is_some_and(|value| !value.is_empty());
    let dumb = term == Some("dumb");
    !(no_color_set || dumb)
}

/// The style of a token: its colour when colour is on, and bold where the theme says so. Bold stays without colour, so
/// the error status is bold in every terminal (design 4.5).
fn paint(theme: &Theme, token: Token, color: bool) -> Style {
    let mut style = Style::new();
    if color {
        style = style.fg(theme.color(token));
    }
    if theme.is_bold(token) {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

/// Draws the whole picker into `area`. `color` comes from `color_enabled`; `theme` gives the colours (design 6).
/// FR-TUI-04, FR-TUI-05, FR-TUI-08, FR-TUI-09; option B (design-options 2): the surface band, the status strip and, at
/// 120 cells and up, the Path column and the Details section.
pub fn draw<T>(model: &PickerModel<T>, area: Rect, frame: &mut Frame, theme: &Theme, color: bool) {
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let line = Rect {
            height: area.height.min(1),
            ..area
        };
        frame.render_widget(Paragraph::new(messages::terminal_too_small()), line);
        return;
    }
    // Design 6.1: the window background, when the theme sets one (`default` leaves the terminal's).
    let background = theme.color(Token::Background);
    if color && background != Color::Reset {
        frame.render_widget(Block::new().style(Style::new().bg(background)), area);
    }
    let chrome = model.chrome();
    // The label in the top edge, one blank before it and after it (P-61: no icon). It is bold and takes the footer key
    // colour, the text token, so the label and the keys read as one line of chrome.
    let title = Span::styled(
        format!(" {} ", chrome.label),
        paint(theme, Token::Text, color).add_modifier(Modifier::BOLD),
    );
    // The footer keys sit in the bottom edge, fitted to the cells between the corners and the one-cell padding of
    // the edge (design 3.1). Pairs are dropped from the end when they do not fit (design 4.2).
    let footer_source = match (&chrome.empty_footer, model.rows().is_empty()) {
        (Some(empty), true) => empty.as_str(),
        _ => chrome.footer.as_str(),
    };
    let footer_text = columns::fit_pairs(footer_source, usize::from(area.width).saturating_sub(4));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(paint(theme, Token::Border, color))
        .title(Line::from(title))
        .title_bottom(footer_line(&footer_text, theme, color));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // The prompt row has no height unless the picker filters (architecture 6.2, D-15) or the edit window's search bar
    // is open or keeps a match (FR-EDIT-13).
    let prompted = model.has_prompt();
    let details = model.shows_details(area.width);
    // Option B: the Details section (rule, title, four facts, rule) sits between the list and the status strip.
    let details_height = if details { DETAILS_ROWS } else { 0 };
    // FR-PROF-13 (S-45): the preview of the selected profile sits under the list, when it is drawn.
    let preview = model.shows_preview(area.height, area.width);
    let preview_height = if preview { PREVIEW_ROWS } else { 0 };
    // The rule between the prompt row and the column titles is drawn only with the prompt row (P-61).
    let [prompt, prompt_rule, header, rule, list, preview_area, details_area, status] =
        Layout::vertical([
            Constraint::Length(u16::from(prompted)),
            Constraint::Length(u16::from(prompted)),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(preview_height),
            Constraint::Length(details_height),
            Constraint::Length(1),
        ])
        .areas(inner);
    let band = surface_band(theme, color);

    // Prompt: "> " and the filter, cursor at its end (architecture 6.2 item 1). The edit window's search bar has the
    // prompt "/" (FR-EDIT-13). The row is on the surface band (option B 2.1).
    if prompted {
        let filter = model.filter();
        let lead = if chrome.search_column.is_some() {
            "> "
        } else {
            messages::edit_search_prompt()
        };
        let line = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(lead.to_string(), paint(theme, Token::TextMuted, color)),
            Span::styled(filter.to_string(), paint(theme, Token::Text, color)),
        ]);
        frame.render_widget(Paragraph::new(line).style(band), prompt);
        let cursor = prompt
            .x
            .saturating_add(u16::try_from(MARGIN + columns::width(lead)).unwrap_or(u16::MAX))
            .saturating_add(u16::try_from(columns::width(filter)).unwrap_or(u16::MAX))
            .min(prompt.right().saturating_sub(1));
        frame.set_cursor_position((cursor, prompt.y));
    }

    let full = usize::from(inner.width);
    let shown = shown_columns(chrome, area.width);
    let widths = column_widths(model, content_width(full), shown);

    if prompted {
        draw_rule(frame, prompt_rule, full, theme, color);
    }

    // Column titles: text_muted and bold, on the surface band (design 3.1, option B 2.1).
    let title_style = paint(theme, Token::TextMuted, color).add_modifier(Modifier::BOLD);
    let header_parts: Vec<Vec<Span<'static>>> = chrome.columns[..shown]
        .iter()
        .zip(&widths)
        .map(|(spec, &w)| {
            let title = if spec.right {
                columns::fit_right(&spec.title, w)
            } else {
                columns::fit(&spec.title, w, Cut::End)
            };
            vec![Span::styled(title, title_style)]
        })
        .collect();
    let mut header_line = vec![Span::raw(" ".repeat(MARGIN))];
    header_line.extend(join(header_parts));
    frame.render_widget(Paragraph::new(Line::from(header_line)).style(band), header);

    // The rule under the titles: a dim line across the inner width (design 3.1).
    draw_rule(frame, rule, full, theme, color);

    let lines = list_lines(model, &widths, theme, color, full, shown);
    frame.render_widget(Paragraph::new(lines), list);

    if preview {
        draw_preview(frame, preview_area, model, theme, color, full);
    }

    if details {
        draw_details(frame, details_area, model, theme, color, full);
    }

    // Status strip, above the bottom border, on the surface band (option B 2.1): the count of rows on the left, the last
    // message on the right. An error is in the error token and bold; information is dim (design 4.5).
    let status_text = model.status().unwrap_or("");
    let status_token = if model.status_is_error() {
        Token::Error
    } else {
        Token::TextMuted
    };
    let count = format!("{} of {}", model.visible().len(), model.rows().len());
    let count_width = columns::width(&count);
    let room = full.saturating_sub(2 * MARGIN + count_width + 1);
    let message = columns::cut_end(status_text, room);
    let gap = full.saturating_sub(2 * MARGIN + count_width + columns::width(&message));
    let status_line = Line::from(vec![
        Span::raw(" ".repeat(MARGIN)),
        Span::styled(count, paint(theme, Token::TextMuted, color)),
        Span::raw(" ".repeat(gap)),
        Span::styled(message, paint(theme, status_token, color)),
        Span::raw(" ".repeat(MARGIN)),
    ]);
    frame.render_widget(Paragraph::new(status_line).style(band), status);
}

/// The surface band (design 2.3): the surface token as a background, when colour is on and the theme sets one. Without
/// colour, and for a theme whose surface is its background or the terminal's, no band is drawn.
fn surface_band(theme: &Theme, color: bool) -> Style {
    let surface = theme.color(Token::Surface);
    if color && surface != Color::Reset {
        Style::new().bg(surface)
    } else {
        Style::new()
    }
}

/// The Details section (option B 2.1): a rule, the title `Details · <name>`, four facts with their labels, and the rule
/// above the status strip. The labels are bold; a fact is cut to the width of the section.
fn draw_details<T>(
    frame: &mut Frame,
    area: Rect,
    model: &PickerModel<T>,
    theme: &Theme,
    color: bool,
    full: usize,
) {
    let rows = Layout::vertical([Constraint::Length(1); DETAILS_ROWS as usize]).split(area);
    draw_rule(frame, rows[0], full, theme, color);
    let details = model
        .selected_row()
        .and_then(|row| row.details.clone())
        .unwrap_or_default();
    let label_style = paint(theme, Token::Text, color).add_modifier(Modifier::BOLD);
    let value_style = paint(theme, Token::Text, color);
    let title = Line::from(vec![
        Span::raw(" ".repeat(MARGIN)),
        Span::styled(messages::DETAILS_LABEL.to_string(), label_style),
        Span::styled(format!(" · {}", details.title), value_style),
    ]);
    frame.render_widget(Paragraph::new(title), rows[1]);
    // The label column is 10 cells; the value gets the rest of the content width.
    let value_room = content_width(full).saturating_sub(DETAIL_LABEL_WIDTH);
    let facts = [
        (
            messages::HEADER_PATH,
            columns::cut_start(&details.path, value_room),
        ),
        (
            messages::HEADER_SESSION,
            columns::cut_end(&details.session, value_room),
        ),
        (
            messages::HEADER_WINDOWS,
            columns::cut_end(&details.windows, value_room),
        ),
        (
            messages::HEADER_PROFILE,
            columns::cut_end(&details.profile, value_room),
        ),
    ];
    for (row, (label, value)) in rows[2..6].iter().zip(facts) {
        let line = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(
                columns::fit(label, DETAIL_LABEL_WIDTH, Cut::End),
                label_style,
            ),
            Span::styled(value, value_style),
        ]);
        frame.render_widget(Paragraph::new(line), *row);
    }
    draw_rule(frame, rows[6], full, theme, color);
}

/// Cells of the label column of the Details section (option B 2.1), and the name column of the preview's windows.
const DETAIL_LABEL_WIDTH: usize = 10;

/// FR-PROF-13 (S-45, P-64): the rows of the preview that are not window lines: the rule, the title and the source.
const PREVIEW_HEADER_ROWS: usize = 3;

/// FR-PROF-13 (S-45, P-64): the preview of the selected profile (`PREVIEW_ROWS` rows): a rule, `Preview · <name>`, the
/// source with the window count, then one line per window with its command. A profile with more windows than the lines
/// shows the first ones and `+ N more` last. An error is drawn in the error token instead of the windows.
fn draw_preview<T>(
    frame: &mut Frame,
    area: Rect,
    model: &PickerModel<T>,
    theme: &Theme,
    color: bool,
    full: usize,
) {
    let rows = Layout::vertical([Constraint::Length(1); PREVIEW_ROWS as usize]).split(area);
    draw_rule(frame, rows[0], full, theme, color);
    let Some(preview) = model.selected_row().and_then(|row| row.preview.as_ref()) else {
        return;
    };
    let label_style = paint(theme, Token::Text, color).add_modifier(Modifier::BOLD);
    let value_style = paint(theme, Token::Text, color);
    let muted_style = paint(theme, Token::TextMuted, color);
    let title = Line::from(vec![
        Span::raw(" ".repeat(MARGIN)),
        Span::styled(messages::PREVIEW_LABEL.to_string(), label_style),
        Span::styled(format!(" · {}", preview.title), value_style),
    ]);
    frame.render_widget(Paragraph::new(title), rows[1]);
    let value_room = content_width(full);
    let source = match preview.error {
        Some(_) => preview.source.clone(),
        None => format!(
            "{} · {}",
            preview.source,
            messages::window_count(preview.windows.len())
        ),
    };
    let source_line = Line::from(vec![
        Span::raw(" ".repeat(MARGIN)),
        Span::styled(columns::cut_start(&source, value_room), value_style),
    ]);
    frame.render_widget(Paragraph::new(source_line), rows[2]);
    let lines = &rows[PREVIEW_HEADER_ROWS..];
    if let Some(error) = &preview.error {
        let error_style = paint(theme, Token::Error, color).add_modifier(Modifier::BOLD);
        let line = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(columns::cut_end(error, value_room), error_style),
        ]);
        frame.render_widget(Paragraph::new(line), lines[0]);
        return;
    }
    // The last line is `+ N more` when the windows do not fit.
    let windows = &preview.windows;
    let fits = windows.len() <= lines.len();
    let shown = if fits { windows.len() } else { lines.len() - 1 };
    let name_room = DETAIL_LABEL_WIDTH;
    let command_room = value_room.saturating_sub(name_room);
    for (line_area, window) in lines.iter().zip(windows.iter().take(shown)) {
        let line = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(columns::fit(&window.name, name_room, Cut::End), value_style),
            Span::styled(columns::cut_end(&window.command, command_room), muted_style),
        ]);
        frame.render_widget(Paragraph::new(line), *line_area);
    }
    if !fits {
        let more = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(messages::more_windows(windows.len() - shown), muted_style),
        ]);
        frame.render_widget(Paragraph::new(more), lines[shown]);
    }
}

/// The number of columns drawn at this window width: the wide-only columns (the Path column) are the last ones and
/// are left out below `WIDE_WIDTH` (option B 2.1).
fn shown_columns(chrome: &Chrome, width: u16) -> usize {
    if width >= WIDE_WIDTH {
        chrome.columns.len()
    } else {
        chrome
            .columns
            .iter()
            .take_while(|spec| !spec.wide_only)
            .count()
    }
}

/// A dim rule of `width` cells in `area` (design 3.1): the border token, dimmed when colour is on.
fn draw_rule(frame: &mut Frame, area: Rect, width: usize, theme: &Theme, color: bool) {
    let mut style = paint(theme, Token::Border, color);
    if color {
        style = style.add_modifier(Modifier::DIM);
    }
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(RULE.repeat(width), style))),
        area,
    );
}

/// A footer, in the bottom edge of the frame: each key and label pair is `key label`, pairs separated by the separator
/// (design 4.2). One blank cell is left on each side of the text, so it does not touch the corners.
fn footer_line(text: &str, theme: &Theme, color: bool) -> Line<'static> {
    let key = paint(theme, Token::Text, color).add_modifier(Modifier::BOLD);
    let label = paint(theme, Token::TextMuted, color);
    let mut spans = vec![Span::raw(" ")];
    for (index, pair) in text.split(columns::PAIR_SEP).enumerate() {
        if index > 0 {
            spans.push(Span::styled(columns::PAIR_SEP.to_string(), label));
        }
        match pair.split_once(' ') {
            Some((name, rest)) => {
                spans.push(Span::styled(name.to_string(), key));
                spans.push(Span::styled(format!(" {rest}"), label));
            }
            None => spans.push(Span::styled(pair.to_string(), key)),
        }
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

/// Column widths from the widest cell of each column, all rows and the title (stable while
/// filtering). `shown` is the number of columns drawn (`shown_columns`).
fn column_widths<T>(model: &PickerModel<T>, total: usize, shown: usize) -> Vec<usize> {
    let chrome = model.chrome();
    let natural: Vec<usize> = chrome.columns[..shown]
        .iter()
        .enumerate()
        .map(|(column, spec)| {
            model
                .rows()
                .iter()
                .map(|row| columns::width(&display_text(row, column, chrome)))
                .chain(std::iter::once(columns::width(&spec.title)))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let fixed: Vec<Option<usize>> = chrome.columns[..shown]
        .iter()
        .map(|c| c.fixed_width)
        .collect();
    columns::widths(&natural, &fixed, chrome.search_column, total)
}

/// Text of one cell with its prefix and suffix: the tree indent and the gray parentheses on the
/// first column (FR-TREE-11, A-04), and the ` (missing)` cue on the missing column (FR-TUI-08, A-04).
fn display_text<T>(row: &PickerRow<T>, column: usize, chrome: &Chrome) -> String {
    let (prefix, raw, suffix) = cell_parts(row, column, chrome);
    format!("{prefix}{raw}{suffix}")
}

fn cell_parts<T>(row: &PickerRow<T>, column: usize, chrome: &Chrome) -> (String, String, String) {
    let raw = row.cells.get(column).cloned().unwrap_or_default();
    let mut prefix = String::new();
    let mut suffix = String::new();
    if chrome.tree_column == Some(column) {
        // The lead (mark and branch glyphs) or the plain indent of two cells per depth (design 3.6, 3.8).
        match &row.lead {
            Some(lead) => prefix.push_str(lead),
            None => prefix.push_str(&"  ".repeat(row.depth)),
        }
        if row.connector {
            prefix.push('(');
            suffix.push(')');
        }
    }
    if row.missing && chrome.missing_column == Some(column) {
        suffix.push_str(" (missing)");
    }
    (prefix, raw, suffix)
}

/// Branch glyphs of a tree, one lead per row, two cells per level (design 3.6, 3.8, section 8 item 11). `depths` are the
/// depths of the rows in tree order. Each level above the row shows `│` while its ancestor has later siblings and
/// blank otherwise; the row's own level shows `├─` or `└─` (its last sibling). With `unicode` false, the ASCII set of
/// design section 7 (`|-`, `` `- ``, `| `) is used.
pub fn tree_guides(depths: &[usize], unicode: bool) -> Vec<String> {
    let (middle, last, pipe) = if unicode {
        ("├─", "└─", "│ ")
    } else {
        ("|-", "`-", "| ")
    };
    let blank = "  ";
    // `is_last[i]`: no later row of the same depth or deeper before a row shallower than row i.
    let is_last: Vec<bool> = (0..depths.len())
        .map(|i| {
            let depth = depths[i];
            match depths[i + 1..].iter().find(|&&later| later <= depth) {
                None => true,
                Some(&later) => later < depth,
            }
        })
        .collect();
    (0..depths.len())
        .map(|i| {
            let depth = depths[i];
            let mut lead = String::new();
            // The ancestor of row i at each level 1..depth; level 0 has no glyph.
            let mut ancestors = Vec::new();
            let mut level = depth;
            for above in (0..i).rev() {
                if level <= 1 {
                    break;
                }
                if depths[above] == level - 1 {
                    ancestors.push(above);
                    level -= 1;
                }
            }
            ancestors.reverse();
            for &ancestor in &ancestors {
                lead.push_str(if is_last[ancestor] { blank } else { pipe });
            }
            if depth > 0 {
                lead.push_str(if is_last[i] { last } else { middle });
            }
            lead
        })
        .collect()
}

fn list_lines<T>(
    model: &PickerModel<T>,
    widths: &[usize],
    theme: &Theme,
    color: bool,
    full: usize,
    shown: usize,
) -> Vec<Line<'static>> {
    let chrome = model.chrome();
    if model.rows().is_empty() {
        return chrome
            .empty_message
            .iter()
            .map(|text| {
                Line::from(vec![
                    Span::raw(" ".repeat(MARGIN)),
                    Span::styled(text.clone(), paint(theme, Token::TextMuted, color)),
                ])
            })
            .collect();
    }
    let selected = model.selected_position();
    model
        .view()
        .filter_map(|position| {
            let row_index = *model.visible().get(position)?;
            let row = model.rows().get(row_index)?;
            let filter = model.filter();
            let positions = match chrome.search_column {
                Some(_) if !filter.is_empty() => fuzzy::fuzzy_match(filter, &row.search_text)
                    .map(|m| m.positions)
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            Some(row_line(
                row,
                position == selected,
                &positions,
                chrome,
                widths,
                &Look {
                    theme,
                    color,
                    full,
                    shown,
                },
            ))
        })
        .collect()
}

/// What a row is drawn with: the theme, whether colour is on, the full inner width of the window, and how many
/// columns are drawn.
#[derive(Clone, Copy)]
struct Look<'a> {
    theme: &'a Theme,
    color: bool,
    full: usize,
    shown: usize,
}

/// One list row (design 3.1, 9.2a). The selected row runs over the whole window width, gaps included: reverse video
/// when the theme sets no selection colour (and without colour), otherwise the theme's selection colours. Matched
/// characters of the search column are bold, underlined and in accent (architecture 6.1).
fn row_line<T>(
    row: &PickerRow<T>,
    selected: bool,
    positions: &[usize],
    chrome: &Chrome,
    widths: &[usize],
    look: &Look<'_>,
) -> Line<'static> {
    let Look {
        theme,
        color,
        full,
        shown,
    } = *look;
    let matched = {
        let style = Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        style.patch(paint(theme, Token::Accent, color))
    };
    let parts: Vec<Vec<Span<'static>>> = chrome.columns[..shown]
        .iter()
        .enumerate()
        .map(|(column, spec)| {
            let width = widths.get(column).copied().unwrap_or(0);
            let highlight: &[usize] = if Some(column) == chrome.search_column {
                positions
            } else {
                &[]
            };
            let base = cell_style(row, column, spec, chrome, theme, color);
            cell_spans(row, column, spec, width, base, matched, highlight, chrome)
        })
        .collect();
    // The left margin, the cells, and the padding that runs to the right edge of the inner width, so a selected row
    // covers both margins too (design 3.1, 5).
    let mut spans = vec![Span::raw(" ".repeat(MARGIN))];
    spans.extend(join(parts));
    if selected {
        let used: usize = spans.iter().map(Span::width).sum();
        spans.push(Span::raw(" ".repeat(full.saturating_sub(used))));
        if color && !theme.selection_reversed() {
            let (fg, bg) = (
                theme.color(Token::SelectionFg),
                theme.color(Token::SelectionBg),
            );
            for span in &mut spans {
                span.style = span.style.bg(bg).fg(fg);
            }
        } else {
            for span in &mut spans {
                span.style = span.style.add_modifier(Modifier::REVERSED);
            }
        }
    }
    Line::from(spans)
}

/// The base style of one cell: the token of its role (connector, missing, path or text), a marker's token on a marker
/// column, and the dim modifier of a missing row when colour is on.
fn cell_style<T>(
    row: &PickerRow<T>,
    column: usize,
    spec: &ColumnSpec,
    chrome: &Chrome,
    theme: &Theme,
    color: bool,
) -> Style {
    let session_cell = chrome.session_column == Some(column);
    let token = if row.connector {
        Token::Connector
    } else if session_cell && row.session_cell == SessionCell::NoSession {
        Token::SessionNone
    } else if row.dimmed {
        Token::Missing
    } else if session_cell && row.session_cell == SessionCell::Live {
        Token::SessionActive
    } else if spec.cut == Cut::Path {
        // A path column (design 6.1 `path`).
        Token::Path
    } else {
        Token::Text
    };
    let mut style = paint(theme, token, color);
    if color && row.dimmed {
        style = style.add_modifier(Modifier::DIM);
    }
    // Marker columns: the bookmark mark `★` is accent (FR-EDIT-06).
    if spec.fixed_width.is_some() {
        let raw = row.cells.get(column).map_or("", String::as_str).trim();
        let marker = match raw {
            "★" => Some(Token::Accent),
            _ => None,
        };
        if let Some(marker) = marker {
            style = style.patch(paint(theme, marker, color));
        }
    }
    style
}

/// The spans of one cell, `width` cells wide. A cell that fits keeps its highlight. A cut cell keeps its tree prefix
/// and its ` (missing)` cue, cuts its text as the column says (design 3.2), and keeps the highlight of the characters
/// that stay.
#[allow(clippy::too_many_arguments)]
fn cell_spans<T>(
    row: &PickerRow<T>,
    column: usize,
    spec: &ColumnSpec,
    width: usize,
    base: Style,
    matched: Style,
    highlight: &[usize],
    chrome: &Chrome,
) -> Vec<Span<'static>> {
    let (prefix, raw, suffix) = cell_parts(row, column, chrome);
    let chars: Vec<char> = raw.chars().collect();
    let char_style = |index: usize| {
        if highlight.contains(&index) {
            base.patch(matched)
        } else {
            base
        }
    };
    let whole = columns::width(&prefix) + columns::width(&raw) + columns::width(&suffix);
    if whole <= width {
        let mut spans = vec![Span::styled(prefix, base)];
        spans.extend(
            chars
                .iter()
                .enumerate()
                .map(|(index, c)| Span::styled(c.to_string(), char_style(index))),
        );
        spans.push(Span::styled(suffix, base));
        let pad = Span::styled(" ".repeat(width - whole), base);
        // A right-aligned column (S-44, the Subfolders count) pads on the left.
        if spec.right {
            spans.insert(0, pad);
        } else {
            spans.push(pad);
        }
        return spans;
    }
    let budget = width.saturating_sub(columns::width(&prefix) + columns::width(&suffix));
    let kept = if budget > 0 {
        columns::kept(&raw, budget, spec.cut)
    } else {
        None
    };
    let Some(kept) = kept else {
        // Too narrow for the text: the whole cell is cut as the column says.
        let whole_text = format!("{prefix}{raw}{suffix}");
        return vec![Span::styled(
            columns::fit(&whole_text, width, spec.cut),
            base,
        )];
    };
    let mut spans = vec![Span::styled(prefix, base)];
    spans.extend(
        (0..kept.head).map(|index| Span::styled(chars[index].to_string(), char_style(index))),
    );
    spans.push(Span::styled("…", base));
    spans.extend(
        (chars.len() - kept.tail..chars.len())
            .map(|index| Span::styled(chars[index].to_string(), char_style(index))),
    );
    spans.push(Span::styled(suffix, base));
    let used: usize = spans.iter().map(Span::width).sum();
    let pad = Span::styled(" ".repeat(width.saturating_sub(used)), base);
    if spec.right {
        spans.insert(0, pad);
    } else {
        spans.push(pad);
    }
    spans
}

/// Joins cell spans with the column gap between them.
fn join(parts: Vec<Vec<Span<'static>>>) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            out.push(Span::raw(" ".repeat(GAP)));
        }
        out.extend(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // FR-TUI-09, A-04: NO_COLOR present and non-empty turns color off
    #[test]
    fn no_color_turns_color_off() {
        assert!(!color_enabled(Some("1"), None));
        assert!(!color_enabled(Some("yes"), Some("xterm-256color")));
    }

    // FR-TUI-09, A-04: an empty NO_COLOR does not count
    #[test]
    fn empty_no_color_keeps_color() {
        assert!(color_enabled(Some(""), Some("xterm-256color")));
        assert!(color_enabled(None, Some("xterm-256color")));
    }

    // FR-TUI-09, A-04: TERM=dumb turns color off
    #[test]
    fn dumb_terminal_turns_color_off() {
        assert!(!color_enabled(None, Some("dumb")));
        assert!(color_enabled(None, None));
    }

    // design 3.6, section 8 item 11: branch glyphs, the last sibling closes the branch, `│` runs past a row with later
    // siblings, two cells per level
    #[test]
    fn tree_guides_draw_branches_per_level() {
        let depths = [0, 1, 2, 1, 0, 1];
        assert_eq!(
            tree_guides(&depths, true),
            vec!["", "├─", "│ └─", "└─", "", "└─"]
        );
        // section 7: the ASCII fallback has the same shape
        assert_eq!(
            tree_guides(&depths, false),
            vec!["", "|-", "| `-", "`-", "", "`-"]
        );
    }
}

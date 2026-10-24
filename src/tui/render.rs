//! Picker rendering (architecture 6.2). `draw` is pure: it reads a `PickerModel` and writes into a
//! ratatui frame, so tests use `TestBackend` (tests/tui_engine.rs). Colors are off when `NO_COLOR` is
//! set or `TERM` is `dumb` (A-04); the text cues are drawn in both cases.

use crate::messages;
use crate::tui::columns::{self, GAP};
use crate::tui::fuzzy;
use crate::tui::picker::{Chrome, PickerModel, PickerRow};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Smallest terminal the picker draws in (A-06). Below it only the too-small line is shown.
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 8;

/// Border label icons per picker (architecture 6.2).
pub mod icon {
    pub const PROJECTS: &str = "▸";
    pub const HISTORY: &str = "↺";
    pub const SESSIONS: &str = "◉";
    pub const BOOKMARKS: &str = "★";
    pub const PROFILES: &str = "≡";
}

/// FR-TUI-09, A-04: colors are on unless `NO_COLOR` is set and non-empty, or `TERM` is `dumb`.
/// The values are passed in so the rule is a pure function.
pub fn color_enabled(no_color: Option<&str>, term: Option<&str>) -> bool {
    let no_color_set = no_color.is_some_and(|value| !value.is_empty());
    let dumb = term == Some("dumb");
    !(no_color_set || dumb)
}

/// Draws the whole picker into `area`. `color` comes from `color_enabled`.
/// FR-TUI-04, FR-TUI-05, FR-TUI-08, FR-TUI-09
pub fn draw<T>(model: &PickerModel<T>, area: Rect, frame: &mut Frame, color: bool) {
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let line = Rect {
            height: area.height.min(1),
            ..area
        };
        frame.render_widget(Paragraph::new(messages::terminal_too_small()), line);
        return;
    }
    let chrome = model.chrome();
    let block = Block::bordered().title(format!("{} {}", chrome.icon, chrome.label));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // The prompt row has no height unless the picker filters (architecture 6.2, D-15) or the edit window's search bar
    // is open or keeps a match (FR-EDIT-13).
    let prompted = model.has_prompt();
    let [prompt, header, list, status, footer] = Layout::vertical([
        Constraint::Length(u16::from(prompted)),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Prompt: "> " and the filter, cursor at its end (architecture 6.2 item 1). The edit window's search bar has the
    // prompt "/" (FR-EDIT-13).
    if prompted {
        let filter = model.filter();
        let lead = if chrome.search_column.is_some() {
            "> "
        } else {
            messages::edit_search_prompt()
        };
        frame.render_widget(Paragraph::new(format!("{lead}{filter}")), prompt);
        let cursor = prompt
            .x
            .saturating_add(u16::try_from(columns::width(lead)).unwrap_or(u16::MAX))
            .saturating_add(u16::try_from(columns::width(filter)).unwrap_or(u16::MAX))
            .min(prompt.right().saturating_sub(1));
        frame.set_cursor_position((cursor, prompt.y));
    }

    let widths = column_widths(model, usize::from(inner.width));

    let header_parts: Vec<Vec<Span<'static>>> = chrome
        .columns
        .iter()
        .zip(&widths)
        .map(|(spec, &w)| {
            vec![Span::styled(
                columns::fit(&spec.title, w, false),
                Style::new().add_modifier(Modifier::BOLD),
            )]
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(join(header_parts))), header);

    let lines = list_lines(model, &widths, color);
    frame.render_widget(Paragraph::new(lines), list);

    let status_text = model.status().unwrap_or("");
    frame.render_widget(
        Paragraph::new(columns::cut_end(status_text, usize::from(inner.width))),
        status,
    );
    frame.render_widget(
        Paragraph::new(columns::cut_end(&chrome.footer, usize::from(inner.width))),
        footer,
    );
}

/// Column widths from the widest cell of each column, all rows and the title (stable while
/// filtering).
fn column_widths<T>(model: &PickerModel<T>, total: usize) -> Vec<usize> {
    let chrome = model.chrome();
    let natural: Vec<usize> = chrome
        .columns
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
    let fixed: Vec<Option<usize>> = chrome.columns.iter().map(|c| c.fixed_width).collect();
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
        prefix.push_str(&"  ".repeat(row.depth));
        if row.connector {
            prefix.push('(');
            suffix.push(')');
        }
    }
    if row.dimmed && chrome.missing_column == Some(column) {
        suffix.push_str(" (missing)");
    }
    (prefix, raw, suffix)
}

fn list_lines<T>(model: &PickerModel<T>, widths: &[usize], color: bool) -> Vec<Line<'static>> {
    let chrome = model.chrome();
    if model.rows().is_empty() {
        return chrome
            .empty_message
            .iter()
            .map(|text| Line::from(text.clone()))
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
                color,
            ))
        })
        .collect()
}

/// One list row. The selected row is reverse video (works without color, A-04). Matched characters
/// of the search column are bold and underlined (architecture 6.1).
fn row_line<T>(
    row: &PickerRow<T>,
    selected: bool,
    positions: &[usize],
    chrome: &Chrome,
    widths: &[usize],
    color: bool,
) -> Line<'static> {
    let mut base = Style::new();
    if color {
        if row.dimmed {
            base = base.add_modifier(Modifier::DIM);
        }
        if row.connector {
            base = base.fg(Color::DarkGray);
        }
    }
    let parts: Vec<Vec<Span<'static>>> = chrome
        .columns
        .iter()
        .enumerate()
        .map(|(column, spec)| {
            let width = widths.get(column).copied().unwrap_or(0);
            let highlight: &[usize] = if Some(column) == chrome.search_column {
                positions
            } else {
                &[]
            };
            let mut spans = cell_spans(row, column, spec.path, width, base, highlight, chrome);
            if selected {
                for span in &mut spans {
                    span.style = span.style.add_modifier(Modifier::REVERSED);
                }
            }
            spans
        })
        .collect();
    Line::from(join(parts))
}

fn cell_spans<T>(
    row: &PickerRow<T>,
    column: usize,
    from_left: bool,
    width: usize,
    base: Style,
    highlight: &[usize],
    chrome: &Chrome,
) -> Vec<Span<'static>> {
    let (prefix, raw, suffix) = cell_parts(row, column, chrome);
    let whole = format!("{prefix}{raw}{suffix}");
    if columns::width(&whole) > width {
        // Truncated cells are drawn plain: highlight positions would no longer line up.
        return vec![Span::styled(columns::fit(&whole, width, from_left), base)];
    }
    let mut spans = vec![Span::styled(prefix, base)];
    let raw_chars: Vec<char> = raw.chars().collect();
    for (index, c) in raw_chars.iter().enumerate() {
        let style = if highlight.contains(&index) {
            base.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            base
        };
        spans.push(Span::styled(c.to_string(), style));
    }
    spans.push(Span::styled(suffix, base));
    let pad = width.saturating_sub(columns::width(&whole));
    spans.push(Span::styled(" ".repeat(pad), base));
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
}

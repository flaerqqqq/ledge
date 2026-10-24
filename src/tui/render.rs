//! Picker rendering (architecture 6.2, design 3 and 6). `draw` is pure: it reads a `PickerModel` and writes into a
//! ratatui frame, so tests use `TestBackend` (tests/tui_engine.rs). Colours come only from the theme's tokens
//! (`tui::theme`). Colours are off when `NO_COLOR` is set or `TERM` is `dumb` (A-04); the text cues, the selection bar,
//! reverse video and bold are drawn in both cases.

use crate::messages;
use crate::tui::columns::{self, ColumnSpec, Cut, GAP};
use crate::tui::fuzzy;
use crate::tui::picker::{Chrome, PickerModel, PickerRow, SessionCell};
use crate::tui::theme::{Theme, Token};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

/// The selection bar, drawn in column 0 of the selected row (design 3.1, 9.2a).
const BAR: &str = "▌";
/// Cells the selection bar takes from the inner width.
const BAR_WIDTH: usize = 1;
/// Blank cells inside the frame on the left and on the right of every line (design 3.1).
const MARGIN: usize = 1;
/// The rule under the prompt row and the rule under the column titles (design 3.1, P-61): one dim line across the
/// inner width.
const RULE: &str = "─";

/// Cells of a list line that the columns take: the inner width less both margins and the selection bar.
fn content_width(full: usize) -> usize {
    full.saturating_sub(2 * MARGIN + BAR_WIDTH)
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
/// FR-TUI-04, FR-TUI-05, FR-TUI-08, FR-TUI-09
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
    // The label in the top edge, one blank before it and after it (P-61: no icon). It takes the footer key colour, the
    // text token, so the label and the keys read as one line of chrome.
    let title = Span::styled(
        format!(" {} ", chrome.label),
        paint(theme, Token::Text, color),
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
    // The rule between the prompt row and the column titles is drawn only with the prompt row (P-61).
    let [prompt, prompt_rule, header, rule, list, status] = Layout::vertical([
        Constraint::Length(u16::from(prompted)),
        Constraint::Length(u16::from(prompted)),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
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
        let line = Line::from(vec![
            Span::raw(" ".repeat(MARGIN)),
            Span::styled(lead.to_string(), paint(theme, Token::TextMuted, color)),
            Span::styled(filter.to_string(), paint(theme, Token::Text, color)),
        ]);
        frame.render_widget(Paragraph::new(line), prompt);
        let cursor = prompt
            .x
            .saturating_add(u16::try_from(MARGIN + columns::width(lead)).unwrap_or(u16::MAX))
            .saturating_add(u16::try_from(columns::width(filter)).unwrap_or(u16::MAX))
            .min(prompt.right().saturating_sub(1));
        frame.set_cursor_position((cursor, prompt.y));
    }

    let full = usize::from(inner.width);
    let widths = column_widths(model, content_width(full));

    if prompted {
        draw_rule(frame, prompt_rule, full, theme, color);
    }

    // Column titles: text_muted and bold (design 3.1). The first cell after the margin is blank, where the selection
    // bar is drawn.
    let title_style = paint(theme, Token::TextMuted, color).add_modifier(Modifier::BOLD);
    let header_parts: Vec<Vec<Span<'static>>> = chrome
        .columns
        .iter()
        .zip(&widths)
        .map(|(spec, &w)| {
            vec![Span::styled(
                columns::fit(&spec.title, w, false),
                title_style,
            )]
        })
        .collect();
    let mut header_line = vec![Span::raw(" ".repeat(MARGIN + BAR_WIDTH))];
    header_line.extend(join(header_parts));
    frame.render_widget(Paragraph::new(Line::from(header_line)), header);

    // The rule under the titles: a dim line across the inner width (design 3.1).
    draw_rule(frame, rule, full, theme, color);

    let lines = list_lines(model, &widths, theme, color, full);
    frame.render_widget(Paragraph::new(lines), list);

    // Status row, above the bottom border: an error is in the error token and bold; information is dim (design 4.5).
    let status_text = model.status().unwrap_or("");
    let status_token = if model.status_is_error() {
        Token::Error
    } else {
        Token::TextMuted
    };
    let status_line = Span::styled(
        columns::cut_end(status_text, full.saturating_sub(2 * MARGIN)),
        paint(theme, status_token, color),
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::raw(" ".repeat(MARGIN)), status_line])),
        status,
    );
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
    let key = paint(theme, Token::Text, color);
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
                &Look { theme, color, full },
            ))
        })
        .collect()
}

/// What a row is drawn with: the theme, whether colour is on, and the full inner width of the window.
#[derive(Clone, Copy)]
struct Look<'a> {
    theme: &'a Theme,
    color: bool,
    full: usize,
}

/// One list row (design 3.1, 9.2a). The selected row has the bar in column 0 and runs over the whole window width,
/// gaps included: reverse video when the theme sets no selection colour (and without colour), otherwise the theme's
/// selection colours. Matched characters of the search column are bold, underlined and in accent (architecture 6.1).
fn row_line<T>(
    row: &PickerRow<T>,
    selected: bool,
    positions: &[usize],
    chrome: &Chrome,
    widths: &[usize],
    look: &Look<'_>,
) -> Line<'static> {
    let Look { theme, color, full } = *look;
    let matched = {
        let style = Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        style.patch(paint(theme, Token::Accent, color))
    };
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
            let base = cell_style(row, column, spec, chrome, theme, color);
            let mark = paint(theme, Token::Accent, color);
            cell_spans(
                row, column, spec, width, base, matched, mark, highlight, chrome,
            )
        })
        .collect();
    // The left margin, the bar (or a blank in its place), the cells, and the padding that runs to the right edge of
    // the inner width, so a selected row covers both margins too (design 3.1, 5).
    let mut spans = vec![
        Span::raw(" ".repeat(MARGIN)),
        if selected {
            Span::styled(BAR, paint(theme, Token::Accent, color))
        } else {
            Span::raw(" ".repeat(BAR_WIDTH))
        },
    ];
    spans.extend(join(parts));
    if selected {
        let used: usize = spans.iter().map(Span::width).sum();
        spans.push(Span::raw(" ".repeat(full.saturating_sub(used))));
        if color && !theme.selection_reversed() {
            let (fg, bg) = (
                theme.color(Token::SelectionFg),
                theme.color(Token::SelectionBg),
            );
            for (index, span) in spans.iter_mut().enumerate() {
                // The bar keeps the accent colour; the rest of the row takes the selection colours.
                span.style = span.style.bg(bg);
                if index > 1 {
                    span.style = span.style.fg(fg);
                }
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
    } else if spec.cut == Cut::Start {
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
    mark: Style,
    highlight: &[usize],
    chrome: &Chrome,
) -> Vec<Span<'static>> {
    let (prefix, raw, suffix) = cell_parts(row, column, chrome);
    // The current mark of a cell is the accent token (design 3.5, P-61).
    let lead_spans = |prefix: String| -> Vec<Span<'static>> {
        match prefix.strip_prefix('*') {
            Some(rest) => vec![
                Span::styled("*".to_string(), base.patch(mark)),
                Span::styled(rest.to_string(), base),
            ],
            None => vec![Span::styled(prefix, base)],
        }
    };
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
        let mut spans = lead_spans(prefix);
        spans.extend(
            chars
                .iter()
                .enumerate()
                .map(|(index, c)| Span::styled(c.to_string(), char_style(index))),
        );
        spans.push(Span::styled(suffix, base));
        spans.push(Span::styled(" ".repeat(width - whole), base));
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
        let from_left = spec.cut == Cut::Start;
        return vec![Span::styled(
            columns::fit(&whole_text, width, from_left),
            base,
        )];
    };
    let mut spans = lead_spans(prefix);
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
    spans.push(Span::styled(" ".repeat(width.saturating_sub(used)), base));
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

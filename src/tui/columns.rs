//! Column widths, alignment and truncation (architecture 6.2, A-06, F-12). Pure: widths are display
//! cells as ratatui counts them, so a CJK character takes two cells.

use ratatui::text::Span;

/// Cells between two columns.
pub const GAP: usize = 1;
const ELLIPSIS: char = '…';
// A-06 shrink floors: last column 6, search column 12, other columns 4.
const FLOOR_LAST: usize = 6;
const FLOOR_SEARCH: usize = 12;
const FLOOR_OTHER: usize = 4;

/// One column of a picker: its title, an optional fixed width (the marker column is 2 cells), and
/// whether long text is cut from the left (paths) or at the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSpec {
    pub title: String,
    pub fixed_width: Option<usize>,
    pub path: bool,
}

impl ColumnSpec {
    pub fn text(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: None,
            path: false,
        }
    }

    pub fn path(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: None,
            path: true,
        }
    }

    pub fn fixed(title: &str, width: usize) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: Some(width),
            path: false,
        }
    }
}

/// Display width of `text` in terminal cells.
pub fn width(text: &str) -> usize {
    Span::raw(text).width()
}

fn char_width(c: char) -> usize {
    width(c.encode_utf8(&mut [0; 4]))
}

/// Column widths for `total` cells (A-06). `natural[i]` is the widest text of column i and
/// `fixed[i]` its fixed width if it has one. When the row does not fit, the shrink order is: the
/// last column down to 6, the search column down to 12, then the other columns down to 4, right to
/// left. A column is never shrunk below its floor, and a fixed column is never shrunk.
pub fn widths(
    natural: &[usize],
    fixed: &[Option<usize>],
    search: Option<usize>,
    total: usize,
) -> Vec<usize> {
    let is_fixed = |i: usize| fixed.get(i).copied().flatten().is_some();
    let mut w: Vec<usize> = natural
        .iter()
        .enumerate()
        .map(|(i, &n)| fixed.get(i).copied().flatten().unwrap_or(n))
        .collect();
    let count = w.len();
    let avail = total.saturating_sub(count.saturating_sub(1) * GAP);
    let mut excess = w.iter().sum::<usize>().saturating_sub(avail);
    if excess == 0 {
        return w;
    }
    let mut steps: Vec<(usize, usize)> = Vec::new();
    if let Some(last) = count.checked_sub(1) {
        let floor = if Some(last) == search {
            FLOOR_SEARCH
        } else {
            FLOOR_LAST
        };
        steps.push((last, floor));
    }
    if let Some(s) = search {
        steps.push((s, FLOOR_SEARCH));
    }
    for i in (0..count).rev() {
        if Some(i) != search {
            steps.push((i, FLOOR_OTHER));
        }
    }
    for (i, floor) in steps {
        if is_fixed(i) {
            continue;
        }
        if let Some(cell) = w.get_mut(i) {
            let cut = cell.saturating_sub(floor).min(excess);
            *cell -= cut;
            excess -= cut;
        }
    }
    w
}

/// Cuts `text` at the end to at most `max` cells, marking the cut with `…`.
pub fn cut_end(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if width(text) <= max {
        return text.to_string();
    }
    let budget = max - char_width(ELLIPSIS);
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = char_width(c);
        if used + w > budget {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push(ELLIPSIS);
    out
}

/// Cuts `text` at the start to at most `max` cells, keeping the tail and marking the cut with `…`.
pub fn cut_start(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if width(text) <= max {
        return text.to_string();
    }
    let budget = max - char_width(ELLIPSIS);
    let mut kept: Vec<char> = Vec::new();
    let mut used = 0;
    for c in text.chars().rev() {
        let w = char_width(c);
        if used + w > budget {
            break;
        }
        kept.push(c);
        used += w;
    }
    let mut out = String::from(ELLIPSIS);
    out.extend(kept.iter().rev());
    out
}

/// Exactly `max` cells: the text cut as needed (from the left when `from_left`), then padded with
/// spaces.
pub fn fit(text: &str, max: usize, from_left: bool) -> String {
    let cut = if width(text) <= max {
        text.to_string()
    } else if from_left {
        cut_start(text, max)
    } else {
        cut_end(text, max)
    };
    let pad = max.saturating_sub(width(&cut));
    format!("{cut}{}", " ".repeat(pad))
}

#[cfg(test)]
mod tests {
    use super::*;

    // FR-TUI-05, A-06: shrink order of the last, search and other columns
    #[test]
    fn shrinks_last_then_search_then_others() {
        let natural = [2, 30, 30, 10];
        let fixed = [Some(2), None, None, None];
        // 40 cells minus 3 gaps leaves 37: the last column goes to 6, the search column to 12,
        // then the third column to fit.
        assert_eq!(widths(&natural, &fixed, Some(1), 40), vec![2, 12, 19, 4]);
    }

    // FR-TUI-05, A-06: a row that fits keeps its natural widths
    #[test]
    fn keeps_natural_widths_when_they_fit() {
        let natural = [2, 8, 5];
        assert_eq!(widths(&natural, &[None, None, None], Some(1), 80), natural);
    }

    // FR-TUI-05, A-06: the fixed marker column stays 2 cells
    #[test]
    fn fixed_column_keeps_its_width_when_shrinking() {
        let natural = [2, 50];
        let fixed = [Some(2), None];
        assert_eq!(widths(&natural, &fixed, Some(1), 20)[0], 2);
    }

    // FR-TUI-05, A-06: long text is cut at the end with an ellipsis
    #[test]
    fn cut_end_marks_the_cut() {
        assert_eq!(cut_end("abcdefgh", 5), "abcd…");
        assert_eq!(cut_end("abc", 5), "abc");
    }

    // FR-TUI-05, A-06: paths keep their tail
    #[test]
    fn cut_start_keeps_the_tail() {
        assert_eq!(cut_start("/home/u/dev/ledge", 7), "…/ledge");
    }

    // FR-TUI-05, A-06: CJK characters count as two cells
    #[test]
    fn cjk_characters_count_two_cells() {
        assert_eq!(width("日本"), 4);
        assert_eq!(cut_end("日本語", 4), "日…");
        assert_eq!(fit("日本語", 4, false), "日… ");
    }

    #[test]
    fn fit_pads_to_the_exact_width() {
        assert_eq!(fit("ab", 5, false), "ab   ");
        assert_eq!(fit("", 3, false), "   ");
    }
}

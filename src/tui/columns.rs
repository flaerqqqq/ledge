//! Column widths, alignment and truncation (architecture 6.2, A-06, F-12). Pure: widths are display
//! cells as ratatui counts them, so a CJK character takes two cells.

use ratatui::text::Span;

/// Cells between two columns (design 3.2).
pub const GAP: usize = 2;
const ELLIPSIS: char = '…';
// A-06 shrink floors: last column 6, search column 12, other columns 4.
const FLOOR_LAST: usize = 6;
const FLOOR_SEARCH: usize = 12;
const FLOOR_OTHER: usize = 4;

/// How long text is cut to fit (design 3.2): paths keep their tail, session names keep their start and their hash,
/// other text keeps its start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    End,
    Start,
    Middle,
}

/// One column of a picker: its title, an optional fixed width (the marker column is 2 cells), and how its long text
/// is cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSpec {
    pub title: String,
    pub fixed_width: Option<usize>,
    pub cut: Cut,
    /// Shown only at 120 cells and up (design 2.1, option B: the Path column). Such columns are the last ones.
    pub wide_only: bool,
    /// Text is right-aligned in the column (S-44: the Subfolders count). The title is aligned the same way.
    pub right: bool,
}

impl ColumnSpec {
    pub fn text(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: None,
            cut: Cut::End,
            wide_only: false,
            right: false,
        }
    }

    /// A count column of `width` cells whose text and title are right-aligned (S-44, option B 4.3).
    pub fn count(title: &str, width: usize) -> Self {
        Self {
            right: true,
            ..Self::fixed(title, width)
        }
    }

    pub fn path(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: None,
            cut: Cut::Start,
            wide_only: false,
            right: false,
        }
    }

    /// A path column shown only at 120 cells and up (design 2.1, option B).
    pub fn wide_path(title: &str) -> Self {
        Self {
            wide_only: true,
            ..Self::path(title)
        }
    }

    /// A session name: cut in the middle, so its hash stays visible (design 3.2, FR-NAME-01).
    pub fn name(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: None,
            cut: Cut::Middle,
            wide_only: false,
            right: false,
        }
    }

    pub fn fixed(title: &str, width: usize) -> Self {
        Self {
            title: title.to_string(),
            fixed_width: Some(width),
            cut: Cut::End,
            wide_only: false,
            right: false,
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

/// Cells of a session name kept after the ellipsis when it is cut in the middle: the hyphen and the four characters
/// of the hash (`-91bc`).
const MIDDLE_TAIL: usize = 5;
/// Separator between the key and label pairs of a footer (design 4.2).
pub const PAIR_SEP: &str = " · ";

/// Which characters of a cut text stay: `head` characters from the start and `tail` from the end, with one ellipsis
/// between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kept {
    pub head: usize,
    pub tail: usize,
}

/// How `text` is cut to `max` cells, or `None` when it fits. The ellipsis is one cell and is always part of the cut.
pub fn kept(text: &str, max: usize, cut: Cut) -> Option<Kept> {
    if width(text) <= max {
        return None;
    }
    let budget = max.saturating_sub(char_width(ELLIPSIS));
    let (head_budget, tail_budget) = match cut {
        Cut::End => (budget, 0),
        Cut::Start => (0, budget),
        Cut::Middle => {
            let tail = budget.min(MIDDLE_TAIL);
            (budget - tail, tail)
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let mut head = 0;
    let mut used = 0;
    for c in &chars {
        let w = char_width(*c);
        if used + w > head_budget {
            break;
        }
        used += w;
        head += 1;
    }
    let mut tail = 0;
    used = 0;
    for c in chars[head..].iter().rev() {
        let w = char_width(*c);
        if used + w > tail_budget {
            break;
        }
        used += w;
        tail += 1;
    }
    Some(Kept { head, tail })
}

/// Cuts `text` to at most `max` cells in the way `cut` names, marking the cut with `…`.
fn cut_with(text: &str, max: usize, cut: Cut) -> String {
    if max == 0 {
        return String::new();
    }
    let Some(kept) = kept(text, max, cut) else {
        return text.to_string();
    };
    let chars: Vec<char> = text.chars().collect();
    let mut out: String = chars[..kept.head].iter().collect();
    out.push(ELLIPSIS);
    out.extend(chars[chars.len() - kept.tail..].iter());
    out
}

/// Cuts `text` at the end to at most `max` cells, marking the cut with `…`.
pub fn cut_end(text: &str, max: usize) -> String {
    cut_with(text, max, Cut::End)
}

/// Cuts `text` at the start to at most `max` cells, keeping the tail and marking the cut with `…`.
pub fn cut_start(text: &str, max: usize) -> String {
    cut_with(text, max, Cut::Start)
}

/// Cuts `text` in the middle to at most `max` cells: the start stays, then `…`, then the last five cells (the hash
/// of a session name, FR-NAME-01, design 3.2).
pub fn cut_middle(text: &str, max: usize) -> String {
    cut_with(text, max, Cut::Middle)
}

/// Fits a footer of key and label pairs (design 4.2, P-59 FR-TUI-13) in `max` cells. When it is too wide, whole
/// pairs are dropped from the end and no key is cut. A first pair that is wider than `max` alone is cut at the end.
pub fn fit_pairs(footer: &str, max: usize) -> String {
    if width(footer) <= max {
        return footer.to_string();
    }
    let pairs: Vec<&str> = footer.split(PAIR_SEP).collect();
    for keep in (1..pairs.len()).rev() {
        let candidate = pairs[..keep].join(PAIR_SEP);
        if width(&candidate) <= max {
            return candidate;
        }
    }
    cut_end(pairs.first().copied().unwrap_or(""), max)
}

/// Exactly `max` cells: the text cut at the end when it is too long, then padded with spaces on the left, so the text
/// is right-aligned (S-44, the Subfolders count).
pub fn fit_right(text: &str, max: usize) -> String {
    let cut = if width(text) <= max {
        text.to_string()
    } else {
        cut_end(text, max)
    };
    let pad = max.saturating_sub(width(&cut));
    format!("{}{cut}", " ".repeat(pad))
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
        // 40 cells minus 3 gaps of 2 leaves 34: the last column goes to 6, the search column to 12,
        // then the last column to 4 and the third column to 16 to fit.
        assert_eq!(widths(&natural, &fixed, Some(1), 40), vec![2, 12, 16, 4]);
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

    // design 3.2: a session name keeps its start and its four-character hash
    #[test]
    fn cut_middle_keeps_the_start_and_the_hash() {
        assert_eq!(cut_middle("api-gateway-service-91bc", 14), "api-gate…-91bc");
        assert_eq!(cut_middle("api-gate-91bc", 13), "api-gate-91bc");
        assert_eq!(cut_middle("api-gateway-91bc", 9), "api…-91bc");
    }

    // design 3.2: CJK characters count as two cells in the middle cut too
    #[test]
    fn cut_middle_counts_cjk_as_two_cells() {
        // 日本語テキスト-91bc: tail "-91bc" is 5 cells, the ellipsis 1, the head 4 cells (日本).
        assert_eq!(cut_middle("日本語テキスト-91bc", 10), "日本…-91bc");
        assert!(width(&cut_middle("日本語テキスト-91bc", 10)) <= 10);
    }

    // design 3.2: a name that fits is kept, and the widest cut keeps nothing but the ellipsis
    #[test]
    fn cut_middle_fits_or_marks_the_cut() {
        assert_eq!(cut_middle("short", 5), "short");
        assert_eq!(cut_middle("abcdef", 1), "…");
        assert_eq!(cut_middle("abcdef", 0), "");
    }

    // design 4.2: whole pairs are dropped from the end, never a pair in half
    #[test]
    fn fit_pairs_drops_whole_pairs_from_the_end() {
        let footer = "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark";
        assert_eq!(fit_pairs(footer, 67), footer);
        assert_eq!(
            fit_pairs(footer, 66),
            "enter open · ^r rebuild · ^p profile · ^x kill"
        );
        assert_eq!(
            fit_pairs(footer, 45),
            "enter open · ^r rebuild · ^p profile"
        );
        assert_eq!(fit_pairs(footer, 10), "enter open");
    }

    // design 4.2: a first pair wider than the window is cut at the end, as cut_end does
    #[test]
    fn fit_pairs_cuts_a_first_pair_that_does_not_fit() {
        assert_eq!(fit_pairs("enter toggle bookmark · esc done", 8), "enter t…");
    }

    // design 4.2: CJK labels count as two cells when a pair is kept or dropped
    #[test]
    fn fit_pairs_counts_cjk_cells() {
        let footer = "enter 開く · ^r 再構築";
        assert_eq!(fit_pairs(footer, 10), "enter 開く");
        assert_eq!(fit_pairs(footer, 9), "enter 開…");
    }

    #[test]
    fn fit_pads_to_the_exact_width() {
        assert_eq!(fit("ab", 5, false), "ab   ");
        assert_eq!(fit("", 3, false), "   ");
    }

    // S-44, option B 4.3: a count is right-aligned in its column, and a long text is cut at the end
    #[test]
    fn fit_right_pads_on_the_left() {
        assert_eq!(fit_right("3", 10), "         3");
        assert_eq!(fit_right("Subfolders", 10), "Subfolders");
        assert_eq!(fit_right("abcdef", 4), "abc…");
    }

    // S-44: a count column is right-aligned and fixed
    #[test]
    fn count_column_is_fixed_and_right_aligned() {
        let spec = ColumnSpec::count("Subfolders", 10);
        assert!(spec.right);
        assert_eq!(spec.fixed_width, Some(10));
    }
}

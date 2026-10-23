//! Fuzzy matcher for the picker filter (FR-TUI-02, D-10). Pure: no I/O, no dependencies.
//!
//! A query matches a text when its characters occur in the text in order, ignoring case. The score
//! rewards consecutive matches and matches at word boundaries, and penalises gaps. The matched
//! character positions (char indexes, not bytes) are returned for highlighting.

/// One successful match: the score (higher is better) and the matched char positions in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub score: i64,
    pub positions: Vec<usize>,
}

/// One visible row of a filter: the index of the row in the input and its match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub index: usize,
    pub matched: Match,
}

const BASE: i64 = 16;
const CONSECUTIVE: i64 = 16;
const BOUNDARY: i64 = 8;
const BOUNDARY_CHARS: &[char] = &['/', '-', '_', '.', ' '];

/// Matches `query` against `text` (D-10). An empty query matches every text with score 0.
/// FR-TUI-02
pub fn fuzzy_match(query: &str, text: &str) -> Option<Match> {
    let query: Vec<char> = query.chars().map(fold).collect();
    let chars: Vec<char> = text.chars().collect();
    if query.is_empty() {
        return Some(Match {
            score: 0,
            positions: Vec::new(),
        });
    }
    let folded: Vec<char> = chars.iter().map(|&c| fold(c)).collect();

    // rows[i][j]: best score with query[i] matched at text[j], and the text index of query[i - 1].
    let mut rows: Vec<Vec<Option<Cell>>> = Vec::with_capacity(query.len());
    for &qc in &query {
        let prev: Option<&Vec<Option<Cell>>> = rows.last();
        let mut row: Vec<Option<Cell>> = vec![None; chars.len()];
        // Best (previous score + previous index) over candidates at least two cells back, so that
        // the gap penalty is applied in constant time per cell.
        let mut run: Option<(i64, usize)> = None;
        for (j, (&tc, slot)) in folded.iter().zip(row.iter_mut()).enumerate() {
            if let (Some(prev), Some(k)) = (prev, j.checked_sub(2)) {
                if let Some(Some(cell)) = prev.get(k) {
                    let value = cell.score + k as i64;
                    if improves(run.map(|(best, _)| best), value) {
                        run = Some((value, k));
                    }
                }
            }
            if tc != qc {
                continue;
            }
            let bonus = boundary_bonus(&chars, j);
            let cell = match prev {
                None => Some(Cell {
                    score: BASE + bonus - j as i64,
                    prev: None,
                }),
                Some(prev) => {
                    let mut best: Option<Cell> = None;
                    if let Some(k) = j.checked_sub(1) {
                        if let Some(Some(cell)) = prev.get(k) {
                            best = Some(Cell {
                                score: cell.score + BASE + CONSECUTIVE + bonus,
                                prev: Some(k),
                            });
                        }
                    }
                    if let Some((value, k)) = run {
                        let score = value - j as i64 + 1 + BASE + bonus;
                        if improves(best.map(|b| b.score), score) {
                            best = Some(Cell {
                                score,
                                prev: Some(k),
                            });
                        }
                    }
                    best
                }
            };
            *slot = cell;
        }
        rows.push(row);
    }

    // The earliest end position wins a tie on score.
    let last = rows.last()?;
    let mut end: Option<(i64, usize)> = None;
    for (j, cell) in last.iter().enumerate() {
        if let Some(cell) = cell {
            if improves(end.map(|(best, _)| best), cell.score) {
                end = Some((cell.score, j));
            }
        }
    }
    let (score, mut j) = end?;

    let mut positions = Vec::with_capacity(query.len());
    for i in (0..rows.len()).rev() {
        positions.push(j);
        let cell = rows.get(i)?.get(j)?.as_ref()?;
        match cell.prev {
            Some(prev) => j = prev,
            None => break,
        }
    }
    positions.reverse();
    Some(Match { score, positions })
}

/// Filters `texts` (D-10). Returns the matching rows in their original order; the query does not
/// reorder them. An empty query keeps every row.
/// FR-TUI-02
pub fn filter<S: AsRef<str>>(query: &str, texts: &[S]) -> Vec<Hit> {
    texts
        .iter()
        .enumerate()
        .filter_map(|(index, text)| {
            fuzzy_match(query, text.as_ref()).map(|matched| Hit { index, matched })
        })
        .collect()
}

/// Position in `hits` of the best match: the highest score, the first one on a tie. This is the
/// row the selection moves to when the filter changes (D-10, architecture 6.1).
/// FR-TUI-02
pub fn best_hit(hits: &[Hit]) -> Option<usize> {
    let mut best: Option<(i64, usize)> = None;
    for (position, hit) in hits.iter().enumerate() {
        if improves(best.map(|(score, _)| score), hit.matched.score) {
            best = Some((hit.matched.score, position));
        }
    }
    best.map(|(_, position)| position)
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    score: i64,
    prev: Option<usize>,
}

/// True when `candidate` beats the current best, or there is no best yet (strictly better, so the
/// earliest of equal candidates is kept).
fn improves(best: Option<i64>, candidate: i64) -> bool {
    match best {
        None => true,
        Some(best) => candidate > best,
    }
}

/// Case folding for comparison only. Positions still refer to the original text.
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Bonus for a match at `j`: at the start, after a separator, or at a camelCase hump.
fn boundary_bonus(chars: &[char], j: usize) -> i64 {
    let Some(&current) = chars.get(j) else {
        return 0;
    };
    let Some(previous) = j.checked_sub(1).and_then(|p| chars.get(p)) else {
        return BOUNDARY;
    };
    if BOUNDARY_CHARS.contains(previous) {
        return BOUNDARY;
    }
    if previous.is_lowercase() && current.is_uppercase() {
        return BOUNDARY;
    }
    0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn score(query: &str, text: &str) -> i64 {
        fuzzy_match(query, text).expect("expected a match").score
    }

    fn positions(query: &str, text: &str) -> Vec<usize> {
        fuzzy_match(query, text)
            .expect("expected a match")
            .positions
    }

    // FR-TUI-02, D-10: empty query matches everything with no positions
    #[test]
    fn empty_query_matches_any_text() {
        assert_eq!(
            fuzzy_match("", "anything"),
            Some(Match {
                score: 0,
                positions: vec![]
            })
        );
        assert_eq!(fuzzy_match("", "").map(|m| m.positions), Some(vec![]));
    }

    // FR-TUI-02, D-10: subsequence in order, not contiguous
    #[test]
    fn matches_characters_in_order_as_a_subsequence() {
        assert_eq!(positions("abc", "xaybzc"), vec![1, 3, 5]);
        assert!(fuzzy_match("ba", "ab").is_none());
    }

    // FR-TUI-02, D-10: no match when a character is missing
    #[test]
    fn rejects_text_missing_a_character() {
        assert!(fuzzy_match("x", "").is_none());
        assert!(fuzzy_match("abcd", "abc").is_none());
        assert!(fuzzy_match("q", "project").is_none());
    }

    // FR-TUI-02, D-10: case-insensitive in both directions
    #[test]
    fn matching_ignores_case() {
        assert_eq!(positions("PRO", "project"), vec![0, 1, 2]);
        assert_eq!(positions("pro", "PROJECT"), vec![0, 1, 2]);
        assert!(fuzzy_match("PRO", "project").is_some());
    }

    // FR-TUI-02, D-10: consecutive matches score higher than spread-out ones
    #[test]
    fn consecutive_matches_score_higher() {
        assert!(score("ab", "zab") > score("ab", "zaqb"));
    }

    // FR-TUI-02, D-10: a match after a separator scores higher than mid-word
    #[test]
    fn word_boundary_after_separator_scores_higher() {
        assert!(score("fb", "foo-bar") > score("fb", "fxxbar"));
        assert!(score("fb", "foo_bar") > score("fb", "fooxbar"));
        assert!(score("fb", "foo.bar") > score("fb", "fooxbar"));
        assert!(score("fb", "foo bar") > score("fb", "fooxbar"));
        assert!(score("fb", "foo/bar") > score("fb", "fooxbar"));
    }

    // FR-TUI-02, D-10: a camelCase hump counts as a word boundary
    #[test]
    fn camel_case_hump_counts_as_boundary() {
        assert!(score("fc", "fooCase") > score("fc", "fooxcase"));
    }

    // FR-TUI-02, D-10: the start of the text counts as a boundary
    #[test]
    fn match_at_start_scores_higher_than_later() {
        assert!(score("p", "project") > score("p", "xxproject"));
    }

    // FR-TUI-02, D-10: the best alignment is found, not the leftmost greedy one
    #[test]
    fn prefers_consecutive_alignment_over_leftmost() {
        assert_eq!(positions("ab", "a_ab"), vec![2, 3]);
    }

    // FR-TUI-02: positions are char indexes, so multi-byte text highlights the right cell
    #[test]
    fn positions_are_char_indexes() {
        assert_eq!(positions("é", "café"), vec![3]);
        assert_eq!(positions("fé", "café"), vec![2, 3]);
    }

    // FR-TUI-02, D-10: filter keeps the rows in their original order, not sorted by score
    #[test]
    fn filter_keeps_original_order() {
        let texts = ["xb", "a-b", "zz", "AB", "ab"];
        let hits = filter("ab", &texts);
        let indexes: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert_eq!(indexes, vec![1, 3, 4]);
    }

    // FR-TUI-02, D-10: an empty filter keeps every row in order with empty positions
    #[test]
    fn filter_empty_query_keeps_all_rows_in_order() {
        let texts = ["c", "a", "b"];
        let hits = filter("", &texts);
        let indexes: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert_eq!(indexes, vec![0, 1, 2]);
        assert!(hits.iter().all(|h| h.matched.positions.is_empty()));
    }

    // FR-TUI-02, D-10: rows with equal score keep their original order
    #[test]
    fn ties_keep_original_order() {
        let texts = ["abc", "abc", "abc"];
        let hits = filter("abc", &texts);
        let indexes: Vec<usize> = hits.iter().map(|h| h.index).collect();
        assert_eq!(indexes, vec![0, 1, 2]);
    }

    // FR-TUI-02, D-10: the selection moves to the best match, the first one on a tie
    #[test]
    fn best_hit_is_highest_score_first_on_tie() {
        let texts = ["a-x-b", "ab", "ab"];
        let hits = filter("ab", &texts);
        assert_eq!(best_hit(&hits), Some(1));
        let tied = filter("ab", &["ab", "ab"]);
        assert_eq!(best_hit(&tied), Some(0));
    }

    // FR-TUI-02: no hits, no best row
    #[test]
    fn best_hit_of_no_rows_is_none() {
        let texts = ["x", "y"];
        assert_eq!(best_hit(&filter("q", &texts)), None);
    }

    // FR-TUI-02: the highlighted positions are the ones the score was computed from
    #[test]
    fn highlight_positions_reach_every_query_character() {
        let found = fuzzy_match("prj", "my-project").expect("match");
        assert_eq!(found.positions.len(), 3);
        assert!(found.positions.windows(2).all(|w| w[0] < w[1]));
    }
}

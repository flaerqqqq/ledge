//! History list: recently opened directories, newest first (architecture 5, 9.4). Pure: no file access.

use super::path::{cache_lines_text, parse_cache_lines, AbsPath};

/// FR-HIST-01: the history keeps at most this many directories.
pub const HISTORY_LIMIT: usize = 50;

/// FR-HIST-01, FR-HIST-02: the directories opened, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct History {
    entries: Vec<AbsPath>,
}

impl History {
    /// Reads the history file text. F-36: a longer file is cut to the newest 50 in memory; the file
    /// itself is rewritten only by the next add or clean. A-09 and D-06 rules are in `parse_cache_lines`.
    pub fn parse(text: &str) -> History {
        let mut entries = parse_cache_lines(text);
        entries.truncate(HISTORY_LIMIT);
        History { entries }
    }

    /// The entries, newest first.
    pub fn entries(&self) -> &[AbsPath] {
        &self.entries
    }

    /// FR-HIST-01, FR-HIST-02: `dir` becomes the newest entry. An earlier entry for the same directory
    /// is removed first, so it is never listed twice; the list is then cut to `HISTORY_LIMIT`.
    pub fn add(&mut self, dir: AbsPath) {
        self.entries.retain(|entry| entry != &dir);
        self.entries.insert(0, dir);
        self.entries.truncate(HISTORY_LIMIT);
    }

    /// The history file text: one path per line, newest first.
    pub fn to_file_text(&self) -> String {
        cache_lines_text(&self.entries)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{History, HISTORY_LIMIT};
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn texts(history: &History) -> Vec<&str> {
        history.entries().iter().map(AbsPath::as_str).collect()
    }

    // FR-HIST-01: the most recently added directory is listed first
    #[test]
    fn add_puts_newest_entry_first() {
        let mut history = History::default();
        history.add(abs("/p/a"));
        history.add(abs("/p/b"));
        assert_eq!(texts(&history), vec!["/p/b", "/p/a"]);
    }

    // FR-HIST-02: opening a directory again moves it to the top and does not duplicate it
    #[test]
    fn add_moves_reopened_directory_to_top_without_duplicate() {
        let mut history = History::default();
        history.add(abs("/p/a"));
        history.add(abs("/p/b"));
        history.add(abs("/p/c"));
        history.add(abs("/p/a"));
        assert_eq!(texts(&history), vec!["/p/a", "/p/c", "/p/b"]);
    }

    // FR-HIST-02, D-06: a trailing slash names the same directory, so it moves the entry too
    #[test]
    fn add_treats_trailing_slash_as_same_directory() {
        let mut history = History::default();
        history.add(abs("/p/a"));
        history.add(abs("/p/b"));
        history.add(abs("/p/a/"));
        assert_eq!(texts(&history), vec!["/p/a", "/p/b"]);
    }

    // FR-HIST-01: at most 50 entries; the oldest is dropped when a 51st is added
    #[test]
    fn add_keeps_at_most_fifty_entries() {
        let mut history = History::default();
        for index in 0..=HISTORY_LIMIT {
            history.add(abs(&format!("/p/{index}")));
        }
        assert_eq!(history.entries().len(), 50);
        assert_eq!(
            history.entries().first().map(AbsPath::as_str),
            Some("/p/50")
        );
        assert_eq!(history.entries().last().map(AbsPath::as_str), Some("/p/1"));
        assert!(!texts(&history).contains(&"/p/0"));
    }

    // F-36: a longer file is trimmed to the newest 50 when read
    #[test]
    fn parse_trims_longer_file_to_newest_fifty() {
        let text: String = (0..60).map(|index| format!("/p/{index}\n")).collect();
        let history = History::parse(&text);
        assert_eq!(history.entries().len(), 50);
        assert_eq!(history.entries().first().map(AbsPath::as_str), Some("/p/0"));
        assert_eq!(history.entries().last().map(AbsPath::as_str), Some("/p/49"));
    }

    // FR-HIST-01: the file lists newest first, one path per line
    #[test]
    fn to_file_text_writes_newest_first_one_per_line() {
        let mut history = History::default();
        history.add(abs("/p/a"));
        history.add(abs("/p/b"));
        assert_eq!(history.to_file_text(), "/p/b\n/p/a\n");
    }

    // FR-HIST-01: file text read back gives the same list
    #[test]
    fn parse_of_file_text_round_trips() {
        let mut history = History::default();
        history.add(abs("/p/a"));
        history.add(abs("/p/b"));
        assert_eq!(History::parse(&history.to_file_text()), history);
    }

    // A-09: relative lines in a hand-edited file are skipped
    #[test]
    fn parse_skips_relative_lines() {
        let history = History::parse("/p/a\nrelative\n/p/b\n");
        assert_eq!(texts(&history), vec!["/p/a", "/p/b"]);
    }

    // D-06: a repeated line in a hand-edited file keeps its first occurrence
    #[test]
    fn parse_keeps_first_occurrence_of_duplicates() {
        let history = History::parse("/p/b\n/p/a\n/p/b\n");
        assert_eq!(texts(&history), vec!["/p/b", "/p/a"]);
    }
}

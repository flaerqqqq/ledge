//! Bookmark list: absolute directory paths in insertion order (architecture 5, 9.4). Pure: no file access.
//! Add, remove and the project rules of FR-BM-02..10 arrive with S-18.

use super::path::{cache_lines_text, parse_cache_lines, AbsPath};

/// FR-BM-01: the bookmarked directories, in the order they were stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bookmarks {
    entries: Vec<AbsPath>,
}

impl Bookmarks {
    /// Reads the bookmarks file text. A-09 and D-06 rules are in `parse_cache_lines`; the file order is kept.
    pub fn parse(text: &str) -> Bookmarks {
        Bookmarks {
            entries: parse_cache_lines(text),
        }
    }

    /// The bookmarked directories in stored order.
    pub fn entries(&self) -> &[AbsPath] {
        &self.entries
    }

    /// FR-BM-01: the bookmarks file text, one path per line, in stored order.
    pub fn to_file_text(&self) -> String {
        cache_lines_text(&self.entries)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::Bookmarks;
    use crate::domain::path::AbsPath;

    fn texts(bookmarks: &Bookmarks) -> Vec<&str> {
        bookmarks.entries().iter().map(AbsPath::as_str).collect()
    }

    // FR-BM-01: entries are read in the order they are stored, not sorted
    #[test]
    fn parse_keeps_stored_order() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a\n/p/m\n");
        assert_eq!(texts(&bookmarks), vec!["/p/z", "/p/a", "/p/m"]);
    }

    // FR-BM-01: one path per line, in stored order
    #[test]
    fn to_file_text_writes_one_path_per_line_in_stored_order() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a\n");
        assert_eq!(bookmarks.to_file_text(), "/p/z\n/p/a\n");
    }

    // FR-BM-01: text written by to_file_text reads back to the same list
    #[test]
    fn parse_of_file_text_round_trips() {
        let bookmarks = Bookmarks::parse("/p/z\n/p/a/\n");
        assert_eq!(Bookmarks::parse(&bookmarks.to_file_text()), bookmarks);
    }

    // A-09: relative lines are skipped
    #[test]
    fn parse_skips_relative_lines() {
        let bookmarks = Bookmarks::parse("app/src\n/p/a\n");
        assert_eq!(texts(&bookmarks), vec!["/p/a"]);
    }

    // D-06: duplicates keep the first occurrence, and the same path with a trailing slash is a duplicate
    #[test]
    fn parse_keeps_first_occurrence_of_duplicates() {
        let bookmarks = Bookmarks::parse("/p/b\n/p/a\n/p/b/\n");
        assert_eq!(texts(&bookmarks), vec!["/p/b", "/p/a"]);
    }
}

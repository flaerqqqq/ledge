//! Custom projects (proposal P-65, FR-CONF-07): directories opened from the command line that lie outside every project.
//! The list is kept in the cache folder, one absolute path per line, as the bookmarks are (architecture 9.4). Pure: no
//! file access; the caller reads and writes the text.

use super::path::{cache_lines_text, parse_cache_lines, AbsPath};

/// P-65: the custom project directories, in the order they were stored.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomProjects {
    entries: Vec<AbsPath>,
}

impl CustomProjects {
    /// Reads the custom projects file text. The rules of `parse_cache_lines` apply (relative and empty lines are skipped,
    /// a repeated path keeps its first occurrence, a trailing slash is dropped).
    pub fn parse(text: &str) -> CustomProjects {
        CustomProjects {
            entries: parse_cache_lines(text),
        }
    }

    /// The custom project directories in stored order.
    pub fn entries(&self) -> &[AbsPath] {
        &self.entries
    }

    /// P-65: the custom projects file text, one path per line, in stored order.
    pub fn to_file_text(&self) -> String {
        cache_lines_text(&self.entries)
    }

    /// True when `dir` is already a custom project.
    pub fn contains(&self, dir: &AbsPath) -> bool {
        self.entries.contains(dir)
    }

    /// P-65: appends `dir`. Returns `false` and changes nothing when it is already listed.
    pub fn add(&mut self, dir: AbsPath) -> bool {
        if self.contains(&dir) {
            return false;
        }
        self.entries.push(dir);
        true
    }

    /// P-65: removes `dir`, keeping the order of the others. Returns `false` when it is not listed.
    pub fn remove(&mut self, dir: &AbsPath) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry != dir);
        self.entries.len() != before
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::CustomProjects;
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    fn texts(custom: &CustomProjects) -> Vec<&str> {
        custom.entries().iter().map(AbsPath::as_str).collect()
    }

    // P-65: entries are read in the order they are stored, not sorted
    #[test]
    fn parse_keeps_stored_order() {
        let custom = CustomProjects::parse("/s/z\n/s/a\n/s/m\n");
        assert_eq!(texts(&custom), vec!["/s/z", "/s/a", "/s/m"]);
    }

    // P-65: relative lines are skipped, as in the bookmarks file
    #[test]
    fn parse_skips_relative_lines() {
        let custom = CustomProjects::parse("scratch\n/s/a\n");
        assert_eq!(texts(&custom), vec!["/s/a"]);
    }

    // P-65: a path given twice, with or without a trailing slash, is listed once
    #[test]
    fn parse_keeps_first_occurrence_of_duplicates() {
        let custom = CustomProjects::parse("/s/b\n/s/a\n/s/b/\n");
        assert_eq!(texts(&custom), vec!["/s/b", "/s/a"]);
    }

    // P-65: one path per line, in stored order, and the text reads back to the same list
    #[test]
    fn to_file_text_round_trips() {
        let custom = CustomProjects::parse("/s/z\n/s/a/\n");
        assert_eq!(custom.to_file_text(), "/s/z\n/s/a\n");
        assert_eq!(CustomProjects::parse(&custom.to_file_text()), custom);
    }

    // P-65: a new custom project goes to the end of the stored order
    #[test]
    fn add_appends_in_insertion_order() {
        let mut custom = CustomProjects::parse("/s/z\n");
        assert!(custom.add(abs("/s/a")));
        assert_eq!(texts(&custom), vec!["/s/z", "/s/a"]);
    }

    // P-65: adding a path that is listed changes nothing and reports it (the trailing slash is cleaned away)
    #[test]
    fn add_of_listed_path_changes_nothing() {
        let mut custom = CustomProjects::parse("/s/a\n/s/b\n");
        assert!(!custom.add(abs("/s/a/")));
        assert_eq!(texts(&custom), vec!["/s/a", "/s/b"]);
    }

    // P-65: removing keeps the order of the others
    #[test]
    fn remove_keeps_order_of_the_rest() {
        let mut custom = CustomProjects::parse("/s/a\n/s/b\n/s/c\n");
        assert!(custom.remove(&abs("/s/b")));
        assert_eq!(texts(&custom), vec!["/s/a", "/s/c"]);
    }

    // P-65: removing a path that is not listed reports it and changes nothing
    #[test]
    fn remove_of_unlisted_path_reports_false() {
        let mut custom = CustomProjects::parse("/s/a\n");
        assert!(!custom.remove(&abs("/s/x")));
        assert_eq!(texts(&custom), vec!["/s/a"]);
    }
}

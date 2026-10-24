//! `AbsPath`: a cleaned, absolute, UTF-8 path (architecture 5). Pure: no file system access.

/// D-06: an absolute path cleaned lexically. Symlinks are not resolved.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsPath(String);

impl AbsPath {
    /// D-06: `input` is made absolute against `cwd` when relative, then cleaned lexically
    /// (`.` and `..` resolved, repeated `/` collapsed, trailing `/` removed).
    pub fn clean(input: &str, cwd: &AbsPath) -> AbsPath {
        let joined = if input.starts_with('/') {
            input.to_string()
        } else {
            format!("{}/{}", cwd.0, input)
        };
        let mut parts: Vec<&str> = Vec::new();
        for part in joined.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }
        AbsPath(format!("/{}", parts.join("/")))
    }

    /// The root folder `/`. Useful as the cwd when the input is already absolute.
    pub fn root() -> AbsPath {
        AbsPath("/".to_string())
    }

    /// The cleaned path text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// FR-HIST-03: the path text with the home folder shown as `~`. `home` is the cleaned home folder, or `None`
    /// when HOME is not set. Only whole components are replaced (`/home/u2` is not under `/home/u`).
    pub fn display_with_home(&self, home: Option<&AbsPath>) -> String {
        let Some(home) = home.filter(|home| home.0 != "/") else {
            return self.0.clone();
        };
        if self.0 == home.0 {
            return "~".to_string();
        }
        match self.0.strip_prefix(home.0.as_str()) {
            Some(rest) if rest.starts_with('/') => format!("~{rest}"),
            _ => self.0.clone(),
        }
    }

    /// A-14, D-06: true when `self` equals `base` or lies inside it by whole path components.
    /// `/p/app` is not inside `/p/ap`.
    pub fn is_inside(&self, base: &AbsPath) -> bool {
        let base = base.0.as_str();
        self.0 == base
            || base == "/"
            || self
                .0
                .strip_prefix(base)
                .is_some_and(|rest| rest.starts_with('/'))
    }
}
/// architecture 9.4, A-09, D-06: the paths of a cache file, one per line. Each line loses its trailing
/// whitespace; empty lines and lines that are not absolute are skipped; the rest are cleaned, and a path
/// that repeats keeps its first occurrence.
pub fn parse_cache_lines(text: &str) -> Vec<AbsPath> {
    let mut paths: Vec<AbsPath> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if !line.starts_with('/') {
            continue;
        }
        let path = AbsPath::clean(line, &AbsPath::root());
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

/// architecture 9.1: the text written to a cache file, one path per line, each line ended by a newline.
/// An empty list gives an empty text.
pub fn cache_lines_text(paths: &[AbsPath]) -> String {
    paths
        .iter()
        .map(|path| format!("{}\n", path.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{cache_lines_text, parse_cache_lines, AbsPath};

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    // D-06: a relative input is made absolute against the working directory
    #[test]
    fn clean_resolves_relative_input_against_cwd() {
        let cwd = abs("/p");
        assert_eq!(AbsPath::clean("x/y", &cwd).as_str(), "/p/x/y");
    }

    // D-06: an absolute input ignores the working directory
    #[test]
    fn clean_keeps_absolute_input_and_ignores_cwd() {
        let cwd = abs("/p");
        assert_eq!(AbsPath::clean("/a/b", &cwd).as_str(), "/a/b");
    }

    // D-06: "." names the working directory itself
    #[test]
    fn clean_resolves_dot_to_working_directory() {
        let cwd = abs("/p/app");
        assert_eq!(AbsPath::clean(".", &cwd).as_str(), "/p/app");
        assert_eq!(AbsPath::clean("./x", &cwd).as_str(), "/p/app/x");
    }

    // D-06: ".." removes the previous component
    #[test]
    fn clean_resolves_dot_dot_lexically() {
        let cwd = abs("/p/q");
        assert_eq!(AbsPath::clean("../x", &cwd).as_str(), "/p/x");
        assert_eq!(AbsPath::clean("/p/app/../ap/x", &cwd).as_str(), "/p/ap/x");
    }

    // D-06: ".." at the root stays at the root
    #[test]
    fn clean_stops_dot_dot_at_root() {
        let cwd = abs("/");
        assert_eq!(AbsPath::clean("/../x", &cwd).as_str(), "/x");
        assert_eq!(AbsPath::clean("/..", &cwd).as_str(), "/");
    }

    // D-06: repeated slashes collapse to one
    #[test]
    fn clean_collapses_repeated_slashes() {
        assert_eq!(abs("/p//app///x").as_str(), "/p/app/x");
    }

    // D-06: a trailing slash is removed, however many there are
    #[test]
    fn clean_removes_trailing_slashes() {
        assert_eq!(abs("/p/app/").as_str(), "/p/app");
        assert_eq!(abs("/p/app//").as_str(), "/p/app");
    }

    // D-06: the root keeps its single slash
    #[test]
    fn clean_keeps_root_as_slash() {
        assert_eq!(abs("/").as_str(), "/");
    }

    // D-06: cleaning is lexical, so a ".." after a missing folder still cancels it (symlinks are
    // not resolved and nothing is read from disk)
    #[test]
    fn clean_is_lexical_and_does_not_check_the_disk() {
        assert_eq!(abs("/no/such/../x").as_str(), "/no/x");
    }

    // D-06, A-14: a path is inside itself
    #[test]
    fn inside_is_true_for_equal_path() {
        assert!(abs("/p/app").is_inside(&abs("/p/app")));
    }

    // D-06, A-14: a descendant is inside its ancestor
    #[test]
    fn inside_is_true_for_descendant() {
        assert!(abs("/p/app/src/main").is_inside(&abs("/p/app")));
    }

    // D-06: whole components are compared, so /p/app is not inside /p/ap
    #[test]
    fn inside_is_false_when_base_is_a_name_prefix() {
        assert!(!abs("/p/app").is_inside(&abs("/p/ap")));
    }

    // D-06: the reverse also holds, /p/ap is not inside /p/app
    #[test]
    fn inside_is_false_when_path_is_a_name_prefix_of_base() {
        assert!(!abs("/p/ap").is_inside(&abs("/p/app")));
    }

    // D-06: a sibling whose name starts with the base name is not inside it
    #[test]
    fn inside_is_false_for_sibling_with_longer_name() {
        assert!(!abs("/p/appx/y").is_inside(&abs("/p/app")));
    }

    // D-06: every absolute path is inside the root
    #[test]
    fn inside_is_true_for_anything_under_root() {
        assert!(abs("/p/x").is_inside(&abs("/")));
    }

    // D-06: the comparison uses cleaned paths, so trailing slashes and ".." do not matter
    #[test]
    fn inside_compares_cleaned_paths() {
        assert!(abs("/p/app/").is_inside(&abs("/p/app/src/..")));
    }

    // D-06: a ".." that leaves the base is not inside it after cleaning
    #[test]
    fn inside_is_false_after_dot_dot_leaves_base() {
        let escaped = AbsPath::clean("/p/app/../ap/x", &AbsPath::root());
        assert!(!escaped.is_inside(&abs("/p/app")));
    }

    // architecture 9.4, A-09: lines that are not absolute paths are skipped; empty lines too
    #[test]
    fn cache_lines_skip_relative_and_empty_lines() {
        let paths = parse_cache_lines("/a\n\nrelative/b\n  /c\n/d\n");
        assert_eq!(texts(&paths), vec!["/a", "/d"]);
    }

    // D-06: a repeated path keeps its first occurrence, and differing trailing slashes are the same path
    #[test]
    fn cache_lines_keep_first_of_duplicates_after_cleaning() {
        let paths = parse_cache_lines("/b/\n/a\n/b\n/a/\n");
        assert_eq!(texts(&paths), vec!["/b", "/a"]);
    }

    // architecture 9.4: trailing whitespace is trimmed from each line before use
    #[test]
    fn cache_lines_trim_trailing_whitespace() {
        let paths = parse_cache_lines("/a \t\r\n/b\n");
        assert_eq!(texts(&paths), vec!["/a", "/b"]);
    }

    // architecture 9.1: one path per line, each ended by a newline, in the given order
    #[test]
    fn cache_lines_text_writes_one_path_per_line_in_order() {
        let paths = vec![abs("/b"), abs("/a/c")];
        assert_eq!(cache_lines_text(&paths), "/b\n/a/c\n");
    }

    // architecture 9.1: an empty list writes an empty text
    #[test]
    fn cache_lines_text_of_empty_list_is_empty() {
        assert_eq!(cache_lines_text(&[]), "");
    }

    // FR-HIST-03: the home folder itself and paths under it show as ~
    #[test]
    fn display_shows_home_as_tilde() {
        let home = abs("/home/u");
        assert_eq!(abs("/home/u").display_with_home(Some(&home)), "~");
        assert_eq!(
            abs("/home/u/code/app").display_with_home(Some(&home)),
            "~/code/app"
        );
    }

    // FR-HIST-03: only whole components of the home folder are replaced
    #[test]
    fn display_keeps_paths_that_only_start_with_home_text() {
        let home = abs("/home/u");
        assert_eq!(
            abs("/home/u2/x").display_with_home(Some(&home)),
            "/home/u2/x"
        );
        assert_eq!(abs("/srv/x").display_with_home(Some(&home)), "/srv/x");
    }

    // FR-HIST-03: without a known home folder the path is shown as it is
    #[test]
    fn display_without_home_is_the_path() {
        assert_eq!(abs("/home/u/x").display_with_home(None), "/home/u/x");
        assert_eq!(abs("/x").display_with_home(Some(&abs("/"))), "/x");
    }

    fn texts(paths: &[AbsPath]) -> Vec<&str> {
        paths.iter().map(AbsPath::as_str).collect()
    }
}

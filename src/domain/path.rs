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
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::AbsPath;

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
}

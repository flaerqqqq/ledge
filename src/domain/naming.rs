//! Session naming (architecture 5 and 7.3, D-14): the tmux session name of a directory.
//! Pure: the digest is computed with the `md-5` crate (architecture 10).

use super::path::AbsPath;
use md5::{Digest, Md5};

/// FR-NAME-01, FR-OPEN-05, D-14: the session name of a clean absolute directory path.
/// `<basename with "." and ":" replaced by "_">-<first 4 lowercase hex chars of MD5 of the path>`.
/// The digest input is the path bytes with no trailing newline.
pub fn session_name(dir: &AbsPath) -> String {
    let path = dir.as_str();
    let basename = path.rsplit('/').find(|part| !part.is_empty()).unwrap_or("");
    let name: String = basename
        .chars()
        .map(|c| if c == '.' || c == ':' { '_' } else { c })
        .collect();
    // The first two digest bytes are the first four lowercase hex characters.
    let digest = Md5::digest(path.as_bytes());
    let suffix: String = digest.iter().take(2).map(|b| format!("{b:02x}")).collect();
    format!("{name}-{suffix}")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::session_name;
    use crate::domain::path::AbsPath;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    // The expected digests below were derived outside the code with GNU coreutils:
    //   printf '%s' '<clean absolute path>' | md5sum | cut -c1-4
    // (no trailing newline in the input, as D-14 states). They are the algorithm applied to the
    // example paths, not the reference pairs of the earlier tool. PROVISIONAL: these vectors stay
    // provisional until the user supplies D-02 reference pairs; FR-NAME-02 stays open until then.

    // FR-NAME-01, D-14, D-02 provisional: "." in the basename becomes "_", digest prefix d7d2
    #[test]
    fn dot_in_basename_becomes_underscore_before_digest() {
        assert_eq!(session_name(&abs("/home/u/projects/my.app")), "my_app-d7d2");
    }

    // FR-NAME-01, D-14, D-02 provisional: ":" in the basename is replaced by "_" too
    #[test]
    fn colon_in_basename_becomes_underscore() {
        assert_eq!(session_name(&abs("/srv/src/api:v2")), "api_v2-8158");
    }

    // FR-NAME-01, D-14, D-02 provisional: a leading dot is replaced like any other dot
    #[test]
    fn leading_dot_in_basename_becomes_underscore() {
        assert_eq!(
            session_name(&abs("/home/u/projects/.dotdir")),
            "_dotdir-cfc4"
        );
    }

    // FR-NAME-01, D-14, D-02 provisional: dashes and underscores in the basename are kept as they are
    #[test]
    fn dash_and_underscore_in_basename_are_kept() {
        assert_eq!(session_name(&abs("/work/my-app_1")), "my-app_1-a0cb");
    }

    // FR-NAME-01: the name ends with a dash and four lowercase hex digits
    #[test]
    fn name_ends_with_dash_and_four_lowercase_hex_digits() {
        for path in [
            "/home/u/projects/my.app",
            "/work/my-app_1",
            "/srv/src/api:v2",
        ] {
            let name = session_name(&abs(path));
            let (_, suffix) = name.rsplit_once('-').expect("name has a dash");
            assert_eq!(suffix.len(), 4, "suffix of {name}");
            assert!(
                suffix.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
                "suffix of {name} is not lowercase hex"
            );
        }
    }

    // FR-NAME-02, D-02 provisional: three example paths produce the self-derived digests above
    #[test]
    fn digests_match_self_derived_vectors() {
        assert_eq!(session_name(&abs("/home/u/projects/my.app")), "my_app-d7d2");
        assert_eq!(session_name(&abs("/srv/src/api:v2")), "api_v2-8158");
        assert_eq!(
            session_name(&abs("/home/u/projects/.dotdir")),
            "_dotdir-cfc4"
        );
    }

    // FR-NAME-01, D-06: the naming input is the clean path, so a trailing slash does not change it
    #[test]
    fn trailing_slash_in_input_gives_same_name() {
        assert_eq!(
            session_name(&abs("/home/u/projects/my.app/")),
            session_name(&abs("/home/u/projects/my.app"))
        );
    }

    // FR-OPEN-05: the same directory always yields the same name, whatever spelling was typed
    #[test]
    fn same_directory_gives_same_name_from_any_spelling() {
        let first = session_name(&abs("/home/u/projects/my.app"));
        let second = session_name(&abs("/home/u/./projects//my.app/"));
        assert_eq!(first, second);
    }

    // FR-OPEN-06: two directories that share a basename get different sessions
    #[test]
    fn two_directories_with_one_basename_get_different_names() {
        let work = session_name(&abs("/work/app"));
        let other = session_name(&abs("/other/app"));
        assert_ne!(work, other);
        assert!(work.starts_with("app-"), "{work}");
        assert!(other.starts_with("app-"), "{other}");
    }

    // FR-OPEN-06, D-02 provisional: the self-derived digests of the two "app" directories
    #[test]
    fn app_directories_match_self_derived_vectors() {
        assert_eq!(session_name(&abs("/work/app")), "app-7d3b");
        assert_eq!(session_name(&abs("/other/app")), "app-6a6e");
    }
}

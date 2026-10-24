//! Which profile applies (architecture 2, decision D-13, FR-PROF-09..12). Pure: the caller reads the
//! config and the profile files. Rebuild precedence (FR-PROF-14) is added with S-14.

/// FR-PROF-09, FR-PROF-10, architecture 2.3: the profile option as the command line gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileOpt {
    /// Neither `--profile` nor `--no-profile` was given.
    Unset,
    /// `--profile NAME`.
    Named(String),
    /// `--no-profile`: the single-window layout, profiles ignored.
    None,
}

/// D-13: the profile an open tries to load, and what a failure to load means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenProfile {
    /// The built-in single-window layout; no file is read.
    Basic,
    /// Named by `--profile`: a failure is the profile's own error (FR-PROF-11).
    Requested(String),
    /// Taken from `default_profile`: a failure is a warning and the single-window layout (FR-PROF-12).
    Default(String),
}

/// FR-PROF-09, FR-PROF-10, FR-PROF-12, D-13: precedence at open is no-profile > `--profile` >
/// `default_profile` > basic.
pub fn open_profile(option: &ProfileOpt, default_profile: Option<&str>) -> OpenProfile {
    match option {
        ProfileOpt::None => OpenProfile::Basic,
        ProfileOpt::Named(name) => OpenProfile::Requested(name.clone()),
        ProfileOpt::Unset => match default_profile {
            Some(name) => OpenProfile::Default(name.to_string()),
            None => OpenProfile::Basic,
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{open_profile, OpenProfile, ProfileOpt};

    // FR-PROF-10: --no-profile ignores a default profile
    #[test]
    fn no_profile_ignores_default_profile() {
        assert_eq!(
            open_profile(&ProfileOpt::None, Some("web")),
            OpenProfile::Basic
        );
    }

    // FR-PROF-09: --profile wins over the default profile
    #[test]
    fn named_profile_wins_over_default_profile() {
        assert_eq!(
            open_profile(&ProfileOpt::Named("dev".to_string()), Some("web")),
            OpenProfile::Requested("dev".to_string())
        );
    }

    // FR-PROF-12, D-13: without an option the default profile applies
    #[test]
    fn default_profile_applies_without_option() {
        assert_eq!(
            open_profile(&ProfileOpt::Unset, Some("web")),
            OpenProfile::Default("web".to_string())
        );
    }

    // D-13: no option and no default is the built-in layout
    #[test]
    fn no_option_and_no_default_is_basic() {
        assert_eq!(open_profile(&ProfileOpt::Unset, None), OpenProfile::Basic);
    }
}

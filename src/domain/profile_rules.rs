//! Which profile applies (architecture 2, decision D-13, FR-PROF-09..12, FR-PROF-14) and the rows of the profile
//! picker (FR-PROF-13, A-11). Pure: the caller reads the config and the profile files.

use crate::domain::profile::ProfileName;

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

/// FR-PROF-14, FR-PROF-15, FR-ACT-12, D-13, D-04: the profile a rebuild tries to load, and what a failure to load
/// means. `Recorded` is the profile stored on the session; a failure is the FR-PROF-15 warning and the fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebuildProfile {
    /// `--no-profile`, or no option, no recorded profile and no default: the built-in layout.
    Basic,
    /// Named by `--profile` (FR-ACT-12): a failure is the profile's own error (FR-PROF-11).
    Requested(String),
    /// The profile recorded on the session (FR-PROF-14): a failure is the FR-PROF-15 warning.
    Recorded(String),
    /// The default profile, used when nothing is recorded: a failure is the FR-PROF-12 warning.
    Default(String),
}

/// FR-PROF-14, FR-ACT-12, D-13: precedence at rebuild is no-profile > `--profile` > recorded profile >
/// `default_profile` > basic. An empty recorded or default name counts as none.
pub fn rebuild_profile(
    option: &ProfileOpt,
    recorded: Option<&str>,
    default_profile: Option<&str>,
) -> RebuildProfile {
    let recorded = recorded.filter(|name| !name.is_empty());
    let default_profile = default_profile.filter(|name| !name.is_empty());
    match option {
        ProfileOpt::None => RebuildProfile::Basic,
        ProfileOpt::Named(name) => RebuildProfile::Requested(name.clone()),
        ProfileOpt::Unset => match (recorded, default_profile) {
            (Some(name), _) => RebuildProfile::Recorded(name.to_string()),
            (None, Some(name)) => RebuildProfile::Default(name.to_string()),
            (None, None) => RebuildProfile::Basic,
        },
    }
}

/// FR-PROF-13, A-11: one row of the profile picker. `current` is the profile of the session being rebuilt, `default`
/// the `default_profile` of the config. A name that is both has one row with both labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileChoice {
    pub name: String,
    pub current: bool,
    pub default: bool,
}

/// FR-PROF-13, A-11: the rows of the profile picker. The current profile comes first, then the default, then every
/// other profile and `basic`, sorted by name. `files` are the profile names found in the profiles folder; names that
/// are not valid profile names are skipped, and `basic` is listed once even when a file of that name exists. A
/// current or default name is listed even when its file is missing, so that choosing it reports the error. An
/// empty name counts as none.
pub fn profile_choices(
    files: &[String],
    current: Option<&str>,
    default: Option<&str>,
) -> Vec<ProfileChoice> {
    let current = current.filter(|name| !name.is_empty());
    let default = default.filter(|name| !name.is_empty());
    let mut rows = Vec::new();
    if let Some(name) = current {
        rows.push(ProfileChoice {
            name: name.to_string(),
            current: true,
            default: default == Some(name),
        });
    }
    if let Some(name) = default.filter(|name| Some(*name) != current) {
        rows.push(ProfileChoice {
            name: name.to_string(),
            current: false,
            default: true,
        });
    }
    let mut others: Vec<String> = files
        .iter()
        .filter(|name| ProfileName::parse(name).is_some_and(|valid| !valid.is_basic()))
        .cloned()
        .chain(std::iter::once("basic".to_string()))
        .filter(|name| !rows.iter().any(|row| row.name == *name))
        .collect();
    others.sort();
    others.dedup();
    rows.extend(others.into_iter().map(|name| ProfileChoice {
        name,
        current: false,
        default: false,
    }));
    rows
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{
        open_profile, profile_choices, rebuild_profile, OpenProfile, ProfileChoice, ProfileOpt,
        RebuildProfile,
    };

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_string()).collect()
    }

    fn choice(name: &str, current: bool, default: bool) -> ProfileChoice {
        ProfileChoice {
            name: name.to_string(),
            current,
            default,
        }
    }

    // FR-PROF-13, A-11: current first, then default, then the other profiles and basic by name
    #[test]
    fn profile_rows_are_current_then_default_then_others_by_name() {
        let files = names(&["web", "dev", "ops"]);
        assert_eq!(
            profile_choices(&files, Some("ops"), Some("web")),
            vec![
                choice("ops", true, false),
                choice("web", false, true),
                choice("basic", false, false),
                choice("dev", false, false),
            ]
        );
    }

    // FR-PROF-13, A-11: a profile that is current and default is one row with both labels
    #[test]
    fn current_and_default_profile_is_one_row() {
        let files = names(&["dev", "web"]);
        assert_eq!(
            profile_choices(&files, Some("dev"), Some("dev")),
            vec![
                choice("dev", true, true),
                choice("basic", false, false),
                choice("web", false, false),
            ]
        );
    }

    // FR-PROF-13: without a current session there is no current row; the default comes first
    #[test]
    fn without_a_session_the_default_is_first() {
        let files = names(&["dev"]);
        assert_eq!(
            profile_choices(&files, None, Some("dev")),
            vec![choice("dev", false, true), choice("basic", false, false)]
        );
    }

    // FR-PROF-13, A-11: with neither a session nor a default the list is the others and basic, by name
    #[test]
    fn without_session_or_default_the_list_is_sorted_by_name() {
        let files = names(&["zeta", "alpha"]);
        assert_eq!(
            profile_choices(&files, None, None),
            vec![
                choice("alpha", false, false),
                choice("basic", false, false),
                choice("zeta", false, false),
            ]
        );
    }

    // A-11: a current profile that has no file is still listed, so that choosing it reports its error
    #[test]
    fn current_profile_without_a_file_is_still_listed() {
        assert_eq!(
            profile_choices(&[], Some("gone"), None),
            vec![choice("gone", true, false), choice("basic", false, false)]
        );
    }

    // D-13: basic is built in and reserved, so a basic file adds no second row; invalid names are not listed
    #[test]
    fn basic_file_and_invalid_names_are_not_listed_twice() {
        let files = names(&["basic", "bad name", "dev"]);
        assert_eq!(
            profile_choices(&files, None, Some("basic")),
            vec![choice("basic", false, true), choice("dev", false, false)]
        );
    }

    // FR-PROF-13: an empty current or default name counts as none
    #[test]
    fn empty_current_and_default_count_as_none() {
        assert_eq!(
            profile_choices(&[], Some(""), Some("")),
            vec![choice("basic", false, false)]
        );
    }

    // FR-PROF-14: without an option the recorded profile of the session is reused
    #[test]
    fn recorded_profile_is_reused_without_option() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::Unset, Some("dev"), Some("web")),
            RebuildProfile::Recorded("dev".to_string())
        );
    }

    // FR-PROF-14, D-13: the recorded profile beats the default profile
    #[test]
    fn recorded_profile_beats_default_profile() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::Unset, Some("basic"), Some("web")),
            RebuildProfile::Recorded("basic".to_string())
        );
    }

    // FR-ACT-12, FR-PROF-14: a profile given on the rebuild wins over the recorded one
    #[test]
    fn requested_profile_wins_over_recorded_profile() {
        assert_eq!(
            rebuild_profile(
                &ProfileOpt::Named("ops".to_string()),
                Some("dev"),
                Some("web")
            ),
            RebuildProfile::Requested("ops".to_string())
        );
    }

    // FR-PROF-10: --no-profile ignores the recorded and the default profile at rebuild
    #[test]
    fn no_profile_ignores_recorded_and_default_at_rebuild() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::None, Some("dev"), Some("web")),
            RebuildProfile::Basic
        );
    }

    // D-13: with no recorded profile the default applies
    #[test]
    fn default_profile_applies_when_nothing_is_recorded() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::Unset, None, Some("web")),
            RebuildProfile::Default("web".to_string())
        );
    }

    // D-13: nothing recorded and no default is the built-in layout
    #[test]
    fn nothing_recorded_and_no_default_is_basic() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::Unset, None, None),
            RebuildProfile::Basic
        );
    }

    // FR-PROF-14: an empty recorded name counts as no recorded profile
    #[test]
    fn empty_recorded_name_counts_as_none() {
        assert_eq!(
            rebuild_profile(&ProfileOpt::Unset, Some(""), Some("web")),
            RebuildProfile::Default("web".to_string())
        );
    }

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

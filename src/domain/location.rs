//! Configuration and cache folders from environment values (architecture 9.1). Pure: the caller
//! reads the variables (through `ports::env::Env`) and passes them in.

use super::path::AbsPath;
use crate::error::{LedgeError, Result};
use crate::messages;

/// FR-CONF-08, A-17: an XDG value counts only when it is non-empty and absolute.
fn usable(value: Option<&str>) -> Option<&str> {
    value.filter(|v| v.starts_with('/'))
}

/// FR-PLAT-02, FR-CONF-08: the two per-user folders ledge uses. The config folder is
/// `<XDG_CONFIG_HOME or $HOME/.config>/ledge`; the cache folder is
/// `<XDG_CACHE_HOME or $HOME/.cache>/ledge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config: AbsPath,
    pub cache: AbsPath,
}

impl Dirs {
    /// FR-PLAT-02, FR-CONF-08, A-17: an XDG value is used only when it is non-empty and absolute;
    /// otherwise the folder falls back to `HOME`. Without a usable XDG value and without `HOME`
    /// the error is `HOME is not set.` (architecture 9.1).
    pub fn resolve(
        xdg_config_home: Option<&str>,
        xdg_cache_home: Option<&str>,
        home: Option<&str>,
    ) -> Result<Dirs> {
        let config_base = match usable(xdg_config_home) {
            Some(base) => base.to_string(),
            None => format!("{}/.config", home_or_error(home)?),
        };
        let cache_base = match usable(xdg_cache_home) {
            Some(base) => base.to_string(),
            None => format!("{}/.cache", home_or_error(home)?),
        };
        let root = AbsPath::root();
        Ok(Dirs {
            config: AbsPath::clean(&format!("{config_base}/ledge"), &root),
            cache: AbsPath::clean(&format!("{cache_base}/ledge"), &root),
        })
    }
}

/// FR-PLAT-02, architecture 9.1: `HOME` is needed only when an XDG value is not usable. An empty
/// or relative `HOME` counts as missing, so the error is returned and no relative path is built.
fn home_or_error(home: Option<&str>) -> Result<&str> {
    usable(home).ok_or_else(|| LedgeError::Fatal(messages::home_not_set().to_string()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::Dirs;
    use crate::error::LedgeError;

    // FR-PLAT-02: without XDG settings the folders are under the home folder
    #[test]
    fn falls_back_to_home_config_and_cache_when_xdg_is_unset() {
        let dirs = Dirs::resolve(None, None, Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
        assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
    }

    // FR-CONF-08: an absolute XDG_CONFIG_HOME is used, with "ledge" appended
    #[test]
    fn uses_absolute_xdg_config_home_with_ledge_appended() {
        let dirs = Dirs::resolve(Some("/x/cfg"), None, Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/x/cfg/ledge");
        assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
    }

    // FR-CONF-08: an absolute XDG_CACHE_HOME is used, with "ledge" appended
    #[test]
    fn uses_absolute_xdg_cache_home_with_ledge_appended() {
        let dirs = Dirs::resolve(None, Some("/x/cache"), Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
        assert_eq!(dirs.cache.as_str(), "/x/cache/ledge");
    }

    // D-06, FR-CONF-08: a trailing slash in an XDG value does not add a component
    #[test]
    fn xdg_value_with_trailing_slash_is_cleaned() {
        let dirs = Dirs::resolve(Some("/x/cfg/"), None, Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/x/cfg/ledge");
    }

    // FR-PLAT-02, A-17: an empty XDG value is ignored and the home fallback is used
    #[test]
    fn empty_xdg_values_are_ignored() {
        let dirs = Dirs::resolve(Some(""), Some(""), Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
        assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
    }

    // FR-PLAT-02, A-17: a relative XDG value is ignored (XDG rule) and the home fallback is used
    #[test]
    fn relative_xdg_values_are_ignored() {
        let dirs = Dirs::resolve(Some("cfg"), Some("./cache"), Some("/home/u")).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/home/u/.config/ledge");
        assert_eq!(dirs.cache.as_str(), "/home/u/.cache/ledge");
    }

    // FR-CONF-08: usable XDG values need no HOME
    #[test]
    fn missing_home_is_fine_when_both_xdg_values_are_usable() {
        let dirs = Dirs::resolve(Some("/x/cfg"), Some("/x/cache"), None).expect("resolves");
        assert_eq!(dirs.config.as_str(), "/x/cfg/ledge");
        assert_eq!(dirs.cache.as_str(), "/x/cache/ledge");
    }

    // FR-PLAT-02, architecture 9.1: no usable XDG value and no HOME gives `HOME is not set.`
    #[test]
    fn missing_home_without_usable_xdg_value_is_an_error() {
        let result = Dirs::resolve(Some("cfg"), None, None);
        assert_eq!(
            result,
            Err(LedgeError::Fatal("HOME is not set.".to_string()))
        );
    }

    // FR-PLAT-02, architecture 9.1: an empty HOME counts as missing; no path is built from it
    #[test]
    fn empty_home_without_usable_xdg_value_is_an_error() {
        let result = Dirs::resolve(None, None, Some(""));
        assert_eq!(
            result,
            Err(LedgeError::Fatal("HOME is not set.".to_string()))
        );
    }

    // FR-PLAT-02, architecture 9.1: a relative HOME counts as missing; no relative path is built
    #[test]
    fn relative_home_without_usable_xdg_value_is_an_error() {
        let result = Dirs::resolve(None, Some("cache"), Some("rel"));
        assert_eq!(
            result,
            Err(LedgeError::Fatal("HOME is not set.".to_string()))
        );
    }
}

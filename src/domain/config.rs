//! Configuration file (architecture 9.2): parsing, defaults, search path expansion. Pure: the caller
//! reads the file and supplies HOME, the variable lookup and the working directory.

use super::path::AbsPath;
use crate::error::{LedgeError, Result};
use crate::messages;
use serde_json::Value;

/// FR-CONF-01, D-03, architecture 9.2: the text written on first run. The search path is the literal
/// "~/projects"; the folder itself is not created.
pub const DEFAULT_CONFIG_TEXT: &str = "{\n  \"search_paths\": [\"~/projects\"]\n}\n";

/// FR-CONF-02, architecture 5: the parsed configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Expanded, cleaned and deduplicated (first occurrence kept), architecture 9.2 (F-24).
    pub search_paths: Vec<AbsPath>,
    /// FR-CONF-02: the profile used when none is named. `None` when the key is absent.
    pub default_profile: Option<String>,
}

/// FR-CONF-02, FR-CONF-03, FR-CONF-04, architecture 9.2. `file` is the path shown in messages.
/// `lookup` reads environment variables, `home` is `HOME`, and `cwd` makes relative entries absolute.
/// A search path that uses `~` with no usable `HOME` gives `HOME is not set.`.
pub fn parse(
    text: &str,
    file: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    home: Option<&str>,
    cwd: &AbsPath,
) -> Result<Config> {
    let value: Value =
        serde_json::from_str(text).map_err(|_| fatal(messages::config_not_valid_json(file)))?;
    let object = value
        .as_object()
        .ok_or_else(|| fatal(messages::config_invalid_search_paths(file)))?;

    let entries = match object.get("search_paths") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(entry) => Ok(entry.as_str()),
                _ => Err(fatal(messages::config_invalid_search_paths(file))),
            })
            .collect::<Result<Vec<&str>>>()?,
        Some(_) => return Err(fatal(messages::config_invalid_search_paths(file))),
    };

    let mut search_paths: Vec<AbsPath> = Vec::new();
    for raw in entries {
        // FR-CONF-04: an empty entry names nothing and is skipped.
        if raw.trim().is_empty() {
            continue;
        }
        let path = AbsPath::clean(&expand_entry(raw, lookup, home)?, cwd);
        // F-24: the first occurrence of a duplicate is kept.
        if !search_paths.contains(&path) {
            search_paths.push(path);
        }
    }

    let default_profile = match object.get("default_profile") {
        None => None,
        Some(Value::String(name)) => Some(name.clone()),
        Some(_) => return Err(fatal(messages::config_invalid_default_profile(file))),
    };

    Ok(Config {
        search_paths,
        default_profile,
    })
}

/// FR-CONF-03: variables first, then a leading "~" for HOME. The HOME value is inserted as it is,
/// so a "$" inside HOME is not expanded again.
fn expand_entry(
    raw: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    home: Option<&str>,
) -> Result<String> {
    let with_vars = expand_vars(raw, lookup)?;
    let Some(rest) = with_vars.strip_prefix('~') else {
        return Ok(with_vars);
    };
    if !(rest.is_empty() || rest.starts_with('/')) {
        return Ok(with_vars);
    }
    // A HOME that is missing, empty or relative counts as missing (architecture 9.1).
    let home = home
        .filter(|h| h.starts_with('/'))
        .ok_or_else(|| fatal(messages::home_not_set().to_string()))?;
    Ok(format!("{home}{rest}"))
}

/// FR-CONF-03: replaces `$NAME` and `${NAME}` with the variable value. A "$" that does not start a
/// valid name stays as it is. An unset variable is an error that names the whole entry.
fn expand_vars(entry: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Result<String> {
    let mut out = String::new();
    let mut rest = entry;
    while let Some((before, after)) = rest.split_once('$') {
        out.push_str(before);
        let (name, tail) = match after.strip_prefix('{') {
            Some(inner) => match inner.split_once('}') {
                Some((name, tail)) if is_var_name(name) => (name, tail),
                _ => {
                    out.push('$');
                    rest = after;
                    continue;
                }
            },
            None => {
                let end = after
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(after.len());
                let (name, tail) = after.split_at(end);
                if !is_var_name(name) {
                    out.push('$');
                    rest = after;
                    continue;
                }
                (name, tail)
            }
        };
        let value =
            lookup(name).ok_or_else(|| fatal(messages::config_undefined_variable(entry, name)))?;
        out.push_str(&value);
        rest = tail;
    }
    out.push_str(rest);
    Ok(out)
}

/// A variable name: a letter or underscore, then letters, digits or underscores.
fn is_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn fatal(text: String) -> LedgeError {
    LedgeError::Fatal(text)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{parse, Config, DEFAULT_CONFIG_TEXT};
    use crate::domain::path::AbsPath;
    use crate::error::LedgeError;

    fn abs(text: &str) -> AbsPath {
        AbsPath::clean(text, &AbsPath::root())
    }

    /// A lookup over a fixed list of variables.
    fn vars(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name: &str| {
            pairs
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    fn run(text: &str) -> Result<Config, LedgeError> {
        parse(
            text,
            "/c/config.json",
            &vars(&[("HOME", "/home/u"), ("WORK", "/srv/w")]),
            Some("/home/u"),
            &abs("/cwd"),
        )
    }

    fn paths(config: &Config) -> Vec<&str> {
        config.search_paths.iter().map(AbsPath::as_str).collect()
    }

    // FR-CONF-01, D-03: the default content is exactly the architecture 9.2 block and parses to "~/projects"
    #[test]
    fn default_text_is_the_architecture_default() {
        assert_eq!(
            DEFAULT_CONFIG_TEXT,
            "{\n  \"search_paths\": [\"~/projects\"]\n}\n"
        );
        let config = run(DEFAULT_CONFIG_TEXT).expect("parses");
        assert_eq!(paths(&config), vec!["/home/u/projects"]);
        assert_eq!(config.default_profile, None);
    }

    // FR-CONF-02: the default profile is read when it is a string
    #[test]
    fn reads_default_profile_string() {
        let config = run("{\"search_paths\": [], \"default_profile\": \"dev\"}").expect("parses");
        assert_eq!(config.default_profile.as_deref(), Some("dev"));
    }

    // FR-CONF-02, architecture 9.2: a missing search_paths key means an empty list
    #[test]
    fn missing_search_paths_is_an_empty_list() {
        let config = run("{}").expect("parses");
        assert!(config.search_paths.is_empty());
    }

    // FR-CONF-02, architecture 9.2: unknown keys are ignored
    #[test]
    fn unknown_keys_are_ignored() {
        let config = run("{\"search_paths\": [\"/a\"], \"colour\": 3}").expect("parses");
        assert_eq!(paths(&config), vec!["/a"]);
    }

    // FR-CONF-05: text that is not JSON gives the "not valid JSON" message with the file
    #[test]
    fn invalid_json_gives_not_valid_json_message() {
        assert_eq!(
            run("{ nope"),
            Err(LedgeError::Fatal(
                "config is not valid JSON: /c/config.json".to_string()
            ))
        );
    }

    // FR-CONF-02, architecture 8.3 [new]: search_paths that is not a list gives the invalid message
    #[test]
    fn search_paths_that_is_not_a_list_is_invalid() {
        assert_eq!(
            run("{\"search_paths\": \"/a\"}"),
            Err(LedgeError::Fatal(
                "config is invalid (expected 'search_paths' to be a list of strings): /c/config.json"
                    .to_string()
            ))
        );
    }

    // FR-CONF-02, architecture 8.3 [new]: a list with a non-string entry is invalid
    #[test]
    fn search_paths_with_non_string_entry_is_invalid() {
        assert!(matches!(
            run("{\"search_paths\": [\"/a\", 3]}"),
            Err(LedgeError::Fatal(text)) if text.starts_with("config is invalid (expected 'search_paths'")
        ));
    }

    // FR-CONF-02, architecture 9.2: default_profile that is not a string is invalid
    #[test]
    fn default_profile_that_is_not_a_string_is_invalid() {
        assert_eq!(
            run("{\"default_profile\": 4}"),
            Err(LedgeError::Fatal(
                "config is invalid (expected 'default_profile' to be a string): /c/config.json"
                    .to_string()
            ))
        );
    }

    // FR-CONF-03: "~" at the start expands to HOME
    #[test]
    fn tilde_expands_to_home() {
        let config = run("{\"search_paths\": [\"~\", \"~/code\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/home/u", "/home/u/code"]);
    }

    // FR-CONF-03: "~" in the middle of an entry is not a home reference
    #[test]
    fn tilde_inside_an_entry_is_kept() {
        let config = run("{\"search_paths\": [\"/a/~b\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/a/~b"]);
    }

    // FR-CONF-03: "$VAR" and "${VAR}" expand from the lookup
    #[test]
    fn dollar_forms_expand_from_lookup() {
        let config = run("{\"search_paths\": [\"$WORK/app\", \"${WORK}/lib\", \"x/${WORK}_y\"]}")
            .expect("parses");
        assert_eq!(
            paths(&config),
            vec!["/srv/w/app", "/srv/w/lib", "/cwd/x/srv/w_y"]
        );
    }

    // FR-CONF-03, architecture 9.2: an undefined variable names the entry and the variable
    #[test]
    fn undefined_variable_is_an_error_naming_entry_and_variable() {
        assert_eq!(
            run("{\"search_paths\": [\"$NOPE/x\"]}"),
            Err(LedgeError::Fatal(
                "config search path '$NOPE/x' uses undefined variable 'NOPE'.".to_string()
            ))
        );
        assert_eq!(
            run("{\"search_paths\": [\"${NOPE}\"]}"),
            Err(LedgeError::Fatal(
                "config search path '${NOPE}' uses undefined variable 'NOPE'.".to_string()
            ))
        );
    }

    // FR-CONF-03: a "$" that does not start a valid name stays literal
    #[test]
    fn dollar_without_a_valid_name_stays_literal() {
        let config = run("{\"search_paths\": [\"/a/$1/$/${bad-name}\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/a/$1/$/${bad-name}"]);
    }

    // FR-CONF-03, architecture 9.1: "~" with no usable HOME is the HOME error
    #[test]
    fn tilde_without_home_is_home_not_set() {
        let result = parse(
            "{\"search_paths\": [\"~/x\"]}",
            "/c/config.json",
            &vars(&[]),
            None,
            &abs("/cwd"),
        );
        assert_eq!(
            result,
            Err(LedgeError::Fatal("HOME is not set.".to_string()))
        );
    }

    // FR-CONF-03: a value of a variable is not expanded again, so a "$" inside it stays
    #[test]
    fn expanded_value_is_not_expanded_again() {
        let config = parse(
            "{\"search_paths\": [\"$V\"]}",
            "/c/config.json",
            &vars(&[("V", "/a/$HOME")]),
            Some("/home/u"),
            &abs("/cwd"),
        )
        .expect("parses");
        assert_eq!(paths(&config), vec!["/a/$HOME"]);
    }

    // FR-CONF-04: an empty entry and a blank entry are both skipped
    #[test]
    fn empty_and_blank_entries_are_skipped() {
        let config = run("{\"search_paths\": [\"\", \"   \", \"/a\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/a"]);
    }

    // architecture 9.2 (F-24): duplicates are removed after cleaning, keeping the first
    #[test]
    fn duplicates_are_removed_after_cleaning_keeping_first() {
        let config =
            run("{\"search_paths\": [\"/a/b\", \"/a//b/\", \"/c\", \"/a/b\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/a/b", "/c"]);
    }

    // architecture 9.2: a relative entry is made absolute against the working directory
    #[test]
    fn relative_entry_is_made_absolute_against_cwd() {
        let config = run("{\"search_paths\": [\"work/app\"]}").expect("parses");
        assert_eq!(paths(&config), vec!["/cwd/work/app"]);
    }
}

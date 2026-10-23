//! Profiles (architecture 5, decision D-13): the file format, name rules and the built-in layout.
//! Pure: the caller reads the file.

use crate::error::{LedgeError, Result};
use crate::messages;
use serde_json::Value;

/// D-13: the built-in single-window layout. Its name is reserved.
pub const BASIC_NAME: &str = "basic";
/// FR-PROF-06: the window of the built-in single-window layout.
pub const BASIC_WINDOW: &str = "Terminal";

/// FR-PROF-01, D-13: a profile name that matches `[A-Za-z0-9_-]+`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileName(String);

impl ProfileName {
    /// D-13: `Some` only when the text is non-empty and uses only letters, digits, `_` and `-`.
    pub fn parse(text: &str) -> Option<ProfileName> {
        let valid = !text.is_empty()
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        valid.then(|| ProfileName(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// D-13: `basic` is the built-in layout, not a file.
    pub fn is_basic(&self) -> bool {
        self.0 == BASIC_NAME
    }
}

/// D-13: one window. `name` is non-empty; `command`, when given, is typed into the window then Enter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub name: String,
    pub command: Option<String>,
}

/// FR-PROF-01, FR-PROF-02, FR-PROF-06: a named list of windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: ProfileName,
    /// Non-empty, in the order the profile lists them (FR-PROF-03).
    pub windows: Vec<Window>,
}

impl Profile {
    /// FR-PROF-06: the single window named "Terminal" with no command, under the name `basic`.
    pub fn basic() -> Profile {
        Profile {
            name: ProfileName(BASIC_NAME.to_string()),
            windows: vec![Window {
                name: BASIC_WINDOW.to_string(),
                command: None,
            }],
        }
    }

    /// FR-PROF-02, D-13: parses the text of `profiles/<name>.json`. Unknown fields are ignored.
    /// JSON syntax errors give `profile '<name>' is invalid JSON: <detail>`; any shape problem gives
    /// `profile '<name>' is invalid (expected a 'windows' array)`.
    pub fn parse(name: &ProfileName, text: &str) -> Result<Profile> {
        let value: Value = serde_json::from_str(text).map_err(|error| {
            LedgeError::Fatal(messages::profile_invalid_json(
                name.as_str(),
                &error.to_string(),
            ))
        })?;
        let invalid = || LedgeError::Fatal(messages::profile_invalid_shape(name.as_str()));
        let items = value
            .get("windows")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .ok_or_else(invalid)?;
        let windows = items
            .iter()
            .map(|item| parse_window(item).ok_or_else(invalid))
            .collect::<Result<Vec<Window>>>()?;
        Ok(Profile {
            name: name.clone(),
            windows,
        })
    }
}

/// FR-PROF-02, D-13: a window is an object with a non-empty string `name` and an optional string
/// `command`. Anything else makes the window entry invalid.
fn parse_window(item: &Value) -> Option<Window> {
    let name = item.get("name")?.as_str().filter(|n| !n.is_empty())?;
    let command = match item.get("command") {
        None => None,
        Some(Value::String(command)) => Some(command.clone()),
        Some(_) => return None,
    };
    Some(Window {
        name: name.to_string(),
        command,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{Profile, ProfileName};
    use crate::error::LedgeError;

    fn name(text: &str) -> ProfileName {
        ProfileName::parse(text).expect("valid name")
    }

    fn shape_error() -> LedgeError {
        LedgeError::Fatal("profile 'dev' is invalid (expected a 'windows' array)".to_string())
    }

    // FR-PROF-01, D-13: letters, digits, "_" and "-" are valid names
    #[test]
    fn names_with_allowed_characters_are_valid() {
        for text in ["dev", "Dev-2", "a_b", "9"] {
            assert!(ProfileName::parse(text).is_some(), "{text}");
        }
    }

    // FR-PROF-01, D-13: other characters and the empty name are not valid names
    #[test]
    fn names_with_other_characters_are_invalid() {
        for text in ["", "a b", "a.b", "a/b", "../x", "é", "a:b"] {
            assert!(ProfileName::parse(text).is_none(), "{text:?}");
        }
    }

    // FR-PROF-06: the built-in layout is one window named Terminal with no command, named basic
    #[test]
    fn basic_is_one_terminal_window_without_command() {
        let basic = Profile::basic();
        assert_eq!(basic.name.as_str(), "basic");
        assert!(basic.name.is_basic());
        assert_eq!(basic.windows.len(), 1);
        assert_eq!(basic.windows[0].name, "Terminal");
        assert_eq!(basic.windows[0].command, None);
    }

    // FR-PROF-02: the windows list is read in order, with optional commands
    #[test]
    fn reads_windows_in_order_with_optional_commands() {
        let profile = Profile::parse(
            &name("dev"),
            "{\"windows\": [{\"name\": \"editor\", \"command\": \"nvim\"}, {\"name\": \"shell\"}]}",
        )
        .expect("parses");
        assert_eq!(profile.name.as_str(), "dev");
        assert_eq!(profile.windows[0].name, "editor");
        assert_eq!(profile.windows[0].command.as_deref(), Some("nvim"));
        assert_eq!(profile.windows[1].name, "shell");
        assert_eq!(profile.windows[1].command, None);
    }

    // D-13: unknown fields are ignored and duplicate window names are allowed
    #[test]
    fn unknown_fields_and_duplicate_names_are_allowed() {
        let profile = Profile::parse(
            &name("dev"),
            "{\"title\": 1, \"windows\": [{\"name\": \"a\", \"size\": 2}, {\"name\": \"a\"}]}",
        )
        .expect("parses");
        assert_eq!(profile.windows.len(), 2);
    }

    // FR-PROF-02, D-13: a profile without a windows key is invalid
    #[test]
    fn missing_windows_is_invalid_shape() {
        assert_eq!(Profile::parse(&name("dev"), "{}"), Err(shape_error()));
    }

    // FR-PROF-02, D-13: an empty windows array is invalid
    #[test]
    fn empty_windows_is_invalid_shape() {
        assert_eq!(
            Profile::parse(&name("dev"), "{\"windows\": []}"),
            Err(shape_error())
        );
    }

    // FR-PROF-02, D-13: windows that is not an array is invalid
    #[test]
    fn windows_that_is_not_an_array_is_invalid_shape() {
        assert_eq!(
            Profile::parse(&name("dev"), "{\"windows\": {\"name\": \"x\"}}"),
            Err(shape_error())
        );
    }

    // FR-PROF-02, D-13: a window without a name, or with an empty name, is invalid
    #[test]
    fn window_without_non_empty_name_is_invalid_shape() {
        for text in [
            "{\"windows\": [{\"command\": \"ls\"}]}",
            "{\"windows\": [{\"name\": \"\"}]}",
            "{\"windows\": [{\"name\": 3}]}",
        ] {
            assert_eq!(
                Profile::parse(&name("dev"), text),
                Err(shape_error()),
                "{text}"
            );
        }
    }

    // D-13: a command that is present but not a string makes the window invalid
    #[test]
    fn non_string_command_is_invalid_shape() {
        assert_eq!(
            Profile::parse(
                &name("dev"),
                "{\"windows\": [{\"name\": \"a\", \"command\": 5}]}"
            ),
            Err(shape_error())
        );
    }

    // D-13: a window entry that is not an object is invalid
    #[test]
    fn window_entry_that_is_not_an_object_is_invalid_shape() {
        assert_eq!(
            Profile::parse(&name("dev"), "{\"windows\": [\"editor\"]}"),
            Err(shape_error())
        );
    }

    // D-13: text that is not JSON gives the invalid JSON message with the name and a detail
    #[test]
    fn invalid_json_gives_invalid_json_message() {
        let result = Profile::parse(&name("dev"), "{ windows");
        assert!(matches!(result, Err(LedgeError::Fatal(text))
            if text.starts_with("profile 'dev' is invalid JSON: ")));
    }
}

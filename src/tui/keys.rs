//! Input to picker action mapping (architecture 6.3). The mapping takes no picker argument, so a key
//! means the same action in every picker (FR-TUI-07). Arrow and page keys: FR-TUI-03.

use crate::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What a key asks for. Choose, cancel and the node or row operations are returned to the caller;
/// movement and filter editing are applied by `PickerModel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Left,
    Right,
    Choose,
    Cancel,
    Backspace,
    Type(char),
    Rebuild,
    Profile,
    Kill,
    DeleteBookmark,
    RebuildNode,
    ProfileNode,
    KillNode,
    DeleteNode,
}

/// FR-TUI-03, FR-TUI-07: maps one input event to an action. Resizes map to nothing.
pub fn map(input: &Input) -> Option<Action> {
    match input {
        Input::Key(key) => map_key(key),
        Input::Resize(..) => None,
    }
}

/// FR-TUI-03, FR-TUI-07, architecture 6.3. ctrl-h arrives as backspace in raw mode (section 6.5), so it
/// is handled by `Backspace` and the edit window decides what it means.
pub fn map_key(key: &KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let action = match key.code {
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        KeyCode::Left => Action::Left,
        KeyCode::Right => Action::Right,
        KeyCode::Enter => Action::Choose,
        KeyCode::Esc => Action::Cancel,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            'c' => Action::Cancel,
            'h' => Action::Left,
            'l' => Action::Right,
            'r' => Action::Rebuild,
            'p' => Action::Profile,
            'x' => Action::Kill,
            'd' => Action::DeleteBookmark,
            _ => return None,
        },
        KeyCode::Char(c) if alt => match c.to_ascii_lowercase() {
            'r' => Action::RebuildNode,
            'p' => Action::ProfileNode,
            'x' => Action::KillNode,
            'd' => Action::DeleteNode,
            _ => return None,
        },
        KeyCode::Char(c) => Action::Type(c),
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> Option<Action> {
        map_key(&KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn ctrl(c: char) -> Option<Action> {
        map_key(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
    }

    fn alt(c: char) -> Option<Action> {
        map_key(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT))
    }

    // FR-TUI-03: arrow and page keys, home and end (A-28)
    #[test]
    fn arrow_and_page_keys_map_to_movement() {
        assert_eq!(key(KeyCode::Up), Some(Action::Up));
        assert_eq!(key(KeyCode::Down), Some(Action::Down));
        assert_eq!(key(KeyCode::PageUp), Some(Action::PageUp));
        assert_eq!(key(KeyCode::PageDown), Some(Action::PageDown));
        assert_eq!(key(KeyCode::Home), Some(Action::Home));
        assert_eq!(key(KeyCode::End), Some(Action::End));
    }

    // FR-TUI-07: the picker-wide keys of architecture 6.3
    #[test]
    fn enter_escape_and_ctrl_c_are_choose_and_cancel() {
        assert_eq!(key(KeyCode::Enter), Some(Action::Choose));
        assert_eq!(key(KeyCode::Esc), Some(Action::Cancel));
        assert_eq!(ctrl('c'), Some(Action::Cancel));
    }

    // FR-TUI-07: ctrl keys of the node and row operations
    #[test]
    fn ctrl_letters_map_to_row_actions() {
        assert_eq!(ctrl('r'), Some(Action::Rebuild));
        assert_eq!(ctrl('p'), Some(Action::Profile));
        assert_eq!(ctrl('x'), Some(Action::Kill));
        assert_eq!(ctrl('d'), Some(Action::DeleteBookmark));
    }

    // FR-TUI-07: alt keys of the tree node actions
    #[test]
    fn alt_letters_map_to_node_actions() {
        assert_eq!(alt('r'), Some(Action::RebuildNode));
        assert_eq!(alt('p'), Some(Action::ProfileNode));
        assert_eq!(alt('x'), Some(Action::KillNode));
        assert_eq!(alt('d'), Some(Action::DeleteNode));
    }

    // FR-TUI-03: printable characters append to the filter, backspace deletes
    #[test]
    fn printable_characters_type_and_backspace_deletes() {
        assert_eq!(key(KeyCode::Char('a')), Some(Action::Type('a')));
        assert_eq!(key(KeyCode::Char(' ')), Some(Action::Type(' ')));
        assert_eq!(key(KeyCode::Backspace), Some(Action::Backspace));
    }

    // FR-TUI-03, architecture 6.3: left and right arrows, ctrl-h and ctrl-l
    #[test]
    fn left_and_right_have_ctrl_aliases() {
        assert_eq!(key(KeyCode::Left), Some(Action::Left));
        assert_eq!(key(KeyCode::Right), Some(Action::Right));
        assert_eq!(ctrl('h'), Some(Action::Left));
        assert_eq!(ctrl('l'), Some(Action::Right));
    }

    // FR-TUI-07: keys without an action in any picker do nothing
    #[test]
    fn unbound_control_and_alt_keys_do_nothing() {
        assert_eq!(ctrl('z'), None);
        assert_eq!(alt('q'), None);
        assert_eq!(key(KeyCode::F(5)), None);
    }

    #[test]
    fn resize_maps_to_nothing() {
        assert_eq!(map(&Input::Resize(80, 24)), None);
    }
}

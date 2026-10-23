//! Picker model (architecture 6.1): rows, filter, selection and scroll. Pure: no terminal and no I/O.
//! Keys arrive as `Action` (tui::keys); choose, cancel and row operations are returned as `Effect`
//! for the caller (the engine, S-08).

use crate::tui::columns::ColumnSpec;
use crate::tui::fuzzy;
use crate::tui::keys::Action;
use std::ops::Range;

/// Screen rows that are not list rows: the two border rows, prompt, header, status and footer.
const CHROME_ROWS: u16 = 6;

/// Rows of the list for a terminal `term_height` rows high (FR-TUI-05).
pub fn list_height(term_height: u16) -> usize {
    usize::from(term_height.saturating_sub(CHROME_ROWS))
}

/// One row of a picker. `cells` has one text per column of the picker's `Chrome`.
#[derive(Debug, Clone)]
pub struct PickerRow<T> {
    pub cells: Vec<String>,
    pub payload: T,
    /// Text the filter matches (architecture 6.1).
    pub search_text: String,
    /// Missing directory: dimmed, with the missing cue (FR-TUI-08).
    pub dimmed: bool,
    /// Gray tree entry that is not openable (FR-TREE-11).
    pub connector: bool,
    /// Tree indent.
    pub depth: usize,
    /// Edit window bookmark marker.
    pub marked: bool,
    /// Row the selection starts on (FR-PICK-04).
    pub preselect: bool,
}

impl<T> PickerRow<T> {
    pub fn new(cells: Vec<String>, search_text: &str, payload: T) -> Self {
        Self {
            cells,
            payload,
            search_text: search_text.to_string(),
            dimmed: false,
            connector: false,
            depth: 0,
            marked: false,
            preselect: false,
        }
    }
}

/// Chrome of one picker (architecture 6.2): label, icon, columns, footer and the behaviour rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chrome {
    pub label: String,
    pub icon: &'static str,
    pub columns: Vec<ColumnSpec>,
    /// Column the filter matches and highlights (`None`: no filter).
    pub search_column: Option<usize>,
    /// Column that carries the ` (missing)` cue of a dimmed row.
    pub missing_column: Option<usize>,
    pub footer: String,
    /// Info text shown in the list when there are no rows.
    pub empty_message: Option<String>,
    /// The session switcher and session tree stay open when they have no rows (FR-TUI-10, D-12).
    pub stay_open_when_empty: bool,
}

impl Chrome {
    /// Chrome with the search on column 0 and no cues or empty-state rules.
    pub fn new(label: &str, icon: &'static str, columns: Vec<ColumnSpec>, footer: &str) -> Self {
        Self {
            label: label.to_string(),
            icon,
            columns,
            search_column: Some(0),
            missing_column: None,
            footer: footer.to_string(),
            empty_message: None,
            stay_open_when_empty: false,
        }
    }
}

/// What the caller must do after `PickerModel::apply`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Movement or filter edit, applied to the model.
    Handled,
    /// Enter on a row: the caller takes the selected row (FR-TUI-07).
    Choose,
    /// Escape, ctrl-c, or enter with no visible row (F-14).
    Cancel,
    /// A row or node operation for the caller.
    Act(Action),
}

/// The picker state: rows, filter, selection, scroll and status line.
#[derive(Debug, Clone)]
pub struct PickerModel<T> {
    rows: Vec<PickerRow<T>>,
    chrome: Chrome,
    filter: String,
    /// Indexes into `rows` that are shown, in row order (tree ancestors included).
    visible: Vec<usize>,
    /// Position in `visible`.
    selected: usize,
    /// First visible position on screen.
    scroll: usize,
    /// List height in rows; 0 until the size is set.
    page: usize,
    status: Option<String>,
}

impl<T> PickerModel<T> {
    /// FR-PICK-04: the selection starts on the row marked `preselect`, else on the first row.
    pub fn new(rows: Vec<PickerRow<T>>, chrome: Chrome) -> Self {
        let selected = rows.iter().position(|row| row.preselect).unwrap_or(0);
        let visible = (0..rows.len()).collect();
        Self {
            rows,
            chrome,
            filter: String::new(),
            visible,
            selected,
            scroll: 0,
            page: 0,
            status: None,
        }
    }

    pub fn chrome(&self) -> &Chrome {
        &self.chrome
    }

    pub fn rows(&self) -> &[PickerRow<T>] {
        &self.rows
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn visible(&self) -> &[usize] {
        &self.visible
    }

    /// Position in `visible` of the selected row.
    pub fn selected_position(&self) -> usize {
        self.selected
    }

    /// The selected row, or `None` when no row is visible.
    pub fn selected_row(&self) -> Option<&PickerRow<T>> {
        self.visible
            .get(self.selected)
            .and_then(|&row| self.rows.get(row))
    }

    /// Positions in `visible` that the list shows.
    pub fn view(&self) -> Range<usize> {
        let end = (self.scroll + self.page.max(1)).min(self.visible.len());
        self.scroll..end
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }

    /// Sets the list height from the terminal size (FR-TUI-05) and keeps the selection visible.
    pub fn set_list_height(&mut self, rows: usize) {
        self.page = rows;
        self.ensure_visible();
    }

    /// FR-TUI-10, FR-CMD-12: a picker with no rows closes unless it is meant to stay open.
    pub fn should_close(&self) -> bool {
        self.rows.is_empty() && !self.chrome.stay_open_when_empty
    }

    /// Applies one action (architecture 6.1, 6.3). Movement never wraps (A-28).
    pub fn apply(&mut self, action: Action) -> Effect {
        let last = self.visible.len().saturating_sub(1);
        let step = self.page.saturating_sub(1).max(1);
        let target = match action {
            Action::Up => self.selected.saturating_sub(1),
            Action::Down => (self.selected + 1).min(last),
            Action::PageUp => self.selected.saturating_sub(step),
            Action::PageDown => (self.selected + step).min(last),
            Action::Home => 0,
            Action::End => last,
            Action::Left | Action::Right => return Effect::Handled,
            Action::Choose => {
                return if self.selected_row().is_some() {
                    Effect::Choose
                } else {
                    Effect::Cancel
                };
            }
            Action::Cancel => return Effect::Cancel,
            Action::Type(c) => {
                self.filter.push(c);
                self.refilter();
                return Effect::Handled;
            }
            Action::Backspace => {
                self.filter.pop();
                self.refilter();
                return Effect::Handled;
            }
            other => return Effect::Act(other),
        };
        self.selected = target;
        self.ensure_visible();
        Effect::Handled
    }

    /// Replaces the rows after an action (D-12 reopen). The filter is kept and the selection keeps
    /// its position, clamped to the new list.
    pub fn reload(&mut self, rows: Vec<PickerRow<T>>) {
        self.rows = rows;
        let (visible, _) = self.match_rows();
        self.visible = visible;
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
        self.ensure_visible();
    }

    /// Recomputes the visible rows after a filter edit (D-10). An empty filter keeps the row selected
    /// before when it is still visible; otherwise the selection moves to the best match.
    fn refilter(&mut self) {
        let previous = self.visible.get(self.selected).copied();
        let (visible, best) = self.match_rows();
        self.visible = visible;
        let target = if self.filter.is_empty() {
            previous
        } else {
            best
        };
        self.selected = target
            .and_then(|row| self.visible.iter().position(|&v| v == row))
            .unwrap_or(0);
        self.ensure_visible();
    }

    /// Visible rows for the current filter, and the best matching row (`None` for an empty filter).
    fn match_rows(&self) -> (Vec<usize>, Option<usize>) {
        if self.filter.is_empty() {
            return ((0..self.rows.len()).collect(), None);
        }
        let texts: Vec<&str> = self
            .rows
            .iter()
            .map(|row| row.search_text.as_str())
            .collect();
        let hits = fuzzy::filter(&self.filter, &texts);
        let best = fuzzy::best_hit(&hits)
            .and_then(|position| hits.get(position))
            .map(|hit| hit.index);
        let matching: Vec<usize> = hits.iter().map(|hit| hit.index).collect();
        (self.with_ancestors(&matching), best)
    }

    /// Adds the ancestors of each matching tree row (D-10). Ancestors are not scored. In a flat list
    /// every row has depth 0 and nothing is added.
    fn with_ancestors(&self, matching: &[usize]) -> Vec<usize> {
        let mut keep = vec![false; self.rows.len()];
        for &index in matching {
            if let Some(flag) = keep.get_mut(index) {
                *flag = true;
            }
            let Some(row) = self.rows.get(index) else {
                continue;
            };
            let mut depth = row.depth;
            for above in (0..index).rev() {
                if depth == 0 {
                    break;
                }
                let Some(candidate) = self.rows.get(above) else {
                    continue;
                };
                if candidate.depth < depth {
                    if let Some(flag) = keep.get_mut(above) {
                        *flag = true;
                    }
                    depth = candidate.depth;
                }
            }
        }
        keep.iter()
            .enumerate()
            .filter_map(|(index, &kept)| kept.then_some(index))
            .collect()
    }

    /// Moves `scroll` so that the selected position is inside the list (FR-TUI-05).
    fn ensure_visible(&mut self) {
        let page = self.page.max(1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + page {
            self.scroll = self.selected + 1 - page;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::columns::ColumnSpec;

    fn chrome() -> Chrome {
        Chrome::new(
            "Projects",
            "▸",
            vec![ColumnSpec::text("Project")],
            "enter open",
        )
    }

    fn flat(names: &[&str]) -> Vec<PickerRow<usize>> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| PickerRow::new(vec![(*name).to_string()], name, i))
            .collect()
    }

    fn tree(entries: &[(&str, usize)]) -> Vec<PickerRow<usize>> {
        entries
            .iter()
            .enumerate()
            .map(|(i, (name, depth))| {
                let mut row = PickerRow::new(vec![(*name).to_string()], name, i);
                row.depth = *depth;
                row
            })
            .collect()
    }

    fn names(model: &PickerModel<usize>) -> Vec<String> {
        model
            .visible()
            .iter()
            .map(|&i| model.rows()[i].cells[0].clone())
            .collect()
    }

    fn type_text(model: &mut PickerModel<usize>, text: &str) {
        for c in text.chars() {
            model.apply(Action::Type(c));
        }
    }

    // FR-PICK-04: the selection starts on the preselected row
    #[test]
    fn starts_on_the_preselected_row() {
        let mut rows = flat(&["alpha", "beta", "gamma"]);
        if let Some(row) = rows.get_mut(1) {
            row.preselect = true;
        }
        let model = PickerModel::new(rows, chrome());
        assert_eq!(model.selected_position(), 1);
        assert_eq!(model.selected_row().map(|r| r.payload), Some(1));
    }

    // FR-TUI-03, A-28: up and down move by one and do not wrap
    #[test]
    fn up_and_down_move_by_one_without_wrapping() {
        let mut model = PickerModel::new(flat(&["a", "b", "c"]), chrome());
        model.apply(Action::Up);
        assert_eq!(model.selected_position(), 0);
        model.apply(Action::Down);
        model.apply(Action::Down);
        model.apply(Action::Down);
        assert_eq!(model.selected_position(), 2);
    }

    // FR-TUI-03: page keys move by the list height minus one
    #[test]
    fn page_keys_move_by_visible_height_minus_one() {
        let names: Vec<String> = (0..40).map(|i| format!("row{i:02}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut model = PickerModel::new(flat(&refs), chrome());
        model.set_list_height(10);
        model.apply(Action::PageDown);
        assert_eq!(model.selected_position(), 9);
        model.apply(Action::PageDown);
        assert_eq!(model.selected_position(), 18);
        model.apply(Action::PageUp);
        assert_eq!(model.selected_position(), 9);
    }

    // FR-TUI-03, A-28: home and end go to the first and last visible row
    #[test]
    fn home_and_end_go_to_the_first_and_last_visible_row() {
        let mut model = PickerModel::new(flat(&["a", "b", "c", "d"]), chrome());
        model.apply(Action::End);
        assert_eq!(model.selected_position(), 3);
        model.apply(Action::Home);
        assert_eq!(model.selected_position(), 0);
    }

    // FR-TUI-02, D-10: typing moves the selection to the best match
    #[test]
    fn typing_moves_the_selection_to_the_best_match() {
        let mut model = PickerModel::new(flat(&["notes", "ledge-app", "lab"]), chrome());
        type_text(&mut model, "led");
        assert_eq!(names(&model), vec!["ledge-app".to_string()]);
        assert_eq!(model.selected_row().map(|r| r.payload), Some(1));
    }

    // D-10: an emptied filter keeps the row that was selected while filtering, if still visible
    #[test]
    fn clearing_the_filter_keeps_the_selected_row() {
        let mut model = PickerModel::new(flat(&["alpha", "beta", "gamma"]), chrome());
        type_text(&mut model, "ga");
        model.apply(Action::Backspace);
        model.apply(Action::Backspace);
        assert_eq!(model.filter(), "");
        assert_eq!(model.selected_row().map(|r| r.payload), Some(2));
    }

    // D-10: in a tree a matching row keeps its ancestors visible, and siblings stay hidden
    #[test]
    fn tree_match_keeps_its_ancestors_visible() {
        let rows = tree(&[("work", 0), ("ledge", 1), ("other", 0), ("docs", 1)]);
        let mut model = PickerModel::new(rows, chrome());
        type_text(&mut model, "docs");
        assert_eq!(names(&model), vec!["other".to_string(), "docs".to_string()]);
        assert_eq!(model.selected_row().map(|r| r.payload), Some(3));
    }

    // FR-TUI-05: the selected row stays inside the list after a resize
    #[test]
    fn selected_row_stays_visible_when_the_list_gets_smaller() {
        let names: Vec<String> = (0..30).map(|i| format!("row{i:02}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut model = PickerModel::new(flat(&refs), chrome());
        model.set_list_height(20);
        model.apply(Action::End);
        assert_eq!(model.view(), 10..30);
        model.set_list_height(2);
        assert!(model.view().contains(&model.selected_position()));
        assert_eq!(model.view(), 28..30);
    }

    // FR-TUI-05: the list height follows the terminal height and never goes below zero
    #[test]
    fn list_height_is_the_terminal_height_minus_the_chrome() {
        assert_eq!(list_height(8), 2);
        assert_eq!(list_height(24), 18);
        assert_eq!(list_height(3), 0);
    }

    // D-12: a reload keeps the filter and the position, clamped to the new list
    #[test]
    fn reload_keeps_filter_and_position_clamped() {
        let mut model = PickerModel::new(flat(&["a1", "a2", "a3", "b1"]), chrome());
        type_text(&mut model, "a");
        model.apply(Action::End);
        assert_eq!(model.selected_position(), 2);
        model.reload(flat(&["a1", "a2"]));
        assert_eq!(model.filter(), "a");
        assert_eq!(model.selected_position(), 1);
        assert_eq!(names(&model), vec!["a1".to_string(), "a2".to_string()]);
    }

    // FR-TUI-10, FR-CMD-12: a picker closes when its last row is gone, unless it stays open
    #[test]
    fn closes_when_empty_unless_it_stays_open() {
        let mut closing = PickerModel::new(flat(&["only"]), chrome());
        closing.reload(Vec::new());
        assert!(closing.should_close());

        let mut staying_chrome = chrome();
        staying_chrome.stay_open_when_empty = true;
        let mut staying = PickerModel::new(flat(&["only"]), staying_chrome);
        staying.reload(Vec::new());
        assert!(!staying.should_close());
    }

    // FR-TUI-10: a picker with rows stays open after an action
    #[test]
    fn stays_open_while_rows_remain() {
        let mut model = PickerModel::new(flat(&["a", "b"]), chrome());
        model.reload(flat(&["b"]));
        assert!(!model.should_close());
    }

    // F-14: enter with no visible row cancels
    #[test]
    fn choose_with_no_visible_row_cancels() {
        let mut model = PickerModel::new(flat(&["alpha"]), chrome());
        type_text(&mut model, "zzz");
        assert_eq!(model.apply(Action::Choose), Effect::Cancel);
        let mut model = PickerModel::new(flat(&["alpha"]), chrome());
        assert_eq!(model.apply(Action::Choose), Effect::Choose);
    }

    // FR-TUI-07: row operations are returned to the caller unchanged
    #[test]
    fn row_actions_are_returned_to_the_caller() {
        let mut model = PickerModel::new(flat(&["a"]), chrome());
        assert_eq!(model.apply(Action::Rebuild), Effect::Act(Action::Rebuild));
        assert_eq!(model.apply(Action::Cancel), Effect::Cancel);
    }
}

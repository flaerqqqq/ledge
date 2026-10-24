//! The project, history, bookmark and session switcher pickers (architecture 2, 6.2, 6.4; FR-PICK-04..13, FR-HIST-04,
//! FR-HIST-05, FR-BM-11..13, FR-SESS-01..04, FR-CMD-08, FR-CMD-12). Each entry point loads its rows through the ports,
//! runs the picker, and on enter opens the chosen directory through the open flow after the terminal is restored
//! (architecture 6.4). Rows are built here; the screen and the keys are `tui`. Row actions: ctrl-x kills, ctrl-r
//! rebuilds, ctrl-p picks a profile in a nested picker and rebuilds with it (S-15, FR-ACT-01, FR-ACT-02, FR-ACT-04,
//! FR-ACT-12). ctrl-d deletes a bookmark and kills its session in the bookmark picker only (S-19, FR-BM-13, FR-ACT-11);
//! the alt-key actions are not bound in these pickers.

use crate::app::actions::{
    current_session, delete_node_bookmarks, kill, kill_node, never_touch_current, rebuild,
    rebuild_node,
};
use crate::app::bookmark_cmd::{read_bookmarks, write_bookmarks};
use crate::app::launch::Editor;
use crate::app::open::{attach_or_switch, open_dir_with_editor};
use crate::app::store;
use crate::app::Ctx;
use crate::app::{home_of, load_projects};
use crate::domain::age::age;
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::ProfileName;
use crate::domain::profile_rules::{
    open_profile, profile_choices, OpenProfile, ProfileChoice, ProfileOpt,
};
use crate::domain::projects::{project_containing, project_rows, Project};
use crate::domain::sessions::SessionInfo;
use crate::domain::sessions::{managed, session_directory, switcher_order, ManagedSession};
use crate::domain::tree::{bookmark_view, session_tree, NodeKind, SessionDir, TreeRow, TreeScope};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;
use crate::tui::columns::ColumnSpec;
use crate::tui::engine::{run_picker, PickerHandler, PickerResult, Step};
use crate::tui::keys::Action;
use crate::tui::picker::{
    Chrome, Details, PickerModel, PickerRow, Preview, PreviewWindow, SessionCell,
};
use crate::tui::render;
use crate::tui::theme::{self, ColourLevel};
use std::path::Path;

/// One picker row: the directory it opens.
type DirRow = PickerRow<AbsPath>;

/// FR-PICK-04..10, FR-PICK-12, FR-PICK-13, FR-CMD-08, FR-CMD-12: runs the project picker and opens the chosen
/// project. `dir` is the directory the picker is launched from; its row comes first and is preselected (FR-PICK-04).
/// `None` means the working directory. The CLI rejects a directory argument on `project pick` (FR-PICK-11); this
/// entry point takes it as the launch directory so that callers and tests can choose it. `option` is the profile
/// option of the open (FR-PICK-12).
pub fn project_picker(ctx: &Ctx<'_>, dir: Option<&str>, option: &ProfileOpt) -> Result<Outcome> {
    project_picker_with_editor(ctx, dir, option, None)
}

/// FR-LAUNCH-01, A-18: as `project_picker`; `editor` is launched on the chosen project when the open creates its
/// session (S-17 passes the `--code`, `--idea` or `--editor` option here).
pub fn project_picker_with_editor(
    ctx: &Ctx<'_>,
    dir: Option<&str>,
    option: &ProfileOpt,
    editor: Option<&Editor>,
) -> Result<Outcome> {
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    if config.search_paths.is_empty() {
        let file = store::config_path(&ctx.dirs);
        return Err(LedgeError::Fatal(messages::no_search_paths(
            &file.display().to_string(),
        )));
    }
    let projects = load_projects(ctx.fs, &config.search_paths);
    if projects.is_empty() {
        // D-12: an empty project list at start closes with an info line, no window.
        return Ok(Outcome::Info(messages::no_projects_found().to_string()));
    }
    let from = AbsPath::clean(dir.unwrap_or(ctx.cwd.as_str()), &ctx.cwd);
    let applied = applied_profile(option, config.default_profile.as_deref());
    let label = messages::projects_label(applied.as_deref());
    let rows = project_dir_rows(ctx, &from, &projects)?;
    let model = PickerModel::new(rows, project_chrome(&label));
    let mut handler = ProjectHandler {
        ctx,
        from,
        search_paths: config.search_paths,
        option: option.clone(),
    };
    run_dir_picker(ctx, model, &mut handler, option, editor)
}

/// FR-HIST-04, FR-HIST-05, D-12: runs the history picker and opens the chosen directory. A missing history file
/// prints `No history cache found.`; an empty one prints `No history.`; both stop without a window.
pub fn history_picker(ctx: &Ctx<'_>, option: &ProfileOpt) -> Result<Outcome> {
    history_picker_with_editor(ctx, option, None)
}

/// FR-LAUNCH-01, A-18: as `history_picker`, with the editor launched when the open creates the session.
pub fn history_picker_with_editor(
    ctx: &Ctx<'_>,
    option: &ProfileOpt,
    editor: Option<&Editor>,
) -> Result<Outcome> {
    if !ctx.fs.exists(&store::history_path(&ctx.dirs)) {
        return Ok(Outcome::Info(messages::no_history_cache().to_string()));
    }
    let history = load_history(ctx)?;
    if history.is_empty() {
        return Ok(Outcome::Info(messages::no_history().to_string()));
    }
    let rows = dir_rows(ctx, &history)?;
    let model = PickerModel::new(rows, history_chrome());
    let mut handler = HistoryHandler {
        ctx,
        option: option.clone(),
    };
    run_dir_picker(ctx, model, &mut handler, option, editor)
}

/// FR-BM-11, FR-BM-12, FR-BM-13, D-12: runs the bookmark picker over the bookmarks in stored order and opens the chosen
/// one. With no bookmarks it prints `No bookmarks. Use 'ledge bm add' to add one.` and stops without a window.
pub fn bookmark_picker(ctx: &Ctx<'_>, option: &ProfileOpt) -> Result<Outcome> {
    let bookmarks = read_bookmarks(ctx)?;
    if bookmarks.entries().is_empty() {
        return Ok(Outcome::Info(messages::no_bookmarks_to_pick().to_string()));
    }
    let rows = dir_rows(ctx, bookmarks.entries())?;
    let model = PickerModel::new(rows, bookmark_chrome());
    let mut handler = BookmarkHandler {
        ctx,
        option: option.clone(),
    };
    run_dir_picker(ctx, model, &mut handler, option, None)
}

/// Enter and the actions of the bookmark picker (architecture 6.4). Ctrl-d deletes the bookmark; the other keys are
/// the project picker's.
struct BookmarkHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
    option: ProfileOpt,
}

impl PickerHandler<AbsPath> for BookmarkHandler<'_, '_> {
    fn choose(&mut self, row: &AbsPath) -> Step {
        choose_dir(self.ctx, row)
    }

    fn act(&mut self, action: Action, row: Option<&AbsPath>) -> Step {
        match (action, row) {
            (Action::DeleteBookmark, Some(dir)) => delete_bookmark(self.ctx, dir),
            (Action::DeleteBookmark, None) => no_action(),
            _ => dir_action(self.ctx, &self.option, action, row),
        }
    }

    fn reload(&mut self) -> Result<Vec<DirRow>> {
        let bookmarks = read_bookmarks(self.ctx)?;
        dir_rows(self.ctx, bookmarks.entries())
    }
}

/// FR-BM-13, FR-ACT-11, D-12, D-17, A-39: ctrl-d on a bookmark row deletes the bookmark, then kills the managed session of
/// its directory. The session the user is in is not killed here; the status line says so and the bookmark is still
/// deleted (ctrl-d is a delete, not a kill, so P-63 does not apply).
/// Deleting the last bookmark closes the picker (D-12).
fn delete_bookmark(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    let mut bookmarks = match read_bookmarks(ctx) {
        Ok(bookmarks) => bookmarks,
        Err(error) => return failed(error),
    };
    if !bookmarks.remove(dir) {
        return no_action();
    }
    if let Err(error) = write_bookmarks(ctx, &bookmarks) {
        return failed(error);
    }
    let name = session_name(dir);
    let status = match delete_session_of(ctx, &name) {
        Ok(status) => status,
        Err(error) => Some(error.to_string()),
    };
    if bookmarks.entries().is_empty() {
        return Step::Close(Outcome::Done);
    }
    Step::Continue {
        status,
        reload: true,
    }
}

/// D-17, A-39: kills the managed session `name` unless the user is in it, which is the status line's refusal. A session
/// that is not running has nothing to kill.
fn delete_session_of(ctx: &Ctx<'_>, name: &str) -> Result<Option<String>> {
    let current = current_session(ctx)?;
    if never_touch_current(name, current.as_deref()) {
        return Ok(Some(messages::cannot_kill_current_session().to_string()));
    }
    let sessions = live_sessions(ctx)?;
    if sessions.iter().any(|session| session.info().name == name) {
        kill(ctx, name)?;
    }
    Ok(None)
}

/// Enter and the actions of the project picker (architecture 6.4).
struct ProjectHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
    from: AbsPath,
    search_paths: Vec<AbsPath>,
    option: ProfileOpt,
}

impl PickerHandler<AbsPath> for ProjectHandler<'_, '_> {
    fn choose(&mut self, row: &AbsPath) -> Step {
        choose_dir(self.ctx, row)
    }

    fn act(&mut self, action: Action, row: Option<&AbsPath>) -> Step {
        dir_action(self.ctx, &self.option, action, row)
    }

    fn reload(&mut self) -> Result<Vec<DirRow>> {
        let projects = load_projects(self.ctx.fs, &self.search_paths);
        project_dir_rows(self.ctx, &self.from, &projects)
    }
}

/// Enter and the actions of the history picker (architecture 6.4).
struct HistoryHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
    option: ProfileOpt,
}

impl PickerHandler<AbsPath> for HistoryHandler<'_, '_> {
    fn choose(&mut self, row: &AbsPath) -> Step {
        choose_dir(self.ctx, row)
    }

    fn act(&mut self, action: Action, row: Option<&AbsPath>) -> Step {
        dir_action(self.ctx, &self.option, action, row)
    }

    fn reload(&mut self) -> Result<Vec<DirRow>> {
        let history = load_history(self.ctx)?;
        dir_rows(self.ctx, &history)
    }
}

/// Enter on a row. An existing directory is accepted: the caller opens it after the terminal is restored. A
/// missing one is reported in the status line and the picker stays open (FR-PICK-09, A-27, FR-TUI-10).
fn choose_dir(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    if ctx.fs.is_dir(Path::new(dir.as_str())) {
        Step::Accept
    } else {
        missing_step(ctx, dir)
    }
}

/// FR-PICK-09: the directory is gone; the message is in the status line and the picker stays open.
fn missing_step(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    Step::Failed {
        message: messages::missing_directory(&dir.display_with_home(home_of(ctx).as_ref())),
        reload: false,
    }
}

/// P-59, FR-TUI-14: a key with no effect on the row says so in the status line; nothing else changes.
fn notice(text: &str) -> Step {
    Step::Continue {
        status: Some(text.to_string()),
        reload: false,
    }
}

/// FR-PICK-09, FR-TUI-08: the directory of a row is gone (its cue is the ` (missing)` text).
fn is_missing(ctx: &Ctx<'_>, dir: &AbsPath) -> bool {
    !ctx.fs.is_dir(Path::new(dir.as_str()))
}

/// An action with no behaviour: the status line is cleared and the picker stays open.
fn no_action() -> Step {
    Step::Continue {
        status: None,
        reload: false,
    }
}

/// A failed action: its message in the status line, the list kept as it was.
fn failed(error: LedgeError) -> Step {
    Step::Failed {
        message: error.to_string(),
        reload: false,
    }
}

/// FR-ACT-01, FR-ACT-02, FR-ACT-04, FR-ACT-12, FR-ACT-13: the row actions of the directory pickers. `row` is `None`
/// on an empty list, which does nothing. Ctrl-p is answered by the picker loop (`Step::Profile`).
fn dir_action(ctx: &Ctx<'_>, option: &ProfileOpt, action: Action, row: Option<&AbsPath>) -> Step {
    let Some(dir) = row else {
        return no_action();
    };
    match action {
        Action::Rebuild => rebuild_row(ctx, dir, option),
        Action::Kill => kill_row(ctx, dir),
        Action::Profile => Step::Profile,
        _ => no_action(),
    }
}

/// FR-ACT-02, FR-ACT-12, FR-PICK-09: ctrl-r rebuilds the session of the row, or creates it when it is missing, with
/// the picker's profile option. A directory that is gone is reported as an enter on it is.
fn rebuild_row(ctx: &Ctx<'_>, dir: &AbsPath, option: &ProfileOpt) -> Step {
    if !ctx.fs.is_dir(Path::new(dir.as_str())) {
        return missing_step(ctx, dir);
    }
    match rebuild(ctx, dir, option) {
        Ok(_) => Step::Continue {
            status: None,
            reload: true,
        },
        Err(error) => failed(error),
    }
}

/// FR-ACT-01, FR-ACT-13, FR-ACT-07, FR-SESS-12, P-63: ctrl-x kills the managed session of the row and the list is shown
/// again. The session the user is in is killed too, after the client switches to another session. A row without a
/// managed session has nothing to kill: the status line says so (P-59).
fn kill_row(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    let sessions = match live_sessions(ctx) {
        Ok(sessions) => sessions,
        Err(error) => return failed(error),
    };
    if live_session(&sessions, dir).is_none() {
        return notice(messages::NO_SESSION_ON_ROW);
    }
    match kill(ctx, &session_name(dir)) {
        Ok(_) => Step::Continue {
            status: None,
            reload: true,
        },
        Err(error) => failed(error),
    }
}

/// FR-PROF-13, FR-ACT-04, FR-ACT-12, A-08: ctrl-p. The nested profile picker lists the profile of the session of `dir`
/// first (when the session exists), then the default profile, then the others and basic by name. Enter rebuilds the
/// session with the chosen profile; a failure is its message in the status line. Escape changes nothing.
fn profile_then_rebuild(ctx: &Ctx<'_>, dir: &AbsPath) -> Result<Step> {
    let sessions = live_sessions(ctx)?;
    let current = live_session(&sessions, dir).and_then(|info| info.profile.clone());
    match pick_profile(ctx, current.as_deref())? {
        PickerResult::Chosen(name) => Ok(match rebuild(ctx, dir, &ProfileOpt::Named(name)) {
            Ok(_) => Step::Continue {
                status: None,
                reload: true,
            },
            Err(error) => failed(error),
        }),
        PickerResult::Cancelled | PickerResult::Profile(_) | PickerResult::ProfileNode(_) => {
            Ok(no_action())
        }
        PickerResult::Closed(outcome) => Ok(Step::Close(outcome)),
    }
}

/// Runs a directory picker and opens the chosen directory (FR-PICK-08, FR-HIST-04). The open runs after the
/// picker has restored the terminal, so an attach works on a normal terminal (architecture 6.5). A ctrl-p request
/// runs the profile picker for the row, then the directory picker is opened again with its rows and the row selected.
fn run_dir_picker(
    ctx: &Ctx<'_>,
    mut model: PickerModel<AbsPath>,
    handler: &mut dyn PickerHandler<AbsPath>,
    option: &ProfileOpt,
    editor: Option<&Editor>,
) -> Result<Outcome> {
    loop {
        let chrome = model.chrome().clone();
        match run_in_terminal(ctx, model, handler)? {
            PickerResult::Chosen(dir) => {
                return open_dir_with_editor(ctx, Some(dir.as_str()), option, editor)
            }
            PickerResult::Cancelled | PickerResult::ProfileNode(_) => {
                return Ok(Outcome::Cancelled)
            }
            PickerResult::Closed(outcome) => return Ok(outcome),
            PickerResult::Profile(dir) => {
                let (status, error) = match profile_then_rebuild(ctx, &dir)? {
                    Step::Close(outcome) => return Ok(outcome),
                    Step::Continue { status, .. } => (status, false),
                    Step::Failed { message, .. } => (Some(message), true),
                    Step::Accept | Step::Profile | Step::ProfileNode | Step::ToParent => {
                        (None, false)
                    }
                };
                let rows = preselect_row(handler.reload()?, &dir);
                model = PickerModel::new(rows, chrome);
                set_flow_status(&mut model, status, error);
            }
        }
    }
}

/// Opens one picker on a fresh terminal and runs it. The terminal is restored when the picker returns (FR-TUI-06).
/// The theme comes from the config (P-60); its warnings go to stderr before the picker opens.
pub(crate) fn run_in_terminal<T: Clone>(
    ctx: &Ctx<'_>,
    model: PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
) -> Result<PickerResult<T>> {
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let color = render::color_enabled(
        ctx.env.var("NO_COLOR").as_deref(),
        ctx.env.var("TERM").as_deref(),
    );
    let level = ColourLevel::detect(
        ctx.env.var("COLORTERM").as_deref(),
        ctx.env.var("TERM").as_deref(),
    );
    let resolved = theme::resolve(config.theme.as_deref(), &config.theme_colors, level);
    for warning in config.warnings.iter().chain(&resolved.warnings) {
        ctx.console.err(warning);
    }
    let term = ctx.terminal.open().map_err(LedgeError::Fatal)?;
    run_picker(model, handler, term, ctx.signals, &resolved.theme, color)
}

/// The status line after a nested flow: an error keeps its error style (design 4.5).
fn set_flow_status<T>(model: &mut PickerModel<T>, status: Option<String>, error: bool) {
    match status {
        Some(text) if error => model.set_error(text),
        status => model.set_status(status),
    }
}

/// Selects the row of `dir` again after a nested picker (the selection returns to the row that was acted on).
fn preselect_row(mut rows: Vec<DirRow>, dir: &AbsPath) -> Vec<DirRow> {
    for row in rows.iter_mut() {
        row.preselect = row.payload.as_str() == dir.as_str();
    }
    rows
}

/// FR-PROF-13, A-11: the nested profile picker. `current` is the profile of the session when there is one; the rows
/// follow `profile_choices`. The result is the picker's, so the caller decides what the choice rebuilds.
fn pick_profile(ctx: &Ctx<'_>, current: Option<&str>) -> Result<PickerResult<String>> {
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let files = store::profile_files(ctx.fs, &ctx.dirs);
    let choices = profile_choices(&files, current, config.default_profile.as_deref());
    let home = home_of(ctx);
    let rows: Vec<PickerRow<String>> = choices
        .iter()
        .map(|choice| {
            let mut row = profile_row(choice);
            row.preview = Some(profile_preview(ctx, &choice.name, home.as_ref()));
            row
        })
        .collect();
    let model = PickerModel::new(rows.clone(), profile_chrome());
    let mut handler = ProfileHandler { rows };
    run_in_terminal(ctx, model, &mut handler)
}

/// Enter and the keys of the profile picker. Ctrl and alt keys do nothing here, so the picker is never nested.
struct ProfileHandler {
    rows: Vec<PickerRow<String>>,
}

impl PickerHandler<String> for ProfileHandler {
    fn choose(&mut self, _row: &String) -> Step {
        Step::Accept
    }

    fn act(&mut self, _action: Action, _row: Option<&String>) -> Step {
        no_action()
    }

    fn reload(&mut self) -> Result<Vec<PickerRow<String>>> {
        Ok(self.rows.clone())
    }
}

/// FR-PROF-13: one row of the profile picker. The current and default labels come first; a profile with both labels
/// is one row (A-11). The search matches the profile name.
fn profile_row(choice: &ProfileChoice) -> PickerRow<String> {
    // A profile that is both current and default shows only the current label.
    let text = if choice.current {
        messages::profile_current(&choice.name)
    } else if choice.default {
        messages::profile_default(&choice.name)
    } else {
        choice.name.clone()
    };
    PickerRow::new(vec![text], &choice.name, choice.name.clone())
}

#[cfg(test)]
mod profile_label_tests {
    use super::*;
    use crate::domain::profile_rules::ProfileChoice;

    fn label(current: bool, default: bool) -> String {
        let choice = ProfileChoice {
            name: "dev".to_string(),
            current,
            default,
        };
        profile_row(&choice).cells[0].clone()
    }

    // FR-PROF-13: a profile that is current and default shows only the current label
    #[test]
    fn current_and_default_shows_only_current() {
        assert_eq!(label(true, true), "[Current: dev]");
    }

    // FR-PROF-13: a default that is not current shows the default label
    #[test]
    fn default_alone_shows_default() {
        assert_eq!(label(false, true), "[Default: dev]");
    }

    // FR-PROF-13: a current profile that is not the default shows the current label
    #[test]
    fn current_alone_shows_current() {
        assert_eq!(label(true, false), "[Current: dev]");
    }

    // FR-PROF-13: any other profile shows its bare name
    #[test]
    fn neither_shows_the_name() {
        assert_eq!(label(false, false), "dev");
    }
}

/// FR-PROF-13, P-64: the preview of profile `name`. `basic` is built in and has no file, so its source is the built-in
/// text. Another profile's source is its file with the home folder as `~`; its windows are its loaded profile, or the
/// error that keeps it from loading.
fn profile_preview(ctx: &Ctx<'_>, name: &str, home: Option<&AbsPath>) -> Preview {
    let is_basic = ProfileName::parse(name).is_some_and(|profile| profile.is_basic());
    let source = if is_basic {
        messages::BUILT_IN.to_string()
    } else {
        let path = store::profile_path(&ctx.dirs, name);
        AbsPath::clean(&path.display().to_string(), &AbsPath::root()).display_with_home(home)
    };
    match store::load_profile(ctx.fs, &ctx.dirs, name) {
        Ok(profile) => Preview {
            title: name.to_string(),
            source,
            windows: profile
                .windows
                .iter()
                .map(|window| PreviewWindow {
                    name: window.name.clone(),
                    command: window.command.clone().unwrap_or_default(),
                })
                .collect(),
            error: None,
        },
        Err(LedgeError::Fatal(text)) => Preview {
            title: name.to_string(),
            source,
            windows: Vec::new(),
            error: Some(text),
        },
    }
}

/// Architecture 6.2: the profile picker chrome. Its rows carry a preview (FR-PROF-13, P-64).
fn profile_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        messages::PROFILES_LABEL,
        vec![ColumnSpec::text(messages::HEADER_PROFILE)],
        messages::PROFILE_PICKER_FOOTER,
    );
    chrome.preview = true;
    chrome
}

/// FR-PICK-13, D-13: the name of the non-basic profile the open uses, or `None` for the basic layout.
fn applied_profile(option: &ProfileOpt, default_profile: Option<&str>) -> Option<String> {
    let name = match open_profile(option, default_profile) {
        OpenProfile::Basic => return None,
        OpenProfile::Requested(name) | OpenProfile::Default(name) => name,
    };
    ProfileName::parse(&name)
        .filter(|profile| !profile.is_basic())
        .map(|_| name)
}

/// FR-PICK-04..07, P-61: one row per project (and the launch directory, FR-PICK-05). The project picker has no marker
/// column and no mark: a live session shows in the session cell. The launch directory's row is preselected.
fn project_dir_rows(ctx: &Ctx<'_>, from: &AbsPath, projects: &[Project]) -> Result<Vec<DirRow>> {
    let sessions = live_sessions(ctx)?;
    let home = home_of(ctx);
    let rows = project_rows(from, projects)
        .into_iter()
        .map(|row| {
            let live = live_session(&sessions, &row.path);
            let shown = if row.project {
                basename(&row.path)
            } else {
                row.path.display_with_home(home.as_ref())
            };
            let path = row.path.display_with_home(home.as_ref());
            let cells = vec![
                shown.clone(),
                session_text(live),
                profile_text(live),
                path.clone(),
            ];
            let mut picker_row = PickerRow::new(cells, &shown, row.path.clone());
            picker_row.missing = is_missing(ctx, &row.path);
            picker_row.dimmed = picker_row.missing;
            picker_row.session_cell = live_cell(live);
            picker_row.preselect = row.current;
            picker_row.details = Some(Details {
                title: shown,
                path,
                session: session_fact(live),
                windows: windows_fact(live),
                profile: profile_text(live),
            });
            picker_row
        })
        .collect();
    Ok(rows)
}

/// FR-HIST-04, FR-BM-11, FR-PICK-06, FR-PICK-07: one row per directory, in the order given (the history, newest first;
/// the bookmarks, in stored order). Missing directories are dimmed.
fn dir_rows(ctx: &Ctx<'_>, history: &[AbsPath]) -> Result<Vec<DirRow>> {
    let sessions = live_sessions(ctx)?;
    let home = home_of(ctx);
    Ok(history
        .iter()
        .map(|dir| {
            let live = live_session(&sessions, dir);
            let shown = dir.display_with_home(home.as_ref());
            let cells = vec![shown.clone(), session_text(live), profile_text(live)];
            let mut picker_row = PickerRow::new(cells, &shown, dir.clone());
            picker_row.missing = is_missing(ctx, dir);
            picker_row.dimmed = picker_row.missing;
            picker_row.session_cell = live_cell(live);
            picker_row
        })
        .collect())
}

/// FR-HIST-01, FR-HIST-05: the entries of the history file, newest first. The caller has checked that the file
/// exists, so a missing file cannot reach here as an error.
fn load_history(ctx: &Ctx<'_>) -> Result<Vec<AbsPath>> {
    let history = store::load_history(ctx.fs, &ctx.dirs)
        .map_err(|error| LedgeError::Fatal(messages::could_not_read_history(&error.to_string())))?;
    Ok(history.entries().to_vec())
}

/// FR-PICK-06, FR-PICK-07, FR-SESS-01 (managed sessions only, FR-NAME-04): the sessions the pickers mark.
pub(crate) fn live_sessions(ctx: &Ctx<'_>) -> Result<Vec<ManagedSession>> {
    ctx.backend
        .list_sessions()
        .map(managed)
        .map_err(backend_fatal)
}

/// FR-PICK-07: the managed session of a directory, found by its name (D-14).
pub(crate) fn live_session<'s>(
    sessions: &'s [ManagedSession],
    dir: &AbsPath,
) -> Option<&'s SessionInfo> {
    let name = session_name(dir);
    sessions
        .iter()
        .map(ManagedSession::info)
        .find(|info| info.name == name)
}

pub(crate) fn session_text(live: Option<&SessionInfo>) -> String {
    live.map(|info| info.name.clone()).unwrap_or_default()
}

/// Design 6.1, 9 step 4b: a live session's name takes the `session_active` token; a row without one is plain.
pub(crate) fn live_cell(live: Option<&SessionInfo>) -> SessionCell {
    if live.is_some() {
        SessionCell::Live
    } else {
        SessionCell::Plain
    }
}

pub(crate) fn profile_text(live: Option<&SessionInfo>) -> String {
    live.and_then(|info| info.profile.clone())
        .unwrap_or_default()
}

/// Option B 2.1: the Session fact of the Details section, the live session's name or `(no session)`.
fn session_fact(live: Option<&SessionInfo>) -> String {
    live.map(|info| info.name.clone())
        .unwrap_or_else(|| messages::no_session().to_string())
}

/// Option B 2.1: the Windows fact of the Details section, the window count of the live session (`SessionInfo::windows`).
fn windows_fact(live: Option<&SessionInfo>) -> String {
    live.map(|info| info.windows.to_string())
        .unwrap_or_default()
}

/// FR-SESS-03, A-10, P-56: the activity text of a session at `now` (Unix seconds): the age text the switcher shows.
fn activity_text(now: i64, activity: i64) -> String {
    messages::activity_age(age(now, activity))
}

/// The last path component of a cleaned absolute path (FR-PICK-06).
fn basename(path: &AbsPath) -> String {
    path.as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// FR-SESS-03, P-55: the Project column of a session: the name of the project that contains `dir`, or `dir` with
/// the home folder written as `~` when it is outside every project.
fn project_text(projects: &[Project], dir: &AbsPath, home: Option<&AbsPath>) -> String {
    match project_containing(dir, projects) {
        Some(project) => project.name.clone(),
        None => dir.display_with_home(home),
    }
}

/// Architecture 6.2, P-61: the project picker chrome. No marker column: the search, the missing cue and the current
/// mark are on the project column (column 0), the live session is in the session column (column 1).
fn project_chrome(label: &str) -> Chrome {
    let mut chrome = Chrome::new(
        label,
        vec![
            ColumnSpec::path(messages::HEADER_PROJECT),
            ColumnSpec::name(messages::HEADER_SESSION),
            ColumnSpec::text(messages::HEADER_PROFILE),
            ColumnSpec::wide_path(messages::HEADER_PATH),
        ],
        messages::PICKER_FOOTER,
    );
    chrome.search_column = Some(0);
    chrome.missing_column = Some(0);
    chrome.session_column = Some(1);
    chrome
}

/// Architecture 6.2: the history picker chrome. No marker column; the search and the missing cue are on the
/// project column.
fn history_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        messages::HISTORY_LABEL,
        vec![
            ColumnSpec::path(messages::HEADER_PROJECT),
            ColumnSpec::name(messages::HEADER_SESSION),
            ColumnSpec::text(messages::HEADER_PROFILE),
        ],
        messages::PICKER_FOOTER,
    );
    chrome.missing_column = Some(0);
    chrome.session_column = Some(1);
    chrome
}

/// Architecture 6.2: the bookmark picker chrome. The search is on the path shown (column 0), which carries the missing cue.
fn bookmark_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        messages::BOOKMARKS_LABEL,
        vec![
            ColumnSpec::path(messages::HEADER_BOOKMARK),
            ColumnSpec::name(messages::HEADER_SESSION),
            ColumnSpec::text(messages::HEADER_PROFILE),
        ],
        messages::BOOKMARK_PICKER_FOOTER,
    );
    chrome.missing_column = Some(0);
    chrome.session_column = Some(1);
    chrome
}

/// FR-SESS-01..04, D-07, D-08, D-12, S-16: runs the session switcher over the managed sessions, the session the user
/// is in first and then by activity (FR-SESS-02). Enter switches the client inside tmux and attaches outside
/// (FR-SESS-04). ctrl-r rebuilds the session in its directory, ctrl-p picks a profile first, ctrl-x kills and the list
/// stays open. An empty list shows `No ledge sessions.` and stays open (D-12).
pub fn session_switcher(ctx: &Ctx<'_>) -> Result<Outcome> {
    let rows = switcher_rows(ctx)?;
    let model = PickerModel::new(rows, switcher_chrome());
    let mut handler = SwitcherHandler { ctx };
    run_switcher(ctx, model, &mut handler)
}

/// Enter and the actions of the session switcher. The payload is the session name.
struct SwitcherHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
}

impl PickerHandler<String> for SwitcherHandler<'_, '_> {
    fn choose(&mut self, _name: &String) -> Step {
        Step::Accept
    }

    fn act(&mut self, action: Action, row: Option<&String>) -> Step {
        let Some(name) = row else {
            return no_action();
        };
        match action {
            Action::Rebuild => switcher_rebuild(self.ctx, name),
            Action::Kill => switcher_kill(self.ctx, name),
            Action::Profile => Step::Profile,
            _ => no_action(),
        }
    }

    fn reload(&mut self) -> Result<Vec<PickerRow<String>>> {
        switcher_rows(self.ctx)
    }
}

/// FR-SESS-02..04, D-07, P-55: one row per managed session in switcher order. The columns are session, project (the
/// Project column of `project_text`; empty when the session has no directory), profile and activity age
/// (FR-SESS-03). A session whose directory is gone is dimmed (FR-TUI-08).
fn switcher_rows(ctx: &Ctx<'_>) -> Result<Vec<PickerRow<String>>> {
    let sessions = live_sessions(ctx)?;
    let current = current_session(ctx)?;
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let projects = load_projects(ctx.fs, &config.search_paths);
    let home = home_of(ctx);
    let now = ctx.clock.now_secs();
    let rows = switcher_order(&sessions, current.as_deref())
        .into_iter()
        .map(|info| {
            let dir = session_directory(info).map(|text| AbsPath::clean(text, &ctx.cwd));
            let project = dir
                .as_ref()
                .map(|dir| project_text(&projects, dir, home.as_ref()))
                .unwrap_or_default();
            let path = dir
                .as_ref()
                .map(|dir| dir.display_with_home(home.as_ref()))
                .unwrap_or_default();
            let cells = vec![
                info.name.clone(),
                project,
                info.profile.clone().unwrap_or_default(),
                activity_text(now, info.activity),
                path.clone(),
            ];
            let mut row = PickerRow::new(cells, &info.name, info.name.clone());
            row.missing = dir.as_ref().is_some_and(|dir| is_missing(ctx, dir));
            row.dimmed = row.missing;
            row.session_cell = SessionCell::Live;
            row.details = Some(Details {
                title: info.name.clone(),
                path,
                session: info.name.clone(),
                windows: info.windows.to_string(),
                profile: info.profile.clone().unwrap_or_default(),
            });
            row
        })
        .collect();
    Ok(rows)
}

/// Architecture 6.2: the switcher chrome. The empty list shows `No ledge sessions.` and stays open (D-12).
fn switcher_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        messages::SESSIONS_LABEL,
        vec![
            ColumnSpec::name(messages::HEADER_SESSION),
            ColumnSpec::project(messages::HEADER_PROJECT),
            ColumnSpec::text(messages::HEADER_PROFILE),
            ColumnSpec::text(messages::HEADER_ACTIVITY),
            ColumnSpec::wide_path(messages::HEADER_PATH),
        ],
        messages::SESSION_SWITCHER_FOOTER,
    );
    chrome.missing_column = Some(1);
    chrome.session_column = Some(0);
    chrome.empty_footer = Some(messages::EMPTY_WINDOW_FOOTER.to_string());
    chrome.empty_message = Some(messages::no_ledge_sessions().to_string());
    chrome.stay_open_when_empty = true;
    chrome
}

/// The directory of the managed session `name`, cleaned against the working directory. `None` when the session is
/// gone or has no recorded or tmux directory.
fn session_directory_of(ctx: &Ctx<'_>, name: &str) -> Result<Option<AbsPath>> {
    let sessions = live_sessions(ctx)?;
    Ok(sessions
        .iter()
        .map(ManagedSession::info)
        .find(|info| info.name == name)
        .and_then(session_directory)
        .map(|text| AbsPath::clean(text, &ctx.cwd)))
}

/// FR-ACT-02, FR-SESS-04 (ctrl-r): rebuilds the session in its directory with its recorded profile. A directory that is
/// gone is reported as the directory pickers report it (FR-PICK-09).
fn switcher_rebuild(ctx: &Ctx<'_>, name: &str) -> Step {
    match session_directory_of(ctx, name) {
        Ok(Some(dir)) => rebuild_row(ctx, &dir, &ProfileOpt::Unset),
        Ok(None) => no_action(),
        Err(error) => failed(error),
    }
}

/// FR-ACT-01, FR-ACT-07, FR-SESS-12, P-63: ctrl-x kills the session and the list is shown again. The session the user is
/// in is killed after the client switches to the most recent other session.
fn switcher_kill(ctx: &Ctx<'_>, name: &str) -> Step {
    match kill(ctx, name) {
        Ok(_) => Step::Continue {
            status: None,
            reload: true,
        },
        Err(error) => failed(error),
    }
}

/// Runs the switcher. Enter returns the session name after the terminal is restored; the attach or switch runs then
/// (FR-SESS-04). ctrl-p runs the profile picker for the session and reopens the switcher with that row selected.
fn run_switcher(
    ctx: &Ctx<'_>,
    mut model: PickerModel<String>,
    handler: &mut dyn PickerHandler<String>,
) -> Result<Outcome> {
    loop {
        let chrome = model.chrome().clone();
        match run_in_terminal(ctx, model, handler)? {
            PickerResult::Chosen(name) => return attach_or_switch(ctx, &name),
            PickerResult::Cancelled | PickerResult::ProfileNode(_) => {
                return Ok(Outcome::Cancelled)
            }
            PickerResult::Closed(outcome) => return Ok(outcome),
            PickerResult::Profile(name) => {
                let step = match session_directory_of(ctx, &name)? {
                    Some(dir) => profile_then_rebuild(ctx, &dir)?,
                    None => no_action(),
                };
                let (status, error) = match step {
                    Step::Close(outcome) => return Ok(outcome),
                    Step::Continue { status, .. } => (status, false),
                    Step::Failed { message, .. } => (Some(message), true),
                    Step::Accept | Step::Profile | Step::ProfileNode | Step::ToParent => {
                        (None, false)
                    }
                };
                let rows = preselect_session(handler.reload()?, &name);
                model = PickerModel::new(rows, chrome);
                set_flow_status(&mut model, status, error);
            }
        }
    }
}

/// Selects the row of the session `name` again after a nested profile picker.
fn preselect_session(mut rows: Vec<PickerRow<String>>, name: &str) -> Vec<PickerRow<String>> {
    for row in rows.iter_mut() {
        row.preselect = row.payload == name;
    }
    rows
}

/// Which tree a tree window shows (architecture 6.2): the bookmark tree or the session tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeKind {
    Bookmarks,
    Sessions,
}

/// One row of a tree window: the tree row (path, kind, openable, session) is the payload.
type TreeItem = PickerRow<TreeRow<NodeKind>>;

/// FR-SESS-06..08, FR-TREE-11, FR-TREE-12, FR-ACT-06..09, D-12, S-22: runs the session tree over the managed sessions.
/// Enter opens an openable entry; the alt keys act on the whole node. The window stays open when its list is empty.
pub fn session_tree_picker(ctx: &Ctx<'_>, option: &ProfileOpt) -> Result<Outcome> {
    run_tree_window(ctx, option, TreeKind::Sessions, &TreeScope::All)
}

/// FR-TREE-05..08, FR-TREE-11, FR-TREE-12, FR-ACT-06..10, D-12, S-22, S-24: runs the bookmark tree for `scope`. The
/// cursor starts on the nearest node of the scope. Enter opens an openable entry; alt-d deletes the bookmarks under
/// the node. The window stays open when its list is empty.
pub fn bookmark_tree_picker(
    ctx: &Ctx<'_>,
    option: &ProfileOpt,
    scope: &TreeScope,
) -> Result<Outcome> {
    run_tree_window(ctx, option, TreeKind::Bookmarks, scope)
}

/// Runs a tree window. Enter opens the chosen entry after the terminal is restored; ctrl-p and alt-p ask a profile
/// and reopen the window with the row selected.
fn run_tree_window(
    ctx: &Ctx<'_>,
    option: &ProfileOpt,
    kind: TreeKind,
    scope: &TreeScope,
) -> Result<Outcome> {
    let (mut rows, cursor) = tree_rows(ctx, kind, scope)?;
    if let Some(row) = rows.get_mut(cursor) {
        row.preselect = true;
    }
    let mut model = PickerModel::new(rows, tree_chrome(kind));
    let mut handler = TreeHandler {
        ctx,
        option: option.clone(),
        kind,
        scope: scope.clone(),
    };
    loop {
        let chrome = model.chrome().clone();
        match run_in_terminal(ctx, model, &mut handler)? {
            PickerResult::Chosen(row) => {
                return open_dir_with_editor(ctx, Some(row.path.as_str()), option, None)
            }
            PickerResult::Cancelled => return Ok(Outcome::Cancelled),
            PickerResult::Closed(outcome) => return Ok(outcome),
            PickerResult::Profile(row) => {
                let (status, error) = match profile_then_rebuild(ctx, &row.path)? {
                    Step::Close(outcome) => return Ok(outcome),
                    Step::Continue { status, .. } => (status, false),
                    Step::Failed { message, .. } => (Some(message), true),
                    Step::Accept | Step::Profile | Step::ProfileNode | Step::ToParent => {
                        (None, false)
                    }
                };
                let rows = preselect_tree_row(handler.reload()?, &row.path);
                model = PickerModel::new(rows, chrome);
                set_flow_status(&mut model, status, error);
            }
            PickerResult::ProfileNode(row) => {
                // FR-ACT-09: one profile for every session under the node; escape changes nothing.
                let status = match pick_profile(ctx, None)? {
                    PickerResult::Chosen(name) => {
                        let result = rebuild_node(ctx, &row.path, &ProfileOpt::Named(name));
                        result.err().map(|error| error.to_string())
                    }
                    PickerResult::Closed(outcome) => return Ok(outcome),
                    PickerResult::Cancelled
                    | PickerResult::Profile(_)
                    | PickerResult::ProfileNode(_) => None,
                };
                let rows = preselect_tree_row(handler.reload()?, &row.path);
                model = PickerModel::new(rows, chrome);
                set_flow_status(&mut model, status, true);
            }
        }
    }
}

/// Architecture 6.2: the tree window chrome. The empty list says so and stays open (D-12, FR-TREE-12).
fn tree_chrome(kind: TreeKind) -> Chrome {
    let (label, footer, empty) = match kind {
        TreeKind::Bookmarks => (
            messages::BOOKMARK_TREE_LABEL,
            messages::BOOKMARK_TREE_FOOTER,
            messages::no_bookmarks(),
        ),
        TreeKind::Sessions => (
            messages::SESSION_TREE_LABEL,
            messages::SESSION_TREE_FOOTER,
            messages::no_ledge_sessions(),
        ),
    };
    // FR-SESS-06, P-54, P-55, P-56: the session tree shows the session first and draws its tree on that column; the
    // Project column comes after it, then the profile and the activity; the filter searches the value shown in Project.
    let (columns, tree_column, search_column) = match kind {
        TreeKind::Bookmarks => (
            vec![
                ColumnSpec::text(messages::HEADER_DIRECTORY),
                ColumnSpec::name(messages::HEADER_SESSION),
                ColumnSpec::text(messages::HEADER_PROFILE),
                ColumnSpec::wide_path(messages::HEADER_PATH),
            ],
            0,
            0,
        ),
        TreeKind::Sessions => (
            vec![
                ColumnSpec::name(messages::HEADER_SESSION),
                ColumnSpec::project(messages::HEADER_PROJECT),
                ColumnSpec::text(messages::HEADER_PROFILE),
                ColumnSpec::text(messages::HEADER_ACTIVITY),
            ],
            0,
            1,
        ),
    };
    let mut chrome = Chrome::new(label, columns, footer);
    chrome.tree_column = Some(tree_column);
    chrome.search_column = Some(search_column);
    // FR-TUI-08, design 3.2: the Directory column of the bookmark tree and the Project column of the session tree.
    chrome.missing_column = Some(match kind {
        TreeKind::Bookmarks => 0,
        TreeKind::Sessions => 1,
    });
    chrome.session_column = Some(match kind {
        TreeKind::Bookmarks => 1,
        TreeKind::Sessions => 0,
    });
    chrome.empty_footer = Some(messages::EMPTY_WINDOW_FOOTER.to_string());
    chrome.empty_message = Some(empty.to_string());
    chrome.stay_open_when_empty = true;
    chrome
}

/// Rows of a tree window, built from the project list, the bookmarks or the managed sessions (A-13, A-37), and the
/// index of the row the cursor starts on. Only the managed sessions with a directory are placed in the session tree
/// (FR-SESS-06). The scope selects the bookmark rows (FR-TREE-05..08); the session tree is always whole.
fn tree_rows(ctx: &Ctx<'_>, kind: TreeKind, scope: &TreeScope) -> Result<(Vec<TreeItem>, usize)> {
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let projects = load_projects(ctx.fs, &config.search_paths);
    let sessions = live_sessions(ctx)?;
    let (rows, cursor) = match kind {
        TreeKind::Bookmarks => {
            let view = bookmark_view(&projects, &read_bookmarks(ctx)?, scope);
            (view.rows, view.cursor)
        }
        TreeKind::Sessions => (session_tree(&projects, &session_dirs(ctx, &sessions)), 0),
    };
    let home = home_of(ctx);
    let now = ctx.clock.now_secs();
    // Design section 7, 8 item 11: branch glyphs in the tree column, ASCII when the locale is not UTF-8.
    let depths: Vec<usize> = rows.iter().map(|row| row.depth).collect();
    let guides = render::tree_guides(&depths, utf8_locale(ctx.env));
    let mut items: Vec<TreeItem> = rows
        .into_iter()
        .map(|row| tree_item(ctx, &sessions, kind, &projects, home.as_ref(), now, row))
        .collect();
    for (item, guide) in items.iter_mut().zip(guides) {
        item.lead = Some(guide);
    }
    Ok((items, cursor))
}

/// Design 7: the locale is UTF-8 when the character set of `LC_ALL`, else `LC_CTYPE`, else `LANG` says so. Unset
/// counts as not UTF-8, so the ASCII branch glyphs are used.
pub(crate) fn utf8_locale(env: &dyn crate::ports::env::Env) -> bool {
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .filter_map(|name| env.var(name))
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase();
    locale.contains("utf-8") || locale.contains("utf8")
}

/// FR-SESS-06, A-37: the managed sessions with their directory (`@ledge_dir`, then tmux's path).
fn session_dirs(ctx: &Ctx<'_>, sessions: &[ManagedSession]) -> Vec<SessionDir> {
    sessions
        .iter()
        .filter_map(|managed| {
            let info = managed.info();
            session_directory(info).map(|text| SessionDir {
                name: info.name.clone(),
                dir: AbsPath::clean(text, &ctx.cwd),
                activity: info.activity,
            })
        })
        .collect()
}

/// One tree row as a picker row: the label is searched, the session and profile are shown, and a row that is not
/// openable is a gray connector (FR-TREE-11). A missing directory is dimmed (FR-TUI-08). In the bookmark tree the
/// session of a row is the managed session of its directory, found by name as the project picker does (FR-TREE-13,
/// FR-PICK-06); the session tree shows the session on its row (P-54), the Project column of `project_text` (P-55),
/// which the filter searches, and the activity age of the session (P-56). A placeholder row (a project without its
/// own session) carries its would-be name and `(no session)`, in gray, and no activity.
fn tree_item(
    ctx: &Ctx<'_>,
    sessions: &[ManagedSession],
    kind: TreeKind,
    projects: &[Project],
    home: Option<&AbsPath>,
    now: i64,
    mut row: TreeRow<NodeKind>,
) -> TreeItem {
    let name = match kind {
        TreeKind::Bookmarks => Some(session_name(&row.path)),
        TreeKind::Sessions => row.session.clone(),
    };
    let live = name.and_then(|name| {
        sessions
            .iter()
            .map(ManagedSession::info)
            .find(|info| info.name == name)
    });
    // The ctrl actions act on `row.session` (FR-TUI-07), so a bookmark row with a live session must carry its name.
    row.session = live.map(|info| info.name.clone());
    // The bookmark filter matches the label; the session filter matches the Project value shown.
    let (cells, search) = match kind {
        TreeKind::Bookmarks => {
            // A directory outside every project is labelled with its path (A-13), shown with the home folder as `~`.
            let label = if project_containing(&row.path, projects).is_none() {
                row.path.display_with_home(home)
            } else {
                row.label.clone()
            };
            // S-46: the Path column is the directory of the row, the home folder as `~`, shown at 120 cells and up.
            let path = row.path.display_with_home(home);
            (
                vec![label.clone(), session_text(live), profile_text(live), path],
                label,
            )
        }
        TreeKind::Sessions => {
            let (session, activity) = match row.kind {
                NodeKind::Placeholder => (
                    format!("{} {}", row.label, messages::no_session()),
                    String::new(),
                ),
                _ => (
                    session_text(live),
                    live.map(|info| activity_text(now, info.activity))
                        .unwrap_or_default(),
                ),
            };
            let project = project_text(projects, &row.path, home);
            let cells = vec![session, project.clone(), profile_text(live), activity];
            (cells, project)
        }
    };
    let depth = row.depth;
    let connector = !row.openable;
    let gray = kind == TreeKind::Sessions && row.dimmed;
    let missing = is_missing(ctx, &row.path);
    // The placeholder is gray but not missing: it has no session, not a directory that is gone.
    let session_cell = match (row.kind, live) {
        (NodeKind::Placeholder, _) => SessionCell::NoSession,
        (_, Some(_)) => SessionCell::Live,
        (_, None) => SessionCell::Plain,
    };
    let mut item = PickerRow::new(cells, &search, row);
    item.depth = depth;
    item.connector = connector;
    item.missing = missing;
    item.dimmed = gray || missing;
    item.session_cell = session_cell;
    item
}

/// Selects the tree row of `path` again after a nested picker (the first row with that path).
fn preselect_tree_row(mut rows: Vec<TreeItem>, path: &AbsPath) -> Vec<TreeItem> {
    for row in rows.iter_mut() {
        row.preselect = row.payload.path == *path;
    }
    rows
}

/// Enter, the ctrl keys on a row with a session, and the alt keys of the tree window (architecture 6.4).
struct TreeHandler<'c, 'a> {
    ctx: &'c Ctx<'a>,
    option: ProfileOpt,
    kind: TreeKind,
    scope: TreeScope,
}

impl PickerHandler<TreeRow<NodeKind>> for TreeHandler<'_, '_> {
    /// FR-TREE-11: enter opens an openable entry; a gray connector does nothing but say so (P-59).
    fn choose(&mut self, row: &TreeRow<NodeKind>) -> Step {
        if !row.openable {
            return notice(messages::NOTHING_TO_OPEN_ON_ROW);
        }
        choose_dir(self.ctx, &row.path)
    }

    fn act(&mut self, action: Action, row: Option<&TreeRow<NodeKind>>) -> Step {
        let Some(row) = row else {
            return no_action();
        };
        match action {
            // The ctrl keys act on the selected session only (FR-TUI-07, architecture 6.3). A row with no session
            // has nothing to act on: the status line says so (P-59, FR-TUI-14).
            Action::Rebuild | Action::Kill | Action::Profile if row.session.is_some() => {
                dir_action(self.ctx, &self.option, action, Some(&row.path))
            }
            Action::Rebuild | Action::Kill | Action::Profile => notice(messages::NO_SESSION_ON_ROW),
            // FR-ACT-08: alt-r ignores the --profile option, so each session keeps its recorded profile (A-38).
            Action::RebuildNode => node_step(rebuild_node(self.ctx, &row.path, &ProfileOpt::Unset)),
            Action::KillNode => node_step(kill_node(self.ctx, &row.path)),
            Action::ProfileNode => Step::ProfileNode,
            Action::DeleteNode if self.kind == TreeKind::Bookmarks => {
                node_step(delete_node_bookmarks(self.ctx, &row.path))
            }
            // FR-BM-13, FR-TUI-07: ctrl-d deletes the selected bookmark row, as in the bookmark picker.
            Action::DeleteBookmark
                if self.kind == TreeKind::Bookmarks && row.kind == NodeKind::Bookmark =>
            {
                delete_bookmark(self.ctx, &row.path)
            }
            // P-59: only a bookmark row can be deleted; the row is not changed.
            Action::DeleteBookmark if self.kind == TreeKind::Bookmarks => {
                notice(messages::ONLY_BOOKMARK_CAN_BE_DELETED)
            }
            _ => no_action(),
        }
    }

    fn reload(&mut self) -> Result<Vec<TreeItem>> {
        tree_rows(self.ctx, self.kind, &self.scope).map(|(rows, _)| rows)
    }
}

/// The step of a node action: the list is read again, and a failure is its message in the status line.
fn node_step(result: Result<()>) -> Step {
    match result {
        Ok(()) => Step::Continue {
            status: None,
            reload: true,
        },
        Err(error) => Step::Failed {
            message: error.to_string(),
            reload: true,
        },
    }
}

/// Architecture 4: a backend failure as the fatal text of the picker. A missing tmux keeps its own message
/// (FR-CMD-10).
pub(crate) fn backend_fatal(error: BackendError) -> LedgeError {
    match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_list_sessions(&detail))
        }
    }
}

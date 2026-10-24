//! The project and history pickers (architecture 2, 6.2, 6.4; FR-PICK-04..13, FR-HIST-04, FR-HIST-05, FR-CMD-08,
//! FR-CMD-12). Each entry point loads its rows through the ports, runs the picker, and on enter opens the chosen
//! directory through the open flow after the terminal is restored (architecture 6.4). Rows are built here; the
//! screen and the keys are `tui`. Row actions: ctrl-x kills, ctrl-r rebuilds, ctrl-p picks a profile in a nested
//! picker and rebuilds with it (S-15, FR-ACT-01, FR-ACT-02, FR-ACT-04, FR-ACT-12). Alt and ctrl-d actions are not
//! bound in these pickers.

use crate::app::actions::{kill, rebuild};
use crate::app::load_projects;
use crate::app::open::open_dir;
use crate::app::store;
use crate::app::Ctx;
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::ProfileName;
use crate::domain::profile_rules::{
    open_profile, profile_choices, OpenProfile, ProfileChoice, ProfileOpt,
};
use crate::domain::projects::{project_rows, Project};
use crate::domain::sessions::SessionInfo;
use crate::domain::sessions::{managed, ManagedSession};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;
use crate::tui::columns::ColumnSpec;
use crate::tui::engine::{run_picker, PickerHandler, PickerResult, Step};
use crate::tui::keys::Action;
use crate::tui::picker::{Chrome, PickerModel, PickerRow};
use crate::tui::render;
use std::path::Path;

/// One picker row: the directory it opens.
type DirRow = PickerRow<AbsPath>;

/// FR-PICK-04..10, FR-PICK-12, FR-PICK-13, FR-CMD-08, FR-CMD-12: runs the project picker and opens the chosen
/// project. `dir` is the directory the picker is launched from; its row comes first and is preselected (FR-PICK-04).
/// `None` means the working directory. The CLI rejects a directory argument on `project pick` (FR-PICK-11); this
/// entry point takes it as the launch directory so that callers and tests can choose it. `option` is the profile
/// option of the open (FR-PICK-12).
pub fn project_picker(ctx: &Ctx<'_>, dir: Option<&str>, option: &ProfileOpt) -> Result<Outcome> {
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
    run_dir_picker(ctx, model, &mut handler, option)
}

/// FR-HIST-04, FR-HIST-05, D-12: runs the history picker and opens the chosen directory. A missing history file
/// prints `No history cache found.`; an empty one prints `No history.`; both stop without a window.
pub fn history_picker(ctx: &Ctx<'_>, option: &ProfileOpt) -> Result<Outcome> {
    if !ctx.fs.exists(&store::history_path(&ctx.dirs)) {
        return Ok(Outcome::Info(messages::no_history_cache().to_string()));
    }
    let history = load_history(ctx)?;
    if history.is_empty() {
        return Ok(Outcome::Info(messages::no_history().to_string()));
    }
    let rows = history_dir_rows(ctx, &history)?;
    let model = PickerModel::new(rows, history_chrome());
    let mut handler = HistoryHandler {
        ctx,
        option: option.clone(),
    };
    run_dir_picker(ctx, model, &mut handler, option)
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
        history_dir_rows(self.ctx, &history)
    }
}

/// Enter on a row. An existing directory is accepted: the caller opens it after the terminal is restored. A
/// missing one is reported in the status line and the picker stays open (FR-PICK-09, A-27, FR-TUI-10).
fn choose_dir(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    if ctx.fs.is_dir(Path::new(dir.as_str())) {
        Step::Accept
    } else {
        missing_step(dir)
    }
}

/// FR-PICK-09: the directory is gone; the message is in the status line and the picker stays open.
fn missing_step(dir: &AbsPath) -> Step {
    Step::Continue {
        status: Some(messages::missing_directory(dir.as_str())),
        reload: false,
    }
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
    Step::Continue {
        status: Some(error.to_string()),
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
        return missing_step(dir);
    }
    match rebuild(ctx, dir, option) {
        Ok(_) => Step::Continue {
            status: None,
            reload: true,
        },
        Err(error) => failed(error),
    }
}

/// FR-ACT-01, FR-ACT-13, FR-ACT-07, D-17: ctrl-x kills the managed session of the row and the list is shown again.
/// A row without a managed session has nothing to kill. The refusal for the session the user is in is the status line.
fn kill_row(ctx: &Ctx<'_>, dir: &AbsPath) -> Step {
    let sessions = match live_sessions(ctx) {
        Ok(sessions) => sessions,
        Err(error) => return failed(error),
    };
    if live_session(&sessions, dir).is_none() {
        return no_action();
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
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    let files = store::profile_files(ctx.fs, &ctx.dirs);
    let choices = profile_choices(
        &files,
        current.as_deref(),
        config.default_profile.as_deref(),
    );
    let rows: Vec<PickerRow<String>> = choices.iter().map(profile_row).collect();
    let model = PickerModel::new(rows.clone(), profile_chrome());
    let mut handler = ProfileHandler { rows };
    match run_in_terminal(ctx, model, &mut handler)? {
        PickerResult::Chosen(name) => Ok(match rebuild(ctx, dir, &ProfileOpt::Named(name)) {
            Ok(_) => Step::Continue {
                status: None,
                reload: true,
            },
            Err(error) => failed(error),
        }),
        PickerResult::Cancelled | PickerResult::Profile(_) => Ok(no_action()),
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
) -> Result<Outcome> {
    loop {
        let chrome = model.chrome().clone();
        match run_in_terminal(ctx, model, handler)? {
            PickerResult::Chosen(dir) => return open_dir(ctx, Some(dir.as_str()), option),
            PickerResult::Cancelled => return Ok(Outcome::Cancelled),
            PickerResult::Closed(outcome) => return Ok(outcome),
            PickerResult::Profile(dir) => {
                let status = match profile_then_rebuild(ctx, &dir)? {
                    Step::Close(outcome) => return Ok(outcome),
                    Step::Continue { status, .. } => status,
                    Step::Accept | Step::Profile => None,
                };
                let rows = preselect_row(handler.reload()?, &dir);
                model = PickerModel::new(rows, chrome);
                model.set_status(status);
            }
        }
    }
}

/// Opens one picker on a fresh terminal and runs it. The terminal is restored when the picker returns (FR-TUI-06).
fn run_in_terminal<T: Clone>(
    ctx: &Ctx<'_>,
    model: PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
) -> Result<PickerResult<T>> {
    let term = ctx.terminal.open().map_err(LedgeError::Fatal)?;
    let color = render::color_enabled(
        ctx.env.var("NO_COLOR").as_deref(),
        ctx.env.var("TERM").as_deref(),
    );
    run_picker(model, handler, term, ctx.signals, color)
}

/// Selects the row of `dir` again after a nested picker (the selection returns to the row that was acted on).
fn preselect_row(mut rows: Vec<DirRow>, dir: &AbsPath) -> Vec<DirRow> {
    for row in rows.iter_mut() {
        row.preselect = row.payload.as_str() == dir.as_str();
    }
    rows
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
    let mut labels = Vec::new();
    if choice.current {
        labels.push(messages::profile_current(&choice.name));
    }
    if choice.default {
        labels.push(messages::profile_default(&choice.name));
    }
    let text = if labels.is_empty() {
        choice.name.clone()
    } else {
        labels.join(" ")
    };
    PickerRow::new(vec![text], &choice.name, choice.name.clone())
}

/// Architecture 6.2: the profile picker chrome.
fn profile_chrome() -> Chrome {
    Chrome::new(
        messages::PROFILES_LABEL,
        render::icon::PROFILES,
        vec![ColumnSpec::text(messages::HEADER_PROFILE)],
        messages::PROFILE_PICKER_FOOTER,
    )
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

/// FR-PICK-04..07: one row per project (and the launch directory, FR-PICK-05). The marker cell is the
/// current-directory flag then the live flag (architecture 6.2). The launch directory's row is preselected.
fn project_dir_rows(ctx: &Ctx<'_>, from: &AbsPath, projects: &[Project]) -> Result<Vec<DirRow>> {
    let sessions = live_sessions(ctx)?;
    let rows = project_rows(from, projects)
        .into_iter()
        .map(|row| {
            let live = live_session(&sessions, &row.path);
            let marker = format!(
                "{}{}",
                if row.current { "*" } else { " " },
                if live.is_some() { "●" } else { "○" }
            );
            let shown = if row.project {
                basename(&row.path)
            } else {
                row.path.as_str().to_string()
            };
            let cells = vec![
                marker,
                shown.clone(),
                session_text(live),
                profile_text(live),
            ];
            let mut picker_row = PickerRow::new(cells, &shown, row.path.clone());
            picker_row.dimmed = !ctx.fs.is_dir(Path::new(row.path.as_str()));
            picker_row.preselect = row.current;
            picker_row
        })
        .collect();
    Ok(rows)
}

/// FR-HIST-04, FR-PICK-06, FR-PICK-07: one row per history entry, newest first (the order of the history).
fn history_dir_rows(ctx: &Ctx<'_>, history: &[AbsPath]) -> Result<Vec<DirRow>> {
    let sessions = live_sessions(ctx)?;
    Ok(history
        .iter()
        .map(|dir| {
            let live = live_session(&sessions, dir);
            let cells = vec![
                dir.as_str().to_string(),
                session_text(live),
                profile_text(live),
            ];
            let mut picker_row = PickerRow::new(cells, dir.as_str(), dir.clone());
            picker_row.dimmed = !ctx.fs.is_dir(Path::new(dir.as_str()));
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
fn live_sessions(ctx: &Ctx<'_>) -> Result<Vec<ManagedSession>> {
    ctx.backend
        .list_sessions()
        .map(managed)
        .map_err(backend_fatal)
}

/// FR-PICK-07: the managed session of a directory, found by its name (D-14).
fn live_session<'s>(sessions: &'s [ManagedSession], dir: &AbsPath) -> Option<&'s SessionInfo> {
    let name = session_name(dir);
    sessions
        .iter()
        .map(ManagedSession::info)
        .find(|info| info.name == name)
}

fn session_text(live: Option<&SessionInfo>) -> String {
    live.map(|info| info.name.clone()).unwrap_or_default()
}

fn profile_text(live: Option<&SessionInfo>) -> String {
    live.and_then(|info| info.profile.clone())
        .unwrap_or_default()
}

/// The last path component of a cleaned absolute path (FR-PICK-06).
fn basename(path: &AbsPath) -> String {
    path.as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Architecture 6.2: the project picker chrome. The search and the missing cue are on the project column.
fn project_chrome(label: &str) -> Chrome {
    let mut chrome = Chrome::new(
        label,
        render::icon::PROJECTS,
        vec![
            ColumnSpec::fixed("", 2),
            ColumnSpec::path(messages::HEADER_PROJECT),
            ColumnSpec::text(messages::HEADER_SESSION),
            ColumnSpec::text(messages::HEADER_PROFILE),
        ],
        messages::PICKER_FOOTER,
    );
    chrome.search_column = Some(1);
    chrome.missing_column = Some(1);
    chrome
}

/// Architecture 6.2: the history picker chrome. No marker column; the search and the missing cue are on the
/// project column.
fn history_chrome() -> Chrome {
    let mut chrome = Chrome::new(
        messages::HISTORY_LABEL,
        render::icon::HISTORY,
        vec![
            ColumnSpec::path(messages::HEADER_PROJECT),
            ColumnSpec::text(messages::HEADER_SESSION),
            ColumnSpec::text(messages::HEADER_PROFILE),
        ],
        messages::PICKER_FOOTER,
    );
    chrome.missing_column = Some(0);
    chrome
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

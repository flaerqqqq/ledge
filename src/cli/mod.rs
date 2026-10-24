//! Command layer (architecture 2, 2.3). `run` parses the arguments (`parse`), checks tmux for the commands that
//! need it (A-29, FR-CMD-10, FR-PLAT-01), and calls the use cases in `app` through `Ctx`. Help text is in `help`.
//! Commands whose slice is not built yet fail with the plain error of proposal P-32.

pub mod help;
pub mod parse;

use crate::app::bookmark_cmd;
use crate::app::doctor;
use crate::app::history_cmd;
use crate::app::open::open_dir_with_editor;
use crate::app::pickers::{
    bookmark_picker, history_picker_with_editor, project_picker_with_editor, session_switcher,
    session_tree_picker,
};
use crate::app::session_cmd;
use crate::app::Ctx;
use crate::domain::profile_rules::ProfileOpt;
use crate::domain::sessions::tmux_version_supported;
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;
use parse::{parse, Command};

/// FR-CMD-07, FR-OPEN-01, FR-OPEN-02, FR-CMD-01..06: parses `args` (the arguments after the program name) and runs
/// the command. The parse error is returned as a fatal error.
pub fn run(args: &[String], ctx: &Ctx<'_>) -> Result<Outcome> {
    let command = parse(args).map_err(LedgeError::Fatal)?;
    dispatch(command, ctx)
}

fn dispatch(command: Command, ctx: &Ctx<'_>) -> Result<Outcome> {
    match command {
        Command::Help => Ok(Outcome::Info(help::HELP.to_string())),
        Command::Usage(group) => Ok(Outcome::Info(help::group_usage(group).to_string())),
        Command::Open { dir, options } => {
            require_tmux(ctx)?;
            open_dir_with_editor(
                ctx,
                dir.as_deref(),
                &options.profile,
                options.editor.as_ref(),
            )
        }
        Command::ProjectPick(options) => {
            require_tmux(ctx)?;
            project_picker_with_editor(ctx, None, &options.profile, options.editor.as_ref())
        }
        Command::HistoryPick(options) => {
            require_tmux(ctx)?;
            history_picker_with_editor(ctx, &options.profile, options.editor.as_ref())
        }
        Command::SsSwitch(_) => {
            require_tmux(ctx)?;
            session_switcher(ctx)
        }
        Command::SsTree(options) => {
            require_tmux(ctx)?;
            session_tree_picker(ctx, &options.profile)
        }
        // A-29: doctor reports a missing tmux as ERR instead of failing, so it never checks tmux here.
        Command::Doctor => doctor::run(ctx),
        Command::HistoryList => history_cmd::list(ctx),
        Command::HistoryClean => history_cmd::clean(ctx),
        Command::BmAdd { dir } => bookmark_cmd::add(ctx, dir.as_deref()),
        Command::BmRemove { dir } => bookmark_cmd::remove(ctx, &dir),
        Command::BmList => bookmark_cmd::list(ctx),
        Command::BmTree { dir, options } => {
            require_tmux(ctx)?;
            bookmark_cmd::tree(ctx, dir.as_deref(), options.session_scope, &options.profile)
        }
        Command::BmPick => {
            require_tmux(ctx)?;
            bookmark_picker(ctx, &ProfileOpt::Unset)
        }
        Command::BmEdit { dir, session_scope } => {
            // A-29: only the session scope needs tmux (the window itself does not).
            if session_scope {
                require_tmux(ctx)?;
            }
            bookmark_cmd::edit(ctx, dir.as_deref(), session_scope)
        }
        Command::SsList => {
            require_tmux(ctx)?;
            session_cmd::list(ctx)
        }
        Command::SsKill { target } => {
            require_tmux(ctx)?;
            session_cmd::kill_target(ctx, &target)
        }
    }
}

/// FR-CMD-10, FR-PLAT-01, A-29: the tmux program is on PATH and its version is 3.2 or later. Checked at the start of
/// each command that needs tmux, so help and the other commands run without it.
fn require_tmux(ctx: &Ctx<'_>) -> Result<()> {
    if !ctx.process.find_program("tmux") {
        return Err(LedgeError::Fatal(
            messages::tmux_not_installed().to_string(),
        ));
    }
    let version = ctx.backend.version().map_err(|error| match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_read_tmux_version(&detail))
        }
    })?;
    if tmux_version_supported(version) {
        Ok(())
    } else {
        let found = format!("{}.{}", version.0, version.1);
        Err(LedgeError::Fatal(messages::tmux_version_too_old(&found)))
    }
}

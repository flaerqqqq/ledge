//! The open flow (architecture 2 and 6; FR-OPEN-03, FR-OPEN-08, FR-OPEN-10, FR-OPEN-11, FR-PROF-09..12).
//! Order: check the directory, read the config, record the directory in the history, choose the profile,
//! create or reuse the session, then attach or switch. Only the ports in `Ctx` are used.

use crate::app::build::ensure_session;
use crate::app::store;
use crate::app::Ctx;
use crate::domain::history::History;
use crate::domain::naming::session_name;
use crate::domain::path::AbsPath;
use crate::domain::profile::Profile;
use crate::domain::profile_rules::{open_profile, OpenProfile, ProfileOpt};
use crate::error::{LedgeError, Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;
use std::path::Path;

/// FR-OPEN-01..03, FR-OPEN-08, FR-OPEN-10, FR-OPEN-11, FR-PROF-09..12: opens the session of `dir`, or of the
/// working directory when `dir` is `None`. `dir` is the argument as typed; it is cleaned against the working
/// directory before use (D-06), and the message for a missing directory echoes it as typed (A-15).
pub fn open_dir(ctx: &Ctx<'_>, dir: Option<&str>, option: &ProfileOpt) -> Result<Outcome> {
    let typed = dir.unwrap_or(ctx.cwd.as_str());
    let path = AbsPath::clean(typed, &ctx.cwd);
    if !ctx.fs.is_dir(Path::new(path.as_str())) {
        return Err(LedgeError::Fatal(messages::directory_not_found(typed)));
    }
    let config = store::load_config(ctx.fs, ctx.env, &ctx.dirs, &ctx.cwd)?;
    record_in_history(ctx, &path);
    let profile = resolve_profile(ctx, option, config.default_profile.as_deref())?;
    ensure_session(ctx.backend, &path, &profile)?;
    attach_or_switch(ctx, &session_name(&path))
}

/// FR-OPEN-10, FR-HIST-02, FR-OPEN-11: the directory moves to the top of the history before the session is
/// opened. A failure to read or write the history is a warning, and the open goes on.
fn record_in_history(ctx: &Ctx<'_>, dir: &AbsPath) {
    let result = store::load_history(ctx.fs, &ctx.dirs).and_then(|mut history: History| {
        history.add(dir.clone());
        store::save_history(ctx.fs, &ctx.dirs, &history)
    });
    if let Err(error) = result {
        let text = messages::could_not_update_history(&error.to_string());
        ctx.console.err(&messages::warning(&text));
    }
}

/// FR-PROF-09..12, D-13: the profile the open uses. A named profile that fails is an error (FR-PROF-11); a
/// default profile that fails is a warning and the single-window layout (FR-PROF-12).
fn resolve_profile(
    ctx: &Ctx<'_>,
    option: &ProfileOpt,
    default_profile: Option<&str>,
) -> Result<Profile> {
    match open_profile(option, default_profile) {
        OpenProfile::Basic => Ok(Profile::basic()),
        OpenProfile::Requested(name) => store::load_profile(ctx.fs, &ctx.dirs, &name),
        OpenProfile::Default(name) => match store::load_profile(ctx.fs, &ctx.dirs, &name) {
            Ok(profile) => Ok(profile),
            Err(_) => {
                ctx.console
                    .err(&messages::warning(&messages::default_profile_missing(
                        &name,
                    )));
                Ok(Profile::basic())
            }
        },
    }
}

/// FR-OPEN-08: inside a tmux session (`TMUX` set and not empty) the client switches to the session; outside
/// one the terminal attaches. The attach itself is manual (traceability, FR-OPEN-08).
fn attach_or_switch(ctx: &Ctx<'_>, name: &str) -> Result<Outcome> {
    let inside = ctx.env.var("TMUX").is_some_and(|value| !value.is_empty());
    let result = if inside {
        ctx.backend.switch_client(name)
    } else {
        ctx.backend.attach(name)
    };
    result.map_err(|error| attach_error(name, error))?;
    Ok(Outcome::Done)
}

/// FR-OPEN-08, FR-CMD-10: a missing tmux keeps its own message; any other failure is the attach message.
fn attach_error(name: &str, error: BackendError) -> LedgeError {
    match error {
        BackendError::NotInstalled => LedgeError::Fatal(messages::tmux_not_installed().to_string()),
        BackendError::Failed { detail } => {
            LedgeError::Fatal(messages::could_not_attach(name, &detail))
        }
    }
}

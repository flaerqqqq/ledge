//! Doctor (architecture 2, 8.4, FR-DOC-01..08): ten read-only checks in a fixed order, one line each. A line with
//! ERR makes the exit code 1. Doctor never creates the config (A-12) and needs no tmux to run (A-29): a missing
//! tmux is an ERR line, and the session check is skipped with a WARN line.

use crate::app::store;
use crate::app::{home_of, load_projects, Ctx};
use crate::domain::bookmarks::{self, BookmarkStatus};
use crate::domain::config::{self, Config};
use crate::domain::path::AbsPath;
use crate::domain::profile::{Profile, ProfileName};
use crate::domain::sessions::{managed, session_directory, tmux_version_supported, ManagedSession};
use crate::error::{Outcome, Result};
use crate::messages;
use crate::ports::backend::BackendError;
use std::path::Path;

/// FR-DOC-08: the severity of one line; the tag is added when the report is rendered.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Ok,
    Warn,
    Err,
}

/// FR-DOC-08: the lines of the report, in the order the checks ran.
#[derive(Default)]
struct Report {
    lines: Vec<(Tag, String)>,
}

impl Report {
    fn ok(&mut self, text: String) {
        self.lines.push((Tag::Ok, text));
    }

    fn warn(&mut self, text: String) {
        self.lines.push((Tag::Warn, text));
    }

    fn err(&mut self, text: String) {
        self.lines.push((Tag::Err, text));
    }

    /// True when a WARN or ERR line was added after the first `mark` lines.
    fn problems_since(&self, mark: usize) -> bool {
        self.lines.iter().skip(mark).any(|(tag, _)| *tag != Tag::Ok)
    }

    /// FR-DOC-08: stdout text in check order; exit 1 when any line is ERR, otherwise 0.
    fn outcome(self) -> Outcome {
        let failed = self.lines.iter().any(|(tag, _)| *tag == Tag::Err);
        let text = self
            .lines
            .iter()
            .map(|(tag, text)| match tag {
                Tag::Ok => messages::doctor_ok(text),
                Tag::Warn => messages::doctor_warn(text),
                Tag::Err => messages::doctor_err(text),
            })
            .collect::<Vec<String>>()
            .join("\n");
        if failed {
            Outcome::Report(text, 1)
        } else {
            Outcome::Info(text)
        }
    }
}

/// FR-DOC-01, architecture 8.4 checks 2 to 4: the optional programs and what needs them. A missing one is a WARN
/// (A-12).
const OPTIONAL_PROGRAMS: [(&str, &str); 3] = [
    ("setsid", "editor launch will not detach"),
    ("code", "needed for --code"),
    ("idea", "needed for --idea"),
];

/// FR-DOC-01..08: runs the checks of architecture 8.4 in order and prints their lines. The outcome is `Info` when no
/// line is ERR, and `Report` with code 1 otherwise.
pub fn run(ctx: &Ctx<'_>) -> Result<Outcome> {
    let mut report = Report::default();
    let tmux_installed = check_tmux(ctx, &mut report);
    check_programs(ctx, &mut report);
    let config = check_config(ctx, &mut report);
    if let Some(config) = &config {
        check_search_paths(ctx, config, &mut report);
        check_default_profile(ctx, config, &mut report);
    }
    check_profiles(ctx, &mut report);
    check_bookmarks(ctx, config.as_ref(), &mut report);
    check_sessions(ctx, tmux_installed, &mut report);
    Ok(report.outcome())
}

/// FR-DOC-01, architecture 8.4 check 1: tmux is on PATH and is 3.2 or later. Returns whether tmux is on PATH.
fn check_tmux(ctx: &Ctx<'_>, report: &mut Report) -> bool {
    if !ctx.process.find_program("tmux") {
        report.err(messages::doctor_tmux_missing().to_string());
        return false;
    }
    match ctx.backend.version() {
        Ok(version) => {
            let found = format!("{}.{}", version.0, version.1);
            if tmux_version_supported(version) {
                report.ok(messages::doctor_tmux_version(&found));
            } else {
                report.err(messages::doctor_tmux_too_old(&found));
            }
        }
        Err(BackendError::NotInstalled) => report.err(messages::doctor_tmux_missing().to_string()),
        Err(BackendError::Failed { detail }) => {
            report.err(messages::could_not_read_tmux_version(&detail))
        }
    }
    true
}

/// FR-DOC-01, architecture 8.4 checks 2 to 4.
fn check_programs(ctx: &Ctx<'_>, report: &mut Report) {
    for (name, note) in OPTIONAL_PROGRAMS {
        if ctx.process.find_program(name) {
            report.ok(messages::doctor_program(name));
        } else {
            report.warn(messages::doctor_program_missing(name, note));
        }
    }
}

/// FR-DOC-02, architecture 8.4 check 5: the config file exists and parses. Read only: a missing file is an ERR and
/// is not created (A-12). Returns the parsed config, or `None` when it cannot be read.
fn check_config(ctx: &Ctx<'_>, report: &mut Report) -> Option<Config> {
    let file = store::config_path(&ctx.dirs);
    let shown = file.display().to_string();
    let text = match ctx.fs.read_text(&file) {
        Ok(Some(text)) => text,
        Ok(None) => {
            report.err(messages::doctor_config_missing(&shown));
            return None;
        }
        Err(error) => {
            report.err(messages::could_not_read_config(&shown, &error.to_string()));
            return None;
        }
    };
    let lookup = |name: &str| ctx.env.var(name);
    let home = ctx.env.var("HOME");
    match config::parse(&text, &shown, &lookup, home.as_deref(), &ctx.cwd) {
        Ok(config) => {
            report.ok(messages::doctor_config_file(&shown));
            Some(config)
        }
        Err(error) => {
            report.err(error.to_string());
            None
        }
    }
}

/// FR-DOC-03, architecture 8.4 check 6: each search path exists, or a WARN; no search path is a WARN.
fn check_search_paths(ctx: &Ctx<'_>, config: &Config, report: &mut Report) {
    if config.search_paths.is_empty() {
        report.warn(messages::doctor_no_search_path().to_string());
        return;
    }
    for path in &config.search_paths {
        if ctx.fs.is_dir(Path::new(path.as_str())) {
            report.ok(messages::doctor_search_path(path.as_str()));
        } else {
            report.warn(messages::doctor_search_path_missing(path.as_str()));
        }
    }
}

/// FR-DOC-06, architecture 8.4 check 7: the default profile loads. No line when none is set.
fn check_default_profile(ctx: &Ctx<'_>, config: &Config, report: &mut Report) {
    let Some(name) = config.default_profile.as_deref() else {
        return;
    };
    match store::load_profile(ctx.fs, &ctx.dirs, name) {
        Ok(_) => report.ok(messages::doctor_default_profile(name)),
        Err(error) => report.warn(messages::doctor_default_profile_invalid(
            name,
            &error.to_string(),
        )),
    }
}

/// FR-DOC-05, architecture 8.4 check 8: every profile file has a windows list. `basic` is reserved and its file is
/// ignored. Nothing is printed when all profiles are valid.
fn check_profiles(ctx: &Ctx<'_>, report: &mut Report) {
    for name in store::profile_files(ctx.fs, &ctx.dirs) {
        let Some(valid) = ProfileName::parse(&name) else {
            report.warn(messages::profile_invalid_shape(&name));
            continue;
        };
        if valid.is_basic() {
            report.warn(messages::doctor_basic_reserved().to_string());
            continue;
        }
        let path = store::profile_path(&ctx.dirs, &name);
        match ctx.fs.read_text(&path) {
            Ok(Some(text)) => {
                if let Err(error) = Profile::parse(&valid, &text) {
                    report.warn(error.to_string());
                }
            }
            Ok(None) => {}
            Err(error) => report.warn(messages::could_not_read_profile(
                &name,
                &path.display().to_string(),
                &error.to_string(),
            )),
        }
    }
}

/// FR-DOC-04, architecture 8.4 check 9: a bookmark whose directory is missing, or which lies outside every project,
/// is a WARN. Without a readable config the projects are unknown, so only a missing directory is judged.
fn check_bookmarks(ctx: &Ctx<'_>, config: Option<&Config>, report: &mut Report) {
    let bookmarks = match store::load_bookmarks(ctx.fs, &ctx.dirs) {
        Ok(bookmarks) => bookmarks,
        Err(error) => {
            report.warn(messages::could_not_read_bookmarks(&error.to_string()));
            return;
        }
    };
    let projects = config.map(|config| load_projects(ctx.fs, &config.search_paths));
    let home = home_of(ctx);
    let mark = report.lines.len();
    for path in bookmarks.entries() {
        let shown = path.display_with_home(home.as_ref());
        let exists = ctx.fs.is_dir(Path::new(path.as_str()));
        let status = match &projects {
            Some(projects) => bookmarks::status(path, exists, projects),
            None if exists => BookmarkStatus::Ok,
            None => BookmarkStatus::Missing,
        };
        match status {
            BookmarkStatus::Ok => {}
            BookmarkStatus::Missing => report.warn(messages::doctor_bookmark_missing(&shown)),
            BookmarkStatus::OutsideProject => {
                report.warn(messages::doctor_bookmark_outside(&shown))
            }
        }
    }
    if !report.problems_since(mark) {
        report.ok(messages::doctor_bookmarks(bookmarks.entries().len()));
    }
}

/// FR-DOC-07, architecture 8.4 check 10: each managed session has a directory and a profile that still exist. Without
/// tmux the check is a WARN line; a tmux that cannot list sessions is an ERR.
fn check_sessions(ctx: &Ctx<'_>, tmux_installed: bool, report: &mut Report) {
    if !tmux_installed {
        report.warn(messages::doctor_sessions_not_checked().to_string());
        return;
    }
    let sessions = match ctx.backend.list_sessions() {
        Ok(all) => managed(all),
        Err(BackendError::NotInstalled) => {
            report.warn(messages::doctor_sessions_not_checked().to_string());
            return;
        }
        Err(BackendError::Failed { detail }) => {
            report.err(messages::could_not_list_sessions(&detail));
            return;
        }
    };
    let mark = report.lines.len();
    for session in &sessions {
        check_session(ctx, session, report);
    }
    if !report.problems_since(mark) {
        report.ok(messages::doctor_sessions(sessions.len()));
    }
}

fn check_session(ctx: &Ctx<'_>, session: &ManagedSession, report: &mut Report) {
    let info = session.info();
    let directory = session_directory(info);
    if !directory.is_some_and(|dir| ctx.fs.is_dir(Path::new(dir))) {
        // FR-HIST-03: the session's directory is shown with a path under HOME as `~`.
        let shown = directory
            .map(|dir| AbsPath::clean(dir, &ctx.cwd).display_with_home(home_of(ctx).as_ref()));
        report.warn(messages::doctor_session_directory_missing(
            &info.name,
            shown.as_deref().unwrap_or("no directory"),
        ));
    }
    if let Some(profile) = info.profile.as_deref().filter(|name| !name.is_empty()) {
        if store::load_profile(ctx.fs, &ctx.dirs, profile).is_err() {
            report.warn(messages::doctor_session_profile_missing(
                &info.name, profile,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::Report;
    use crate::error::Outcome;

    // FR-DOC-08: a report with an ERR line is rendered with its tags and exits 1
    #[test]
    fn report_with_an_error_exits_one() {
        let mut report = Report::default();
        report.ok("tmux 3.4".to_string());
        report.err("tmux is not installed".to_string());
        assert_eq!(
            report.outcome(),
            Outcome::Report("OK    tmux 3.4\nERR   tmux is not installed".to_string(), 1)
        );
    }

    // FR-DOC-08: warnings alone keep the exit at 0
    #[test]
    fn report_with_only_warnings_exits_zero() {
        let mut report = Report::default();
        report.warn("setsid is not installed (editor launch will not detach)".to_string());
        assert_eq!(
            report.outcome(),
            Outcome::Info(
                "WARN  setsid is not installed (editor launch will not detach)".to_string()
            )
        );
    }
}

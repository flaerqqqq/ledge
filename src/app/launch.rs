//! Editor launch (architecture 2 and 13 A-18; FR-LAUNCH-01..03). Pure parts and the spawn only; the open flow
//! decides when to launch (only when it creates the session, FR-LAUNCH-01).

use crate::messages;
use crate::ports::console::Console;
use crate::ports::process::ProcessRunner;

/// FR-LAUNCH-02, architecture 2.3: the editor the open option names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editor {
    /// `--code`: VS Code, run as `code`.
    Code,
    /// `--idea`: IntelliJ IDEA, run as `idea`.
    Idea,
    /// `--editor CMD`: the command already split on whitespace (see `split_command`).
    Command(Vec<String>),
}

impl Editor {
    /// The program and its fixed arguments, without the directory.
    pub fn words(&self) -> Vec<String> {
        match self {
            Editor::Code => vec!["code".to_string()],
            Editor::Idea => vec!["idea".to_string()],
            Editor::Command(words) => words.clone(),
        }
    }
}

/// FR-LAUNCH-02, architecture 2.3: the `--editor` value is split on whitespace; there is no shell quoting.
pub fn split_command(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// FR-LAUNCH-01..03, A-18: starts the editor on `dir`, detached. With `setsid` on PATH the process is
/// `setsid -f <cmd> <dir>`; without it the editor is started directly with a warning. A missing editor, or one
/// that fails to start, is a warning; the caller goes on.
pub fn launch_editor(
    process: &dyn ProcessRunner,
    console: &dyn Console,
    editor: &Editor,
    dir: &str,
) {
    let words = editor.words();
    let Some((program, fixed)) = words.split_first() else {
        return;
    };
    let command = words.join(" ");
    if !process.find_program(program) {
        warn_start_failed(console, &command, messages::command_not_found());
        return;
    }
    let result = if process.find_program("setsid") {
        let mut args = vec!["-f".to_string()];
        args.extend(words.iter().cloned());
        args.push(dir.to_string());
        process.spawn_detached("setsid", &args)
    } else {
        console.err(&messages::warning(messages::setsid_not_installed()));
        let mut args = fixed.to_vec();
        args.push(dir.to_string());
        process.spawn_detached(program, &args)
    };
    if let Err(error) = result {
        warn_start_failed(console, &command, &error.to_string());
    }
}

/// A-18: `Warning: could not start editor '<command>': <detail>`.
fn warn_start_failed(console: &dyn Console, command: &str, detail: &str) {
    console.err(&messages::warning(&messages::could_not_start_editor(
        command, detail,
    )));
}

//! Composition root (architecture 2): builds the adapters, the `Ctx`, and calls `cli::run`; maps the outcome to
//! output and exit code. The only place that calls `std::process::exit` (FR-CMD-07).
#![forbid(unsafe_code)]

use ledge::adapters::clock::SystemClock;
use ledge::adapters::console::StdConsole;
use ledge::adapters::env::RealEnv;
use ledge::adapters::fs::RealFs;
use ledge::adapters::process::SystemProcess;
use ledge::adapters::signals::SignalFlags;
use ledge::adapters::terminal::CrosstermFactory;
use ledge::adapters::tmux::TmuxBackend;
use ledge::app::Ctx;
use ledge::domain::location::Dirs;
use ledge::domain::path::AbsPath;
use ledge::error::{LedgeError, Outcome, Result};
use ledge::messages;
use ledge::ports::env::Env;
use std::io::Write;

/// FR-CMD-07: prints the outcome (architecture 8.2) and returns the exit code.
/// Fatal: `Error: <text>` on stderr, 1. Cancelled: `Cancelled.` on stdout, 0. Info: text on stdout, 0.
/// Done: nothing, 0. Interrupted: nothing, the given code.
pub(crate) fn report(result: Result<Outcome>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match result {
        Ok(Outcome::Done) => 0,
        Ok(Outcome::Info(text)) => {
            let _ = writeln!(out, "{text}");
            0
        }
        Ok(Outcome::Cancelled) => {
            let _ = writeln!(out, "{}", messages::cancelled());
            0
        }
        Ok(Outcome::Interrupted(code)) => code,
        Err(error) => {
            let _ = writeln!(err, "Error: {error}");
            1
        }
    }
}

/// S-17: builds the real adapters and the `Ctx` (P-21, P-24), then runs the command line. The arguments are the
/// words after the program name.
fn run() -> Result<Outcome> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let env = RealEnv;
    let dirs = Dirs::resolve(
        env.var("XDG_CONFIG_HOME").as_deref(),
        env.var("XDG_CACHE_HOME").as_deref(),
        env.var("HOME").as_deref(),
    )?;
    let cwd_text = env.cwd().map_err(|error| {
        LedgeError::Fatal(messages::could_not_read_working_directory(
            &error.to_string(),
        ))
    })?;
    let cwd = AbsPath::clean(&cwd_text, &AbsPath::root());
    let signals = SignalFlags::new().map_err(|error| {
        LedgeError::Fatal(messages::could_not_register_signals(&error.to_string()))
    })?;
    let process = SystemProcess;
    // LEDGE_TMUX_SERVER names a tmux socket for the gate's isolated server; empty means the default server.
    let server = env.var("LEDGE_TMUX_SERVER").filter(|name| !name.is_empty());
    let backend = TmuxBackend::new(&process, server);
    let fs = RealFs;
    let clock = SystemClock;
    let console = StdConsole;
    let terminal = CrosstermFactory;
    let ctx = Ctx {
        env: &env,
        fs: &fs,
        clock: &clock,
        backend: &backend,
        console: &console,
        process: &process,
        dirs,
        terminal: &terminal,
        signals: &signals,
        cwd,
    };
    ledge::cli::run(&args, &ctx)
}

fn main() {
    ledge::adapters::terminal::install_panic_hook();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let code = report(run(), &mut stdout.lock(), &mut stderr.lock());
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::report;
    use ledge::error::{LedgeError, Outcome, Result};

    fn run(result: Result<Outcome>) -> (i32, String, String) {
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let code = report(result, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).expect("stdout is UTF-8"),
            String::from_utf8(err).expect("stderr is UTF-8"),
        )
    }

    // FR-CMD-07
    #[test]
    fn done_exits_zero_and_writes_nothing() {
        let (code, out, err) = run(Ok(Outcome::Done));
        assert_eq!(code, 0);
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    // FR-CMD-07
    #[test]
    fn info_goes_to_stdout_and_exits_zero() {
        let (code, out, err) = run(Ok(Outcome::Info("help text".to_string())));
        assert_eq!(code, 0);
        assert_eq!(out, "help text\n");
        assert_eq!(err, "");
    }

    // FR-CMD-07, FR-CMD-08, message "Cancelled." (stdout, exit 0)
    #[test]
    fn cancelled_prints_cancelled_on_stdout_and_exits_zero() {
        let (code, out, err) = run(Ok(Outcome::Cancelled));
        assert_eq!(code, 0);
        assert_eq!(out, "Cancelled.\n");
        assert_eq!(err, "");
    }

    // FR-CMD-07, message "Error: <message>" (stderr, exit 1)
    #[test]
    fn fatal_prints_error_prefix_on_stderr_and_exits_one() {
        let (code, out, err) = run(Err(LedgeError::Fatal("boom".to_string())));
        assert_eq!(code, 1);
        assert_eq!(out, "");
        assert_eq!(err, "Error: boom\n");
    }

    // FR-CMD-07
    #[test]
    fn every_fatal_error_exits_one_whatever_the_text() {
        for text in ["a", "Directory 'x' not found."] {
            let (code, _, _) = run(Err(LedgeError::Fatal(text.to_string())));
            assert_eq!(code, 1, "fatal text {text:?}");
        }
    }
}

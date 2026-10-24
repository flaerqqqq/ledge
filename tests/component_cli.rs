//! Component tests for the command line (`cli::run` over the fakes of tests/support, docs/testing.md level 2) and
//! for the built binary's help and error output. Covers FR-CMD-01..06, FR-CMD-09..11, FR-OPEN-01, FR-OPEN-02,
//! FR-OPEN-04, FR-PICK-11, FR-BM-14, FR-TREE-09, FR-TREE-10, FR-SESS-05, and the wiring of the commands that exist.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use ledge::cli::run;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::error::{LedgeError, Outcome, Result};
use std::process::Command;
use support::ctx::Harness;
use support::fake_process::FakeProcess;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

/// The fatal text of a command that must fail.
fn fatal(result: Result<Outcome>) -> String {
    match result {
        Err(LedgeError::Fatal(text)) => text,
        other => panic!("expected a fatal error, got {other:?}"),
    }
}

/// The text of a command that must print information.
fn info(result: Result<Outcome>) -> String {
    match result {
        Ok(Outcome::Info(text)) => text,
        other => panic!("expected info output, got {other:?}"),
    }
}

// FR-OPEN-01: no command word opens the session of the working directory
#[test]
fn bare_form_opens_the_working_directory() {
    let h = Harness::new("cli-bare-cwd");
    let outcome = run(&args(&[]), &h.ctx()).expect("open runs");
    assert_eq!(outcome, Outcome::Done);
    assert_eq!(h.backend.attached_to(), Some(session_name(&h.cwd)));
}

// FR-OPEN-02: the single directory argument of the bare form opens that directory
#[test]
fn bare_directory_argument_opens_that_directory() {
    let h = Harness::new("cli-bare-dir");
    let dir = h.project("app");
    run(&args(&[&dir]), &h.ctx()).expect("open runs");
    let expected = session_name(&AbsPath::clean(&dir, &AbsPath::root()));
    assert_eq!(h.backend.attached_to(), Some(expected));
}

// FR-OPEN-02: `project open DIR` is the same open
#[test]
fn project_open_directory_argument_opens_that_directory() {
    let h = Harness::new("cli-project-open");
    let dir = h.project("app");
    run(&args(&["project", "open", &dir]), &h.ctx()).expect("open runs");
    let expected = session_name(&AbsPath::clean(&dir, &AbsPath::root()));
    assert_eq!(h.backend.attached_to(), Some(expected));
}

// FR-OPEN-04: two directories are rejected with the exact message, in both forms
#[test]
fn two_directories_are_rejected_in_both_forms() {
    let h = Harness::new("cli-two-dirs");
    let a = h.project("a");
    let b = h.project("b");
    let expected = "project open takes at most one directory argument.";
    assert_eq!(
        fatal(run(&args(&["project", "open", &a, &b]), &h.ctx())),
        expected
    );
    assert_eq!(fatal(run(&args(&[&a, &b]), &h.ctx())), expected);
    assert_eq!(h.backend.attached_to(), None);
}

// FR-OPEN-03 through the command: a missing directory keeps its message
#[test]
fn missing_directory_argument_is_reported() {
    let h = Harness::new("cli-missing-dir");
    let missing = format!("{}/projects/nope", h.root().display());
    assert_eq!(
        fatal(run(&args(&[&missing]), &h.ctx())),
        format!("Directory '{missing}' not found.")
    );
}

// FR-CMD-01: help, -h and --help print help that lists every command of D-11
#[test]
fn help_words_print_help_listing_every_command() {
    let h = Harness::new("cli-help");
    let commands = [
        "ledge [DIR]",
        "ledge project open",
        "ledge project pick",
        "ledge history list",
        "ledge history pick",
        "ledge history clean",
        "ledge bm add",
        "ledge bm remove",
        "ledge bm list",
        "ledge bm pick",
        "ledge bm edit",
        "ledge bm tree",
        "ledge ss",
        "ledge ss list",
        "ledge ss tree",
        "ledge ss kill",
        "ledge doctor",
        "ledge help",
    ];
    for word in ["help", "-h", "--help"] {
        let text = info(run(&args(&[word]), &h.ctx()));
        for command in commands {
            assert!(text.contains(command), "{word}: help lacks '{command}'");
        }
    }
}

// FR-CMD-02: a group word without a sub-command prints its usage and exits 0
#[test]
fn group_words_without_sub_command_print_usage() {
    let h = Harness::new("cli-group-usage");
    let project = info(run(&args(&["project"]), &h.ctx()));
    assert!(project.contains("ledge project open"));
    assert!(project.contains("ledge project pick"));
    let history = info(run(&args(&["history"]), &h.ctx()));
    assert!(history.contains("ledge history list"));
    assert!(history.contains("ledge history clean"));
    let bm = info(run(&args(&["bm"]), &h.ctx()));
    for sub in ["add", "remove", "list", "pick", "edit", "tree"] {
        assert!(
            bm.contains(&format!("ledge bm {sub}")),
            "bm usage lacks {sub}"
        );
    }
}

// FR-CMD-03: an unknown sub-command names the group
#[test]
fn unknown_group_command_is_rejected() {
    let h = Harness::new("cli-unknown-group");
    assert_eq!(
        fatal(run(&args(&["history", "nope"]), &h.ctx())),
        "Unknown history command: nope (run 'ledge history' for the list)."
    );
    assert_eq!(
        fatal(run(&args(&["project", "nope"]), &h.ctx())),
        "Unknown project command: nope (run 'ledge project' for the list)."
    );
    assert_eq!(
        fatal(run(&args(&["bm", "nope"]), &h.ctx())),
        "Unknown bm command: nope (run 'ledge bm' for the list)."
    );
}

// FR-CMD-04: an option the command does not take, or an unknown one, is rejected
#[test]
fn unknown_or_unaccepted_option_is_rejected() {
    let h = Harness::new("cli-unknown-option");
    let expected = |opt: &str| format!("Unknown option: {opt} (run 'ledge help' for usage).");
    assert_eq!(
        fatal(run(&args(&["--bogus"]), &h.ctx())),
        expected("--bogus")
    );
    assert_eq!(
        fatal(run(&args(&["history", "list", "--code"]), &h.ctx())),
        expected("--code")
    );
    assert_eq!(
        fatal(run(
            &args(&["project", "open", "--session-scope"]),
            &h.ctx()
        )),
        expected("--session-scope")
    );
    assert_eq!(
        fatal(run(&args(&["bm", "add", "--profile", "x"]), &h.ctx())),
        expected("--profile")
    );
}

// FR-CMD-05: a profile option without a value
#[test]
fn profile_without_value_is_rejected() {
    let h = Harness::new("cli-profile-value");
    let expected = "--profile requires a profile name.";
    assert_eq!(fatal(run(&args(&["--profile"]), &h.ctx())), expected);
    assert_eq!(
        fatal(run(&args(&["project", "open", "--profile"]), &h.ctx())),
        expected
    );
    assert_eq!(
        fatal(run(&args(&["ss", "--profile", "--code"]), &h.ctx())),
        expected
    );
}

// FR-CMD-06: an editor option without a command
#[test]
fn editor_without_command_is_rejected() {
    let h = Harness::new("cli-editor-value");
    let expected = "--editor requires a command.";
    assert_eq!(fatal(run(&args(&["--editor"]), &h.ctx())), expected);
    assert_eq!(
        fatal(run(&args(&["project", "open", "--editor", ""]), &h.ctx())),
        expected
    );
}

// Messages section and D-11: the profile options exclude each other, and only one editor option may be given
#[test]
fn conflicting_profile_and_editor_options_are_rejected() {
    let h = Harness::new("cli-conflicts");
    assert_eq!(
        fatal(run(&args(&["--profile", "dev", "--no-profile"]), &h.ctx())),
        "--profile cannot be combined with --no-profile."
    );
    assert_eq!(
        fatal(run(&args(&["--code", "--idea"]), &h.ctx())),
        "only one of --code, --idea, --editor may be given."
    );
}

// FR-CMD-11: flags only after the command word; the bare form takes no command word after a leading flag
#[test]
fn flags_are_rejected_before_a_command_word() {
    let h = Harness::new("cli-flags-first");
    let expected = "Unknown option: --profile (run 'ledge help' for usage).";
    assert_eq!(
        fatal(run(&args(&["--profile", "dev", "bm"]), &h.ctx())),
        expected
    );
    assert_eq!(
        fatal(run(&args(&["--no-profile", "history", "list"]), &h.ctx())),
        "Unknown option: --no-profile (run 'ledge help' for usage)."
    );
}

// FR-CMD-11: after the command word the options are read, and the open flow receives the profile
#[test]
fn flags_after_the_command_word_reach_the_open_flow() {
    let h = Harness::new("cli-flags-after");
    let dir = h.project("app");
    let text = fatal(run(
        &args(&["project", "open", &dir, "--profile", "dev"]),
        &h.ctx(),
    ));
    assert!(
        text.starts_with("profile 'dev' not found at "),
        "got {text}"
    );
    assert_eq!(h.backend.attached_to(), None);
}

// FR-CMD-07 (warning-free path): an editor option launches the editor when the open creates the session
#[test]
fn editor_option_launches_the_editor_on_created_session() {
    let h = Harness::new("cli-editor");
    let dir = h.project("app");
    run(
        &args(&["project", "open", &dir, "--editor", "nvim -p"]),
        &h.ctx(),
    )
    .expect("open runs");
    let calls = h.process.calls();
    let launch = calls
        .iter()
        .find(|call| call.program == "setsid")
        .expect("setsid launch recorded");
    assert_eq!(launch.args, args(&["-f", "nvim", "-p", &dir]));
}

// FR-PICK-11: the project picker takes no directory
#[test]
fn project_pick_takes_no_directory() {
    let h = Harness::new("cli-pick-dir");
    let dir = h.project("app");
    assert_eq!(
        fatal(run(&args(&["project", "pick", &dir]), &h.ctx())),
        "project pick takes no directory argument."
    );
}

// Wiring: project pick with no search paths reaches the picker service and its message
#[test]
fn project_pick_reaches_the_picker_service() {
    let h = Harness::new("cli-pick-wiring");
    h.write_config("{\"search_paths\": []}");
    let text = fatal(run(&args(&["project", "pick"]), &h.ctx()));
    assert!(
        text.starts_with("No search paths defined in "),
        "got {text}"
    );
}

// FR-BM-14: the bookmark picker takes no directory and points to bm add
#[test]
fn bm_pick_takes_no_directory() {
    let h = Harness::new("cli-bm-pick");
    let dir = h.project("app");
    assert_eq!(
        fatal(run(&args(&["bm", "pick", &dir]), &h.ctx())),
        "bm pick takes no directory argument; use 'ledge bm add DIR'."
    );
}

// FR-TREE-09: the bookmark tree takes at most one directory
#[test]
fn bm_tree_takes_at_most_one_directory() {
    let h = Harness::new("cli-bm-tree-two");
    let a = h.project("a");
    let b = h.project("b");
    assert_eq!(
        fatal(run(&args(&["bm", "tree", &a, &b]), &h.ctx())),
        "bm tree takes at most one directory."
    );
}

// FR-TREE-10: the session-scope option cannot be combined with a directory
#[test]
fn session_scope_cannot_be_combined_with_a_directory() {
    let h = Harness::new("cli-session-scope");
    let dir = h.project("app");
    assert_eq!(
        fatal(run(
            &args(&["bm", "tree", &dir, "--session-scope"]),
            &h.ctx()
        )),
        "--session-scope cannot be combined with a directory."
    );
}

// FR-SESS-05: the switcher takes no directory; `ss tree` says the same for its own name
#[test]
fn ss_takes_no_directory() {
    let h = Harness::new("cli-ss-dir");
    let dir = h.project("app");
    assert_eq!(
        fatal(run(&args(&["ss", &dir]), &h.ctx())),
        "ss takes no directory argument."
    );
    assert_eq!(
        fatal(run(&args(&["ss", "tree", &dir]), &h.ctx())),
        "ss tree takes no directory argument."
    );
}

// Extra positional words for a command that takes none (architecture 2.3 rule 7)
#[test]
fn extra_words_for_a_command_without_arguments_are_rejected() {
    let h = Harness::new("cli-no-args");
    assert_eq!(
        fatal(run(&args(&["history", "list", "x"]), &h.ctx())),
        "'history list' takes no arguments."
    );
}

// FR-CMD-10: without tmux the commands that need it fail at start; help does not need it
#[test]
fn missing_tmux_fails_the_commands_that_need_it() {
    let mut h = Harness::new("cli-no-tmux");
    h.process = FakeProcess::missing();
    let expected = "Required tool 'tmux' is not installed.";
    assert_eq!(fatal(run(&args(&[]), &h.ctx())), expected);
    assert_eq!(fatal(run(&args(&["ss"]), &h.ctx())), expected);
    assert_eq!(fatal(run(&args(&["project", "pick"]), &h.ctx())), expected);
    let text = info(run(&args(&["help"]), &h.ctx()));
    assert!(text.contains("ledge doctor"));
}

// FR-PLAT-01: tmux older than 3.2 fails at start
#[test]
fn tmux_older_than_3_2_is_rejected() {
    let mut h = Harness::new("cli-old-tmux");
    h.backend = support::fake_backend::FakeBackend::new().with_version(3, 1);
    assert_eq!(
        fatal(run(&args(&[]), &h.ctx())),
        "tmux 3.2 or later is required; found 3.1."
    );
}

// FR-CMD-07: commands that are not built yet fail with the plain error, exit 1 (the binary maps it)
#[test]
fn commands_not_built_yet_fail_with_a_plain_error() {
    let h = Harness::new("cli-not-built");
    assert_eq!(
        fatal(run(&args(&["doctor"]), &h.ctx())),
        "'doctor' is not available yet."
    );
    assert_eq!(
        fatal(run(&args(&["bm", "list"]), &h.ctx())),
        "'bm list' is not available yet."
    );
}

fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ledge"));
    command.env_remove("TMUX");
    command
}

// FR-CMD-01: the built binary prints the help and exits 0
#[test]
fn binary_prints_help_and_exits_zero() {
    let output = binary().arg("--help").output().expect("binary runs");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert!(stdout.contains("ledge doctor"), "stdout: {stdout}");
    assert!(output.stderr.is_empty());
}

// FR-CMD-03, FR-CMD-07: the built binary prints the error on stderr and exits 1
#[test]
fn binary_unknown_group_command_exits_one() {
    let output = binary()
        .args(["history", "nope"])
        .output()
        .expect("binary runs");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert_eq!(
        stderr,
        "Error: Unknown history command: nope (run 'ledge history' for the list).\n"
    );
    assert!(output.stdout.is_empty());
}

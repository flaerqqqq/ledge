//! The pure command grammar (architecture 2.3, D-11). argv in, `Command` or the error text out; no I/O. Covers
//! FR-OPEN-01, FR-OPEN-02, FR-OPEN-04, FR-CMD-02..06, FR-CMD-11, FR-PICK-11, FR-BM-07, FR-BM-10, FR-BM-14,
//! FR-TREE-09, FR-TREE-10, FR-SESS-05, FR-CUST-01..04. The texts come from `messages`.

use crate::app::launch::{split_command, Editor};
use crate::domain::profile_rules::ProfileOpt;
use crate::messages;

/// The group words of the explicit commands (D-11). `ss` is a command word and also the switcher; `doctor` is a
/// command word with no sub-command.
const COMMAND_WORDS: [&str; 6] = ["project", "history", "bm", "ss", "doctor", "custom"];

/// The commands that have sub-commands and print a usage text when given none (FR-CMD-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Project,
    History,
    Bm,
    Custom,
}

/// The open options (architecture 2.3 OPEN-OPTS). The defaults are no profile option, no editor and no scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// FR-PROF-09, FR-PROF-10: `--profile NAME` or `--no-profile`.
    pub profile: ProfileOpt,
    /// FR-LAUNCH-01: `--code`, `--idea` or `--editor CMD`; at most one.
    pub editor: Option<Editor>,
    /// FR-TREE-06, FR-EDIT-09: `--session-scope`.
    pub session_scope: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            profile: ProfileOpt::Unset,
            editor: None,
            session_scope: false,
        }
    }
}

/// One parsed command. Commands whose slice is not built yet carry no data; their arguments are checked here and
/// the dispatch reports them as not available (proposal P-32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Usage(Group),
    /// FR-OPEN-01, FR-OPEN-02: the bare form and `project open`.
    Open {
        dir: Option<String>,
        options: Options,
    },
    /// FR-PICK-01: `project pick`.
    ProjectPick(Options),
    /// FR-HIST-04: `history pick`.
    HistoryPick(Options),
    /// FR-SESS-01: bare `ss`, the session switcher.
    SsSwitch(Options),
    HistoryList,
    HistoryClean,
    /// FR-BM-05, FR-BM-07: `bm add [DIR]`; no directory means the working directory.
    BmAdd {
        dir: Option<String>,
    },
    /// FR-BM-09, FR-BM-10: `bm remove DIR`.
    BmRemove {
        dir: String,
    },
    BmList,
    BmPick,
    /// FR-EDIT-08, FR-EDIT-09: `bm edit [DIR] [--session-scope]`; no directory means the working directory.
    BmEdit {
        dir: Option<String>,
        session_scope: bool,
    },
    /// FR-TREE-05..08: `bm tree [DIR] [--session-scope]` with the open options; no directory means every tree.
    BmTree {
        dir: Option<String>,
        options: Options,
    },
    SsList,
    /// FR-SESS-06: `ss tree`, the session tree window; takes the open options (P-36 applies: the editor is not used).
    SsTree(Options),
    /// FR-SESS-10: `ss kill NAME|DIR`; the one word is the exact session name or a directory.
    SsKill {
        target: String,
    },
    /// FR-CUST-01: `custom add [DIR]`; no directory means the working directory.
    CustomAdd {
        dir: Option<String>,
    },
    /// FR-CUST-02: `custom remove DIR`.
    CustomRemove {
        dir: String,
    },
    /// FR-CUST-03: `custom list`.
    CustomList,
    /// FR-CUST-04: `custom pick [OPTIONS]`, the project picker over the custom projects.
    CustomPick(Options),
    Doctor,
}

impl Command {
    /// The command words as typed, for the messages that name the command.
    pub fn words(&self) -> &'static str {
        match self {
            Command::Help => "help",
            Command::Usage(Group::Project) | Command::ProjectPick(_) => "project pick",
            Command::Usage(Group::History) | Command::HistoryPick(_) => "history pick",
            Command::Usage(Group::Bm) | Command::BmPick => "bm pick",
            Command::Usage(Group::Custom) => "custom",
            Command::CustomAdd { .. } => "custom add",
            Command::CustomRemove { .. } => "custom remove",
            Command::CustomList => "custom list",
            Command::CustomPick(_) => "custom pick",
            Command::Open { .. } => "project open",
            Command::SsSwitch(_) => "ss",
            Command::HistoryList => "history list",
            Command::HistoryClean => "history clean",
            Command::BmAdd { .. } => "bm add",
            Command::BmRemove { .. } => "bm remove",
            Command::BmList => "bm list",
            Command::BmEdit { .. } => "bm edit",
            Command::BmTree { .. } => "bm tree",
            Command::SsList => "ss list",
            Command::SsTree(_) => "ss tree",
            Command::SsKill { .. } => "ss kill",
            Command::Doctor => "doctor",
        }
    }
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some(first) = args.first() else {
        return Ok(Command::Open {
            dir: None,
            options: Options::default(),
        });
    };
    match first.as_str() {
        "help" | "-h" | "--help" => {
            let (words, opts) = split(scan(&args[1..])?);
            check(&opts, NONE)?;
            no_arguments(&words, "help")?;
            Ok(Command::Help)
        }
        word if COMMAND_WORDS.contains(&word) => explicit(word, &args[1..]),
        _ => bare(args),
    }
}

/// One token of the argument list: a word, or an option with its value already read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Opt(Opt),
}

/// An option as typed. `text` is the flag name (`--profile`, not the value), or the whole token when unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Opt {
    text: String,
    flag: Flag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Flag {
    Profile(String),
    NoProfile,
    Code,
    Idea,
    Editor(Vec<String>),
    SessionScope,
    Unknown,
}

/// Which options a command takes (architecture 2.3 table).
#[derive(Clone, Copy)]
struct Accepts {
    open: bool,
    scope: bool,
}

const NONE: Accepts = Accepts {
    open: false,
    scope: false,
};
const OPEN: Accepts = Accepts {
    open: true,
    scope: false,
};
const OPEN_SCOPE: Accepts = Accepts {
    open: true,
    scope: true,
};
const SCOPE: Accepts = Accepts {
    open: false,
    scope: true,
};

/// Reads the tokens. A `--profile` or `--editor` value is the next word, and it must exist and must not start with
/// `-` (FR-CMD-05, FR-CMD-06). Other options are read here and checked against the command later.
fn scan(args: &[String]) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while let Some(word) = args.get(index) {
        if !word.starts_with('-') {
            tokens.push(Token::Word(word.clone()));
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .filter(|next| !next.is_empty() && !next.starts_with('-'));
        let (flag, consumed) = match word.as_str() {
            "--profile" => match value {
                Some(name) => (Flag::Profile(name.clone()), 2),
                None => return Err(messages::profile_requires_name().to_string()),
            },
            "--editor" => {
                let words = value.map(|text| split_command(text)).unwrap_or_default();
                if words.is_empty() {
                    return Err(messages::editor_requires_command().to_string());
                }
                (Flag::Editor(words), 2)
            }
            "--no-profile" => (Flag::NoProfile, 1),
            "--code" => (Flag::Code, 1),
            "--idea" => (Flag::Idea, 1),
            "--session-scope" => (Flag::SessionScope, 1),
            _ => (Flag::Unknown, 1),
        };
        let text = match &flag {
            Flag::Profile(_) => "--profile".to_string(),
            Flag::Editor(_) => "--editor".to_string(),
            _ => word.clone(),
        };
        tokens.push(Token::Opt(Opt { text, flag }));
        index += consumed;
    }
    Ok(tokens)
}

/// Splits the tokens into the words (in order) and the options (in order).
fn split(tokens: Vec<Token>) -> (Vec<String>, Vec<Opt>) {
    let mut words = Vec::new();
    let mut opts = Vec::new();
    for token in tokens {
        match token {
            Token::Word(word) => words.push(word),
            Token::Opt(opt) => opts.push(opt),
        }
    }
    (words, opts)
}

/// FR-CMD-04: the first option the command does not take is the error.
fn check(opts: &[Opt], accepts: Accepts) -> Result<(), String> {
    for opt in opts {
        let accepted = match opt.flag {
            Flag::Profile(_) | Flag::NoProfile | Flag::Code | Flag::Idea | Flag::Editor(_) => {
                accepts.open
            }
            Flag::SessionScope => accepts.scope,
            Flag::Unknown => false,
        };
        if !accepted {
            return Err(messages::unknown_option(&opt.text));
        }
    }
    Ok(())
}

/// FR-PROF-09, FR-PROF-10, FR-LAUNCH-01, D-11: builds the options. The last profile value wins; a profile option
/// with `--no-profile` is an error, and so is more than one editor option.
fn options(opts: &[Opt]) -> Result<Options, String> {
    let mut profile = ProfileOpt::Unset;
    let mut named = false;
    let mut no_profile = false;
    let mut editors = Vec::new();
    let mut session_scope = false;
    for opt in opts {
        match &opt.flag {
            Flag::Profile(name) => {
                profile = ProfileOpt::Named(name.clone());
                named = true;
            }
            Flag::NoProfile => no_profile = true,
            Flag::Code => editors.push(Editor::Code),
            Flag::Idea => editors.push(Editor::Idea),
            Flag::Editor(words) => editors.push(Editor::Command(words.clone())),
            Flag::SessionScope => session_scope = true,
            Flag::Unknown => {}
        }
    }
    if no_profile {
        if named {
            return Err(messages::profile_with_no_profile().to_string());
        }
        profile = ProfileOpt::None;
    }
    if editors.len() > 1 {
        return Err(messages::only_one_editor_option().to_string());
    }
    Ok(Options {
        profile,
        editor: editors.pop(),
        session_scope,
    })
}

/// FR-CMD-11: the arguments after an explicit command word. Options may appear anywhere after the word.
fn explicit(word: &str, rest: &[String]) -> Result<Command, String> {
    let (words, opts) = split(scan(rest)?);
    match word {
        "project" => project(&words, &opts),
        "history" => history(&words, &opts),
        "bm" => bm(&words, &opts),
        "ss" => ss(&words, &opts),
        "custom" => custom(&words, &opts),
        _ => {
            check(&opts, NONE)?;
            no_arguments(&words, "doctor")?;
            Ok(Command::Doctor)
        }
    }
}

fn project(words: &[String], opts: &[Opt]) -> Result<Command, String> {
    let Some((sub, tail)) = words.split_first() else {
        check(opts, NONE)?;
        return Ok(Command::Usage(Group::Project));
    };
    match sub.as_str() {
        "open" => {
            check(opts, OPEN)?;
            if tail.len() > 1 {
                return Err(messages::open_takes_one_directory().to_string());
            }
            Ok(Command::Open {
                dir: tail.first().cloned(),
                options: options(opts)?,
            })
        }
        "pick" => {
            check(opts, OPEN)?;
            if !tail.is_empty() {
                return Err(messages::project_pick_takes_no_directory().to_string());
            }
            Ok(Command::ProjectPick(options(opts)?))
        }
        other => Err(messages::unknown_group_command("project", other)),
    }
}

fn history(words: &[String], opts: &[Opt]) -> Result<Command, String> {
    let Some((sub, tail)) = words.split_first() else {
        check(opts, NONE)?;
        return Ok(Command::Usage(Group::History));
    };
    match sub.as_str() {
        "list" => {
            check(opts, NONE)?;
            no_arguments(tail, "history list")?;
            Ok(Command::HistoryList)
        }
        "clean" => {
            check(opts, NONE)?;
            no_arguments(tail, "history clean")?;
            Ok(Command::HistoryClean)
        }
        "pick" => {
            check(opts, OPEN)?;
            no_arguments(tail, "history pick")?;
            Ok(Command::HistoryPick(options(opts)?))
        }
        other => Err(messages::unknown_group_command("history", other)),
    }
}

fn bm(words: &[String], opts: &[Opt]) -> Result<Command, String> {
    let Some((sub, tail)) = words.split_first() else {
        check(opts, NONE)?;
        return Ok(Command::Usage(Group::Bm));
    };
    match sub.as_str() {
        "add" => {
            check(opts, NONE)?;
            if tail.len() > 1 {
                return Err(messages::bm_add_takes_at_most_one_directory().to_string());
            }
            Ok(Command::BmAdd {
                dir: tail.first().cloned(),
            })
        }
        "remove" => {
            check(opts, NONE)?;
            match tail {
                [] => Err(messages::bookmark_remove_requires_directory().to_string()),
                [dir] => Ok(Command::BmRemove { dir: dir.clone() }),
                _ => Err(messages::command_not_available("bm remove")),
            }
        }
        "list" => {
            check(opts, NONE)?;
            no_arguments(tail, "bm list")?;
            Ok(Command::BmList)
        }
        "pick" => {
            check(opts, OPEN)?;
            if !tail.is_empty() {
                return Err(messages::bm_pick_takes_no_directory().to_string());
            }
            options(opts)?;
            Ok(Command::BmPick)
        }
        "edit" => {
            check(opts, SCOPE)?;
            if tail.len() > 1 {
                return Err(messages::command_not_available("bm edit"));
            }
            let parsed = options(opts)?;
            // P-39 (a): the scope option takes the current session's directory, so a directory with it is refused.
            if parsed.session_scope && !tail.is_empty() {
                return Err(messages::session_scope_with_directory().to_string());
            }
            Ok(Command::BmEdit {
                dir: tail.first().cloned(),
                session_scope: parsed.session_scope,
            })
        }
        "tree" => {
            check(opts, OPEN_SCOPE)?;
            let parsed = options(opts)?;
            if tail.len() > 1 {
                return Err(messages::bm_tree_takes_at_most_one_directory().to_string());
            }
            if parsed.session_scope && !tail.is_empty() {
                return Err(messages::session_scope_with_directory().to_string());
            }
            Ok(Command::BmTree {
                dir: tail.first().cloned(),
                options: parsed,
            })
        }
        other => Err(messages::unknown_group_command("bm", other)),
    }
}

/// FR-CUST-01..04: the custom project sub-commands. `custom pick` takes the open options, as `project pick` does.
fn custom(words: &[String], opts: &[Opt]) -> Result<Command, String> {
    let Some((sub, tail)) = words.split_first() else {
        check(opts, NONE)?;
        return Ok(Command::Usage(Group::Custom));
    };
    match sub.as_str() {
        "add" => {
            check(opts, NONE)?;
            if tail.len() > 1 {
                return Err(messages::custom_add_takes_at_most_one_directory().to_string());
            }
            Ok(Command::CustomAdd {
                dir: tail.first().cloned(),
            })
        }
        "remove" => {
            check(opts, NONE)?;
            match tail {
                [] => Err(messages::custom_remove_requires_directory().to_string()),
                [dir] => Ok(Command::CustomRemove { dir: dir.clone() }),
                _ => Err(messages::custom_remove_takes_one_directory().to_string()),
            }
        }
        "list" => {
            check(opts, NONE)?;
            no_arguments(tail, "custom list")?;
            Ok(Command::CustomList)
        }
        "pick" => {
            check(opts, OPEN)?;
            no_arguments(tail, "custom pick")?;
            Ok(Command::CustomPick(options(opts)?))
        }
        other => Err(messages::unknown_group_command("custom", other)),
    }
}

fn ss(words: &[String], opts: &[Opt]) -> Result<Command, String> {
    match words.first().map(String::as_str) {
        Some("list") => {
            check(opts, NONE)?;
            no_arguments(&words[1..], "ss list")?;
            Ok(Command::SsList)
        }
        Some("tree") => {
            check(opts, OPEN)?;
            if words.len() > 1 {
                return Err(messages::ss_tree_takes_no_directory().to_string());
            }
            Ok(Command::SsTree(options(opts)?))
        }
        Some("kill") => {
            check(opts, NONE)?;
            match words.get(1..).unwrap_or_default() {
                [] => Err(messages::kill_requires_target().to_string()),
                [target] => Ok(Command::SsKill {
                    target: target.clone(),
                }),
                _ => Err(messages::ss_kill_takes_one_target().to_string()),
            }
        }
        _ => {
            check(opts, OPEN)?;
            if !words.is_empty() {
                return Err(messages::ss_takes_no_directory().to_string());
            }
            Ok(Command::SsSwitch(options(opts)?))
        }
    }
}

/// FR-OPEN-01, FR-OPEN-02, FR-OPEN-04, FR-CMD-03..04, FR-CMD-11: the first word is not a command word, or it starts
/// with `-`. Every positional word is a directory. A command word after a leading flag is an unknown option.
fn bare(args: &[String]) -> Result<Command, String> {
    let tokens = scan(args)?;
    if args[0].starts_with('-') {
        let command_after = tokens.iter().skip(1).any(|token| match token {
            Token::Word(word) => COMMAND_WORDS.contains(&word.as_str()),
            Token::Opt(opt) => {
                opt.flag == Flag::Unknown && matches!(opt.text.as_str(), "-h" | "--help")
            }
        });
        if command_after {
            if let Some(Token::Opt(first)) = tokens.first() {
                return Err(messages::unknown_option(&first.text));
            }
        }
    }
    let (words, opts) = split(tokens);
    check(&opts, OPEN)?;
    if words.len() > 1 {
        return Err(messages::open_takes_one_directory().to_string());
    }
    Ok(Command::Open {
        dir: words.into_iter().next(),
        options: options(&opts)?,
    })
}

/// Architecture 2.3 rule 7: extra words for a command that takes none.
fn no_arguments(words: &[String], command: &str) -> Result<(), String> {
    if words.is_empty() {
        Ok(())
    } else {
        Err(messages::takes_no_arguments(command))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{parse, Command, Group, Options};
    use crate::app::launch::Editor;
    use crate::domain::profile_rules::ProfileOpt;

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    fn open(dir: Option<&str>) -> Command {
        Command::Open {
            dir: dir.map(str::to_string),
            options: Options::default(),
        }
    }

    // FR-OPEN-01: no words open the working directory
    #[test]
    fn no_words_is_the_bare_open() {
        assert_eq!(parse(&argv(&[])), Ok(open(None)));
    }

    // FR-OPEN-02: one directory word is the bare open of that directory
    #[test]
    fn one_directory_word_opens_it() {
        assert_eq!(parse(&argv(&["/p/app"])), Ok(open(Some("/p/app"))));
        assert_eq!(parse(&argv(&["./bm"])), Ok(open(Some("./bm"))));
    }

    // FR-OPEN-02, FR-CMD-11: project open takes the same argument
    #[test]
    fn project_open_takes_one_directory() {
        assert_eq!(
            parse(&argv(&["project", "open", "/p"])),
            Ok(open(Some("/p")))
        );
        assert_eq!(parse(&argv(&["project", "open"])), Ok(open(None)));
    }

    // FR-OPEN-04: two directories are rejected, bare or with project open
    #[test]
    fn two_directories_are_rejected() {
        let expected = "project open takes at most one directory argument.";
        assert_eq!(parse(&argv(&["/a", "/b"])), Err(expected.to_string()));
        assert_eq!(
            parse(&argv(&["project", "open", "/a", "/b"])),
            Err(expected.to_string())
        );
    }

    // FR-CMD-02: group words without a sub-command print their usage
    #[test]
    fn group_words_alone_are_usage() {
        assert_eq!(
            parse(&argv(&["project"])),
            Ok(Command::Usage(Group::Project))
        );
        assert_eq!(
            parse(&argv(&["history"])),
            Ok(Command::Usage(Group::History))
        );
        assert_eq!(parse(&argv(&["bm"])), Ok(Command::Usage(Group::Bm)));
    }

    // FR-CMD-01: the help words
    #[test]
    fn help_words_are_help() {
        assert_eq!(parse(&argv(&["help"])), Ok(Command::Help));
        assert_eq!(parse(&argv(&["-h"])), Ok(Command::Help));
        assert_eq!(parse(&argv(&["--help"])), Ok(Command::Help));
    }

    // FR-SESS-01: bare ss is the switcher, and a directory is refused (FR-SESS-05)
    #[test]
    fn ss_is_the_switcher_and_takes_no_directory() {
        assert_eq!(
            parse(&argv(&["ss"])),
            Ok(Command::SsSwitch(Options::default()))
        );
        assert_eq!(
            parse(&argv(&["ss", "/a"])),
            Err("ss takes no directory argument.".to_string())
        );
        assert_eq!(
            parse(&argv(&["ss", "tree", "/a"])),
            Err("ss tree takes no directory argument.".to_string())
        );
    }

    // FR-SESS-06: `ss tree` takes the open options and no directory
    #[test]
    fn ss_tree_takes_the_open_options() {
        assert_eq!(
            parse(&argv(&["ss", "tree"])),
            Ok(Command::SsTree(Options::default()))
        );
        assert_eq!(
            parse(&argv(&["ss", "tree", "--profile", "web"])),
            Ok(Command::SsTree(Options {
                profile: ProfileOpt::Named("web".to_string()),
                ..Options::default()
            }))
        );
    }

    // FR-PICK-11: the project picker takes no directory and does take options
    #[test]
    fn project_pick_rejects_a_directory() {
        assert_eq!(
            parse(&argv(&["project", "pick"])),
            Ok(Command::ProjectPick(Options::default()))
        );
        assert_eq!(
            parse(&argv(&["project", "pick", "/a"])),
            Err("project pick takes no directory argument.".to_string())
        );
    }

    // FR-BM-14: the bookmark picker points to bm add
    #[test]
    fn bm_pick_rejects_a_directory() {
        assert_eq!(
            parse(&argv(&["bm", "pick", "/a"])),
            Err("bm pick takes no directory argument; use 'ledge bm add DIR'.".to_string())
        );
    }

    // FR-TREE-09, FR-TREE-10: the tree takes at most one directory and no scope with one
    #[test]
    fn bm_tree_rules() {
        assert_eq!(
            parse(&argv(&["bm", "tree", "/a", "/b"])),
            Err("bm tree takes at most one directory.".to_string())
        );
        assert_eq!(
            parse(&argv(&["bm", "tree", "/a", "--session-scope"])),
            Err("--session-scope cannot be combined with a directory.".to_string())
        );
        assert_eq!(
            parse(&argv(&["bm", "tree", "--session-scope"])),
            Ok(Command::BmTree {
                dir: None,
                options: Options {
                    session_scope: true,
                    ..Options::default()
                },
            })
        );
    }

    // FR-TREE-05, FR-TREE-08: `bm tree` takes one directory or none, and the open options
    #[test]
    fn bm_tree_takes_a_directory_and_the_open_options() {
        assert_eq!(
            parse(&argv(&["bm", "tree"])),
            Ok(Command::BmTree {
                dir: None,
                options: Options::default(),
            })
        );
        assert_eq!(
            parse(&argv(&["bm", "tree", "/a", "--profile", "web"])),
            Ok(Command::BmTree {
                dir: Some("/a".to_string()),
                options: Options {
                    profile: ProfileOpt::Named("web".to_string()),
                    ..Options::default()
                },
            })
        );
    }

    // FR-BM-10: remove needs its directory
    #[test]
    fn bm_remove_needs_a_directory() {
        assert_eq!(
            parse(&argv(&["bm", "remove"])),
            Err("bookmark remove requires a directory.".to_string())
        );
        assert_eq!(
            parse(&argv(&["bm", "remove", "/a"])),
            Ok(Command::BmRemove {
                dir: "/a".to_string()
            })
        );
    }

    // FR-BM-07: bm add takes one directory or none (the working directory); two are refused
    #[test]
    fn bm_add_takes_at_most_one_directory() {
        assert_eq!(
            parse(&argv(&["bm", "add"])),
            Ok(Command::BmAdd { dir: None })
        );
        assert_eq!(
            parse(&argv(&["bm", "add", "/a"])),
            Ok(Command::BmAdd {
                dir: Some("/a".to_string())
            })
        );
        assert_eq!(
            parse(&argv(&["bm", "add", "/a", "/b"])),
            Err("bm add takes at most one directory.".to_string())
        );
    }

    // FR-CMD-03: an unknown sub-command names the group
    #[test]
    fn unknown_sub_command_names_the_group() {
        assert_eq!(
            parse(&argv(&["bm", "zap"])),
            Err("Unknown bm command: zap (run 'ledge bm' for the list).".to_string())
        );
    }

    // FR-CMD-04: options the command does not take are rejected, and so are unknown ones
    #[test]
    fn options_are_checked_per_command() {
        assert_eq!(
            parse(&argv(&["history", "list", "--code"])),
            Err("Unknown option: --code (run 'ledge help' for usage).".to_string())
        );
        assert_eq!(
            parse(&argv(&["--bogus"])),
            Err("Unknown option: --bogus (run 'ledge help' for usage).".to_string())
        );
    }

    // FR-CMD-05, FR-CMD-06: values are required
    #[test]
    fn profile_and_editor_need_values() {
        assert_eq!(
            parse(&argv(&["--profile"])),
            Err("--profile requires a profile name.".to_string())
        );
        assert_eq!(
            parse(&argv(&["--profile", "--code"])),
            Err("--profile requires a profile name.".to_string())
        );
        assert_eq!(
            parse(&argv(&["--editor"])),
            Err("--editor requires a command.".to_string())
        );
        assert_eq!(
            parse(&argv(&["--editor", "  "])),
            Err("--editor requires a command.".to_string())
        );
    }

    // FR-PROF-09, D-11: the profile name and the no-profile option
    #[test]
    fn profile_options_are_read() {
        let named = Options {
            profile: ProfileOpt::Named("dev".to_string()),
            ..Options::default()
        };
        assert_eq!(
            parse(&argv(&["--profile", "dev"])),
            Ok(Command::Open {
                dir: None,
                options: named
            })
        );
        let none = Options {
            profile: ProfileOpt::None,
            ..Options::default()
        };
        assert_eq!(
            parse(&argv(&["project", "open", "/a", "--no-profile"])),
            Ok(Command::Open {
                dir: Some("/a".to_string()),
                options: none
            })
        );
    }

    // Architecture 2.3 rule 6: the profile options exclude each other
    #[test]
    fn profile_and_no_profile_conflict() {
        assert_eq!(
            parse(&argv(&["--profile", "dev", "--no-profile"])),
            Err("--profile cannot be combined with --no-profile.".to_string())
        );
    }

    // FR-LAUNCH-01, D-11: one editor option; the editor command is split on whitespace, no quoting
    #[test]
    fn editor_options_are_one_and_split() {
        assert_eq!(
            parse(&argv(&["--code", "--idea"])),
            Err("only one of --code, --idea, --editor may be given.".to_string())
        );
        let editor = Options {
            editor: Some(Editor::Command(vec!["nvim".to_string(), "-p".to_string()])),
            ..Options::default()
        };
        assert_eq!(
            parse(&argv(&["--editor", "nvim -p", "/a"])),
            Ok(Command::Open {
                dir: Some("/a".to_string()),
                options: editor
            })
        );
    }

    // FR-CMD-11: flags after a leading flag may not be followed by a command word
    #[test]
    fn leading_flag_before_a_command_word_is_an_unknown_option() {
        assert_eq!(
            parse(&argv(&["--profile", "dev", "bm"])),
            Err("Unknown option: --profile (run 'ledge help' for usage).".to_string())
        );
        assert_eq!(
            parse(&argv(&["--code", "--help"])),
            Err("Unknown option: --code (run 'ledge help' for usage).".to_string())
        );
    }

    // Architecture 2.3 rule 7: extra words for a command that takes none
    #[test]
    fn extra_words_for_a_command_without_arguments() {
        assert_eq!(
            parse(&argv(&["history", "list", "x"])),
            Err("'history list' takes no arguments.".to_string())
        );
        assert_eq!(
            parse(&argv(&["doctor", "x"])),
            Err("'doctor' takes no arguments.".to_string())
        );
    }

    // FR-EDIT-08, FR-EDIT-09: `bm edit` takes an optional directory and the scope option
    #[test]
    fn bm_edit_forms() {
        assert_eq!(
            parse(&argv(&["bm", "edit"])),
            Ok(Command::BmEdit {
                dir: None,
                session_scope: false
            })
        );
        assert_eq!(
            parse(&argv(&["bm", "edit", "/a/app"])),
            Ok(Command::BmEdit {
                dir: Some("/a/app".to_string()),
                session_scope: false
            })
        );
        assert_eq!(
            parse(&argv(&["bm", "edit", "/a", "--session-scope"])),
            Err("--session-scope cannot be combined with a directory.".to_string())
        );
        assert_eq!(
            parse(&argv(&["bm", "edit", "/a", "/b"])),
            Err("'bm edit' is not available yet.".to_string())
        );
    }

    // FR-SESS-10: `ss kill` takes the name or directory as its one word
    #[test]
    fn ss_kill_takes_one_target() {
        assert_eq!(
            parse(&argv(&["ss", "kill", "app-1a2b"])),
            Ok(Command::SsKill {
                target: "app-1a2b".to_string()
            })
        );
    }

    // FR-SESS-13: `ss kill` with no target fails before tmux is asked
    #[test]
    fn ss_kill_without_target_is_refused() {
        assert_eq!(
            parse(&argv(&["ss", "kill"])),
            Err("kill requires a session name or directory.".to_string())
        );
    }

    // FR-SESS-10, P-46: two targets for `ss kill` are refused
    #[test]
    fn ss_kill_with_two_targets_is_refused() {
        assert_eq!(
            parse(&argv(&["ss", "kill", "a", "b"])),
            Err("ss kill takes one session name or directory.".to_string())
        );
    }

    // FR-SESS-09: `ss list` takes no arguments
    #[test]
    fn ss_list_parses_and_takes_no_arguments() {
        assert_eq!(parse(&argv(&["ss", "list"])), Ok(Command::SsList));
        assert_eq!(
            parse(&argv(&["ss", "list", "x"])),
            Err("'ss list' takes no arguments.".to_string())
        );
    }

    // Architecture 2.3: the commands whose slice is not built yet still parse
    #[test]
    fn unbuilt_commands_parse() {
        assert_eq!(
            parse(&argv(&["history", "clean"])),
            Ok(Command::HistoryClean)
        );
        assert_eq!(
            parse(&argv(&["bm", "edit", "--session-scope"])),
            Ok(Command::BmEdit {
                dir: None,
                session_scope: true
            })
        );
        assert_eq!(parse(&argv(&["doctor"])), Ok(Command::Doctor));
    }
}

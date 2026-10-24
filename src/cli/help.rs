//! Help text and group usage text (architecture 2, FR-CMD-01, FR-CMD-02). Every command of D-11 and the custom project
//! commands of P-67 are listed in `HELP`.

use super::parse::Group;

/// FR-CMD-01: printed for `help`, `-h` and `--help`. Lists every command of D-11 and the options.
pub const HELP: &str = "Usage:
  ledge [DIR] [OPTIONS]                              open the session of DIR (default: the current directory)
  ledge project open [DIR] [OPTIONS]                 the same open, as a project command
  ledge project pick [OPTIONS]                       pick a project and open it
  ledge history list                                 list the directories opened before
  ledge history pick [OPTIONS]                       pick a directory from the history and open it
  ledge history clean                                delete the history file
  ledge bm add [DIR]                                 bookmark DIR (default: the current directory)
  ledge bm remove DIR                                remove the bookmark of DIR
  ledge bm list                                      list the bookmarks
  ledge bm pick [OPTIONS]                            pick a bookmark and open it
  ledge bm edit [DIR] [--session-scope]              edit the bookmarks of the project of DIR
  ledge bm tree [DIR] [--session-scope] [OPTIONS]    show the bookmark tree and open a choice
  ledge custom add [DIR]                             record DIR as a custom project (default: the current directory)
  ledge custom remove DIR                            remove the custom project DIR with its bookmarks and sessions
  ledge custom list                                  list the custom projects
  ledge custom pick [OPTIONS]                        pick a custom project and open it
  ledge ss [OPTIONS]                                 switch between ledge sessions
  ledge ss list                                      list the ledge sessions
  ledge ss tree [OPTIONS]                            show the sessions as a tree of directories
  ledge ss kill NAME|DIR                             kill a ledge session
  ledge doctor                                       check the installation and the sessions
  ledge help | -h | --help                           show this help

Options:
  --profile NAME     use the profile NAME
  --no-profile       use the single-window layout, no profile
  --code             open the directory in VS Code (code)
  --idea             open the directory in IntelliJ IDEA (idea)
  --editor CMD       open the directory in CMD (split on spaces, no quoting)
  --session-scope    use the directory of the current session (bm edit, bm tree)";

/// FR-CMD-02: printed for a group word without a sub-command.
pub fn group_usage(group: Group) -> &'static str {
    match group {
        Group::Project => {
            "Usage:
  ledge project open [DIR] [OPTIONS]
  ledge project pick [OPTIONS]"
        }
        Group::History => {
            "Usage:
  ledge history list
  ledge history pick [OPTIONS]
  ledge history clean"
        }
        Group::Bm => {
            "Usage:
  ledge bm add [DIR]
  ledge bm remove DIR
  ledge bm list
  ledge bm pick [OPTIONS]
  ledge bm edit [DIR] [--session-scope]
  ledge bm tree [DIR] [--session-scope] [OPTIONS]"
        }
        Group::Custom => {
            "Usage:
  ledge custom add [DIR]
  ledge custom remove DIR
  ledge custom list
  ledge custom pick [OPTIONS]"
        }
    }
}

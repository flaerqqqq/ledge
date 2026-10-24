# ledge

ledge manages tmux sessions for your project directories. Pick a project from a terminal list and
ledge opens it in a tmux session with the windows you want, or switches to the session if it is
already running. It also keeps bookmarks, a history, and custom projects, and has its own
terminal pickers and trees for them.

ledge is written in Rust and runs on Linux with tmux 3.2 or later.

## Features

- **Project picker** (`ledge project pick`): lists the directories directly under your search paths,
  filters as you type, and opens the one you choose.
- **Sessions**: one tmux session per directory, named from the directory and a short hash of its path, so
  two projects with the same name never clash. Sessions are marked as managed by ledge.
- **Profiles**: named window layouts (for example an editor, a shell and a log window). A profile can
  start a command in each window. Profiles are shown with a preview before you pick one.
- **Session switcher** (`ledge ss`): the running ledge sessions, most recently used first, with their
  project, profile and activity.
- **Session tree** (`ledge ss tree`): sessions nested by directory.
- **Bookmarks** (`ledge bm`): directories inside a project that you want to reach quickly. Bookmark picker,
  bookmark tree and an edit window (`ledge bm edit`) for choosing bookmarks from a project's folders.
- **History** (`ledge history`): the directories you opened before.
- **Custom projects** (`ledge custom`): directories outside your search paths that you want to treat as
  projects, so you can bookmark them too.
- **Doctor** (`ledge doctor`): checks tmux, the configuration, search paths, bookmarks, profiles and sessions.
- **Themes**: built-in colour themes (`default`, `ink`, `mist`, `zen`, `pearl`) and per-token overrides.

## Requirements

- Linux
- tmux 3.2 or later
- A Rust toolchain (rustc and cargo 1.80 or later) to build ledge
- Optional: `setsid` (detached editor launch), `code` (VS Code), `idea` (IntelliJ IDEA)

## Install

From the project folder:

```sh
bash scripts/install.sh
```

This builds a release binary with `cargo build --release --locked`, copies it to `~/.local/bin/ledge`,
and records the source folder. Make sure `~/.local/bin` is on your `PATH`; ledge warns when it is not.

To remove it again:

```sh
bash scripts/install.sh --uninstall
```

Uninstall removes the binary only when it was installed from this folder. Your configuration, profiles,
history and bookmarks are left in place.

The install folder and the state folder can be changed with `LEDGE_INSTALL_BIN_DIR` and
`LEDGE_INSTALL_STATE_DIR`.

## Quick start

```sh
ledge doctor                 # check tmux, the config and the search paths
ledge                        # open the session of the current directory
ledge project pick           # pick a project from your search paths
ledge ss                     # switch between running ledge sessions
ledge bm add                 # bookmark the current directory (inside a project)
ledge bm pick                # pick a bookmark and open it
```

When ledge runs inside tmux it switches the client to the session; outside tmux it attaches to it.

## Commands

| Command | What it does |
| --- | --- |
| `ledge [DIR] [OPTIONS]` | Open the session of DIR (default: the current directory). |
| `ledge project open [DIR] [OPTIONS]` | The same open, as a project command. |
| `ledge project pick [OPTIONS]` | Pick a project and open it. |
| `ledge history list` | List the directories opened before. |
| `ledge history pick [OPTIONS]` | Pick a directory from the history and open it. |
| `ledge history clean` | Delete the history file. |
| `ledge bm add [DIR]` | Bookmark DIR (inside a project). |
| `ledge bm remove DIR` | Remove the bookmark of DIR. |
| `ledge bm list` | List the bookmarks. |
| `ledge bm pick [OPTIONS]` | Pick a bookmark and open it. |
| `ledge bm edit [DIR] [--session-scope]` | Edit the bookmarks of the project of DIR. |
| `ledge bm tree [DIR] [--session-scope] [OPTIONS]` | Show the bookmark tree and open a choice. |
| `ledge custom add [DIR]` | Record DIR as a custom project (default: the current directory). |
| `ledge custom remove DIR` | Remove the custom project DIR with its bookmarks and sessions. |
| `ledge custom list` | List the custom projects. |
| `ledge custom pick [OPTIONS]` | Pick a custom project and open it. |
| `ledge ss [OPTIONS]` | Switch between ledge sessions. |
| `ledge ss list` | List the ledge sessions. |
| `ledge ss tree [OPTIONS]` | Show the sessions as a tree of directories. |
| `ledge ss kill NAME\|DIR` | Kill a ledge session. |
| `ledge doctor` | Check the installation and the sessions. |
| `ledge help` | Show the help. |

Options:

| Option | Meaning |
| --- | --- |
| `--profile NAME` | Use the profile NAME. |
| `--no-profile` | Use the single-window layout, no profile. |
| `--code` | Open the directory in VS Code. |
| `--idea` | Open the directory in IntelliJ IDEA. |
| `--editor CMD` | Open the directory in CMD (split on spaces, no quoting). |
| `--session-scope` | Use the directory of the current session (`bm edit`, `bm tree`). |

Flags come after the command word.

## Keys

The pickers and trees share one key map. The footer of each window shows the keys that apply to it.

| Key | Action |
| --- | --- |
| enter | Open the selected row. |
| escape, ctrl-c | Cancel. |
| up, down, page keys | Move the selection. |
| ctrl-j, ctrl-k | Move the selection down and up (vim keys). |
| ctrl-r | Rebuild the session of the selected row (creates it if missing). |
| ctrl-p | Choose a profile, then rebuild the session. |
| ctrl-x | Kill the session of the selected row. |
| ctrl-d | Delete the selected bookmark; in the project picker, remove a custom project. |
| alt-r, alt-p, alt-x, alt-d | The same actions on every session under a tree node. |

In the **bookmark edit window**:

| Key | Action |
| --- | --- |
| enter | Toggle the bookmark of the selected folder. |
| l, right arrow | Expand a folder with subfolders. |
| h, left arrow | Collapse a folder, or move to its parent. |
| j, k | Move down and up. |
| `.` | Show or hide folders whose names start with a dot (hidden by default; gray when shown). |
| `/` | Open the search bar; typing filters the folders; enter keeps the match, escape clears it. |
| escape | Close the window. |

Killing the session you are in is allowed. ledge switches the client to the most recent other session;
if no other session is left, tmux closes.

## Configuration

ledge reads `config.json` from its configuration folder: `~/.config/ledge/` by default, or
`$XDG_CONFIG_HOME/ledge/` when `XDG_CONFIG_HOME` is set. The file is created on first run.

```json
{
  "search_paths": ["~/projects", "~/work"],
  "default_profile": "dev",
  "theme": "zen",
  "theme_colors": {"accent": "#7fb4ca"}
}
```

| Key | Meaning |
| --- | --- |
| `search_paths` | Folders whose immediate subfolders are projects. `~` and `$VAR` are expanded. |
| `default_profile` | Profile used when none is given. Missing or invalid: ledge warns and uses one window. |
| `theme` | `default`, `ink`, `mist`, `zen` or `pearl`. |
| `theme_colors` | Overrides for single colour tokens, as `"token": "#rrggbb"`. |

Environment variables:

| Variable | Meaning |
| --- | --- |
| `XDG_CONFIG_HOME` | Configuration folder base (`ledge` is added). |
| `XDG_CACHE_HOME` | Cache folder base for history, bookmarks and custom projects. |
| `LEDGE_TMUX_SERVER` | Use a separate tmux server (tests use this; the default is your own tmux server). |

## Profiles

A profile is a JSON file in `~/.config/ledge/profiles/NAME.json`:

```json
{
  "windows": [
    { "name": "Code", "command": "nvim ." },
    { "name": "Terminal" },
    { "name": "Run" }
  ]
}
```

Each window has a `name` and an optional `command`, typed into the window when the session is built.
The first window is selected after the build. `basic` is built in: one window named `Terminal`.

A profile that changes after a session is built is not applied to that session until it is rebuilt
(`ctrl-r`).

## Files

| Path | Contents |
| --- | --- |
| `~/.config/ledge/config.json` | Search paths, default profile, theme. |
| `~/.config/ledge/profiles/` | Profile files. |
| `~/.cache/ledge/history` | Directories opened, newest first, at most 50. |
| `~/.cache/ledge/bookmarks` | Bookmarked directories. |
| `~/.cache/ledge/custom-projects` | Custom projects. |
| `~/.cache/ledge/bm-edit-open` | Folders expanded in the bookmark edit window. |

## tmux

ledge's pickers open in a tmux popup, so a useful binding is a `display-popup` that runs the picker:

```tmux
bind f display-popup -E -B -w 70% -h 65% "ledge project pick"
bind s display-popup -E -B -w 70% -h 65% "ledge ss tree"
bind b display-popup -E -B -w 70% -h 65% "ledge bm tree --session-scope"
bind e display-popup -E -B -w 70% -h 65% "ledge bm edit --session-scope"
```

## Development

The project follows the rules in `CLAUDE.md`. The gate runs every check:

```sh
bash scripts/gate.sh
```

It runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, the build and the tests,
the tmux integration tests when tmux is on the PATH, the layer check (`scripts/check_layers.sh`), and
the installer tests (`scripts/test_install.sh`). Shell lint with `shellcheck` runs when it is installed.

The tmux integration tests use their own server name (`LEDGE_TMUX_SERVER`) and never touch your tmux server.

Documentation:

- `requirements.md`: what ledge must do, as numbered requirements (FR-IDs).
- `docs/architecture.md`: crates, layers and ports.
- `docs/design.md`, `docs/design-options.md`, `docs/design/themes.md`: the interface and the themes.
- `docs/traceability.md`: which requirement is covered by which code and test.
- `docs/decisions.md` and `docs/proposals.md`: the decisions taken and the open changes.

## License

No license has been chosen yet.

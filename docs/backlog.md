# Backlog

STATUS: APPROVED
Owner: architect defined the slices in Phase 0 (gate A; gate B waived). The coordinator updates `status`
and `branch` only. Each FR-ID of `requirements.md` is in exactly one slice (checked; see
`docs/traceability.md`, generated from this file).

Status values: `blocked` (deps not done) | `ready` (all deps done, or none) | `in-progress` | `in-review` |
`done` | `escalated`. At Phase 0 end, only slices without dependencies are `ready`; the rest are `blocked`.

Dependencies form a DAG (S-00 is the root). Slices are ordered so that a valid order is the number order.

## Slice template

```
### S-<nn> <name>
- status: ready | blocked
- deps: S-xx, S-yy   (or "none")
- FR-IDs: FR-..., FR-...
- scope: modules/files touched
- acceptance: observable behaviours + exact messages to verify
- size: S | M   (L must be split)
- branch: slice/<nn>-<name>
```

Each slice also has a `tests:` expectation in the test-engineer's brief (testing.md levels). Slices that
have no FR-IDs state why.

## Slices

### S-00 Skeleton and gate
- status: done
- deps: none
- FR-IDs: FR-CMD-07
- scope: `Cargo.toml` (package `ledge`, four dependencies pinned as in architecture section 10 are added in the
  slice that first uses them, not here; `[lints]` from conventions; `rust-version`), `Cargo.lock`, `src/main.rs`,
  `src/lib.rs` (module tree from architecture section 2 as empty modules), `src/error.rs` (`LedgeError`,
  `Outcome`, exit mapping in `main`), `src/messages.rs` (empty, with the verbatim test scaffold),
  `scripts/gate.sh`, `scripts/check_layers.sh`, `tests/support/mod.rs` (empty), `tests/messages_verbatim.rs`
  (scaffold).
- acceptance: `main` exits 1 and prints `Error: <text>` on stderr for `Fatal`, exit 0 for `Done`, `Info` (stdout),
  and `Cancelled` (`Cancelled.` on stdout); unit tests on the exit mapping
  (FR-CMD-07); `scripts/gate.sh` runs fmt, clippy, build, test and layer check and exits non-zero on any failure;
  `scripts/check_layers.sh` fails on a planted violation.
- size: S
- branch: slice/00-skeleton

### S-01 Ports, real adapters for files and environment, paths and session naming
- status: done
- deps: S-00
- FR-IDs: FR-PLAT-02, FR-CONF-08, FR-OPEN-05, FR-OPEN-06, FR-NAME-01, FR-NAME-02
- scope: `ports/fs.rs`, `ports/env.rs`, `ports/clock.rs`, `ports/process.rs`, `ports/console.rs`, `ports/signals.rs`
  (traits only), `adapters/fs.rs`, `adapters/env.rs`, `adapters/clock.rs`, `adapters/process.rs`,
  `adapters/console.rs`, `domain/path.rs`, `domain/naming.rs`, `domain/location.rs`; `md-5` dependency added here;
  `tests/support/fake_env.rs`, `fake_clock.rs`, `fake_console.rs`, `fake_process.rs`, `temp.rs`.
- acceptance: `AbsPath::clean` handles relative input, `.`, `..`, repeated `/`, trailing `/` (D-06); `inside`
  compares whole components (`/p/ap` is not inside `/p/app`); `session_name("/home/u/projects/my.app")` equals
  `my_app-` followed by the first four hex characters of md5 of the path without newline (D-14), with fixed vectors
  from the user once supplied (until then, the vectors in the test are marked `// D-02 provisional`); two directories
  with one basename give different names (FR-OPEN-06); `location` uses `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` only when
  absolute and non-empty (A-17). Real adapters have unit tests on temp folders.
- size: M
- branch: slice/01-paths-naming

### S-02 Configuration and profile files
- status: done
- deps: S-01
- FR-IDs: FR-CONF-01, FR-CONF-02, FR-CONF-03, FR-CONF-04, FR-CONF-05, FR-CONF-06, FR-PROF-01, FR-PROF-02, FR-PROF-06
- scope: `domain/config.rs`, `domain/profile.rs`, `app/store.rs` (config load and create), `messages.rs` (config and
  profile messages).
- acceptance: first run writes exactly the default content of architecture 9.2 when `config.json` is missing and creates
  `profiles/`; a second run changes nothing (D-05). `config is not valid JSON: <file>` (FR-CONF-05). `search_paths` with
  `~`, `$VAR` and `${VAR}` expanded; undefined variable gives `config search path '<entry>' uses undefined variable '<VAR>'.`
  [new]; empty entries skipped; duplicates removed. Profile name `[A-Za-z0-9_-]+`; file `<name>.json`; errors
  `profile '<name>' not found at <path>`, `profile '<name>' is invalid JSON: <detail>`,
  `profile '<name>' is invalid (expected a 'windows' array)`. `basic` is built in; `Profile::basic()` is one window
  `Terminal`.
- size: M
- branch: slice/02-config-profiles

### S-03 History and bookmark stores
- status: done
- deps: S-01
- FR-IDs: FR-HIST-01, FR-HIST-02, FR-BM-01
- scope: `domain/history.rs`, `domain/bookmarks.rs` (list rules and file format only), `app/store.rs` (history and
  bookmark files, atomic write from architecture 9.3).
- acceptance: history keeps newest first, at most 50, moves a re-opened directory to the top without duplicates
  (FR-HIST-01/02); a longer file is trimmed in memory on read (F-36); bookmarks keep insertion order and one path per
  line (FR-BM-01); relative lines are skipped (A-09); duplicates keep the first occurrence (D-06); the write uses a temp
  file in the same folder then rename (D-05), tested by a write that fails after the temp file and leaves the old file.
- size: M
- branch: slice/03-history-bookmark-stores

### S-04 Projects, scope and current directory row
- status: done
- deps: S-02
- FR-IDs: FR-PICK-01, FR-PICK-02, FR-PICK-03, FR-PICK-04, FR-PICK-05, FR-CONF-07, FR-SCOPE-01, FR-SCOPE-02
- scope: `domain/projects.rs` (listing from a list of child names, sort, current-directory row, `project_containing`,
  scope membership), `domain/sessions.rs` (group selection only: managed sessions whose directory equals or is inside a
  group entry).
- acceptance: every visible child directory of each search path is a project; names starting with `.` skipped (FR-PICK-02);
  sorted by full path (FR-PICK-03); current directory first and preselected (FR-PICK-04); `*` row rules and worked
  examples from architecture A-16 and F-23 (project current directory listed once; non-project current directory shows its
  path); only direct children are projects (FR-CONF-07); group action selection uses whole-component inside (FR-SCOPE-02).
- size: M
- branch: slice/04-projects-scope

### S-05 Tree construction (bookmark and session trees)
- status: done
- deps: S-04
- FR-IDs: FR-TREE-01, FR-TREE-02, FR-TREE-03, FR-TREE-04, FR-TREE-11, FR-SESS-06, FR-SESS-07
- scope: `domain/tree.rs` (both builders, `TreeRow` with `openable`, `connector`, `dimmed`, `depth`).
- acceptance: bookmark tree: one root per project with a bookmark, bookmarks nested under nearest shown ancestor, gray
  connectors between root and bookmark, shared parent gray, bookmarks outside every project left out (A-13); session tree:
  roots are projects with a managed session, project root without session gray and openable (FR-SESS-07 name), gray
  connectors not openable, a session outside every project is a root labelled with its path (A-13); enter on a connector
  not openable (FR-TREE-11). Pure tests only.
- size: M
- branch: slice/05-trees

### S-06 Fuzzy matcher
- status: done
- deps: S-00
- FR-IDs: FR-TUI-02
- scope: `tui/fuzzy.rs`.
- acceptance: case-insensitive subsequence match; score rewards consecutive matches and word-boundary matches (`/ - _ .`
  space and camelCase); ties keep original order; empty query matches all in original order (D-10); returns matched
  character positions for highlighting. Unit tests only.
- size: S
- branch: slice/06-fuzzy

### S-07 Picker model, keys and rendering (headless)
- status: done
- deps: S-06
- FR-IDs: FR-TUI-03, FR-TUI-04, FR-TUI-05, FR-TUI-07, FR-TUI-08, FR-TUI-09, FR-TUI-10
- scope: `tui/picker.rs`, `tui/keys.rs`, `tui/columns.rs`, `tui/render.rs`, `tui/terminal.rs` (trait only, no adapter yet),
  `tests/support/fake_terminal.rs`, `tests/tui_engine.rs` (model part).
- acceptance: arrow and page keys, home and end (FR-TUI-03, A-28); header, border label with icon, footer, prompt `> `
  (FR-TUI-04, section 6.2 table); resize keeps the selected row visible at 40x8 and larger; below 40x8 the single line
  `Terminal too small: need at least 40x8.` (A-06); same key same action across pickers (FR-TUI-07, 6.3); missing directory
  rows carry the ` (missing)` cue and dimming (FR-TUI-08, A-04); no-color cues and `NO_COLOR` rule (FR-TUI-09); a picker
  stays open after an action that leaves rows (FR-TUI-10, model side). Column shrink order from A-06 tested.
- size: M
- branch: slice/07-picker-model

### S-08 Terminal engine and restore
- status: done
- deps: S-07
- FR-IDs: FR-TUI-01, FR-TUI-06
- scope: `tui/engine.rs` (`run_picker`, `PickerHandler`, `Step`, `PickerResult`), `adapters/terminal.rs` (`CrosstermTerminal`,
  `CrosstermFactory`: raw mode, alternate screen, Drop restores), `adapters/signals.rs` (signal-hook flags), panic hook
  in `main.rs`; `signal-hook` dependency added here.
- acceptance: `TerminalFactory::open` fails with `a terminal is required for this picker.` when stdout is not a tty
  (exit 1); handler error returns through the drop path, verified by a fake terminal that records restore (FR-TUI-06);
  `FakeSignals` interrupt returns `Closed(Outcome::Interrupted(130))` and restores; ctrl-c is a cancel (A-08); every
  picker is drawn by this engine (FR-TUI-01).
- size: M
- branch: slice/08-terminal-engine

### S-09 tmux adapter and managed marker
- status: done
- deps: S-01
- FR-IDs: FR-PLAT-01, FR-OPEN-09, FR-NAME-03, FR-NAME-04, FR-NAME-05
- scope: `adapters/tmux.rs` (`TmuxBackend`, every command of architecture 7.1 and the error mapping of 7.2),
  `ports/backend.rs` (`SessionBackend`, `SessionInfo`, `NewSession`, `BackendError`), `domain/sessions.rs` (managed filter,
  `ManagedSession`), `tests/support/fake_backend.rs`, `tests/it_tmux_adapter.rs` (`#[ignore]`).
- acceptance: fake-process test: argv of each operation matches architecture 7.1 exactly, with `-L` only when
  `LEDGE_TMUX_SERVER` is set; `list-sessions` with `no server running` is an empty list (FR-PICK-07 empty markers);
  missing binary gives `BackendError::NotInstalled`. Integration (ignored): create with windows, options `@ledge_managed`,
  `@ledge_profile`, `@ledge_dir` set and read back (FR-NAME-03); unmanaged sessions are filtered out (FR-NAME-04); a session
  with `@ledge_managed` set by hand is adopted (FR-NAME-05); `#{session_path}` check on tmux 3.2 (A-17). Server starts with
  the first session (FR-OPEN-09).
- size: M
- branch: slice/09-tmux-adapter

### S-10 Building a session from a profile
- status: done
- deps: S-02, S-09
- FR-IDs: FR-OPEN-07, FR-PROF-03, FR-PROF-04, FR-PROF-05, FR-PROF-07, FR-PROF-08
- scope: `app/build.rs`, `app/mod.rs` (`Ctx`), `domain/naming.rs` (collision rule use).
- acceptance: windows created in profile order with the first window as the session's first (FR-PROF-03, FR-PROF-05),
  each command typed then Enter (FR-PROF-04), profile name recorded on the session with `basic` for the single window
  (FR-PROF-07), profile used only at creation (FR-PROF-08); session created only when no session of the name exists
  (FR-OPEN-07); a managed session of the same name with another `@ledge_dir` fails with
  `could not create session '<name>': name already used by <dir>` (D-14); an unmanaged session of the name is reused
  without marking (D-14, A-05 for adoption); a failed build kills the partial session (A-07). Component tests with
  `FakeBackend`.
- size: M
- branch: slice/10-build-session

### S-11 Open flow: directory checks, history, attach or switch, profile precedence at open
- status: done
- deps: S-10, S-03
- FR-IDs: FR-OPEN-03, FR-OPEN-08, FR-OPEN-10, FR-OPEN-11, FR-PROF-09, FR-PROF-10, FR-PROF-11, FR-PROF-12
- scope: `app/open.rs`, `domain/profile_rules.rs` (open precedence: no-profile > `--profile` > default > basic).
- acceptance: `Directory '<dir>' not found.` echoes the argument as typed (A-15, D-06 for the cleaned form in other
  messages); directory is recorded in history before the open (FR-OPEN-10); a history write failure prints
  `Warning: could not update history: <detail>` and the open continues (FR-OPEN-11); outside tmux the open attaches
  (`attach-session`), inside it switches (`switch-client`), decided by `TMUX` (FR-OPEN-08); `--profile` missing or invalid
  gives the profile's error (FR-PROF-11); missing default profile gives `Warning: default_profile '<name>' is missing or
  invalid; using single-window layout.` and basic (FR-PROF-12, D-04 rules); `--no-profile` ignores profiles (FR-PROF-10);
  `--profile` over default (FR-PROF-09). Attach failure prints `could not attach to '<name>': <detail>`.
- size: M
- branch: slice/11-open-flow

### S-12 Editor launch
- status: done
- deps: S-11
- FR-IDs: FR-LAUNCH-01, FR-LAUNCH-02, FR-LAUNCH-03
- scope: `app/launch.rs`, `adapters/process.rs` (`spawn_detached`, `setsid` lookup).
- acceptance: the editor is launched only when the open path creates the session (FR-LAUNCH-01, A-18); `code` and `idea`
  run as the program name, `--editor CMD` split on whitespace, directory appended last (FR-LAUNCH-02); with `setsid`
  present the process is `setsid -f <cmd> <dir>` (FR-LAUNCH-03); without `setsid`, `Warning: setsid is not installed; the editor is started without detaching.`
  and stdio null; a missing editor gives `Warning: could not start editor '<command>': <detail>` and the session still opens
  (A-18). Component tests with `FakeProcess`.
- size: S
- branch: slice/12-editor-launch

### S-13 Project and history pickers (open, navigation, empty states)
- status: done
- deps: S-03, S-04, S-08, S-09, S-11
- FR-IDs: FR-PICK-06, FR-PICK-07, FR-PICK-08, FR-PICK-09, FR-PICK-10, FR-PICK-12, FR-PICK-13, FR-HIST-04, FR-HIST-05, FR-CMD-08, FR-CMD-12
- scope: `app/pickers.rs` (project and history pickers), `app/history_cmd.rs` (pick), `tui/` wiring of the project and
  history screen tables.
- acceptance: rows show basename, session name, profile when a session exists (FR-PICK-06); filled marker for managed live
  sessions, empty otherwise (FR-PICK-07); enter opens the chosen project through the S-11 open flow (FR-PICK-08);
  `Missing directory: <path>` in the status line with the picker open (FR-PICK-09); no search paths gives
  `Error: No search paths defined in <config file>.` (FR-PICK-10, D-16 error class); the border label
  `Projects (Profile: <name>)` or `Projects & Submodules` (FR-PICK-13, A-16); `--profile` applies to the open (FR-PICK-12);
  history picker: no file gives `No history cache found.` and stops (FR-HIST-05); empty file gives `No history.` (D-12);
  picker with no rows after an action closes with `Cancelled.` only on cancel (FR-CMD-08); the empty-at-start rows of D-12;
  FR-CMD-12 closes a picker without rows, except the switcher and trees. TUI model tests for both pickers.
- size: M
- branch: slice/13-project-history-pickers

### S-14 Rebuild and kill services
- status: done
- deps: S-10, S-09
- FR-IDs: FR-ACT-02, FR-ACT-03, FR-ACT-05, FR-ACT-07, FR-ACT-13, FR-PROF-14, FR-PROF-15
- scope: `app/actions.rs` (kill of managed sessions, rebuild, rebuild of the current session, refusal rules), `domain/profile_rules.rs`
  (rebuild precedence).
- acceptance: rebuild kills and rebuilds from profile (FR-ACT-03), creates the session when missing (FR-ACT-02); recorded
  profile reused without a `--profile` (FR-PROF-14); recorded profile missing gives
  `Warning: session profile '<name>' is missing or invalid; using the default profile.` (FR-PROF-15, D-04 second warning when the
  default is also bad); rebuild of the current session follows D-17 (`<name>-rebuild`, switch, kill old, rename) and on failure
  the original stays and the error is returned (A-07); kill and rebuild refuse unmanaged sessions with
  `'<name>' is not a ledge session.` (FR-ACT-13, D-14); kill of the current session refused with `cannot kill the session you are in.`
  (D-17); alt-key guard function `never_touch_current` (FR-ACT-07) used by S-22. Component tests with `FakeBackend`.
- size: M
- branch: slice/14-rebuild-kill

### S-15 Picker actions and the profile picker
- status: done
- deps: S-13, S-14, S-08
- FR-IDs: FR-ACT-01, FR-ACT-04, FR-ACT-12, FR-PROF-13
- scope: `app/pickers.rs` (action wiring: ctrl-r, ctrl-p, ctrl-x for project, history, switcher-ready helpers), `domain/profile_rules.rs`
  (picker row order), profile picker screen in `tui`.
- acceptance: kill key ctrl-x in every picker that lists sessions, no-op on rows without a managed session (FR-ACT-01, D-17);
  ctrl-r rebuilds or creates (FR-ACT-02 via S-14); ctrl-p opens the profile picker and rebuilds with the choice (FR-ACT-04); the
  profile option applies to rebuild actions (FR-ACT-12, D-13); profile list rows: `[Current: <name>]`, `[Default: <name>]`, then
  others and `basic` by name, one row when current equals default, preselected first row (FR-PROF-13, A-11); escape in the nested
  profile picker returns to the parent without output (A-08); a profile that fails to load shows its message in the status line.
- size: M
- branch: slice/15-picker-actions

### S-16 Session switcher
- status: done
- deps: S-15, S-11
- FR-IDs: FR-SESS-01, FR-SESS-02, FR-SESS-03, FR-SESS-04
- scope: `app/pickers.rs` (switcher), `app/session_cmd.rs` (switcher entry), `domain/age.rs`, `domain/sessions.rs` (switcher order).
- acceptance: only managed sessions (FR-SESS-01); current session first, then by activity, ties by name (FR-SESS-02, A-10); columns
  session, project, profile, activity with age text `just now`, `<n>m ago`, `<n>h ago`, `<n>d ago` (FR-SESS-03, A-10 boundaries 60 s, 60 min,
  24 h); enter switches inside a session and attaches outside (FR-SESS-04); ctrl-x kills and the list stays open (screen table);
  empty list shows `No ledge sessions.` and stays open (D-12); sessions outside every search path appear with an empty project column (D-07).
  TUI model tests and age unit tests.
- size: M
- branch: slice/16-switcher

### S-17 Command line grammar, help, and command-level checks
- status: done
- deps: S-01
- FR-IDs: FR-OPEN-01, FR-OPEN-02, FR-OPEN-04, FR-CMD-01, FR-CMD-02, FR-CMD-03, FR-CMD-04, FR-CMD-05, FR-CMD-06, FR-CMD-09, FR-CMD-10, FR-CMD-11, FR-PICK-11, FR-BM-14, FR-TREE-09, FR-TREE-10, FR-SESS-05
- scope: `cli/parse.rs`, `cli/help.rs`, `cli/mod.rs` (`run`), `messages.rs` (usage and error texts), `main.rs` (wiring of `Ctx`).
  Also: the tmux presence and version check at start (`Required tool 'tmux' is not installed.`, `tmux 3.2 or later is required; found <version>. [new]`)
  for commands that need tmux (A-29), via `ProcessRunner::find_program` and the adapter's version call.
- acceptance: grammar of architecture 2.3 as a pure function with unit tests; `ledge` with no word opens the cwd (FR-OPEN-01, via
  `Command::Open`); `ledge <dir>` and `project open <dir>` (FR-OPEN-02); two directories gives `project open takes at most one directory argument.`
  (FR-OPEN-04); help and `-h` and `--help` print help listing every command of D-11 (FR-CMD-01); `project`, `history`, `bm` without
  sub-command print group usage, exit 0 (FR-CMD-02); `Unknown <group> command: <word> (run 'ledge <group>' for the list).` (FR-CMD-03);
  `Unknown option: <option> (run 'ledge help' for usage).` (FR-CMD-04); `--profile requires a profile name.` (FR-CMD-05);
  `--editor requires a command.` (FR-CMD-06); flags only after the command word (FR-CMD-11, A-36); `<name> is required for this command.`
  helper for non-tmux programs (A-01, P-2); `project pick takes no directory argument.` (FR-PICK-11); `bm pick takes no directory argument; use 'ledge bm add DIR'.`
  (FR-BM-14); `bm tree takes at most one directory.` and `--session-scope cannot be combined with a directory.` (FR-TREE-09, FR-TREE-10);
  `ss takes no directory argument.` (FR-SESS-05). Tests: `tests/component_cli.rs` through `cli::run` with fakes. Commands not yet built
  return `Outcome::Info` placeholder only in tests of the dispatch table, never in the binary (each later slice wires its command).
- size: M
- branch: slice/17-cli

### S-18 History and bookmark commands (file-only)
- status: done
- deps: S-03, S-04, S-17
- FR-IDs: FR-HIST-03, FR-HIST-06, FR-HIST-07, FR-HIST-08, FR-BM-02, FR-BM-03, FR-BM-04, FR-BM-05, FR-BM-06, FR-BM-07, FR-BM-08, FR-BM-09, FR-BM-10
- scope: `app/history_cmd.rs` (list, clean), `app/bookmark_cmd.rs` (add, remove, list), `domain/bookmarks.rs` (status and rules).
- acceptance: `history list` one per line, home shown as `~`, `No history.` when empty (FR-HIST-03); `history clean` deletes the file and
  prints `Project history cache clear successfully.`, or `No history cache found.` when there is no file (FR-HIST-06, FR-HIST-07); bookmarks
  untouched (FR-HIST-08); `bm add` default cwd (FR-BM-07), `Bookmarked <path>` (FR-BM-05), `Already bookmarked: <path>` change nothing (FR-BM-06),
  `<path> is not inside a project. Bookmarks go inside a directory from search_paths.` (FR-BM-03), `<path> is a project; open it with 'ledge project pick'.`
  (FR-BM-04), inside-only with whole components (FR-BM-02, A-14), missing path gives `Directory '<dir>' not found.` (A-19), `bm add takes at most one directory.`
  (FR-BM-07); `bm list` with `ok`, `missing`, `outside project` (FR-BM-08) and `No bookmarks.`; `bm remove DIR` gives `Removed bookmark <path>`
  (FR-BM-09), `'<path>' is not a bookmark.` (FR-BM-10), `bookmark remove requires a directory.` without a target. Component tests.
- size: M
- branch: slice/18-history-bookmark-commands

### S-19 Bookmark picker
- status: in-progress
- deps: S-15, S-18
- FR-IDs: FR-BM-11, FR-BM-12, FR-BM-13, FR-ACT-11
- scope: `app/pickers.rs` (bookmark picker), `app/bookmark_cmd.rs` (pick), bookmark picker screen in `tui`.
- acceptance: stored order, missing directories dimmed with ` (missing)` (FR-BM-11); empty list prints `No bookmarks. Use 'ledge bm add' to add one.` and
  closes with exit 0 (FR-BM-12, D-12); ctrl-d deletes the bookmark and kills its managed session, then reopens the list (FR-BM-13, A-39); ctrl-d on
  a non-bookmark row does nothing (FR-ACT-11); enter opens the bookmarked directory's session (A-19); ctrl-r, ctrl-p, ctrl-x as in the project picker.
  TUI model tests.
- size: M
- branch: slice/19-bookmark-picker

### S-20 Bookmark edit window state and listing
- status: blocked
- deps: S-03, S-04
- FR-IDs: FR-EDIT-01, FR-EDIT-03, FR-EDIT-04, FR-EDIT-05, FR-EDIT-06, FR-EDIT-11
- scope: `domain/bm_edit.rs` (expanded folders, lazy listing of subdirectories, `+` and `-` markers, pruning of missing directories on write),
  `app/store.rs` (`bm-edit-open` file).
- acceptance: subfolders of the project listed, hidden folders skipped (FR-EDIT-01); expand and collapse state of a folder (FR-EDIT-03);
  `+` collapsed with subfolders, `-` expanded (FR-EDIT-04); expanded state persists in `bm-edit-open` (FR-EDIT-05, D-09: global, pruned of missing
  directories on write, collapse removes only that folder's entry); bookmark marker in its own column (FR-EDIT-06); project without subdirectories
  exits with no output (FR-EDIT-11, D-12). Pure and component tests.
- size: M
- branch: slice/20-edit-state

### S-21 Bookmark edit window: keys, project choice, and command
- status: blocked
- deps: S-20, S-08, S-17
- FR-IDs: FR-EDIT-02, FR-EDIT-07, FR-EDIT-08, FR-EDIT-09, FR-EDIT-10
- scope: `app/bookmark_cmd.rs` (`bm edit`), `domain/projects.rs` (project containing a directory), `tui` edit window screen (6.2, 6.3 table).
- acceptance: enter toggles the bookmark of the row without closing (FR-EDIT-02), the project root row does nothing (D-15); escape closes (FR-EDIT-07);
  the project is the one containing the current directory or the given directory, a project contains itself (FR-EDIT-08, A-14); `--session-scope` uses
  the current session's directory, and outside a session gives `--session-scope requires running inside a session.` (FR-EDIT-09, D-16); a directory
  outside every project gives `<path> is not inside a project from search_paths.` (FR-EDIT-10); right and left arrows and ctrl-l, ctrl-h, backspace as in
  6.3 (ctrl-h is collapse, A-04/6.5). TUI model tests and component tests.
- size: M
- branch: slice/21-edit-command

### S-22 Tree node actions (alt keys) and tree window behaviour
- status: blocked
- deps: S-05, S-14, S-15
- FR-IDs: FR-ACT-06, FR-ACT-08, FR-ACT-09, FR-TREE-12
- scope: `app/actions.rs` (node actions over `domain/sessions.rs` node membership), tree picker wiring in `app/pickers.rs`, `tui/keys.rs` (alt keys).
- acceptance: alt-r rebuilds every managed session under the node with its own recorded profile even when `--profile` was given (FR-ACT-08, A-38); alt-p asks
  one profile and rebuilds every session under the node (FR-ACT-09); alt-x kills every managed session under the node (FR-ACT-06); the session the user is
  in is never touched by alt keys (FR-ACT-07); the tree window stays open when its last entry is removed (FR-TREE-12); node membership uses the session
  directory (A-37, FR-SCOPE-02). Component and TUI model tests.
- size: M
- branch: slice/22-node-actions

### S-23 Session tree command
- status: blocked
- deps: S-22, S-11
- FR-IDs: FR-SESS-08
- scope: `app/session_cmd.rs` (`ss tree`), session tree screen (6.2, 6.3 table), `ss tree` in `cli` wiring.
- acceptance: enter on a session-tree entry with a directory opens that directory's project through the open flow (FR-SESS-08); gray folders without a session open
  and show their would-be name (A-13); connectors do not open; empty state `No ledge sessions.` stays open (D-12); `ss tree` with a directory gives
  `ss tree takes no directory argument.` (S-17 parsing, reused). TUI model tests.
- size: S
- branch: slice/23-session-tree

### S-24 Bookmark tree command
- status: blocked
- deps: S-22, S-18
- FR-IDs: FR-TREE-05, FR-TREE-06, FR-TREE-07, FR-TREE-08, FR-ACT-10
- scope: `app/bookmark_cmd.rs` (`bm tree`), bookmark tree screen, `domain/tree.rs` (scope selection).
- acceptance: only the tree containing the given directory with the cursor on its nearest node (FR-TREE-05); `--session-scope` shows the tree of the current session's
  directory (FR-TREE-06); a directory outside every tree shows only the root directory (FR-TREE-07, A-13); no argument shows all trees (FR-TREE-08); alt-d deletes
  every bookmark under the node and kills their managed sessions (FR-ACT-10); empty tree shows `No bookmarks.` and stays open (D-12). TUI model and component tests.
- size: M
- branch: slice/24-bookmark-tree

### S-25 Session list and kill commands
- status: blocked
- deps: S-14, S-09, S-17
- FR-IDs: FR-SESS-09, FR-SESS-10, FR-SESS-11, FR-SESS-12, FR-SESS-13
- scope: `app/session_cmd.rs` (`ss list`, `ss kill`), table formatting in `tui/columns.rs` reused or in `app/session_cmd.rs`.
- acceptance: `ss list` table of name, windows, state (attached or detached), path, header row, sorted by name, `No ledge sessions.` when none (FR-SESS-09, A-34);
  `ss kill NAME` kills the exact managed name (FR-SESS-10); otherwise the directory's session (A-20); a non-managed or unknown target gives
  `'<name>' is not a ledge session.` (FR-SESS-11); success prints `Killed session '<name>'.` (FR-SESS-12); no target gives `kill requires a session name or directory.`
  (FR-SESS-13); the current session is refused with `cannot kill the session you are in.` (A-20). Component tests.
- size: S
- branch: slice/25-session-list-kill

### S-26 Doctor
- status: blocked
- deps: S-02, S-03, S-04, S-09, S-17
- FR-IDs: FR-DOC-01, FR-DOC-02, FR-DOC-03, FR-DOC-04, FR-DOC-05, FR-DOC-06, FR-DOC-07, FR-DOC-08
- scope: `app/doctor.rs`, `cli` wiring of `doctor`.
- acceptance: the ten checks of architecture 8.4 in that order, with the exact lines (FR-DOC-01 includes setsid, code, idea as WARN; tmux ERR when missing or
  older than 3.2); config exists and parses (FR-DOC-02); search path warnings (FR-DOC-03); bookmark warnings (FR-DOC-04); each profile file checked (FR-DOC-05);
  default profile checked (FR-DOC-06); each managed session with missing directory or missing profile warned (FR-DOC-07, skipped with a WARN line when tmux is
  missing); `OK`, `WARN`, `ERR` prefixes padded to six characters, exit 1 on any ERR (FR-DOC-08); doctor never creates the config (A-12). Component tests.
- size: M
- branch: slice/26-doctor

### S-27 Install and uninstall script
- status: blocked
- deps: S-00
- FR-IDs: none (the installation statements have no FR-IDs in `requirements.md`; see proposal P-1). Traced by its acceptance list until P-1 is accepted.
- scope: `scripts/install.sh`, `scripts/test_install.sh`, gate wiring in `scripts/gate.sh`.
- acceptance: no cargo gives `Error: build toolchain is required to build ledge.` exit 1 (requirement text); install copies the binary to
  `$LEDGE_INSTALL_BIN_DIR` (default `~/.local/bin`) and writes the source record; PATH warning `Warning: <dir> is not on PATH; add it to use ledge.` (A-21);
  `--uninstall` removes only when the record names this folder, prints `Removed <path>`; other source gives `Not removed: <path> was not installed from <source>`
  exit 1; missing binary gives `Nothing to remove: <path> does not exist`; config, profiles, history and bookmarks untouched. `test_install.sh` runs all of these
  with a fake `cargo` on PATH and temp folders.
- size: M
- branch: slice/27-install

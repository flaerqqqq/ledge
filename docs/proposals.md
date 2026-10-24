# Proposals

Agents append proposed changes to user-owned docs here. The coordinator escalates open ones.
The user resolves them (edits the doc, then marks the proposal `Accepted` or `Rejected`).

Format:

```
### P-<n> <title>
- from: <agent> / slice <id>
- doc: <requirements.md | docs/architecture.md | ...>
- change: <exact proposed change>
- why: <reason, with FR-IDs if relevant>
- status: Open | Accepted | Rejected
```

## Open

### P-1 Give the Installation statements FR-IDs and messages
- from: architect / Phase 0 (slice S-27)
- doc: requirements.md
- change: add an "Installation and removal (FR-INST)" group with these IDs, one per existing bullet, wording unchanged:
  FR-INST-01 build and install by the provided script into the local binary folder, which must be on PATH;
  FR-INST-02 warn when the local binary folder is not on PATH after install;
  FR-INST-03 record the source folder of the install;
  FR-INST-04 remove the binary on the uninstall option when it came from the same source folder, report "Removed <path>";
  FR-INST-05 refuse to remove a binary installed from elsewhere, "Not removed: <path> was not installed from <source>", exit 1;
  FR-INST-06 report "Nothing to remove: <path> does not exist" when no binary is present;
  FR-INST-07 fail with "Error: build toolchain is required to build ledge." when no build toolchain is present;
  FR-INST-08 leave the configuration, profiles, history and bookmarks in place on removal.
  Add the four installation messages to the Messages section, and state that the install script owns uninstall
  (the binary has no uninstall command). Warning text proposed: "Warning: <dir> is not on PATH; add it to use ledge."
- why: the Installation section has no FR-IDs, so it cannot be traced (analysis F-30). Architecture A-21 and slice S-27 use
  FR-INST-01..08 once this is accepted. Until then S-27 has no FR rows in traceability.
- status: Open

### P-2 FR-CMD-09 has no runtime program besides tmux
- from: architect / Phase 0 (slice S-17, architecture A-01)
- doc: requirements.md
- change: either keep FR-CMD-09 as a generic rule and state that tmux is the only required program (its message is FR-CMD-10),
  or reword it to name the program that uses it. The setsid, code and idea programs are optional (FR-LAUNCH, FR-DOC-01 WARN),
  so they do not trigger "<name> is required for this command.".
- why: a rule with no program to apply it to cannot be tested except through a helper (F-07, F-08). The architect implements the
  generic helper and its test; the change only states the intent.
- status: Open

### P-3 Messages added by the design
- from: architect / Phase 0 (architecture section 8.3)
- doc: requirements.md (Messages)
- change: confirm or reject these new user-facing messages, each with its trigger in architecture 8.3:
  "config is invalid (expected 'search_paths' to be a list of strings): <file>";
  "could not create config '<file>': <detail>";
  "config search path '<entry>' uses undefined variable '<VAR>'.";
  "HOME is not set.";
  "Warning: could not start editor '<command>': <detail>";
  "Warning: setsid is not installed; the editor is started without detaching.";
  "tmux 3.2 or later is required; found <version>.";
  "a terminal is required for this picker.";
  "Terminal too small: need at least 40x8.";
  "'<command words>' takes no arguments.";
  "Installed <path>";
  "Warning: <dir> is not on PATH; add it to use ledge.";
  and the doctor lines of architecture 8.4.
  Also confirm D-17 text "cannot kill the session you are in." and the "Error:" class of "No search paths defined in <config file>." (FR-PICK-10).
- why: each of these is needed by a requirement that has no message (analysis F-07, F-26, F-27, F-24), and the Messages section is the
  only place where the user can see exact wording.
- status: Open

### P-4 Domain layer may use crate::error and crate::messages
- from: implementer / slice S-01
- doc: docs/architecture.md (section 3, layer table, domain row)
- change: in the domain row, "May use" column, replace the cell with:
  `core types and collections (`String`, `Vec`, `Option`), `serde_json` (Value), `md5`, `crate::error` (`LedgeError`, `Result`, `Outcome`), `crate::messages``
  and add after the table: "Domain may use `crate::error` and `crate::messages`: both are plain data and constant strings with no I/O."
  `scripts/check_layers.sh` already does not forbid them.
- why: domain code returns `LedgeError` and user-facing text from `messages`; the table as written forbids what the code needs (S-01 domain/path.rs, domain/naming.rs, domain/location.rs). Touches section 3 layering rules.
- status: Open

### P-5 Crate name md-5 in the domain row
- from: implementer / slice S-01
- doc: docs/architecture.md (section 3, layer table, domain row)
- change: replace `md5` with `md-5` in the domain row "May use" cell (the dependency name in Cargo.toml and section 10 is `md-5`; the Rust path stays `md5`).
- why: the crate that the S-01 scope adds is `md-5 = "0.10"`; naming the crate by its import path misleads readers about the dependency (FR-NAME-01, D-14).
- status: Open

### P-6 Port creation order: each port is created by its first user
- from: implementer / slice S-01
- doc: docs/architecture.md (section 4, port table, after the table)
- change: add this paragraph: "Port creation order: S-01 creates `Env` and `FileSystem` with their real adapters and fakes. Each other port is created by the first slice that uses it: `Signals` by S-08, `ProcessRunner` by S-09, `Console` by S-11 (it emits the history warning through `Console::err`, FR-OPEN-11; S-12 emits the editor warnings, FR-LAUNCH-03), `Clock` by S-16 (age text, FR-SESS-03)."
- why: S-01 acceptance criteria need only Env and FileSystem; creating the other ports earlier adds traits and fakes with no test that uses them. Touches the port table in section 4 and slice order in docs/backlog.md.
- status: Open

### P-7 Backlog S-17 needs S-09 as a dependency
- from: implementer / slice S-01
- doc: docs/backlog.md (S-17 entry)
- change: `- deps: S-01` becomes `- deps: S-01, S-09`.
- why: S-17 uses `ProcessRunner::find_program` for the tmux and command presence checks (FR-CMD-09, FR-CMD-10, tmux version check), and `ProcessRunner` is created by S-09 (see P-6). The reviewer found the missing dependency.
- status: Open

### P-8 Backlog S-01 scope lists only what S-01 creates
- from: implementer / slice S-01
- doc: docs/backlog.md (S-01 entry, scope line)
- change: replace the scope line with: "`ports/fs.rs`, `ports/env.rs` (traits only), `adapters/fs.rs`, `adapters/env.rs`, `domain/path.rs`, `domain/naming.rs`, `domain/location.rs`; `md-5` dependency added here; `tests/support/fake_env.rs`, `tests/support/temp.rs`. The clock, process, console and signals ports are created by the slices that first use them (see P-6)."
- why: no S-01 acceptance criterion needs the clock, process, console or signals ports, and the scope line lists them as created here. Narrowing it keeps S-01 to FR-PLAT-02, FR-CONF-08, FR-OPEN-05, FR-OPEN-06, FR-NAME-01, FR-NAME-02.
- status: Open

### P-9 Messages S-02 needs beyond the architecture 8.3 table
- from: implementer / slice S-02
- doc: docs/architecture.md (section 8.3 table, section 9.2) and requirements.md (Messages)
- change: add these messages, each class error, all in `messages.rs` and in `tests/messages_verbatim.rs`:
  "config is invalid (expected 'default_profile' to be a string): <file>" (trigger: `default_profile` present and not a string, architecture 9.2);
  "could not read config '<file>': <detail>" (trigger: `config.json` exists but cannot be read);
  "could not read profile '<name>' at <path>: <detail>" (trigger: profile file exists but cannot be read).
  Also note in architecture 8.3 that "could not create config '<path>': <detail>" is used for the config folder and the profiles folder as well as the file.
- why: architecture 9.2 requires a type check on `default_profile` and the table in 8.3 has no text for it; the read failures of an existing file need a message that names the situation (conventions, Errors: an io::Error is never shown raw). S-02 scope names the config and profile messages. Added [new] strings are a message change, so the user should confirm them with P-3.
- status: Open

### P-10 S-03 failed-write test: reword the acceptance to the temp-file step
- from: implementer / slice S-03
- doc: docs/backlog.md (S-03 acceptance)
- change: replace "tested by a write that fails after the temp file and leaves the old file" with "tested by a write that
  fails at the temp-file step (the temp file cannot be created) and leaves the old file unchanged".
- why: with `RealFs` and no fake file system, a failure after the temp file exists cannot be produced while the target
  still holds an old file (rename fails only when the target is a folder). The S-03 test (tests/component_store.rs,
  failed_history_write_leaves_old_file_unchanged) forces the failure with a folder at the temp-file name (architecture 9.3
  name, `.history.tmp-<pid>`). A seam in `FileSystem` for a rename failure would also work, but adds a port method (architecture 4).
- status: Open

### P-11 S-03 branch name in the backlog
- from: implementer / slice S-03
- doc: docs/backlog.md (S-03 `branch:` line)
- change: `slice/03-history-bookmark-stores` becomes `slice/S-03-history-bookmarks`, the branch in use and the example in
  docs/conventions.md (Git).
- why: conventions give `slice/<id>-<name>`; the branch already checked out for this slice uses the other spelling.
- status: Open

### P-12 Tree rules the spec does not state (bookmark equal to a project, row order)
- from: implementer / slice S-05
- doc: docs/architecture.md (section 5 and A-13) and requirements.md (Bookmark tree and session tree)
- change: add to A-13: (a) a bookmark whose path equals a project path is that project's root row, kind Bookmark, openable and not dimmed (it is not a gray root); (b) rows of a tree are in tree preorder: the blocks are ordered by root path, and inside a block siblings and their subtrees are ordered by path components (`a/x` before `a-b`), not by characters; the caller does not sort.
- why: domain/tree.rs needs both rules. A-13 says the bookmark-tree root is gray and not openable, which does not cover a bookmark equal to a project (FR-BM-02 refuses adding a project, so this only comes from an edited file). Row order is not stated anywhere (requirements: "The list opens with the tree top-down and does not sort."). Tests: domain/tree.rs, bookmark_equal_to_project_is_its_own_openable_root and rows_follow_tree_order_by_path_components.
- status: Open

### P-13 S-05 branch name in the backlog
- from: implementer / slice S-05
- doc: docs/backlog.md (S-05 `branch:` line)
- change: `slice/05-trees` becomes `slice/S-05-trees`, the branch in use and the form in docs/conventions.md (Git).
- why: same as P-11 for S-03; the branch already checked out for this slice uses the other spelling.
- status: Open

### P-14 Picker model fields added in S-07 (architecture 6.1, 6.2, 4)
- from: implementer / slice S-07
- doc: docs/architecture.md (sections 4, 6.1, 6.2, 6.4)
- change: record the S-07 shapes that the code uses, in place of the older wording:
  (a) `PickerRow` has `preselect: bool` (the row FR-PICK-04 starts on; 6.1 refers to it but the field list omits it).
  (b) Chrome keeps the column titles in `columns: Vec<ColumnSpec>`, where `ColumnSpec` is `{title, fixed_width, path}`
      (the marker column is `fixed_width` 2; `path` marks the column cut from the left), in place of `headers`.
  (c) Chrome adds `missing_column: Option<usize>` (where the ` (missing)` cue goes) and `stay_open_when_empty: bool`
      (session switcher and session tree, D-12). `search_column` and `empty_message` are as in 6.1.
  (d) `PickerModel::apply(Action) -> Effect`, with `Effect = Handled | Choose | Cancel | Act(Action)`. An enter with no
      visible row is `Cancel` (F-14). `Action` is one flat enum in `tui::keys` (movement, filter edits, choose, cancel,
      and the eight row and node operations of 6.3).
  (e) List height is the terminal height minus 6 (border 2, prompt, header, status, footer); below 40x8 only the
      too-small line is drawn. `render::draw(model, area, frame, color)` takes the color switch from
      `render::color_enabled(NO_COLOR, TERM)`.
  (f) `tui::terminal` holds `Input { Key(KeyEvent), Resize(u16, u16) }`, `TerminalSession::{poll(timeout), draw(&mut dyn FnMut(&mut Frame))}`
      and `TerminalFactory::open()`. The key type is ratatui's crossterm re-export, so no second crossterm dependency.
  (g) Shrink order: the last column is cut to 6 first (12 when it is also the search column), then the search column
      to 12, then the other columns to 4, right to left. Column gap is 1 cell.
- why: the code needs these fields to make the 6.1 rules testable (preselection, the switcher staying open, the
  missing cue, fixed marker width). The rules themselves do not change. S-08 builds `run_picker` on these shapes.
- status: Open

### P-15 History picker footer: requirements.md and architecture 6.2 differ
- from: implementer / slice S-07
- doc: requirements.md (Screens and interactions, History picker)
- change: say the history footer is "enter open · ^r rebuild · ^p profile · ^x kill" (architecture 6.2 and A-03), or keep
  "enter open" and accept that FR-TUI-04 (a footer listing its keys) is then met only in part.
- why: requirements.md gives the history footer as "enter open" while it also says "Same keys as the project picker", and
  FR-TUI-04 requires a footer that lists the keys. Architecture 6.2 follows FR-TUI-04. S-07 does not set the history footer
  (the footer strings come from the picker services in later slices), so no code depends on the choice yet.
- status: Open

### P-16 Message for terminal read and write failures
- from: implementer / slice S-08
- doc: docs/architecture.md (section 8.3, Messages)
- change: add to the `[new]` table: `could not use the terminal: <detail>` | error | drawing or reading the terminal fails (FR-TUI-06). Add it to the `[new]` list with a single function `messages::terminal_failed(detail)`.
- why: an `io::Error` must never be shown raw, and 8.3 has no text for a failed terminal call. The engine (`run_picker`) and `CrosstermFactory::open` return this text. Implemented as written in `messages.rs` and checked in `tests/messages_verbatim.rs`.
- status: Open

### P-17 run_picker takes the color switch as a parameter
- from: implementer / slice S-08
- doc: docs/architecture.md (section 6.4)
- change: the engine signature becomes `run_picker(model, handler, term, signals, color: bool)`, where `color` is `render::color_enabled(NO_COLOR, TERM)` computed by the caller (app). Section 6.4 has no `color` argument.
- why: the tui layer may not read the environment (section 3), and the color rule (FR-TUI-09, A-04) is already a pure function that the caller evaluates. Without the argument the engine cannot draw in color. The loop and the drop-based restore are as in 6.4.
- status: Open


### P-18 set-option target with a trailing colon (architecture 7.1)
- from: implementer / slice S-09
- doc: docs/architecture.md (section 7.1, table row "set option")
- change: the set-option row reads `set-option -t =<name>: @ledge_managed 1`, `set-option -t =<name>: @ledge_profile <profile>`, `set-option -t =<name>: @ledge_dir <dir>`. Other session commands keep `=<name>`.
- why: on tmux 3.5a, `set-option -t =<name>` fails with `no such session: =<name>` while `set-option -t =<name>:` sets the session option (checked with a scratch server). `new-window -t =<name>`, `kill-session -t =<name>`, `rename-session -t =<name>` and `switch-client -t =<name>` work with the bare form. The trailing colon keeps the exact-name match. Not yet checked on tmux 3.2 (the minimum); the S-09 integration tests run on the installed tmux only.
- status: Open

### P-19 S-09 ProcessRunner and tests: what S-09 creates
- from: implementer / slice S-09
- doc: docs/architecture.md (section 4, `ProcessRunner` row) and docs/testing.md (test layout)
- change: (a) `ProcessRunner` in S-09 has `find_program`, `capture` and `inherit`. `spawn_detached` is not added here; S-12 adds it with the editor launch, its first user (P-6 rule). (b) The test layout gains `tests/component_tmux.rs` (fake-process argv of the tmux commands, FR-PLAT-01, FR-NAME-03/04); the table in testing.md does not list it.
- why: P-6 says each port is created by its first user, so a method with no caller in S-09 is deferred the same way. The argv checks belong with the other component tests and need `FakeProcess` from tests/support.
- status: Open

### P-20 S-10 build module: Ctx, option constants in the port, test file name
- from: implementer / slice S-10
- doc: docs/architecture.md (section 2 module tree, section 4 `Ctx`, section 7.1), docs/testing.md (test layout)
- change: (a) `app::build` takes `&dyn SessionBackend` (and `&AbsPath`, `&Profile`) as parameters. `app::Ctx` is not created in S-10: it bundles all ports and arrives with the open flow (S-11), the first caller that needs more than the backend. (b) The option names `@ledge_managed`, `@ledge_profile`, `@ledge_dir` move from `adapters/tmux.rs` to `ports/backend.rs` (`OPT_MANAGED`, `OPT_PROFILE`, `OPT_DIR`); `adapters/tmux.rs` re-exports them, so `tests/component_tmux.rs` is unchanged. Reason: `scripts/check_layers.sh` forbids `app` from naming `adapters`. (c) The test layout gains `tests/component_build.rs` (FR-OPEN-07, FR-PROF-03..08, FR-NAME-03, D-14, A-07 over `FakeBackend`); the table in testing.md lists `component_open.rs` for FR-OPEN and does not list this file. (d) The messages `could not create session '<name>': <detail>`, `... name already used by <dir>` and `Required tool 'tmux' is not installed.` are functions in `messages.rs` with checks in `tests/messages_verbatim.rs`.
- why: the build flow needs only the backend port; creating `Ctx` now would wire unused ports (P-6 rule). The layer check makes the constants a port concern. Test file naming follows P-19.
- status: Open

### P-21 S-11 Ctx and Console: which ports exist when the open flow lands
- from: implementer / slice S-11
- doc: docs/architecture.md (section 4 `Ctx` and port table, section 2 module tree)
- change: `app::Ctx` is created in S-11 with the fields the open flow uses: `env`, `fs`, `backend`, `console`, `dirs`, `cwd`. The fields `process`, `clock`, `terminal`, `signals` and `home` are not added here; each arrives with its first user (P-6). The `Console` port (`ports/console.rs`, `out` and `err`) is created here because the open flow prints warnings before it attaches. Its real adapter `StdConsole` and the `main` wiring of `Ctx` arrive with S-17. The open flow's ProfileOpt type lives in `domain/profile_rules.rs` (the architecture 5 and 2.3 name it `ProfileOpt` under cli::parse); S-17 imports it from there.
- why: P-6 and P-20 say each port and field is created by its first user. The open flow needs the console for FR-OPEN-11 warnings, and precedence (D-13) is a domain rule, so it is pure and sits with profile rules.
- status: Open

### P-22 S-11 open-time decisions that need the user's confirmation
- from: implementer / slice S-11
- doc: requirements.md (FR-PROF-11, FR-PROF-12, FR-OPEN-08) and docs/architecture.md (section 6, 7.1)
- change: (a) The profile is resolved on every open, also when the session already exists, so a bad `--profile` fails and a bad `default_profile` warns even when the session is only attached (the profile is still applied only at creation, FR-PROF-08). Alternative: resolve only when a session is created. (b) A failed switch-client inside tmux uses the attach message `could not attach to '<name>': <detail>` (the requirements name only the attach failure). (c) FR-OPEN-08 is marked `manual` in traceability, not `covered`, because the real attach and switch need a tmux client on a tty; the automated decision and command are covered by tests/component_open.rs.
- why: the Messages section gives one attach text and FR-PROF-11/12 do not say whether an existing session still triggers them. These are the open-flow choices made in S-11 so the slice could proceed; the user can reverse any of them.
- status: Open

### P-23 S-12 editor launch: design choices the spec does not state
- from: implementer / slice S-12
- doc: docs/architecture.md (sections 2.3, 4, 5, 8.3, 13 A-18) and requirements.md (FR-LAUNCH-01..03, FR-CMD-06, FR-CMD-09)
- change: (a) `ProcessRunner::spawn_detached` is added as P-19 planned; `SystemProcess` waits only when the program is `setsid`. (b) `app::Ctx` gains `process: &dyn ProcessRunner` (first user: the launch, P-21 rule). (c) The open flow gets a new function `app::open::open_dir_with_editor(ctx, dir, option, Option<&Editor>)`; `open_dir` keeps its signature and calls it with `None`. S-17 parses the option and calls the new function. (d) `Editor { Code, Idea, Command(Vec<String>) }` lives in `app/launch.rs`, not `cli::parse` as architecture 5 names it; S-17 imports it from there (as P-21 does for `ProfileOpt`). (e) Before launching, the editor program is looked up with `find_program`; when it is not on PATH the warning is `could not start editor '<command>': command not found` (the detail text `command not found` is new; 8.3 does not give one). This check is needed because `setsid -f` returns 0 even when the program is missing. (f) The `setsid` warning is printed only when an editor is actually launched, and only after the editor is found. (g) FR-CMD-09 (`<name> is required for this command.`) is not used by the editor path: a missing editor is the A-18 warning, and the session still opens. FR-CMD-09 and FR-CMD-06 stay with S-17.
- why: the S-12 acceptance needs the open path to launch only on creation (FR-LAUNCH-01), and the spec gives no detail text for a missing editor. Each choice is the smallest one that passes the S-12 tests; the user can reverse any of them.
- status: Open

### P-24 S-13 picker services: Ctx fields, entry points and the [new] messages
- from: implementer / slice S-13
- doc: docs/architecture.md (sections 2, 4, 6.4, 8.3) and docs/backlog.md (S-13 scope)
- change: (a) `app::Ctx` gains `terminal: &dyn TerminalFactory` (tui::terminal) and `signals: &dyn Signals` (ports::signals); the picker services are their first users (P-21 rule). (b) The entry points are `app::pickers::project_picker(ctx, dir: Option<&str>, option: &ProfileOpt) -> Result<Outcome>` and `app::pickers::history_picker(ctx, option: &ProfileOpt) -> Result<Outcome>`. The directory is the launch directory of FR-PICK-04 (the current row, first and preselected); `None` is the working directory. The CLI rejects a directory argument on `project pick` (FR-PICK-11, S-17), so this argument is for callers and tests. The profile option is added for FR-PICK-12. (c) The editor options (`--code`, `--idea`, `--editor`), which A-18 says apply to pickers, are not passed by these entry points; S-17 adds them with `open_dir_with_editor`. (d) [new] messages, to be confirmed: `could not read history: <detail>` (history file unreadable) and `could not list sessions: <detail>` (tmux list failure other than missing tmux). (e) `app::store` gains `config_path` and `history_path` (architecture 9.1). (f) Row actions ctrl-r, ctrl-p, ctrl-x, alt-* and ctrl-d return no change in S-13; S-15 wires them. (g) The history footer and the project footer are both `enter open · ^r rebuild · ^p profile · ^x kill` (architecture 6.2 and A-03), so P-15 stays open with that choice.
- why: the S-13 acceptance needs a picker that runs from the library and opens through the S-11 flow. The picker needs the terminal and interrupt ports, which the first picker users create (P-6 rule). Editor launch from pickers and the directory argument belong to the CLI slice.
- status: Open

### P-25 S-13 picker rules the spec leaves open or conflicts on
- from: implementer / slice S-13
- doc: docs/decisions.md (D-12), docs/architecture.md (A-16, 6.2) and requirements.md (FR-PICK-13)
- change: (a) D-12 says an empty project list at start prints `No projects found in search paths.` (info, no window). A-16 always shows the launch directory as a first row, so the picker would never be empty. S-13 follows D-12: when the search paths list no projects, the info line is printed and no window opens; with at least one project the A-16 rows apply. Confirm or reverse. (b) FR-PICK-13 "when a non-basic profile applies": S-13 counts `default_profile` as applying (open_profile, F-18 precedence), so the label names it. `basic` never gives a profile label. (c) Project rows: the marker cell is the launch-directory flag `*` or a space, then the live flag `●` or `○`; the Project cell is the basename, or the path for a launch directory that is not a project; the Session and Profile cells come from the managed session of the same name (D-14), empty when none. (d) History rows: the Project cell is the cleaned absolute path (architecture 6.2 path cells), with no marker column. (e) A chosen directory that is missing is reported with `Missing directory: <path>` in the status line; the history picker dims it and shows ` (missing)` (A-27).
- why: each choice is the smallest one that passes the S-13 tests; the user can reverse any of them. (a) is a conflict between D-12 and A-16 that only the user can settle.
- status: Open

### P-26 [new] messages for a failed kill and a failed rename during a rebuild
- from: implementer / slice S-14
- doc: requirements.md (Messages) and docs/architecture.md 8.3
- change: confirm two [new] error messages (stderr, `Error: ` prefix, exit 1; in pickers the status line):
  `could not kill session '<name>': <detail>` (a kill of the old session fails during a rebuild or kill, D-17, A-07) and
  `could not rename session '<old>' to '<new>': <detail>` (the rebuilt copy cannot take the original name, D-17).
  Both are in `messages.rs` and in `tests/messages_verbatim.rs`.
- why: architecture 7.2 says the caller formats "the kill/rename message" but gives no text, so the rebuild of the current
  session (FR-ACT-05, D-17) has no literal message for those two failures. The copy-name collision reuses
  `could not create session '<name>-rebuild': name already used` from A-07.
- status: Open

### P-27 S-14 rebuild and kill rules the spec leaves open
- from: implementer / slice S-14
- doc: docs/architecture.md (A-07, A-20, A-38) and docs/backlog.md (S-14, S-22, S-25)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) kill of a name with no session is a no-op (`Outcome::Done`). The picker row has nothing to kill (S-15). `ss kill` (S-25) keeps the FR-SESS-11 error for an unknown target.
  (b) kill refuses the session the user is in with `cannot kill the session you are in.` on every kill path (ctrl-x, `ss kill`). The alt-key paths (S-22) skip it silently through `never_touch_current` and do not print this message; FR-ACT-07 does not say how the alt-key report looks.
  (c) rebuild refuses a managed session whose recorded directory (`@ledge_dir`) is another directory, with the D-14 text `could not create session '<name>': name already used by <dir>`. A-24 states the collision rule for open only.
  (d) the profile is resolved before anything is killed, so a bad `--profile` (FR-PROF-11) leaves the session running.
  (e) rebuild of the current session: when the switch to the copy fails, the copy is killed and the error is `could not attach to '<copy>': <detail>`. A-07 covers build failures only.
  (f) D-04 warning order: a recorded profile that fails gives the FR-PROF-15 warning, then the default profile; a default that also fails gives the FR-PROF-12 warning and the single window. Each warning once per rebuild.
- why: each choice is the smallest one that passes the S-14 tests; none changes an accepted decision. Items (b) and (e) are the open edges.
- status: Open

### P-28 S-15 picker engine: ctrl-p hands the row to the caller
- from: implementer / slice S-15
- doc: docs/architecture.md (6.4 engine loop, the `Step` and `PickerResult` listings; 6.3 ctrl-p row)
- change: (a) `tui::engine::Step` gains `Step::Profile`, returned by `act` for ctrl-p on a row. The engine returns `PickerResult::Profile(row)` for it (`PickerResult` gains that variant). `Step::Profile` with no row is a no-op. (b) The caller (`app::pickers`) then runs the profile picker in a new terminal, applies the choice with the rebuild service (FR-ACT-04, FR-ACT-12), and opens the directory picker again on a new terminal with its rows reloaded and the acted row selected. The profile picker is never nested inside the engine loop, because the engine owns the one terminal. Architecture 6.4 lists the three `Step` variants and three `PickerResult` variants; this is the change to those listings.
- why: the engine owns the terminal and returns only a result, so a second picker cannot run inside its loop without a redesign. The smallest change keeps the policy in the handler and the loop in the engine. Tests: tests/tui_engine.rs (profile_step_returns_the_row_as_a_profile_request), tests/tui_project_picker.rs (profile_* and escape_in_the_profile_picker_*).
- status: Open

### P-29 S-15 picker action rules the spec leaves open
- from: implementer / slice S-15
- doc: requirements.md (Screens and interactions, Profiles FR-PROF-13, FR-ACT-02), docs/architecture.md (6.2, A-11, A-08) and docs/backlog.md (S-15)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) after a profile choice or an escape in the profile picker, the directory picker reopens with its rows reloaded and the acted row selected; the filter text is cleared (A-08 says "no change"; keeping the filter needs the engine to return the model, see P-28).
  (b) ctrl-r on a directory that is gone shows `Missing directory: <path>` and builds nothing, as enter does (FR-PICK-09). The spec says ctrl-r is "Always" and says nothing about a missing directory.
  (c) a profile row shows its labels as the row text, joined with one space when both apply: `[Current: dev] [Default: dev]`, `[Current: ops]`, `[Default: web]`; other rows show the profile name. FR-PROF-13 names the labels but not the row layout.
  (d) the profile list is the profile files whose names are valid (D-13), plus `basic`, minus those already listed as current or default. A current or default profile is listed even when its file is missing, so choosing it reports the profile error (A-11).
  (e) ctrl-x on a row without a managed session does nothing and does not reopen the list; ctrl-x on the session the user is in shows `cannot kill the session you are in.` in the status line (D-17), as P-27 (b) already states.
- why: each choice is the smallest one that passes the S-15 tests; none changes an accepted decision. (a) and (c) are the visible ones.
- status: Open


### P-30 S-16 session switcher rules the spec leaves open
- from: implementer / slice S-16
- doc: requirements.md (Screens and interactions, Session switcher; FR-SESS-02, FR-SESS-03), docs/architecture.md (6.2, A-10, A-37) and docs/backlog.md (S-16)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) the project column shows the name of the project that contains the session directory (`project_containing` over the search paths, FR-PICK-04). A session directory outside every search path (D-07) leaves the column empty. The directory is `@ledge_dir`, else tmux's `session_path` (A-37).
  (b) ctrl-r in the switcher rebuilds the session in its directory with its recorded profile (`ProfileOpt::Unset`). The switcher has no `--profile` option in S-16. A directory that is gone shows `Missing directory: <path>` and builds nothing (P-29 b).
  (c) the switcher reads the config for the search paths, so a config error fails the switcher with that message, as the open flow does.
  (d) escape cancels the switcher with `Cancelled.` (exit 0). Enter attaches or switches and returns `Outcome::Done`. Enter on an empty list cancels (A-08).
- why: each choice is the smallest one that passes the S-16 tests in tests/tui_switcher.rs and the unit tests in domain/sessions.rs and domain/age.rs. None changes an accepted decision (D-07, D-08, D-12).
- status: Open

### P-31 S-16 switcher has no directory cell for the missing cue; session_cmd location
- from: implementer / slice S-16
- doc: docs/architecture.md (6.2 "Missing directory" cue, 2 module list for app/session_cmd.rs) and docs/backlog.md (S-16 scope)
- change: (a) architecture 6.2 puts the ` (missing)` suffix on the Directory or Bookmark cell. The switcher has no such column (session, project, profile, activity). The implementation dims a row whose session directory is gone (FR-TUI-08) but adds no text cue, so with `NO_COLOR` the missing state is not shown. Proposed: the suffix goes on the project cell when the directory is gone, or the session cell. (b) the switcher service is `app::pickers::session_switcher`. `app/session_cmd.rs` is not created in S-16, because the `ss` command glue (the CLI route, FR-SESS-05) arrives with S-17 and S-23/S-25 add the list and kill subcommands. The backlog S-16 scope names `app/session_cmd.rs`; this is the change to that scope.
- why: the screen table in requirements.md gives no missing cue for the switcher and FR-TUI-09 asks for readable cues without color. The layout is a doc gap, not a decision. The module split follows the smallest change that keeps the picker services in one file.
- status: Open

### P-32 S-17 commands not built yet: the plain error and the argument checks left open
- from: implementer / slice S-17
- doc: requirements.md (Messages), docs/architecture.md (8.3, 2.3) and docs/backlog.md (S-17 scope)
- change: (a) the commands whose slice is later (history list and clean, bm add, remove, list, pick, edit, tree, ss list, ss tree, ss kill, doctor) parse, and the binary then fails with `'<command words>' is not available yet.` on stderr, exit 1 (FR-CMD-07). The text is `messages::command_not_available`. No message in requirements.md or architecture 8.3 fits, so this text is new; the user may give a better one. (b) Argument checks that the spec does not give a message for are not decided here, and these commands return the (a) text: `bm remove` with two directories, `bm edit` with two directories, `ss kill` with any count (its "kill requires a session name or directory." is FR-SESS-13, S-25). (c) `bm add`, `bm remove`, `bm tree` and `ss tree` argument messages are checked in S-17 because the spec names them in S-17's scope, and `bookmark remove requires a directory.` (FR-BM-10) is checked here too.
- why: the S-17 gate needs a binary that runs the commands built so far and says something clear about the rest, and the rule for messages is "do not invent". The argument checks are the S-17 scope list; the others need the slice that owns them.
- status: Open

### P-33 S-17 start-up messages with no text in the spec
- from: implementer / slice S-17
- doc: requirements.md (Messages), docs/architecture.md (8.3) and docs/backlog.md (S-17 scope)
- change: three [new] texts, each used by `main.rs` or `cli/mod.rs` at start, all errors (exit 1): `could not read the working directory: <detail>` (`env.cwd()` fails); `could not register interrupt handlers: <detail>` (`SignalFlags::new` fails, FR-TUI-06); `could not read the tmux version: <detail>` (`tmux -V` output has no version number). Each has a function in `messages.rs` and a literal test in `tests/messages_verbatim.rs`.
- why: there is no text for these failures in the spec, and the composition root needs one to report them. The user may reword them or add them to architecture 8.3.
- status: Open

### P-34 S-17 grammar and start-up rules the spec leaves open
- from: implementer / slice S-17
- doc: docs/architecture.md (2.3, 9.1) and docs/backlog.md (S-17 scope)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) `ss` takes the open options (2.3 table) and accepts them, but the switcher has no profile or editor input (P-30 b), so they are ignored. An editor is not launched from the switcher, because it never creates a session (A-18 names `ss`, but the launch rule of FR-LAUNCH-01 cannot apply to it).
  (b) the project and history pickers take the editor option (A-18); `project_picker_with_editor` and `history_picker_with_editor` pass it to `open_dir_with_editor` for the chosen row. The pickers keep the old entry points, which pass no editor.
  (c) a value that starts with `-` is not a value: `--profile --code` and `--editor -x` are `--profile requires a profile name.` and `--editor requires a command.`.
  (d) `main` resolves the folders (`Dirs::resolve`) before the command line is parsed, so `ledge --help` fails with `HOME is not set.` when neither HOME nor an XDG variable is set. FR-PLAT-02 and architecture 9.1 say the folders are needed only when a command needs them. Proposed: resolve the folders inside the commands that use them, or let help run first. Not changed in S-17.
  (e) `bm edit DIR --session-scope` is not rejected at parse time; no FR gives a message for it, and S-21 (FR-EDIT-09) owns that rule.
- why: each choice is the smallest one that passes the S-17 tests in tests/component_cli.rs and the unit tests in src/cli/parse.rs. None changes an accepted decision (D-11, D-12, D-13, D-16).
- status: Open

### P-35 S-18 history and bookmark texts and rules the spec leaves open
- from: implementer / slice S-18
- doc: requirements.md (Messages, FR-HIST-06, FR-BM-08), docs/architecture.md (8.3, 9.4, A-19) and docs/backlog.md (S-18 scope)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) three [new] error texts, each with a function in `messages.rs` and a literal test in `tests/messages_verbatim.rs`: `could not clear history: <detail>` (`history clean` cannot delete the file; error, exit 1); `could not read bookmarks: <detail>` and `could not write bookmarks: <detail>` (the bookmarks file cannot be read or written; error, exit 1). They follow the wording of `could not read history: <detail>` (P-24). No text in the spec names these failures.
  (b) the `bm list` row is the status word padded to 16 columns, then the path, one bookmark per line in stored order (`ok              /p/app/src`). FR-BM-08 gives the words but no layout. The status precedence is: a directory that does not exist is `missing`; one that exists and lies in no project is `outside project`; otherwise `ok`. A bookmark that equals a project counts as inside it for the list (FR-BM-02 refuses adding one, so this only comes from an edited file).
  (c) the checks of `bm add`, in order: the directory must exist (`Directory '<dir>' not found.`, A-19, argument echoed as typed); an already bookmarked directory gives `Already bookmarked: <path>` and changes nothing, even when a project rule would now refuse it; then the project rules (FR-BM-04 before FR-BM-03).
  (d) `history list` marks no entry as missing (FR-HIST-03 gives no cue) and prints `No history.` for a missing file as well as an empty one (D-12).
  (e) `history clean` and `bm add`, `bm remove`, `bm list` are now built, so the proposed text of P-32 (a) no longer covers them; P-32 itself is not changed and stays open for the other commands.
- why: each choice is the smallest one that passes the S-18 tests in tests/component_history.rs, tests/component_bookmarks.rs, the unit tests in domain/bookmarks.rs and domain/path.rs, and the literal checks of tests/messages_verbatim.rs. None changes an accepted decision (D-06, D-12, D-16, A-14, A-19).
- status: Open

### P-36 S-19 bm pick accepts open options but ignores them
- from: implementer / slice S-19
- doc: docs/architecture.md (2.3 grammar: `ledge bm add [DIR] | remove DIR | list | pick [OPEN-OPTS]`), src/cli/parse.rs (S-17)
- change: the grammar gives `bm pick` the open options (`--profile`, `--no-profile`, `--code`, `--idea`, `--editor`). The parser validates them and
  then drops them (`Command::BmPick` has no options). S-19 wires `bm pick` with `ProfileOpt::Unset` and no editor, so `ledge bm pick --profile dev`
  opens the picker with the default profile. Proposed: `Command::BmPick(Options)` and pass `options.profile` and `options.editor` to the bookmark
  picker, as `project pick` and `history pick` do. Not changed in S-19 (parser is S-17 and FR-BM-11..13 do not name options).
- why: the grammar says `pick [OPEN-OPTS]`, and FR-PICK-12 gives the profile option to the pickers. Each choice keeps the S-19 tests green.
- status: Open

### P-37 S-19 backlog acceptance omits the current-session exception for ctrl-d
- from: implementer / slice S-19
- doc: docs/backlog.md (S-19 acceptance), docs/architecture.md (A-39), docs/decisions.md (D-17)
- change: the S-19 acceptance reads "ctrl-d deletes the bookmark and kills its managed session, then reopens the list (FR-BM-13, A-39)".
  Proposed wording: "ctrl-d deletes the bookmark and kills its managed session, except when that session is the one the user is in: then the
  bookmark is deleted, the session is kept, and the status line says `cannot kill the session you are in.` (D-17, A-39), then the list reopens."
  Implemented that way in S-19 (src/app/pickers.rs, delete_session_of). The architecture and D-17 already say so; only the backlog text is short.
- why: FR-BM-13 and the screen table say "kill its session" with no exception; D-17 and A-39 make the exception. The code follows D-17. Only the user
  can change the backlog text.
- status: Open

### P-38 S-20 project root row of the edit window is always expanded
- from: implementer / slice S-20
- doc: docs/backlog.md (S-20, FR-EDIT-05), docs/architecture.md (6.3: "enter toggle bookmark (not on the project root row)", D-09)
- change: the edit window lists the project root as row 0 and always lists its subfolders below it. The root is never recorded in
  `bm-edit-open`, so its expanded state is not remembered and no "+" or "-" depends on a saved entry. Its marker is "-" when it has
  subfolders. Left arrow on the root row is left to S-21 (6.3 "collapse, or move to parent"). Implemented that way in S-20
  (src/domain/bm_edit.rs, visible_rows).
- why: FR-EDIT-01 says the window lists the subdirectories of a project, so a collapsed root on first open would show nothing. No
  document says whether the root has an expansion state. D-09 keeps only expanded folders, and this choice adds no entry for the root.
- status: Open

### P-39 S-21 bookmark edit window: choices the spec leaves open
- from: implementer / slice S-21
- doc: requirements.md (FR-EDIT-02, FR-EDIT-07, FR-EDIT-08, FR-EDIT-09, Screens and interactions, Bookmark edit window), docs/architecture.md (6.2, 6.3, 6.5, 8.3) and docs/backlog.md (S-21)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) `bm edit DIR --session-scope` is refused with `--session-scope cannot be combined with a directory.` (the FR-TREE-10 text, reused). This closes P-34 (e). The grammar takes `bm edit [DIR] [--session-scope]`, so the refusal is a choice, not a given.
  (b) escape closes the edit window with no output (`Outcome::Done`), not with `Cancelled.`. The footer says "esc done" and FR-EDIT-07 says "close". D-12 prints `Cancelled.` for the other pickers only.
  (c) `--session-scope` uses the directory of the current session (FR-EDIT-09). A current session with no directory gives the same refusal as outside tmux, `--session-scope requires running inside a session.`.
  (d) `bm edit` with `--session-scope` requires tmux (A-29). Without the option it does not.
  (e) The expanded folders and the bookmarks are written at each change (enter, right, left), not when the window closes. A write failure shows in the status line and the window stays open (its `could not save the bookmark edit state: <detail>` text is [new]).
  (f) `could not read the bookmark edit state: <detail>` is [new] (edit state file unreadable, error, exit 1). Like P-35 (a), no text in the spec names it.
  (g) Right arrow on a folder that is not collapsed, and left arrow on the project root, do nothing. Left on a folder that is not expanded (or a leaf) moves to the parent row (D-15, architecture 6.3).
  (h) The window has no prompt row (architecture 6.2, D-15), so its list gets one row more than the picker chrome allows (`tui::engine::draw`).
  (i) A directory argument that does not exist is not refused by the window. It is looked up in the search paths like any path (FR-EDIT-08, A-14), so the window opens for the project that contains it, or refuses with FR-EDIT-10.
- why: each choice is the smallest one that passes the S-21 tests in tests/tui_edit_window.rs, tests/component_edit.rs, the unit tests in src/cli/parse.rs and src/tui/picker.rs, and tests/messages_verbatim.rs. None changes an accepted decision (D-09, D-12, D-15, D-16, A-14, A-29).
- status: Open

### P-40 S-21 edit window footer: requirements.md and architecture 6.2 differ
- from: implementer / slice S-21
- doc: requirements.md (Screens and interactions, Bookmark edit window: footer "enter toggle bookmark · ^l open · ^h close · esc done") and docs/architecture.md (6.2 and 6.3, footer "enter toggle bookmark · →/^l open · ←/^h close · esc done"; D-15 names the arrows)
- change: the implementation shows the architecture text, which names the arrow keys as well as ctrl-l and ctrl-h, and matches D-15. The requirements text is kept unchanged. Only the user can pick one, so the text in `messages::EDIT_WINDOW_FOOTER` follows the architecture until the user decides.
- why: the two documents disagree on the footer. D-15 and the architecture 6.2 table are the more specific ones, and the screen table is the one with the shorter key names.
- status: Open

### P-41 S-22 node actions and tree window: choices the spec leaves open
- from: implementer / slice S-22
- doc: requirements.md (FR-ACT-06..10, FR-TREE-11, FR-TREE-12, FR-SCOPE-02, Screens and interactions, Bookmark tree and session tree), docs/architecture.md (6.2, 6.3, 6.4) and docs/backlog.md (S-22, S-23, S-24)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) alt-r, alt-p and alt-x skip the session the user is in silently. No status line is shown (D-17 gives the status line only for ctrl-x and ctrl-d on that row). FR-ACT-07 is met either way.
  (b) A node action tries every session under the node. The first failure is the status line. Sessions are not skipped on failure.
  (c) A managed session whose name is not `session_name` of its directory (an adopted session) is left out of node actions. Rebuild finds sessions by directory name (D-14), so acting on it could build a second session.
  (d) alt-d on a bookmark node deletes every bookmark inside the node, the node itself included, and kills their managed sessions except the one the user is in. It is in the bookmark tree only (FR-ACT-10 is S-24).
  (e) The tree windows bind ctrl-r, ctrl-p and ctrl-x only on rows with a session. ctrl-d (delete the selected bookmark, screen table) is not bound in the bookmark tree window yet; it belongs to S-24 with the bookmark tree command.
  (f) The tree windows open every tree (`pickers::session_tree_picker`, `pickers::bookmark_tree_picker`). Scope selection (FR-TREE-05..08) and the commands `ss tree` and `bm tree` are S-23 and S-24.
  (g) alt-r and alt-p with no managed session under the node do nothing and show no status line ("Node has sessions" in the screen table).
- why: each choice is the smallest that passes tests/component_actions.rs (node tests) and tests/tui_trees.rs. None changes an accepted decision (D-12, D-14, D-17, A-37, A-38).
- status: Open

### P-42 S-22 engine: a step and a result for alt-p on a node
- from: implementer / slice S-22
- doc: docs/architecture.md (6.4 engine loop, `Step` and `PickerResult` description) and requirements.md (FR-ACT-09)
- change: `tui::engine` gains `Step::ProfileNode` (returned by `act`) and `PickerResult::ProfileNode(T)`, the node version of the existing `Step::Profile` and `PickerResult::Profile`. The caller asks one profile and rebuilds every session under the row's node. The architecture 6.4 text does not list these variants; the file is not edited here, so the architecture is left as it is until the user accepts this change.
- why: `Profile(T)` hands the row back for a one-session rebuild and carries no action, so the caller cannot tell alt-p from ctrl-p. Adding a variant is the smallest change that keeps the picker loop and the handler separate.
- status: Open

### P-43 S-23 ss tree: open options accepted, editor not used by the window
- from: implementer / slice S-23
- doc: docs/architecture.md (2.3 grammar: `ledge ss tree [OPEN-OPTS]`), docs/backlog.md (S-23), requirements.md (FR-SESS-06..08, Screens and interactions)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) `Command::SsTree(Options)` keeps the open options from the parser. `ss tree` passes `options.profile` to `pickers::session_tree_picker` (the enter-open flow uses the profile). `--editor`, `--code` and `--idea` are accepted by the parser and not used by the tree window, the same choice as P-36 for `bm pick`. The window opens no editor on enter.
  (b) The session tree window shows every managed session (all trees). The scope options of FR-TREE-05..08 are not in the S-23 backlog scope (they are S-24), so `ss tree` takes no `--session-scope` (the parser refuses it as an unknown option).
  (c) Enter on a gray folder without a session opens its directory under the session name it would get (FR-SESS-07, FR-SESS-08). A gray folder is openable only in the session tree (A-13), so it does not depend on a session existing.
- why: each choice keeps the grammar of architecture 2.3 and the S-22 window. Tests: tests/tui_trees.rs and tests/component_cli.rs cover the enter path and the empty window.
- status: Open

### P-44 S-24 bookmark tree scope: choices the spec leaves open (FR-TREE-05..08)
- from: implementer / slice S-24
- doc: requirements.md (FR-TREE-05..08), docs/architecture.md (A-13, 6.2), docs/backlog.md (S-24)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) "The tree that contains a directory" is the tree of its deepest project that holds at least one bookmark. A directory inside a project with no bookmarks is in no tree.
  (b) The cursor starts on the nearest node: the deepest row of that tree whose path is the directory or one of its ancestors (a gray connector counts as a node). With no directory, the cursor is on the first row.
  (c) FR-TREE-07, "shows only the root directory": when the directory is in no tree, the window shows one gray, non-openable row. Its path is the root of the project that contains the directory (labelled with the project basename, as the other roots). When the directory is outside every project, the row is the directory itself, labelled with its path (the A-13 rule for sessions outside projects).
  (d) `--session-scope` uses the directory of the current session (A-37 directory, `session_dir` of `bm edit`); outside tmux or without such a session it gives `--session-scope requires running inside a session.` (D-16). A directory given to `bm tree` is cleaned against the working directory and need not exist.
  (e) `bm tree` with no directory is the whole tree set (FR-TREE-08); it does not default to the working directory.
- why: the spec does not say which tree "contains" a directory, where the cursor starts, or what the root-only view labels. These follow A-13 and the `bm edit` project rule (FR-EDIT-08). Tests: domain/tree.rs unit tests (all_trees_are_shown_when_no_scope_is_given, directory_shows_only_the_tree_that_contains_it, cursor_starts_on_the_nearest_shown_node, directory_outside_every_tree_shows_only_its_root, directory_outside_every_project_shows_only_itself) and tests/tui_trees.rs (scope tests).
- status: Open

### P-45 S-24 bm tree: open options accepted, editor not used by the window
- from: implementer / slice S-24
- doc: docs/architecture.md (2.3 grammar `ledge bm tree [DIR] [--session-scope] [OPEN-OPTS]`, A-18 editor options apply to `bm tree`), requirements.md (FR-TREE-11), docs/backlog.md (S-24)
- change: `bm tree` takes the open options from the parser and passes `options.profile` to the window (enter opens the chosen directory with that profile). `--editor`, `--code` and `--idea` are accepted and not used by the window, the same choice as P-36 (`bm pick`) and P-43 (`ss tree`). A-18 says editor options apply to `bm tree`; this slice does not implement that for the window, and the user may decide whether the window launches the editor on enter.
- why: keeps the window behaviour of S-22 and S-23 the same for all trees. Tests: tests/component_cli.rs and src/cli/parse.rs (parsing); tests/tui_trees.rs (enter path).
- status: Open

### P-46 S-25 session list and kill: messages and target rules the spec leaves open
- from: implementer / slice S-25
- doc: requirements.md (FR-SESS-09..13, Messages), docs/architecture.md (A-20, A-34, 2.3 grammar, 8.3), docs/analysis.md (F-29), docs/proposals.md (P-27)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) `ss kill TARGET` takes the managed session with the exact name first (F-29, the suggested rule), then the managed session of the directory `TARGET` (cleaned against the working directory, D-06). An unmanaged session of the exact name does not stop the directory lookup, so a directory whose managed session exists is still killed.
  (b) Every miss gives `'<target>' is not a ledge session.` with the argument as typed (FR-SESS-11 for both the name and the directory case). A directory whose session is not running is therefore reported the same way, as F-29 asks.
  (c) `ss kill` with two or more words is refused with `ss kill takes one session name or directory.` (error, exit 1). This text is new and not in requirements.md or architecture 8.3; the user may give a better one. P-32 (b) had left the count open.
  (d) `ss kill` with no word is refused in the grammar with `kill requires a session name or directory.` before tmux is checked (FR-SESS-13).
  (e) `ss list` header row is `Session`, `Windows`, `State`, `Path` (A-34), with two spaces between columns and the last column unpadded. The header words and the state words are constants in `messages.rs`.
  (f) The kill of the session the user is in is refused by the existing kill service (D-17, P-27 b), so `ss kill` prints `cannot kill the session you are in.` as an error.
- why: each choice is the smallest one that passes the S-25 tests in tests/component_actions.rs, tests/component_cli.rs, src/cli/parse.rs and tests/messages_verbatim.rs. None changes an accepted decision (D-08, D-14, D-16, A-20, A-34).
- status: Open

### P-47 S-26 doctor exit 1 with a printed report: a new outcome
- from: implementer / slice S-26
- doc: docs/architecture.md (8.1, 8.2), docs/backlog.md (S-26), requirements.md (FR-DOC-08)
- change: add `Outcome::Report(String, i32)` to `error.rs` and a row to the 8.2 table: `Outcome::Report(text, code)` | stdout | none | code | FR-DOC-08 (doctor). `main::report` prints the text and returns the code. The doctor prints its ten checks through it when a line is ERR (exit 1); otherwise it returns `Outcome::Info` (exit 0).
- why: FR-DOC-08 needs the lines printed and exit 1 when any check is ERR. `LedgeError::Fatal` would add `Error: ` before the report, and `Outcome::Interrupted` is for signals. Tests: tests/component_doctor.rs, src/main.rs (report test).
- status: Open

### P-48 S-26 doctor texts that 8.3 does not give
- from: implementer / slice S-26
- doc: docs/architecture.md (8.3, 8.4), requirements.md (Messages)
- change: the implementation makes these choices; the user may confirm or reverse each:
  (a) The 8.4 doctor lines are not in the 8.3 table. They are added to `messages.rs` as `doctor_*` functions, with the 8.4 wording, and literal tests in tests/messages_verbatim.rs.
  (b) Check 1 uses `tmux is not installed` (8.4), not `Required tool 'tmux' is not installed.` (FR-CMD-10). Two texts for one fact; the commands keep FR-CMD-10.
  (c) Check 7 for a missing or invalid default profile prints `default_profile '<name>' is missing or invalid: <reason>`, where the reason is the profile message of FR-PROF wording (for example `profile 'ghost' not found at <path>`). 8.4 gives no reason text.
  (d) An unreadable bookmarks file is `WARN  could not read bookmarks: <detail>` (existing message `could_not_read_bookmarks`). A tmux that cannot list sessions is `ERR   could not list sessions: <detail>` (existing message `could_not_list_sessions`). 8.4 names neither.
- why: the spec gives no text for these cases, and the doctor must print one line per check (FR-DOC-08). Tests: tests/component_doctor.rs and tests/messages_verbatim.rs.
- status: Open

### P-49 S-26 optional programs: WARN or OK/ERR
- from: implementer / slice S-26
- doc: requirements.md (FR-DOC-01), docs/architecture.md (8.4 checks 2 to 4, A-12), docs/backlog.md (S-26)
- change: FR-DOC-01 says doctor prints "OK" or "ERR" for each external program it needs. Architecture 8.4 and A-12 print `WARN` for setsid, code and idea (only tmux is an ERR). The implementation follows 8.4. The user may change FR-DOC-01 to name the optional programs as warnings, or the architecture may give them ERR.
- why: the two documents disagree; the backlog acceptance follows the architecture. Tests: tests/component_doctor.rs (missing_setsid_code_and_idea_are_warnings).
- status: Open

### P-50 S-26 doctor and the cache folder; config unusable
- from: implementer / slice S-26
- doc: docs/architecture.md (A-35, 8.4 check 9, 9.4), docs/backlog.md (S-26)
- change: A-35 says doctor does not read the cache folder, but check 9 must read the bookmarks file, which is in the cache folder (9.4). The implementation reads `bookmarks` read-only (never created). Also, when the config is missing or invalid, the search path and default profile checks print nothing, and the bookmark check judges only whether each directory exists (no project test). A managed session with no directory is reported as `directory is missing: no directory`.
- why: check 9 and A-35 cannot both hold; the user should pick A-35 or check 9. Tests: tests/component_doctor.rs (bookmarks_missing_or_outside_projects_are_warnings, good_bookmarks_give_one_ok_line_with_the_count, missing_config_is_an_error_and_is_not_created).
- status: Open

### P-51 S-27 installer messages and choices the spec leaves open
- from: implementer / slice S-27
- doc: requirements.md (Messages, Installation and removal), docs/architecture.md (8.3, 11), docs/backlog.md (S-27)
- change: the installer (scripts/install.sh) uses the wording of architecture 11 and requirements.md "Installation and removal". The gaps and the choices made are:
  (a) Architecture 8.3 has the rows `Installed <path>` and the PATH warning, but not `Removed <path>`, `Not removed: ...`, `Nothing to remove: ...` or `Error: build toolchain is required to build ledge.`. Those four appear only in architecture 11 and in requirements.md "Installation and removal", not in requirements.md "Messages". P-1 already asks for the Messages entries; 8.3 needs the same four rows. Until then the wording is taken from 11 and tested literally in scripts/test_install.sh (not in tests/messages_verbatim.rs, which covers the binary).
  (b) `Installed <path>` is `[new]` in 8.3 and is not in requirements.md. Printed on stdout after a successful install.
  (c) Unknown arguments to `scripts/install.sh` print `Error: usage: scripts/install.sh [--uninstall]` on stderr, exit 1. No document gives this text.
  (d) `--uninstall` with no binary keeps any stale record (Nothing to remove, exit 0). With a binary and no record, the binary is kept and `Not removed: ...` is printed (architecture 11 says so; the record is what proves the source).
  (e) The installer tests copy scripts/install.sh into a temp source folder, so the fake cargo writes its stub under that temp folder and the real target/ of the checkout is not overwritten. HOME is also a temp folder. The architecture says only that the fake cargo writes a stub binary.
- why: the spec gives no text for (c), and (a) and (b) leave two documents disagreeing on which messages exist. Tests: scripts/test_install.sh.
- status: Open

### P-52 Vim navigation keys (FR-TUI-11)
- from: user request / coordinator, slice S-28
- status: Open
- doc: requirements.md (new FR-TUI-11)
- change: add "FR-TUI-11: ledge shall move the selection with j and k in the tree and edit windows, which have no filter, and with ctrl-j and ctrl-k in the pickers that filter as you type, so that j and k can be typed into the filter there. Arrow keys keep working everywhere."
- why: the user uses vim keys and asked for them in list and tree navigation. FR-TUI-03 covers only arrows and page keys. In pickers that filter as you type (project, history, switcher, bookmark), a plain j or k must still type into the filter (FR-TUI-02), so those pickers use ctrl-j and ctrl-k.

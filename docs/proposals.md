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

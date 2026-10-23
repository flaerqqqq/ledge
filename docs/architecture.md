# Architecture

STATUS: APPROVED
Owner: user. Drafted by the architect in Phase 0 under gate A (accepted decisions D-01..D-17).
Gate B is waived: the user chose fully autonomous mode. Decisions D-11..D-17 override the vaguer
wording of `requirements.md`. Every finding of `docs/analysis.md` that was not settled by D-11..D-17
is settled in section 13 "Assumptions".

Conventions for this document: `[new]` marks a user-facing message that the requirements do not
contain. It is listed in section 8.3 and in section 13. Proposals for `requirements.md` are in
`docs/proposals.md`.

## 1. Goals and constraints

- Linux only, tmux 3.2 or later is the only session backend (FR-PLAT-01). The tmux CLI is driven as a
  process. No tmux library.
- One program, `ledge`, built from source by `scripts/install.sh`. No runtime dependency on the build
  folder (the binary is copied to the local binary folder).
- Own TUI for every picker (FR-TUI-01). No external picker program.
- Testable without tmux: pure core plus trait seams for the session backend, filesystem, environment,
  clock, process runner, console, terminal and signals (see section 4).
- Terminal restored on exit, error and interrupt (FR-TUI-06).
- Byte-for-byte session names with earlier sessions (D-02, FR-NAME-02).
- Atomic writes for every file ledge writes (D-05).
- Few dependencies: four runtime crates (section 10).
- Exit status: 0 for success, info and cancel; 1 for every error; 130/129/143 when interrupted by
  SIGINT/SIGHUP/SIGTERM (section 8.2).

## 2. Crate and module layout

Single crate, binary and library in one package named `ledge`. A workspace is not needed for one
program with one binary, and a library target lets the integration tests reach the modules.

```
ledge/
  Cargo.toml, Cargo.lock           Cargo.lock is committed (binary crate, reproducible build)
  src/
    main.rs                 composition root: builds adapters, calls cli::run, maps Outcome to exit code
    lib.rs                  module tree, #![forbid(unsafe_code)]
    error.rs                LedgeError (Fatal), Outcome (Done, Info, Cancelled, Interrupted), Result alias
    messages.rs             every user-facing string as a const or a fn (section 8.3)
    cli/mod.rs              run(): parse, route to app, print Outcome
    cli/parse.rs            pure grammar: argv -> Command (section 2.3)
    cli/help.rs             help text, group usage text
    domain/mod.rs           re-exports; no I/O
    domain/path.rs          AbsPath: clean, make absolute, is_inside (whole components), display with ~
    domain/naming.rs        session_name(AbsPath) (FR-NAME-01)
    domain/location.rs      Dirs from env values, XDG rules (FR-PLAT-02, FR-CONF-08)
    domain/config.rs        Config parse, defaults, search path expansion and validation (FR-CONF-*)
    domain/profile.rs       Profile, Window, ProfileName, validation, built-in basic (FR-PROF-01/02/06)
    domain/profile_rules.rs which profile applies to open and rebuild, picker row order (FR-PROF-09..14, D-13, FR-PROF-13)
    domain/history.rs       History list: add-to-top, dedupe, limit 50, file format (FR-HIST-01..03)
    domain/bookmarks.rs     Bookmarks list, file format, add/remove, status (FR-BM-01, FR-BM-08)
    domain/bm_edit.rs       expanded folder state (FR-EDIT-05), pruning of missing directories (D-09)
    domain/projects.rs      project listing rules, current directory row, project_containing, scope (FR-PICK-01..05, FR-CONF-07, FR-SCOPE-*)
    domain/tree.rs          session tree and bookmark tree builders with openable flags (FR-TREE-*, FR-SESS-06/07)
    domain/sessions.rs      managed filter, switcher order, session directory, node membership (FR-NAME-04, FR-SESS-01/02, FR-ACT-06/07)
    domain/age.rs           relative age text (FR-SESS-03)
    ports/mod.rs            trait definitions only
    ports/backend.rs        SessionBackend, SessionInfo, NewSession, BackendError
    ports/fs.rs             FileSystem
    ports/env.rs            Env
    ports/clock.rs          Clock
    ports/process.rs        ProcessRunner, Captured
    ports/console.rs        Console (stdout and stderr lines)
    ports/signals.rs        Signals (interrupt flag)
    app/mod.rs              Ctx (bundle of port references), shared helpers
    app/store.rs            ConfigStore, HistoryStore, BookmarkStore, EditStateStore: read, parse, atomic write
    app/build.rs            build a session from a profile (FR-OPEN-07, FR-PROF-03..08, FR-NAME-03)
    app/open.rs             open flow: directory checks, history, create or reuse, attach or switch (FR-OPEN-03/08/10/11)
    app/launch.rs           editor launch (FR-LAUNCH-*)
    app/actions.rs          kill, rebuild, rebuild-current, node actions, refusal rules (FR-ACT-*)
    app/history_cmd.rs      history list, clean, pick glue
    app/bookmark_cmd.rs     bm add, remove, list, edit, tree, pick glue
    app/session_cmd.rs      ss list, kill, tree, switcher glue
    app/pickers.rs          picker services: project, history, bookmark, switcher, session tree, bookmark tree, profile picker
    app/doctor.rs           doctor checks (FR-DOC-*)
    tui/mod.rs
    tui/fuzzy.rs            fuzzy matcher (D-10), pure
    tui/picker.rs           PickerModel<T>: rows, filter, selection, scroll offset, status line, pure
    tui/keys.rs             Input -> Action mapping (section 6.3)
    tui/columns.rs          column widths, alignment, truncation (pure, uses display width)
    tui/render.rs           draws PickerModel into a ratatui Frame
    tui/engine.rs           run_picker loop, PickerHandler trait, Step, PickerResult
    tui/terminal.rs         TerminalSession trait (the screen port; implemented in adapters)
  adapters/
    mod.rs
    tmux.rs                 TmuxBackend: SessionBackend via the tmux CLI (section 7)
    fs.rs                   RealFs: std::fs with atomic write
    env.rs                  RealEnv: std::env, home and cwd
    clock.rs                SystemClock
    process.rs              SystemProcess: std::process, PATH lookup
    console.rs              StdConsole: locked stdout and stderr
    signals.rs              SignalFlags: signal-hook registration
    terminal.rs             CrosstermTerminal: raw mode, alternate screen, Drop restores, panic hook
  scripts/
    gate.sh                 fmt, clippy, build, test, ignored (tmux) tests, layer check
    check_layers.sh         grep-based dependency rule check (section 3)
    install.sh              build and install, --uninstall (section 11)
    test_install.sh         installer tests with a fake cargo and temp folders
  tests/                    see docs/testing.md
```

Size rule (conventions): a module over about 400 lines is split. The app and tui folders are the
likely places.

### 2.1 Why `cli` and `app` are separate

`cli` only parses arguments and prints outcomes. `app` runs the use cases against the ports. A test
can call `app::open::open_directory(&ctx, ...)` with fakes without parsing argv.

### 2.2 Why `messages.rs` is top level

Every layer prints text. A single file makes the verbatim check (`tests/messages_verbatim.rs`) a
single grep, and keeps wording out of logic.

### 2.3 CLI grammar (D-11, F-01)

```
ledge [DIR] [OPEN-OPTS]                 open DIR or cwd (implicit project open; options may appear anywhere)
ledge project open [DIR] [OPEN-OPTS]
ledge project pick [OPEN-OPTS]          project picker
ledge history list | pick [OPEN-OPTS] | clean
ledge bm add [DIR] | remove DIR | list | pick [OPEN-OPTS]
ledge bm edit [DIR] [--session-scope]
ledge bm tree [DIR] [--session-scope] [OPEN-OPTS]
ledge ss [OPEN-OPTS]                    session switcher (bare ss IS the switcher)
ledge ss list | tree [OPEN-OPTS] | kill NAME|DIR
ledge doctor
ledge help | -h | --help
OPEN-OPTS: --profile NAME | --no-profile ; --code | --idea | --editor CMD
```

Parse rules (pure, `cli/parse.rs`):

1. First word is `help`, `-h` or `--help`: help text, exit 0.
2. First word is a command word (`project`, `history`, `bm`, `ss`, `doctor`): explicit command.
   Options are accepted only after the command word(s) (FR-CMD-11). An option the command does not
   accept (table below) gives `Unknown option: <opt> (run 'ledge help' for usage).` (FR-CMD-04).
3. First word is anything else, or starts with `-`: bare form. Positional words are directory
   candidates. More than one positional: `project open takes at most one directory argument.`
   (FR-OPEN-04). A leading flag followed by a command word anywhere after it is rejected with
   `Unknown option: <first flag> (run 'ledge help' for usage).` A word that is a command word is
   always the command (`ledge bm` is the group form). `./bm` or any word containing `/` is a
   directory (D-11: "./bm forces the directory reading").
4. `project`, `history`, `bm` without a sub-command: group usage on stdout, exit 0 (FR-CMD-02).
   `ss` without a sub-command is the switcher.
5. Unknown sub-command: `Unknown <group> command: <word> (run 'ledge <group>' for the list).` (FR-CMD-03).
6. Option value rules: `--profile` needs a value (`--profile requires a profile name.`, FR-CMD-05).
   `--editor` needs a value (`--editor requires a command.`, FR-CMD-06). `--editor` takes the rest of the
   argument as one command line split on whitespace (no shell quoting); the directory is appended last.
   `--profile` with `--no-profile`: `--profile cannot be combined with --no-profile.`. More than one
   of `--code`, `--idea`, `--editor`: `only one of --code, --idea, --editor may be given.`.
   `--session-scope` with a directory: `--session-scope cannot be combined with a directory.` (FR-TREE-10).
   `--session-scope` outside a session: `--session-scope requires running inside a session.` (D-16).
7. Extra positional words for a command that takes none: `'<command words>' takes no arguments. [new]`
   (for example `'history list' takes no arguments.`).

Options accepted per command:

| Command | Directory | Options |
|---|---|---|
| bare form, `project open`, `project pick`, `history pick`, `bm pick`, `ss`, `ss tree` | bare form and `project open` take one; the pickers, `ss` and `ss tree` take none | OPEN-OPTS |
| `bm tree` | one | OPEN-OPTS, `--session-scope` |
| `bm edit` | one | `--session-scope` |
| `bm add`, `bm remove` | one (`bm remove` requires it) | none |
| `history list`, `history clean`, `bm list`, `ss list`, `ss kill`, `doctor`, `help` | none (`ss kill` takes NAME or DIR as its one word) | none |

Option on a command that does not accept it: `Unknown option: <opt> (run 'ledge help' for usage).`.
Repeated options: the last value wins, except editor options which follow the "only one" rule.

## 3. Layering and dependency rules

Layers, from the bottom:

| Layer | Modules | May use | Must not use |
|---|---|---|---|
| domain | `domain/*` | core types and collections (`String`, `Vec`, `Option`), `serde_json` (Value), `md5` | `std::fs`, `std::env`, `std::process`, `std::io`, `std::path`, anything in ports, app, tui, adapters, cli, ratatui |
| ports | `ports/*` | domain, std::io error type | adapters, app, tui |
| tui | `tui/*` | domain, messages, ratatui | ports (except `ports::signals` through the engine parameter), app, adapters, std::fs/process/env |
| app | `app/*` | domain, ports, tui, messages, error | adapters, ratatui directly (goes through tui), cli |
| cli | `cli/*` | app, domain, ports (for Ctx), messages, error | adapters |
| adapters | `adapters/*` | ports, domain, tui::terminal (trait), std, ratatui, crossterm (via ratatui), signal-hook | app, cli |
| main | `main.rs` | everything | nothing else |

Exception, recorded here on purpose: `tui::terminal::TerminalSession` is a trait owned by the tui
layer, so `adapters::terminal` depends on `tui` for that one trait. The rule that matters is that
`app` and `domain` never see ratatui or crossterm.

`scripts/check_layers.sh` enforces the table with grep: `domain/` must not contain `std::fs`,
`std::env`, `std::process`, `std::io`, `crate::ports`, `crate::app`, `crate::tui`, `crate::adapters`,
`ratatui`; `app/` must not contain `crate::adapters` or `ratatui`; `tui/` must not contain `std::fs`,
`std::process`, `std::env`. The check fails the gate.

Composition: only `main.rs` names concrete adapter types. `cli::run` takes a `&Ctx` built in `main`.

## 4. Trait boundaries for testability

All ports take `&self`. Fakes that record calls use `RefCell` inside the fake (tests are single
threaded). Ports are passed as `&dyn Trait` inside `app::Ctx`, so no generics leak into the commands.

```rust
// app/mod.rs
pub struct Ctx<'a> {
    pub env: &'a dyn Env,
    pub fs: &'a dyn FileSystem,
    pub clock: &'a dyn Clock,
    pub process: &'a dyn ProcessRunner,
    pub console: &'a dyn Console,
    pub backend: &'a dyn SessionBackend,
    pub terminal: &'a dyn TerminalFactory,   // tui::terminal; opens a TerminalSession or fails
    pub signals: &'a dyn Signals,
    pub dirs: Dirs,                           // resolved once in main from Env (domain::location)
    pub cwd: AbsPath,
    pub home: String,
}
```

| Port | Methods (exact signatures in `ports/*`) | Real adapter | Fake (tests/support) |
|---|---|---|---|
| `SessionBackend` | `version`, `list_sessions`, `create_session(NewSession) -> String` (returns first window id), `add_window`, `send_line`, `select_window`, `set_option`, `attach`, `switch_client`, `kill_session`, `rename_session`, `current_session`, `kill_server` (tests) | `adapters::tmux::TmuxBackend` | `FakeBackend` (in-memory sessions and options, scripted failures) |
| `FileSystem` | `read_text(&Path) -> io::Result<Option<String>>` (None if missing), `write_atomic`, `create_new` (returns bool), `create_dir_all`, `is_dir` (follows symlinks), `exists`, `list_dirs`, `list_files`, `remove_file` | `adapters::fs::RealFs` | none: tests use `RealFs` on temp folders (`CARGO_TARGET_TMPDIR`) |
| `Env` | `var(&str) -> Option<String>` (empty string returned as is; callers decide), `cwd() -> io::Result<String>` | `RealEnv` | `FakeEnv` (a map) |
| `Clock` | `now_secs() -> i64` (Unix seconds) | `SystemClock` | `FakeClock` (settable) |
| `ProcessRunner` | `find_program(&str) -> bool`, `capture(&str, &[String]) -> io::Result<Captured{status, stdout, stderr}>`, `inherit(&str, &[String]) -> io::Result<i32>` (inherits stdio, waits), `spawn_detached(&str, &[String]) -> io::Result<()>` (stdio null, waits only for `setsid -f`) | `SystemProcess` | `FakeProcess` (scripted replies, records argv) |
| `Console` | `out(&str)`, `err(&str)` (each adds the newline) | `StdConsole` | `FakeConsole` (captures both streams) |
| `Signals` | `take_interrupt() -> Option<i32>` (exit code of a received signal, once) | `SignalFlags` | `FakeSignals` (scripted) |
| `TerminalFactory` (tui) | `open() -> Result<Box<dyn TerminalSession>, String>` (fails when stdout is not a tty) | `CrosstermFactory` | `FakeTerminal` (scripted `Input` events, `TestBackend`) |

Seam list for the requirement-analysis item F-31: inside or outside a session (FR-OPEN-08, FR-SESS-04)
is `env.var("TMUX")` non-empty; the live-session marker (FR-PICK-07) is `backend.list_sessions()`;
the process spawn (FR-LAUNCH-*) is `ProcessRunner`; terminal size, raw mode and signals are the tui
ports.

Pure logic needs none of the ports: naming, config and profile parsing, history and bookmark list
logic, tree construction, fuzzy match, age text, column layout, CLI grammar, precedence rules.

## 5. Domain model

| Type | Module | Content | Notes |
|---|---|---|---|
| `AbsPath` | domain::path | cleaned absolute path string (UTF-8) | constructed only by `AbsPath::clean(input, cwd)`; `Eq`, `Ord` by bytes |
| `Dirs` | domain::location | `config: AbsPath`, `cache: AbsPath` | config folder is `<XDG_CONFIG_HOME or ~/.config>/ledge` |
| `Config` | domain::config | `search_paths: Vec<AbsPath>` (deduplicated, expanded), `default_profile: Option<String>` | parsed from `config.json` text |
| `ProfileName` | domain::profile | validated `[A-Za-z0-9_-]+` | `basic` is reserved |
| `ProfileChoice` | domain::profile | `Basic` or `Named(ProfileName)` | what is recorded on a session: the name, `basic` for the single-window layout |
| `Profile` | domain::profile | `name`, `windows: Vec<Window>` (non-empty) | `Profile::basic()` = one window `Terminal`, no command |
| `Window` | domain::profile | `name: String` (non-empty), `command: Option<String>` | unknown fields ignored, duplicate names allowed |
| `Project` | domain::projects | `path: AbsPath`, `name: String` (basename) | from a search path entry |
| `SessionName` | domain::naming | `String` | `session_name(&AbsPath)` |
| `SessionInfo` | ports::backend | name, windows, attached, activity (epoch), managed, profile, dir, path | one line of `list-sessions` |
| `ManagedSession` | domain::sessions | a `SessionInfo` with `managed == true` | only this type reaches lists, pickers and actions (FR-NAME-04) |
| `History` | domain::history | `Vec<AbsPath>`, at most 50, newest first | file `history` |
| `Bookmarks` | domain::bookmarks | `Vec<AbsPath>`, insertion order | file `bookmarks` |
| `ExpandedFolders` | domain::bm_edit | `Vec<AbsPath>` | file `bm-edit-open` |
| `TreeRow<K>` | domain::tree | `depth`, `label`, `path`, `kind`, `openable`, `session: Option<SessionName>`, `dimmed` | flattened, top-down, unsorted by the caller |
| `Command` | cli::parse | enum of all commands with parsed options | section 2.3 |
| `OpenOpts` | cli::parse | `profile: ProfileOpt::{Unset, Named(String), None}`, `editor: Option<Editor>` | `Editor::{Code, Idea, Command(Vec<String>)}` |
| `Outcome` | error | `Done`, `Info(String)`, `Cancelled`, `Interrupted(i32)` | exit code mapping in 8.2 |
| `LedgeError` | error | `Fatal(String)` | the only error variant; text is built in `messages` |

Session directory (used by switcher, session tree, node actions, scope): `@ledge_dir` option when set,
else tmux `session_path`. A session without either is not in any directory scope.

Project membership: a directory belongs to the project P when it equals P or is inside P by whole
components (D-06, F-21). A bookmark must be strictly inside P (FR-BM-02).

## 6. TUI engine

### 6.1 Model

`PickerModel<T>` holds `rows: Vec<PickerRow<T>>`, `filter: String`, `visible: Vec<usize>` (indexes of
rows that match the filter, in the original order, with tree ancestors kept), `selected: usize`
(position in `visible`), `scroll: usize`, `status: Option<String>`, and the chrome:
`label` (border label without icon), `icon`, `headers`, `footer`, `search_column` (index, or `None` for
the edit window), `empty_message` (info text shown when there are no rows).

`PickerRow<T>` holds `cells: Vec<String>` (one per column), `payload: T`, `search_text: String`,
`dimmed: bool` (missing directory), `connector: bool` (gray entry, not openable), `depth: usize`
(tree indent), `marked: bool` (edit window bookmark marker).

Rules:
- Filter: fuzzy match of `filter` against `search_text` (empty filter matches all). Ties keep the
  original order. Matched characters are bold and underlined in the rendering. Tree rows: a matching
  row keeps all its ancestors visible (ancestors are not scored).
- When the filter changes, `selected` moves to the best match (D-10): the first visible row when the
  filter is non-empty, otherwise the row kept from before when it is still visible.
- Empty filter: the preselected row of FR-PICK-04 is kept (the model starts with `selected` on the row
  marked `preselect`).
- Navigation: up and down move by one; page up and page down move by the visible height minus one;
  home and end go to the first and last visible row. No wrap-around, no mouse (F-37).
- After a reload (refresh after an action) the filter is kept and `selected` keeps the same position,
  clamped to the new list (D-12 "reopen").
- `scroll` is adjusted so that `selected` is inside the visible list area after every key and every
  resize (FR-TUI-05).

### 6.2 Layout and columns

Border label line: `<icon> <label>` (for example `▸ Projects & Submodules`). The icon is a constant
per picker in `tui::render`: projects `▸`, history `↺`, sessions `◉`, session tree `◉`, bookmarks `★`,
bookmark tree `★`, bookmark edit window `★`, profiles `≡`.

Inside the border, top to bottom:
1. prompt row: `> ` followed by the filter (cursor at its end). The edit window has no filter row (D-15).
2. header row: column titles.
3. list rows (scrolled to `selected`).
4. status row: the status message, empty when there is none.
5. footer row: the key list.

Column titles and footer per picker (F-10):

| Picker | Border label | Headers | Search column | Footer |
|---|---|---|---|---|
| project | `Projects & Submodules` or `Projects (Profile: <name>)` (FR-PICK-13) | `""`, `Project`, `Session`, `Profile` | 1 (Project) | `enter open · ^r rebuild · ^p profile · ^x kill` |
| history | `History` | `Project`, `Session`, `Profile` | 0 | `enter open · ^r rebuild · ^p profile · ^x kill` (A-03) |
| sessions (switcher) | `Sessions` | `Session`, `Project`, `Profile`, `Activity` | 0 | `enter switch · ^r rebuild · ^p profile · ^x kill` (given) |
| bookmarks | `Bookmarks` | `Bookmark`, `Session`, `Profile` | 0 (full path) | `enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark` (given) |
| session tree | `Session tree` | `Directory`, `Session`, `Profile` | 0 (basename without glyphs) | `enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node` (given) |
| bookmark tree | `Bookmark tree` | `Directory`, `Session`, `Profile` | 0 | `enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node` (given) |
| bookmark edit | `Bookmarks in <project basename>` | `""`, `Directory` | none (no filter, D-15) | `enter toggle bookmark · →/^l open · ←/^h close · esc done` (given) |
| profile | `Profiles` | `Profile` | 0 | `enter choose · esc cancel` (FR-TUI-04 needs a key footer; the screen said "none") |

Path cells show the cleaned absolute path. Long paths are truncated from the left with `…` (the tail
is kept). Other cells are truncated at the end with `…`.

Truncation and fit (F-12):
- Minimum terminal size 40 columns by 8 rows. Smaller: one line `Terminal too small: need at least 40x8. [new]`
  and no list; keys still work (escape and ctrl-c cancel).
- Widths use display width (CJK characters count as 2; `ratatui::text::Span::width`, so no direct
  `unicode-width` dependency).
- Marker column: 2 cells, fixed. Project picker: `*` for the current directory row, `●` live session,
  `○` none. The two cells are the current-directory flag and the live flag.
- Shrink order when the row does not fit: the last column first, down to 6 cells, then the search
  column down to 12 cells, then the others down to 4 cells. Then each cell is truncated.

Dim and no-color cues (F-11, FR-TUI-08, FR-TUI-09):
- Color is off when `NO_COLOR` is present and non-empty, or `TERM` is `dumb`. Pickers need a tty
  (section 8.3), so a tty is never the color switch.
- Missing directory: suffix ` (missing)` on the Directory or Bookmark cell always; with color, the row
  is drawn faint.
- Gray connector (tree rows that are not openable): the name is wrapped in parentheses `(name)` always;
  with color it is drawn in bright black.
- Selected row: reverse video (works without color).
- Matched characters: bold and underlined.
- Markers `●`, `○`, `*` are text, so they stay readable without color.

Rendering is pure: `render::draw(model, area, frame)`. The view is tested with ratatui's `TestBackend`
(tests/tui_*).

### 6.3 Keys (FR-TUI-07, FR-TUI-03, D-15, F-14, F-37)

| Key | Action in pickers | Action in the edit window |
|---|---|---|
| enter | choose the selected row (no row: cancel) | toggle bookmark (not on the project root row) |
| esc, ctrl-c | cancel | done (close) |
| up, down | move | move |
| page up, page down | move by page | move by page |
| home, end | first, last visible row | first, last row |
| printable char (no ctrl or alt) | append to filter | ignored |
| backspace | delete last filter char | collapse (same as left and ctrl-h, see 6.5) |
| ctrl-r | rebuild selected row's session | (not used) |
| ctrl-p | profile picker, then rebuild selected | (not used) |
| ctrl-x | kill selected session | (not used) |
| ctrl-d | delete bookmark and kill its session (bookmark pickers only; other pickers: no-op) | (not used) |
| alt-r, alt-p, alt-x, alt-d | node actions (trees only; alt-d bookmark tree only) | (not used) |
| right, ctrl-l | (no-op) | expand folder |
| left, ctrl-h | (no-op) | collapse, or move to parent |

Cancel rules (F-14): an empty choice is enter with no visible row; it cancels. Cancel at the top level
prints `Cancelled.` on stdout, exit 0 (FR-CMD-08). Cancel in the profile picker opened from another
picker returns to that picker with no output and no change (nested picker, not a top-level cancel).
ctrl-c is a key event (raw mode, no ISIG), so it is a cancel, not an interrupt.

### 6.4 Engine loop

```rust
// tui/engine.rs
pub trait PickerHandler<T: Clone> {
    fn choose(&mut self, row: &T) -> Step;                 // enter: Step::Accept is returned to the caller after restore
    fn act(&mut self, action: Action, row: Option<&T>) -> Step; // ctrl-*, alt-* ; row None when the list is empty
    fn reload(&mut self) -> Result<Vec<PickerRow<T>>, LedgeError>;
}
pub enum Step { Continue { status: Option<String>, reload: bool }, Accept, Close(Outcome) }
pub enum PickerResult<T> { Chosen(T), Cancelled, Closed(Outcome) }

pub fn run_picker<T: Clone>(
    model: PickerModel<T>,
    handler: &mut dyn PickerHandler<T>,
    mut term: Box<dyn TerminalSession>,
    signals: &dyn Signals,
) -> Result<PickerResult<T>, LedgeError>
```

Loop: draw; poll input with a 100 ms timeout; check `signals.take_interrupt()` on every poll; on
`Input::Resize` recompute scroll; on a key, map it with `keys::map` to an `Action`, then:
- `Choose`: `handler.choose(row)`; `Step::Accept` returns `Chosen(row)` (the caller attaches or switches
  after the terminal is restored).
- `Cancel`: `Cancelled`.
- other actions: `handler.act`.
- `Continue { reload: true }` calls `handler.reload()` and keeps filter and position (6.1).
- `Close(outcome)` returns `Closed(outcome)`.

Before returning, `term` is dropped (this restores the terminal). Errors from the handler also return
through the drop path, so the terminal is restored on error (FR-TUI-06). An interrupt returns
`Closed(Outcome::Interrupted(code))` after the drop.

### 6.5 Terminal restore and interrupts (FR-TUI-06)

- `CrosstermTerminal::open` checks that stdout is a tty, enables raw mode, enters the alternate screen,
  hides the cursor. Its `Drop` leaves the alternate screen, shows the cursor and disables raw mode. Drop
  runs on every return path including `?`.
- A panic hook installed by `main` first restores the terminal (same function as Drop), then calls the
  previous hook. Panics in non-test code are forbidden (conventions), so this is a safety net.
- Signals: `SignalFlags` registers SIGINT, SIGHUP and SIGTERM with signal-hook (flag set, no handler
  work). The engine polls the flag every 100 ms. In raw mode ISIG is off, so ctrl-c arrives as a key.
  An external signal leads to the interrupt path: terminal restored, exit 130 (SIGINT), 129 (SIGHUP),
  143 (SIGTERM).
- ctrl-h sends byte 0x08, which crossterm reports as backspace. The edit window has no filter, so it
  treats backspace as collapse, which makes ctrl-h work. Pickers with a filter use backspace to edit
  the filter.
- Editing actions that need a tmux attach or switch happen after the picker has been dropped, so the
  attach runs on a normal terminal.

## 7. tmux adapter

### 7.1 Invocation

`TmuxBackend { runner: &dyn ProcessRunner, server: Option<String> }`. `server` comes from
`LEDGE_TMUX_SERVER` (non-empty) and is passed as `-L <server>` before every tmux subcommand (the
version check excepted). Without it, the default server is used. `-L` selects the socket explicitly,
so the `TMUX` variable of a parent session does not redirect commands. Integration tests also remove
`TMUX` from their own environment (testing.md).

Every command below is `tmux [-L <server>] <argv>`. Session targets are written `=<name>` (exact
match). Window targets are the window id returned by tmux (`@<n>`), so window index and base-index
settings do not matter.

| Operation | argv | Notes |
|---|---|---|
| version | `-V` | output `tmux 3.4` or `tmux next-3.5`; parse the first number pair; no `-L` |
| list sessions | `list-sessions -F '#{session_name}\t#{session_windows}\t#{session_attached}\t#{session_activity}\t#{@ledge_managed}\t#{@ledge_profile}\t#{@ledge_dir}\t#{session_path}'` | `session_attached` > 0 means attached; `@ledge_*` empty when unset |
| create session | `new-session -d -s <name> -c <dir> -n <first window name> -P -F '#{window_id}'` | starts the server when none runs (FR-OPEN-09); the printed id is the first window |
| set option | `set-option -t =<name> @ledge_managed 1`, `set-option -t =<name> @ledge_profile <profile>`, `set-option -t =<name> @ledge_dir <dir>` | called right after create, before windows are added |
| add window | `new-window -d -t =<name> -n <window name> -c <dir> -P -F '#{window_id}'` | `-d` keeps the first window current until the end |
| run command in window | `send-keys -t <window id> -l -- <command>` then `send-keys -t <window id> Enter` | literal text, then Enter (FR-PROF-04) |
| select first window | `select-window -t <first window id>` | FR-PROF-05 |
| switch client | `switch-client -t =<name>` | inside a session (FR-OPEN-08, FR-SESS-04) |
| attach | `attach-session -t =<name>` via `ProcessRunner::inherit` | outside a session; blocks until detach |
| kill | `kill-session -t =<name>` | |
| rename | `rename-session -t =<old> <new>` | rebuild of the current session (D-17) |
| current session | `display-message -p '#{session_name}'` | only when `TMUX` is set |
| kill server | `kill-server` | tests only (integration teardown) |

Session directory fallback: `#{session_path}` in the list. Its availability in the target tmux version
is checked by the S-09 acceptance test on tmux 3.2 (the minimum); if it is missing there, the switcher
uses `@ledge_dir` only and shows an empty directory for others (A-17).

### 7.2 Error mapping

| Condition | BackendError | Message |
|---|---|---|
| binary not found on PATH | `NotInstalled` | `Required tool 'tmux' is not installed.` (FR-CMD-10) |
| exit status 1 and stderr contains `no server running`, `error connecting to` or `no sessions` on `list-sessions` | not an error: empty list | (FR-PICK-07: no live markers) |
| any other non-zero exit | `Failed { detail: stderr trimmed }` | caller formats: `could not create session '<name>': <detail>`, `could not attach to '<name>': <detail>`, or the kill/rename message |
| version below 3.2 | checked at start of commands that use tmux | `tmux 3.2 or later is required; found <version>. [new]` |

Session creation cleanup: when a step after `new-session` fails, the caller (app::build) kills the partial
session (`kill-session`) and returns the error (A-07).

### 7.3 Managed marker and naming

Managed: `@ledge_managed` = `1`. Profile: `@ledge_profile` = profile name (`basic` for the single
window). Directory: `@ledge_dir` = clean absolute path. A user adopts a session by setting
`@ledge_managed` to `1` (FR-NAME-05); `@ledge_dir` may then be empty, and then no collision check is done
(A-05).

## 8. Error handling and messages

### 8.1 Error type

```rust
// error.rs
pub enum LedgeError { Fatal(String) }    // stderr "Error: <text>", exit 1
pub enum Outcome { Done, Info(String), Cancelled, Interrupted(i32) }
pub type Result<T> = std::result::Result<T, LedgeError>;
```

`LedgeError` implements `Display` by hand (no `thiserror`, conventions). Every backend, file or process
failure is converted to `Fatal` by the caller that knows the wording, using a `messages` function. The
`io::Error` never reaches the user unformatted. Exit codes are not stored in the error: all errors are 1
(FR-CMD-07).

### 8.2 Mapping to output and exit code

| Result | stream | prefix | exit | Source |
|---|---|---|---|---|
| `Outcome::Done` | none | none | 0 | |
| `Outcome::Info(text)` | stdout | none | 0 | D-16 info list, help, group usage |
| `Outcome::Cancelled` | stdout | none, text `Cancelled.` | 0 | FR-CMD-08 |
| `LedgeError::Fatal(text)` | stderr | `Error: ` | 1 | D-16 error list |
| warning (`Console::err`) | stderr | `Warning: ` | unchanged | D-16 warnings |
| `Outcome::Interrupted(code)` | none (terminal restored) | none | code | section 6.5 |

`main` prints the outcome and returns the exit code. A warning never changes the exit code.

### 8.3 Messages

`messages.rs` has one `const` or `fn` per text, named by its role (`no_bookmarks()`,
`bookmarked(path)`, `profile_not_found(name, path)`). Each function returns `String` with the
placeholders filled. The texts are exactly those of `requirements.md` "Messages", plus those of
D-11..D-17, plus the `[new]` entries below. `tests/messages_verbatim.rs` holds the literal expected
strings and fails on any drift.

Messages without a triggering FR in the requirements (F-07) map to the nearest FR in this table:

| Message | Nearest FR |
|---|---|
| `ss tree takes no directory argument.` | FR-SESS-05 (same rule on the tree command) |
| `bookmark remove requires a directory.` | FR-BM-10 |
| `bm add takes at most one directory.` | FR-BM-07 |
| `could not create session '<name>': <detail>` | FR-OPEN-07, FR-ACT-02 |
| `could not attach to '<name>': <detail>` | FR-OPEN-08, FR-SESS-04 |
| `project pick takes no directory argument.` | FR-PICK-11 (D-16) |
| `cannot kill the session you are in.` | FR-ACT-07 (D-17) |
| `--session-scope requires running inside a session.` | FR-EDIT-09, FR-TREE-06 (D-16) |
| `No search paths defined in <config file>.` | FR-PICK-10 (error class) |

Stream and class for every message are in D-16 (error, warning, info) and in section 8.2. Messages
without a class in D-16 are errors.

Messages added by the architect are marked `[new]`. Their assumptions are recorded in section 13 (A-01,
A-12, A-18, A-21, A-29, A-31, A-33). Each has a single function in `messages.rs`:

| Text | Class | Trigger |
|---|---|---|
| `config is invalid (expected 'search_paths' to be a list of strings): <file>` | error | `search_paths` has the wrong type (F-24) |
| `could not create config '<file>': <detail>` | error | config folder not writable (F-24) |
| `config search path '<entry>' uses undefined variable '<VAR>'.` | error | undefined `$VAR` or `${VAR}` (F-24) |
| `HOME is not set.` | error | no `HOME` and no XDG variable for a needed folder |
| `could not start editor '<command>': <detail>` | warning | editor program missing or failing to start (F-26) |
| `setsid is not installed; the editor is started without detaching.` | warning | `setsid` missing when an editor is launched (F-26) |
| `tmux 3.2 or later is required; found <version>.` | error | tmux older than 3.2 (FR-PLAT-01) |
| `a terminal is required for this picker.` | error | a picker started without a tty |
| `Terminal too small: need at least 40x8.` | status line | terminal below 40x8 (F-12) |
| `'<command words>' takes no arguments.` | error | extra positional words (2.3) |
| `Installed <path>` | info (install script) | install finished |
| `Warning: <dir> is not on PATH; add it to use ledge.` | warning (install script) | FR-INST warning (P-1) |

`cannot kill the session you are in.` comes from D-17 and is an error in the command form and a status
line in pickers.

### 8.4 Doctor lines (F-27)

Output format: the tag padded to six characters (`OK    `, `WARN  `, `ERR   `), then the check text.
Checks run in this fixed order. Program checks use `ProcessRunner::find_program`.

| # | Check | OK line | Other line |
|---|---|---|---|
| 1 | tmux present and version | `OK    tmux <version>` | `ERR   tmux is not installed` or `ERR   tmux <version> is older than 3.2` |
| 2 | setsid | `OK    setsid` | `WARN  setsid is not installed (editor launch will not detach)` |
| 3 | code | `OK    code` | `WARN  code is not installed (needed for --code)` |
| 4 | idea | `OK    idea` | `WARN  idea is not installed (needed for --idea)` |
| 5 | config file | `OK    config file <path>` | `ERR   config file <path> is missing` or the parse message (`config is not valid JSON: <file>`) or the invalid message |
| 6 | search paths | `OK    search path <path>` (one line per entry) | `WARN  no search path is configured` or `WARN  search path <path> does not exist` |
| 7 | default profile | `OK    default profile <name>`, or no line when none is set | `WARN  default_profile '<name>' is missing or invalid` (the profile reason follows the FR-PROF wording) |
| 8 | profiles | nothing when all are valid | `WARN  profile '<name>' is invalid (expected a 'windows' array)` per file (FR-DOC-05); `WARN  profile 'basic' is reserved; profiles/basic.json is ignored` |
| 9 | bookmarks | `OK    bookmarks (<n>)` | `WARN  bookmark <path> is missing` or `WARN  bookmark <path> is outside every project` |
| 10 | sessions | `OK    sessions (<n>)` | `WARN  session '<name>' directory is missing: <dir>` or `WARN  session '<name>' profile '<profile>' is missing or invalid`; when tmux is missing: `WARN  sessions not checked (tmux is not installed)` |

Doctor exits 1 when any line is `ERR`, otherwise 0. Doctor never creates the config (A-12). A missing
config is an `ERR` line.

## 9. Configuration, paths and persistence

### 9.1 Folders (FR-PLAT-02, FR-CONF-08, F-25)

- Config folder: `$XDG_CONFIG_HOME/ledge` when `XDG_CONFIG_HOME` is non-empty and absolute, else
  `$HOME/.config/ledge`.
- Cache folder: `$XDG_CACHE_HOME/ledge` when non-empty and absolute, else `$HOME/.cache/ledge`.
- Relative or empty values are ignored (XDG rule).
- `HOME` missing and no usable XDG value: `HOME is not set.` (error), only for commands that need the folder.
- `help` and `history`/`bm` usage do not need the folders.

Files:

| File | Path | Format |
|---|---|---|
| config | `<config>/config.json` | JSON object |
| profile | `<config>/profiles/<name>.json` | JSON object with `windows` |
| history | `<cache>/history` | one absolute path per line, newest first |
| bookmarks | `<cache>/bookmarks` | one absolute path per line, insertion order |
| bm-edit state | `<cache>/bm-edit-open` | one absolute path per line (expanded folders) |

Test folder isolation: tests set `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` to temp folders (testing.md).

### 9.2 Config (FR-CONF-*)

Default content written on first run, exactly:

```json
{
  "search_paths": ["~/projects"]
}
```

(D-03: the literal `~/projects`; the folder itself is not created.) The file is written only when it
does not exist, using `create_new` (D-05: never rewritten). The profiles folder is created when the
config is loaded and missing (FR-CONF-06).

Parsing (`domain::config::parse`):
- Not valid JSON: `config is not valid JSON: <file>` (FR-CONF-05).
- `search_paths` missing: empty list. Not a list of strings: `config is invalid (...)` [new].
- Entries: `~` at the start expands to `$HOME`; `$VAR` and `${VAR}` expand from the environment. An undefined
  variable is an error `config search path '<entry>' uses undefined variable '<VAR>'.` [new] (F-24, the
  analyst's suggestion). Empty entries are skipped (FR-CONF-04). Each entry is made absolute and cleaned,
  and duplicates are removed keeping the first (F-24).
- Unknown keys are ignored. `default_profile` must be a string when present; any other type is the invalid
  message.
- A search path that does not exist is kept in the config; pickers skip it silently and doctor warns
  (F-24).
- Symlinked directories count as projects (`is_dir` follows links).

### 9.3 Atomic writes (D-05)

`FileSystem::write_atomic(path, text)`:
1. create the parent folder if missing;
2. write `text` to `<dir>/.<file name>.tmp-<pid>` (a file in the same folder);
3. `sync_all` the temp file;
4. `rename` it over the target.

A crash leaves the old or the new file. Two writers: the last rename wins (no locking). The temp file name
includes the pid so two ledge processes do not share it.

### 9.4 Reading the cache files

- Missing file: empty list (FR-HIST-05 and FR-HIST-03 messages come from the command, not from here).
- Lines are trimmed of trailing whitespace. Empty lines are skipped. Lines that are not absolute paths are
  skipped (A-09). The rest are cleaned (`AbsPath::clean`, D-06) and deduplicated keeping the first
  occurrence.
- History keeps at most 50 entries when read (F-36). A longer file is trimmed in memory and rewritten
  only by the next add or clean.
- Bookmark and history entries are not checked for existence on read; the display marks missing ones.

## 10. Dependencies

| Crate | Version (Cargo.toml) | Used for | Why this and not std or own code |
|---|---|---|---|
| `ratatui` | `0.29` (the newest 0.x release of at least two weeks at S-00; the lock file records the exact one) | TUI layout, `Buffer`, `TestBackend` for model tests; re-exports `crossterm` as `ratatui::crossterm` for raw mode, events and the alternate screen | Writing a terminal renderer and a crossterm-compatible event layer ourselves is hundreds of lines with platform risk. ratatui also provides the display width used for truncation. Its crossterm re-export avoids a second crossterm dependency. |
| `serde_json` | `1` | parsing `config.json` and profile files into `serde_json::Value`, so the shape errors can be reported with the exact FR-PROF and FR-CONF messages | Writing a JSON parser is a bad trade. No derive macros are used (no `serde` derive), so the tree stays small. |
| `md-5` | `0.10` | MD5 digest of the cleaned absolute path for the session name (FR-NAME-01) | A hand-written MD5 is about 100 lines and must match the earlier tool byte for byte; the RustCrypto crate is small and has test vectors we reuse. Not used for security. |
| `signal-hook` | `0.3` | registers SIGINT, SIGHUP and SIGTERM as flags (FR-TUI-06) | Catching signals needs an unsafe handler or libc calls. signal-hook is the safe, mainstream wrapper. Only the flag module is used. |

Not used, with reasons:
- No CLI framework (`clap` and similar): the grammar has error strings that a generic framework would
  reword, and flags are only accepted after command words (FR-CMD-11). The hand parser is about 250 lines
  and tested as a pure function.
- No `thiserror`: the error type has one variant; a hand `Display` is shorter than the derive setup.
- No `dirs`/`xdg`: the two folders are two environment variables and a fallback (domain::location).
- No `tempfile`, `chrono`, `time`, `regex`, `fuzzy-matcher`, `nucleo`: the fuzzy rules are ours (D-10),
  age text needs only seconds, profile names are checked by a short loop.
- No `libc`: not needed once signal-hook and crossterm are used.
- No `unicode-width` direct dependency: display width comes through ratatui.
- Dev dependencies: none. Tests use `std` and `CARGO_TARGET_TMPDIR`.

Adding a crate later needs user approval (conventions). `Cargo.lock` is committed.

## 11. Build, install and uninstall

Source of truth: `scripts/install.sh` (bash, POSIX-compatible where possible).

Install (`scripts/install.sh`):
1. If `cargo` is not on PATH: `Error: build toolchain is required to build ledge.` on stderr, exit 1.
2. `cargo build --release --locked` in the folder of the script.
3. Copy `target/release/ledge` to `$LEDGE_INSTALL_BIN_DIR` (default `$HOME/.local/bin`) with mode 0755. The
   folder is created when missing. An existing binary is replaced (A-16).
4. Write the source record: the absolute source folder (the folder of the script) as one line to
   `$LEDGE_INSTALL_STATE_DIR/install-source` (default `${XDG_STATE_HOME:-$HOME/.local/state}/ledge`).
5. Print `Installed <path>` on stdout [new].
6. If the bin folder is not in `PATH` (split on `:`): `Warning: <dir> is not on PATH; add it to use ledge.` on
   stderr [new]. Exit code stays 0.

Uninstall (`scripts/install.sh --uninstall`):
- Binary at `<bin dir>/ledge` missing: `Nothing to remove: <path> does not exist` (exit 0).
- Binary present and the record names this source folder: delete binary and record, print `Removed <path>`,
  exit 0.
- Binary present and the record is missing or names another folder: `Not removed: <path> was not installed
  from <source>` on stderr, exit 1. `<source>` is this source folder.
- Configuration, profiles, history and bookmarks are never touched (requirement).

Which program owns uninstall (F-30): the install script. The `ledge` binary has no uninstall command.

Build and runtime: `Cargo.toml` sets `edition = "2021"`, `rust-version` (conventions), and
`[lints]` (conventions). The binary has no runtime file lookup: profiles, config and cache are found from
the environment only.

Testing the installer: `scripts/test_install.sh` puts a fake `cargo` on PATH that writes a stub binary,
and sets both install variables to temp folders. It never touches the real `~/.local`.

## 12. Requirement-to-module map

| FR group | Module(s) | Slices |
|---|---|---|
| FR-PLAT | adapters::tmux (FR-PLAT-01), domain::location (FR-PLAT-02) | S-09, S-01 |
| FR-TUI | tui::fuzzy, tui::picker, tui::columns, tui::render, tui::keys, tui::engine, adapters::terminal, adapters::signals | S-06, S-07, S-08 |
| FR-OPEN | cli::parse (01, 02, 04), app::open, app::build, domain::naming, adapters::tmux | S-17, S-11, S-10, S-01, S-09 |
| FR-PICK | domain::projects (01..05), app::pickers (06..10, 12, 13), cli::parse (11) | S-04, S-13, S-17 |
| FR-HIST | domain::history, app::history_cmd, app::pickers (04, 05) | S-03, S-18, S-13 |
| FR-BM | domain::bookmarks, app::bookmark_cmd, app::pickers (bookmark picker), cli::parse (14) | S-03, S-18, S-19, S-17 |
| FR-EDIT | domain::bm_edit, domain::projects (08..10), tui (edit window), app::bookmark_cmd | S-20, S-21 |
| FR-TREE | domain::tree, app::session_cmd (tree), app::bookmark_cmd (tree), cli::parse (09, 10) | S-05, S-22, S-23, S-24, S-17 |
| FR-SESS | domain::sessions, domain::tree (06, 07), app::session_cmd, app::pickers (switcher), domain::age | S-16, S-05, S-23, S-25, S-17 |
| FR-ACT | app::actions, domain::sessions (node membership), app::pickers (keys), tui::keys | S-14, S-15, S-19, S-22, S-24 |
| FR-PROF | domain::profile, domain::profile_rules, app::build, app::open, app::actions | S-02, S-10, S-11, S-14, S-15 |
| FR-CONF | domain::config, app::store, domain::location | S-02, S-04 (07) |
| FR-NAME | domain::naming, adapters::tmux (marker and profile options), domain::sessions (managed filter) | S-01, S-09 |
| FR-SCOPE | domain::projects (group membership), domain::sessions (group selection) | S-04 |
| FR-DOC | app::doctor, section 8.4 | S-26 |
| FR-CMD | cli::parse, cli::help, error, main (exit codes), app (picker Cancelled) | S-00, S-17, S-13 |
| FR-LAUNCH | app::launch, adapters::process | S-12 |

Installation has no FR-IDs yet (see proposal P-1). Its design is section 11 and slice S-27.

## 13. Assumptions

Each entry is a finding of `docs/analysis.md` that the gate A decisions (D-11..D-17) did not settle, or a
design choice this document makes on top of them. The user can override any of them; an override changes
only the named section.

| ID | Finding | Assumption | Where |
|---|---|---|---|
| A-01 | F-07 messages without FR, FRs without messages | Each message is mapped to its nearest FR (8.3). `FR-CMD-09` has no runtime program other than tmux (tmux uses FR-CMD-10), so its message is produced by a helper `messages::required_program(name)` with a unit test only. Proposal P-2. | 8.3, S-17 |
| A-02 | F-09 search column per picker | Named per picker (6.2 table): project name, history project, session name, full bookmark path, node basename without glyphs. | 6.2 |
| A-03 | F-10 chrome | Labels, icons, headers and footers per picker in 6.2. History footer lists the project picker keys (FR-TUI-04). Profile picker has a key footer. | 6.2 |
| A-04 | F-11 dimming without color | Text cues: ` (missing)` suffix, `(name)` for connectors, `●`/`○`/`*` markers, reverse video for the selection. Color off on `NO_COLOR` (present and non-empty) or `TERM=dumb`. | 6.2 |
| A-05 | F-17 and FR-NAME-05 adoption | An adopted session with no `@ledge_dir` skips the collision check. | 7.3 |
| A-06 | F-12 truncation and minimum size | Shrink order and 40x8 minimum (6.2). CJK counted as 2 cells. | 6.2 |
| A-07 | F-16 and build failures | Partial sessions are killed on failure. Rebuilding the current session: `<name>-rebuild` is built, the client switches, the old session is killed, the new one is renamed (D-17). If `<name>-rebuild` already exists it is a build failure with `could not create session '<name>-rebuild': name already used`. The error is shown in the picker status line; the picker stays open. | 7.2, S-14 |
| A-08 | F-14 cancel details | Empty choice cancels. Escape and ctrl-c in a nested profile picker return to the parent with no output. ctrl-c is a cancel (exit 0), not an interrupt. | 6.3 |
| A-09 | F-24 and cache file edge cases | Relative lines in cache files are skipped. The 50-entry limit applies on read in memory (F-36). | 9.4 |
| A-10 | F-13 age text | Exactly the analyst's suggestion: under 60 s `just now`, under 60 min `<n>m ago`, under 24 h `<n>h ago`, else `<n>d ago`, rounded down, no weeks unit. Ties in activity sort by session name. | domain::age, S-16 |
| A-11 | F-19 profile picker rows | Order: current (rebuild of one session), default, then all other valid profiles and `basic` sorted by name. A profile that is current and default is one row with both labels. Preselection: the first row. Profiles that are invalid are listed and fail with their message in the status line when chosen (the picker stays open). | domain::profile_rules, S-15 |
| A-12 | F-27 doctor severity and config | Program absences: tmux ERR, setsid, code and idea WARN. Profile and bookmark problems WARN, config problems ERR. Doctor never creates the config. Check order fixed (8.4). | 8.4 |
| A-13 | F-20 tree node sets | Session tree: one root per project containing a managed session (gray when it has no session, openable); connectors between root and sessions are gray and not openable; a session outside every project is its own root labelled with its path. Bookmark tree: one root per project with a bookmark (gray, not openable); every directory between the root and a bookmark is a gray connector; the shared parent is one of those. A gray folder with no session in the session tree (openable) shows its would-be name (FR-SESS-07). | domain::tree, S-05 |
| A-14 | F-21 "inside" | `inside(child, base)` is equal or whole-component descent. A project is not a bookmark candidate (FR-BM-02). `bm edit <project>` works (the project contains itself, FR-EDIT-08). | domain::path |
| A-15 | F-22 path placeholders | Cleaned absolute path everywhere. Exception: `Directory '<dir>' not found.` echoes the argument as typed (D-06). Path tail-truncation in tables. | 5, 6.2 |
| A-16 | F-23 current-directory row, Submodules | The `*` flag in the first marker cell marks the current directory. A project current directory is listed once, first. A non-project current directory is an extra first row showing its path. Git submodules are ordinary directories; the label `Projects & Submodules` is kept as required by FR-PICK-13. Install over an existing binary replaces it. | domain::projects, S-04, S-27 |
| A-17 | F-25 and the tmux format | Environment names are `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` (absolute values only). `#{session_path}` must exist in tmux 3.2; S-09 checks it. | 9.1, 7.1 |
| A-18 | F-26 editor launch | Editor options apply to every open path (`ledge`, project open, pickers, `bm tree`, `ss`, `ss tree`). Launch happens only when the open path creates the session (FR-LAUNCH-01). A rebuild does not launch an editor. A missing editor is a warning and the session still opens. `setsid -f` is used when present; without it the editor is spawned with stdio null and a warning. Editor command is split on whitespace; the directory is appended. Messages in 8.3 are `[new]`. | 2.3, app::launch, S-12 |
| A-19 | F-28 bookmark picker and `bm add` | Enter opens the bookmarked directory's own session (named from that directory). `bm add` on a missing path uses `Directory '<dir>' not found.`. Delete kills only a managed session of that directory. | S-19 |
| A-20 | F-29 kill target | Exact managed session name first, then the directory of the argument (cleaned); both misses use `'<name>' is not a ledge session.`. The current session is refused in `ss kill` as in D-17 (`cannot kill the session you are in.`). | S-25 |
| A-21 | F-30 installation | Install and uninstall live in `scripts/install.sh`, not in the binary. The messages are `[new]` (8.3). There are no FR-IDs yet; the requirement change is proposal P-1. The installer is tested by `scripts/test_install.sh`. | 11, S-27 |
| A-22 | F-31 seams | Covered by the ports in section 4. | 4 |
| A-23 | F-32 overlapping requirements | Harmless. Module map (section 12) cites both sides for each pair. | 12 |
| A-24 | F-33 collisions | Accepted and handled by D-14 (refuse with `name already used by <dir>`). Recorded in section 7.2. | 7.2, S-10 |
| A-25 | F-34 odd wording | `Project history cache clear successfully.` is kept byte for byte (the analyst asked for confirmation; no change made). | 8.3 |
| A-26 | F-35 manual versus automated | Picker keys and layouts are automated (headless: fake terminal and `TestBackend`). Manual only: real attach and switch, GUI editor launch, true color and terminal restore look. See testing.md. | testing.md |
| A-27 | F-36 history edge cases | History is written before the open (FR-OPEN-10), so a failed open still records the entry (intended, per the requirement order). Missing directories in the history picker are dimmed and report `Missing directory: <path>` on choose. | S-03, S-13 |
| A-28 | F-37 navigation | Home and end go to the first and last row. No wrap-around, no mouse. Filter change moves the selection to the best match; preselection kept on an empty filter. | 6.1 |
| A-29 | F-08 tmux needed by command | Commands needing tmux: open, project pick, history pick, bm pick, bm tree, bm edit with `--session-scope`, ss, ss tree, ss list, ss kill, and rebuild or kill actions. Commands not needing tmux: help, doctor (reports instead), history list and clean, bm add, remove, list, bm edit without `--session-scope`. | 2.3, S-17 |
| A-30 | F-06 streams for "print and stop" | Taken from D-16. Messages not in D-16 are errors. | 8.2 |
| A-31 | Typing `<cmd> <extra words>` | `'<command words>' takes no arguments. [new]` (2.3) | 2.3 |
| A-32 | Tilde in CLI directories | A leading `~/` in a CLI directory argument is the home folder, as the shell would do (not a requirement). | domain::path |
| A-33 | Signals | Interrupt exit codes 130 (SIGINT), 129 (SIGHUP), 143 (SIGTERM) (6.5). | 6.5 |
| A-34 | `ss` list order and format | `ss list` table: header row `Session  Windows  State  Path`, rows sorted by name, columns padded to the widest value, state `attached` or `detached`. | S-25 |
| A-35 | Doctor and `HOME` | Doctor does not read the cache folder. | 8.4 |
| A-36 | Bare-form flags | `ledge --profile dev` opens cwd with profile dev; explicit commands reject leading flags (2.3). | 2.3 |
| A-37 | Session directory | `@ledge_dir` first, then tmux `session_path`. Node membership uses the same directory (FR-SCOPE-02). | 5 |
| A-38 | `--profile` on rebuild | FR-ACT-12 applies to rebuild and ctrl-p. alt-r uses each session's recorded profile even with `--profile` (FR-ACT-08, D-13). | S-14, S-22 |
| A-39 | Kill key on the bookmark current session | ctrl-d deletes the bookmark and does not kill the current session; the status line says `cannot kill the session you are in.` (D-17). | S-19 |
| A-40 | Rebuild launches | A rebuild never launches an editor (see A-18). | S-14 |
| A-41 | Terminal restore test | Restore on error and on panic is tested with a fake terminal (drop order) and by hand (testing.md). | S-08 |
| A-42 | F-24 search path and config edge cases | A search path that does not exist is kept, skipped silently by pickers, and reported by doctor (WARN). Entries are made absolute, cleaned and deduplicated keeping the first. Symlinked directories count as projects. An undefined variable is an error (`config search path ... undefined variable`, [new]). A wrong type for `search_paths` or `default_profile` is the invalid-config message [new]. Unknown keys are ignored. An unwritable config folder is `could not create config` [new]. Only commands that load the config create it; `help` and `doctor` do not. | 9.2, S-02 |

Requirements proposals raised by this design are in `docs/proposals.md` (P-1 installation IDs, P-2 FR-CMD-09,
P-3 new messages for the user to confirm).

# Testing

STATUS: APPROVED
Owner: user. Finalised by the architect in Phase 0 (gate A; gate B waived). The quality section of
`requirements.md` is the basis, and F-35 (picker keys headless) is applied.

## Principles
- Tests are derived from requirements (FR-IDs and the exact Messages), not from the implementation.
- Every FR-ID is covered by at least one automated test, or is listed in `docs/traceability.md` as `manual`
  with a reason in the Note column.
- Tests are deterministic: no wall clock (use `FakeClock`), no real home folder (use `FakeEnv` with temp
  folders), no network, no user tmux server (integration tests use their own server name).
- A test that needs a fact from the requirements cites the FR-ID; a test that checks text compares it to the
  literal string from the requirements, not to a value taken from the code.

## Levels

1. **Unit (inline, default).** `#[cfg(test)] mod tests` in `domain/*`, `tui/fuzzy.rs`, `tui/columns.rs`,
   `cli/parse.rs`. Pure functions only. Required for: session naming and vectors (FR-NAME-01/02), path
   cleaning and `inside` (D-06, A-14), config parsing and defaults, profile validation, history add/read/limit,
   bookmark add/remove/read and normalisation, tree construction and scope, column alignment and truncation,
   relative age text, rebuild and open precedence, picker filtering and selection (fuzzy), CLI grammar.
2. **Component (`tests/component_*.rs`).** `app` services against `FakeBackend`, `FakeProcess`, `FakeConsole`,
   `FakeEnv`, `FakeClock`, `FakeSignals` and `RealFs` on temp folders. Covers command behaviour, messages,
   exit outcomes, rebuild and kill rules, the open flow, history and bookmark commands, doctor, editor launch.
3. **TUI model (`tests/tui_*.rs`).** `run_picker` driven by scripted `Input` events through `FakeTerminal`
   (ratatui `TestBackend`). Asserts rows, selection, status line, footer, header, resize, cancel and choose.
   This is where picker keys and layouts are verified (F-35). Each picker in the screen tables gets at least
   one test per key in its table.
4. **Integration (tmux, `tests/it_tmux_*.rs`).** Real tmux, marked `#[ignore = "requires tmux"]`. Run with
   `cargo test -- --ignored`. Gate: `scripts/gate.sh` runs them when `tmux` is on PATH and prints `SKIP` otherwise.
   Each test uses its own server name (see below).
5. **Messages (`tests/messages_verbatim.rs`).** One assertion per literal message from `requirements.md`
   "Messages" and D-11..D-17 and each `[new]` message in architecture section 8.3. A message change breaks this
   file on purpose.
6. **Checks.** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --locked`,
   `cargo test --locked`, `scripts/check_layers.sh`, and (from S-27) `scripts/test_install.sh`.

## tmux isolation (FR-NAME-02 tests, quality section)
- Env var for the separate server name: `LEDGE_TMUX_SERVER` (kept as the architect's name; architecture 7.1).
  When set and non-empty, every tmux command uses `-L $LEDGE_TMUX_SERVER`. When unset, the default server
  is used, so integration tests must always set it.
- Integration tests set `LEDGE_TMUX_SERVER` to `ledge-it-<pid>-<n>` (unique per test: pid plus a counter in the
  test helper). A `TestServer` guard calls `tmux -L <name> kill-server` on drop and ignores failures.
- Integration tests remove `TMUX` from the environment of the test process and of the spawned `ledge` binary, so
  a developer running the tests inside tmux does not reach their own session.
- The gate script (`scripts/gate.sh`) sets `LEDGE_TMUX_SERVER=ledge-gate-<pid>` for the ignored run and kills
  that server in a `trap ... EXIT`.
- Tests never read or write the user's real `~/.config/ledge` or `~/.cache/ledge`. Component and integration tests
  set `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` to folders under `CARGO_TARGET_TMPDIR`. Installer tests set
  `LEDGE_INSTALL_BIN_DIR` and `LEDGE_INSTALL_STATE_DIR` to temp folders.
- Integration tests use the built binary through `env!("CARGO_BIN_EXE_ledge")` for the end-to-end cases and
  `TmuxBackend` directly for the adapter cases.

## Test layout

```
src/**/*.rs                     unit tests, inline (#[cfg(test)] mod tests)
tests/
  support/
    mod.rs                      re-exports the fakes and helpers
    fake_backend.rs             FakeBackend: SessionBackend in memory (sessions, options, windows, scripted failures)
    fake_process.rs             FakeProcess: ProcessRunner with scripted replies, records argv and env
    fake_console.rs             FakeConsole: captures out and err lines
    fake_env.rs                 FakeEnv: map-backed Env, cwd, home
    fake_clock.rs               FakeClock: settable now_secs
    fake_signals.rs             FakeSignals: scripted interrupt
    fake_terminal.rs            FakeTerminal: scripted Input events, TestBackend, records draws
    temp.rs                     TempTree: unique folder under CARGO_TARGET_TMPDIR, removed on drop
    ctx.rs                      builds app::Ctx from the fakes plus a TempTree (one call per test)
  component_config.rs           FR-CONF, FR-PROF validation through the config service
  component_open.rs             FR-OPEN, FR-PROF precedence, FR-LAUNCH
  component_actions.rs          FR-ACT, FR-SESS-10..13
  component_history.rs          FR-HIST
  component_bookmarks.rs        FR-BM
  component_edit.rs             FR-EDIT (state and commands)
  component_doctor.rs           FR-DOC
  component_cli.rs              FR-CMD: parsing through cli::run with fakes, exit outcomes, help lists every command
  tui_project_picker.rs         FR-PICK, FR-TUI keys on the project and history pickers
  tui_switcher.rs               FR-SESS-02..04 screen and keys
  tui_trees.rs                  FR-TREE-11/12, FR-ACT-06/08/09 keys in both trees
  tui_bookmark_picker.rs        FR-BM-11..13, FR-ACT-11
  tui_edit_window.rs            FR-EDIT-02, 03, 07 keys and view
  tui_engine.rs                 FR-TUI-05, FR-TUI-06 (restore on error via drop order)
  messages_verbatim.rs          the literal message table
  it_tmux_adapter.rs            #[ignore]: TmuxBackend against real tmux (commands in architecture 7.1)
  it_tmux_cli.rs                #[ignore]: ledge binary end to end: open (detached create), ss list, kill, rebuild
scripts/test_install.sh         installer tests (bash)
```

Fakes live only in `tests/support/`. Unit tests never need fakes (they are pure). Production code never
depends on `tests/support`. A new fake is added to `tests/support` by the test-engineer, once, and reused.

## Rules for fakes
- Each fake implements the port trait exactly; no test-only methods on production types.
- `FakeBackend` models tmux semantics the code relies on: the server exists when a session exists; `list`
  returns sessions in creation order; `kill` removes windows; options survive rename; `new_session` on an
  existing name fails with `duplicate session`. A scripted failure can be set for any operation.
- `FakeProcess` records every `(program, argv, env override)` and returns the scripted `Captured`. Default for an
  unscripted call is status 0 and empty output.
- `FakeTerminal` returns the scripted `Input` values in order; after the last one it returns no input. The engine
  is run with a guard of 1000 polls, so a test that forgets to close the picker fails instead of hanging.

## Picker test pattern
1. Build a `PickerModel` from fixture rows (`tui::picker::PickerModel::new`).
2. Feed `Input::Key` events through `run_picker` with a `FakeHandler` that records `choose` and `act` calls.
3. Assert on the returned `PickerResult` and the recorded calls, and on the last `TestBackend` buffer
   (header, footer, status, `>` prompt, selected row).
Resize: `Input::Resize(w, h)` followed by a draw. Assert the selected row is inside the buffer.

## Coverage of the quality section
- Picker filtering and selection: unit (fuzzy, picker model) and TUI model tests.
- Session naming, config, profiles, history, bookmarks, trees, columns, age text, rebuild rules: unit and
  component tests (no tmux).
- Picker keys and layouts: TUI model tests, automated (F-35).
- Real tmux effects (create, attach, switch, rebuild, kill, option tags): integration tests (`#[ignore]`).
- Editor launch: component test with `FakeProcess` (the process argv and detachment), not a real editor.

## Manual only (documented in traceability with a reason)
- Real attach and switch from a real terminal (FR-OPEN-08, FR-SESS-04): needs a real client and tty.
- GUI editor launch (FR-LAUNCH-02 for `code` and `idea`): needs a desktop session.
- Look and feel: colors and dimming on a real terminal (FR-TUI-08, FR-TUI-09), and terminal restore on a real
  interrupt (FR-TUI-06 the signal path).
- Installer on a real machine: `scripts/install.sh` with a real `cargo` (the fake-cargo test covers the logic).

## Acceptance for each slice
A slice is done when: its FR-IDs have tests (or a manual reason), the gate passes, `docs/traceability.md` shows
the tests and code columns and status `covered` (or `manual`), and the reviewer approves.

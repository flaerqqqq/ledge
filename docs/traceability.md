# Traceability

STATUS: APPROVED
Updated by test-engineer (tests column) and implementer (code column) in the same slice.
The reviewer rejects a slice whose FR-IDs are not reflected here.

Status: `open` | `covered` | `manual` (needs a reason in the Note column).
Seeded by the architect in Phase 0 from `docs/backlog.md`: one row per FR-ID of `requirements.md` (160 rows),
each in exactly one slice. Installation statements have no FR-IDs yet (proposal P-1).

| FR-ID | Slice | Code | Tests | Status | Note |
|---|---|---|---|---|---|
| FR-PLAT-01 | S-09 | src/ports/backend.rs; src/adapters/tmux.rs; src/adapters/process.rs | tests/component_tmux.rs; tests/it_tmux_adapter.rs (ignored, real tmux); src/adapters/tmux.rs (unit) | covered | Argv and error mapping on FakeProcess (default gate). Real tmux on an isolated server: version 3.2 minimum, create, list, kill, rename (ignored, run by the gate when tmux is on PATH). Manual: the `#{session_path}` check on a real tmux 3.2 (A-17), since the gate has tmux 3.5a only. |
| FR-PLAT-02 | S-01 | domain/location.rs; adapters/env.rs; ports/env.rs | domain/location.rs; tests/component_location.rs | covered |  |
| FR-TUI-01 | S-08 | src/tui/engine.rs; src/adapters/terminal.rs | tests/tui_engine.rs; src/adapters/terminal.rs (unit) | covered | The picker is drawn by `run_picker` on a fake terminal. Real drawing on a tty is in the FR-TUI-06 manual check. |
| FR-TUI-02 | S-06 | src/tui/fuzzy.rs | src/tui/fuzzy.rs (unit tests) | covered |  |
| FR-TUI-03 | S-07 | src/tui/keys.rs; src/tui/picker.rs | src/tui/keys.rs, src/tui/picker.rs (unit); tests/tui_engine.rs | covered |  |
| FR-TUI-04 | S-07 | src/tui/render.rs | tests/tui_engine.rs | covered | Label, icon, header, prompt, status and footer text. Footer strings come from the caller (S-13 onward). |
| FR-TUI-05 | S-07 | src/tui/picker.rs; src/tui/columns.rs; src/tui/render.rs | src/tui/picker.rs, src/tui/columns.rs (unit); tests/tui_engine.rs; tests/messages_verbatim.rs | covered | Resize and 40x8 minimum, on TestBackend. Real terminal resize is checked with the S-08 engine. |
| FR-TUI-06 | S-08 | src/tui/engine.rs; src/adapters/terminal.rs; src/adapters/signals.rs; src/main.rs | tests/tui_engine.rs; src/adapters/signals.rs, src/adapters/terminal.rs (unit) | covered | Headless: the terminal is dropped (restore counted by FakeTerminal) on accept, cancel, interrupt (130, 129, 143 via FakeSignals), handler error and terminal error. Manual: the real raw-mode and alternate-screen restore and real SIGINT/SIGHUP/SIGTERM delivery, with the panic hook; reason: needs a real tty and OS signals, which tests do not have. |
| FR-TUI-07 | S-07 | src/tui/keys.rs | src/tui/keys.rs (unit); tests/tui_engine.rs | covered | One key map for all pickers. Footers of the screen tables are set by later picker slices. |
| FR-TUI-08 | S-07 | src/tui/render.rs; src/tui/picker.rs | tests/tui_engine.rs | covered | Text cue and dim style automated on TestBackend; color appearance is manual. |
| FR-TUI-09 | S-07 | src/tui/render.rs | src/tui/render.rs (unit); tests/tui_engine.rs | covered | Text cues, reverse video and NO_COLOR/TERM=dumb rule automated; color appearance is manual. |
| FR-TUI-10 | S-07 | src/tui/picker.rs; src/app/pickers.rs | src/tui/picker.rs (unit); tests/tui_engine.rs; tests/tui_project_picker.rs | covered | Model: should_close and reload (S-07). Picker side (S-13): a choose on a missing directory keeps the history picker open with its rows. Row actions that reload (ctrl-r, ctrl-p, ctrl-x) are no-ops until S-15. |
| FR-OPEN-01 | S-17 |  |  | open |  |
| FR-OPEN-02 | S-17 |  |  | open |  |
| FR-OPEN-03 | S-11 | src/app/open.rs; src/messages.rs | tests/component_open.rs; tests/messages_verbatim.rs | covered | Missing and non-directory targets on temp folders; the argument is echoed as typed (A-15). |
| FR-OPEN-04 | S-17 |  |  | open |  |
| FR-OPEN-05 | S-01 | domain/naming.rs; domain/path.rs | domain/naming.rs | covered |  |
| FR-OPEN-06 | S-01 | domain/naming.rs | domain/naming.rs | covered |  |
| FR-OPEN-07 | S-10 | src/app/build.rs | tests/component_build.rs | covered | Create only when no session of the name exists; existing unmanaged reused, managed with another @ledge_dir refused (D-14). Ctx not created yet (proposal P-20).
| FR-OPEN-08 | S-11 | src/app/open.rs; src/ports/backend.rs | tests/component_open.rs | manual | Automated: TMUX non-empty switches, otherwise attaches, with the session name and failure messages checked on FakeBackend. Manual: the real `attach-session` and `switch-client` need a tmux client on a real tty, which the gate does not have. |
| FR-OPEN-09 | S-09 | src/adapters/tmux.rs | tests/it_tmux_adapter.rs (ignored, real tmux) | covered | With no server, list is empty; the first create starts the server (real tmux, isolated server name). |
| FR-OPEN-10 | S-11 | src/app/open.rs; src/app/store.rs | tests/component_open.rs | covered | History written before the session is built, so a failed build still records the directory (A-27). |
| FR-OPEN-11 | S-11 | src/app/open.rs; src/messages.rs | tests/component_open.rs; tests/messages_verbatim.rs | covered | History folder blocked by a file: one `Warning: could not update history:` line on stderr, session still created and attached. |
| FR-PICK-01 | S-04 | domain/projects.rs; app/mod.rs | domain/projects.rs; tests/component_projects.rs | covered |  |
| FR-PICK-02 | S-04 | domain/projects.rs; app/mod.rs | domain/projects.rs; tests/component_projects.rs | covered |  |
| FR-PICK-03 | S-04 | domain/projects.rs | domain/projects.rs; tests/component_projects.rs | covered |  |
| FR-PICK-04 | S-04 | domain/projects.rs; src/app/pickers.rs | domain/projects.rs; tests/tui_project_picker.rs | covered | Current row first and preselected on screen, also with a launch directory argument. |
| FR-PICK-05 | S-04 | domain/projects.rs; src/app/pickers.rs | domain/projects.rs; tests/tui_project_picker.rs | covered | The '*' cell and the path of a launch directory that is not a project, on TestBackend. |
| FR-PICK-06 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | Basename, session name and profile of a live session, on TestBackend. Manual: the look of the rows on a real terminal (testing.md, look and feel); the cells and rendering are automated because the gate has no tty. |
| FR-PICK-07 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | Filled marker for a managed session found by name (D-14), empty marker otherwise, on FakeBackend. Real tmux listing is FR-PLAT-01 (integration). |
| FR-PICK-08 | S-13 | src/app/pickers.rs; src/app/open.rs | tests/tui_project_picker.rs | covered | Enter runs the open flow: session created, attach called on FakeBackend, history recorded. Manual: the real attach on a tty (FR-OPEN-08). |
| FR-PICK-09 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | Enter on a directory that is gone: status line, picker open, nothing opened. The directory is removed after the list is read (FakeTerminalFactory.remove_on_open). History entries missing from the start are marked (missing). |
| FR-PICK-10 | S-13 | src/app/pickers.rs; src/messages.rs (no_search_paths) | tests/tui_project_picker.rs; tests/messages_verbatim.rs | covered | Error with the config file path; no terminal opens. |
| FR-PICK-11 | S-17 |  |  | open |  |
| FR-PICK-12 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | --profile applies to the session the open creates (windows of the profile). Editor options for pickers are not in this entry point (proposal P-24). |
| FR-PICK-13 | S-13 | src/app/pickers.rs; src/messages.rs (projects_label) | tests/tui_project_picker.rs; tests/messages_verbatim.rs | covered | Label names the profile that applies: --profile, or default_profile (proposal P-25). Basic and --no-profile give the plain label. |
| FR-HIST-01 | S-03 | domain/history.rs; app/store.rs | domain/history.rs; tests/component_store.rs | covered |  |
| FR-HIST-02 | S-03 | domain/history.rs; app/store.rs | domain/history.rs; tests/component_store.rs | covered |  |
| FR-HIST-03 | S-18 |  |  | open |  |
| FR-HIST-04 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | Newest first, label and footer on screen, enter opens the chosen entry. Footer follows architecture 6.2 (proposal P-15 still open). Manual: the look on a real terminal (testing.md, look and feel). |
| FR-HIST-05 | S-13 | src/app/pickers.rs; src/messages.rs (no_history_cache) | tests/tui_project_picker.rs; tests/messages_verbatim.rs | covered | No history file: the message, stop, no window. |
| FR-HIST-06 | S-18 |  |  | open |  |
| FR-HIST-07 | S-18 |  |  | open |  |
| FR-HIST-08 | S-18 |  |  | open |  |
| FR-BM-01 | S-03 | domain/bookmarks.rs; app/store.rs | domain/bookmarks.rs; tests/component_store.rs | covered |  |
| FR-BM-02 | S-18 |  |  | open |  |
| FR-BM-03 | S-18 |  |  | open |  |
| FR-BM-04 | S-18 |  |  | open |  |
| FR-BM-05 | S-18 |  |  | open |  |
| FR-BM-06 | S-18 |  |  | open |  |
| FR-BM-07 | S-18 |  |  | open |  |
| FR-BM-08 | S-18 |  |  | open |  |
| FR-BM-09 | S-18 |  |  | open |  |
| FR-BM-10 | S-18 |  |  | open |  |
| FR-BM-11 | S-19 |  |  | open |  |
| FR-BM-12 | S-19 |  |  | open |  |
| FR-BM-13 | S-19 |  |  | open |  |
| FR-BM-14 | S-17 |  |  | open |  |
| FR-EDIT-01 | S-20 |  |  | open |  |
| FR-EDIT-02 | S-21 |  |  | open |  |
| FR-EDIT-03 | S-20 |  |  | open |  |
| FR-EDIT-04 | S-20 |  |  | open |  |
| FR-EDIT-05 | S-20 |  |  | open |  |
| FR-EDIT-06 | S-20 |  |  | open |  |
| FR-EDIT-07 | S-21 |  |  | open |  |
| FR-EDIT-08 | S-21 |  |  | open |  |
| FR-EDIT-09 | S-21 |  |  | open |  |
| FR-EDIT-10 | S-21 |  |  | open |  |
| FR-EDIT-11 | S-20 |  |  | open |  |
| FR-TREE-01 | S-05 | domain/tree.rs | domain/tree.rs | covered |  |
| FR-TREE-02 | S-05 | domain/tree.rs | domain/tree.rs | covered |  |
| FR-TREE-03 | S-05 | domain/tree.rs | domain/tree.rs | covered | Shared parent is a gray connector row, listed once (A-13). |
| FR-TREE-04 | S-05 | domain/tree.rs | domain/tree.rs | covered |  |
| FR-TREE-05 | S-24 |  |  | open |  |
| FR-TREE-06 | S-24 |  |  | open |  |
| FR-TREE-07 | S-24 |  |  | open |  |
| FR-TREE-08 | S-24 |  |  | open |  |
| FR-TREE-09 | S-17 |  |  | open |  |
| FR-TREE-10 | S-17 |  |  | open |  |
| FR-TREE-11 | S-05 | domain/tree.rs | domain/tree.rs | covered | Domain flag only (`openable`). Enter handling in the tree picker is wired in a later slice. |
| FR-TREE-12 | S-22 |  |  | open |  |
| FR-SESS-01 | S-16 |  |  | open |  |
| FR-SESS-02 | S-16 |  |  | open |  |
| FR-SESS-03 | S-16 |  |  | open |  |
| FR-SESS-04 | S-16 |  |  | open | Expected manual: attach and switch need a real client and tty; argv covered by adapter test. |
| FR-SESS-05 | S-17 |  |  | open |  |
| FR-SESS-06 | S-05 | domain/tree.rs | domain/tree.rs | covered |  |
| FR-SESS-07 | S-05 | domain/tree.rs | domain/tree.rs | covered | Label of a gray project root is `session_name` of its path. |
| FR-SESS-08 | S-23 |  |  | open |  |
| FR-SESS-09 | S-25 |  |  | open |  |
| FR-SESS-10 | S-25 |  |  | open |  |
| FR-SESS-11 | S-25 |  |  | open |  |
| FR-SESS-12 | S-25 |  |  | open |  |
| FR-SESS-13 | S-25 |  |  | open |  |
| FR-ACT-01 | S-15 |  |  | open |  |
| FR-ACT-02 | S-14 | src/app/actions.rs | tests/component_actions.rs | covered | `rebuild` builds a missing session from the profile (no kill). The rebuild key itself is S-15. |
| FR-ACT-03 | S-14 | src/app/actions.rs; src/app/build.rs (build_session) | tests/component_actions.rs | covered | The managed session is killed (its windows and running commands go) and built again from the profile. |
| FR-ACT-04 | S-15 |  |  | open |  |
| FR-ACT-05 | S-14 | src/app/actions.rs (rebuild_current) | tests/component_actions.rs | covered | D-17 on FakeBackend: `<name>-rebuild` built, client switched, old killed, copy renamed to `<name>`; a copy failure keeps the original (A-07). The real switch and rename on tmux are not run by the gate; the tmux commands are in tests/component_tmux.rs. |
| FR-ACT-06 | S-22 |  |  | open |  |
| FR-ACT-07 | S-14 | src/app/actions.rs (never_touch_current, kill) | tests/component_actions.rs; src/app/actions.rs (unit) | covered | Kill of the session the user is in is refused with `cannot kill the session you are in.`; `never_touch_current` tested. The alt-key callers that skip it are S-22. |
| FR-ACT-08 | S-22 |  |  | open |  |
| FR-ACT-09 | S-22 |  |  | open |  |
| FR-ACT-10 | S-24 |  |  | open |  |
| FR-ACT-11 | S-19 |  |  | open |  |
| FR-ACT-12 | S-15 |  |  | open |  |
| FR-ACT-13 | S-14 | src/app/actions.rs | tests/component_actions.rs | covered | Kill and rebuild refuse an unmanaged session with `'<name>' is not a ledge session.` and leave it unchanged. |
| FR-PROF-01 | S-02 | app/store.rs; domain/profile.rs | tests/component_config.rs | covered |  |
| FR-PROF-02 | S-02 | domain/profile.rs | domain/profile.rs; tests/component_config.rs | covered |  |
| FR-PROF-03 | S-10 | src/app/build.rs | tests/component_build.rs | covered | Windows created in profile order, duplicate names allowed.
| FR-PROF-04 | S-10 | src/app/build.rs | tests/component_build.rs | covered | Command typed per window via send_line (FakeBackend records typed lines).
| FR-PROF-05 | S-10 | src/app/build.rs | tests/component_build.rs | covered | First window is the session's first and is selected last.
| FR-PROF-06 | S-02 | domain/profile.rs | domain/profile.rs; tests/component_config.rs | covered |  |
| FR-PROF-07 | S-10 | src/app/build.rs | tests/component_build.rs | covered | @ledge_profile records the profile name; basic for the single-window layout.
| FR-PROF-08 | S-10 | src/app/build.rs | tests/component_build.rs | covered | Profile applied at creation only; reopening a session with another profile changes nothing.
| FR-PROF-09 | S-11 | src/domain/profile_rules.rs; src/app/open.rs | src/domain/profile_rules.rs (unit); tests/component_open.rs | covered | `--profile` over a valid default profile. |
| FR-PROF-10 | S-11 | src/domain/profile_rules.rs; src/app/open.rs | src/domain/profile_rules.rs (unit); tests/component_open.rs | covered | `--no-profile` gives the single Terminal window, records basic, and prints no warning. |
| FR-PROF-11 | S-11 | src/app/open.rs; src/app/store.rs | tests/component_open.rs | covered | Missing and invalid `--profile` return the profile error and create no session. |
| FR-PROF-12 | S-11 | src/domain/profile_rules.rs; src/app/open.rs | src/domain/profile_rules.rs (unit); tests/component_open.rs; tests/messages_verbatim.rs | covered | Missing or invalid default profile: warning, single-window layout. Valid default applies when no option is given. |
| FR-PROF-13 | S-15 |  |  | open |  |
| FR-PROF-14 | S-14 | src/domain/profile_rules.rs (rebuild_profile); src/app/actions.rs | tests/component_actions.rs; src/domain/profile_rules.rs (unit) | covered | Rebuild without a profile option reuses the recorded profile, ahead of `default_profile`. |
| FR-PROF-15 | S-14 | src/app/actions.rs (resolve_profile, default_or_basic); src/messages.rs | tests/component_actions.rs; tests/messages_verbatim.rs | covered | Warning `Warning: session profile '<name>' is missing or invalid; using the default profile.`, then the default profile, or basic with the FR-PROF-12 warning when the default fails too (D-04). |
| FR-CONF-01 | S-02 | app/store.rs; domain/config.rs | domain/config.rs; tests/component_config.rs | covered | Default text compared literally; the trailing newline after the architecture 9.2 block is an assumption. |
| FR-CONF-02 | S-02 | domain/config.rs; app/store.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-03 | S-02 | domain/config.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-04 | S-02 | domain/config.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-05 | S-02 | domain/config.rs; app/store.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-06 | S-02 | app/store.rs | tests/component_config.rs | covered |  |
| FR-CONF-07 | S-04 | domain/projects.rs; app/mod.rs | domain/projects.rs; tests/component_projects.rs | covered |  |
| FR-CONF-08 | S-01 | domain/location.rs; adapters/env.rs; ports/env.rs | domain/location.rs; tests/component_location.rs | covered |  |
| FR-NAME-01 | S-01 | domain/naming.rs; domain/path.rs | domain/naming.rs | covered | Vectors provisional until user supplies reference pairs (D-02). |
| FR-NAME-02 | S-01 | domain/naming.rs | domain/naming.rs | open | Vectors provisional until user supplies reference pairs (D-02); stays open until those pairs are supplied. |
| FR-NAME-03 | S-09 | src/adapters/tmux.rs; src/ports/backend.rs | tests/component_tmux.rs; tests/it_tmux_adapter.rs (ignored, real tmux) | covered | set-option argv on FakeProcess; real options read back, windows added. set-option uses `=<name>:` (proposal P-18). |
| FR-NAME-04 | S-09 | src/domain/sessions.rs; src/adapters/tmux.rs | tests/component_tmux.rs; tests/it_tmux_adapter.rs (ignored, real tmux) | covered | `managed()` keeps only marked sessions; real server session without the marker is filtered out. |
| FR-NAME-05 | S-09 | src/adapters/tmux.rs | tests/it_tmux_adapter.rs (ignored, real tmux); src/adapters/tmux.rs (unit) | covered | A session given `@ledge_managed 1` by hand (tmux CLI) is listed as managed; no `@ledge_dir` gives no directory (A-05). |
| FR-SCOPE-01 | S-04 | domain/projects.rs | domain/projects.rs | covered |  |
| FR-SCOPE-02 | S-04 | domain/sessions.rs | domain/sessions.rs | open | Directory membership only. The managed filter and session directory come with the session slices. |
| FR-DOC-01 | S-26 |  |  | open |  |
| FR-DOC-02 | S-26 |  |  | open |  |
| FR-DOC-03 | S-26 |  |  | open |  |
| FR-DOC-04 | S-26 |  |  | open |  |
| FR-DOC-05 | S-26 |  |  | open |  |
| FR-DOC-06 | S-26 |  |  | open |  |
| FR-DOC-07 | S-26 |  |  | open |  |
| FR-DOC-08 | S-26 |  |  | open |  |
| FR-CMD-01 | S-17 |  |  | open |  |
| FR-CMD-02 | S-17 |  |  | open |  |
| FR-CMD-03 | S-17 |  |  | open |  |
| FR-CMD-04 | S-17 |  |  | open |  |
| FR-CMD-05 | S-17 |  |  | open |  |
| FR-CMD-06 | S-17 |  |  | open |  |
| FR-CMD-07 | S-00 | src/main.rs (report, main); src/error.rs (LedgeError Display); src/messages.rs (cancelled) | src/main.rs (report, 5 tests); tests/messages_verbatim.rs; tests/crate_targets.rs; tests/gate_scripts.rs | covered | Also covers S-00 acceptance for the gate and layer scripts (tests/gate_scripts.rs), which the backlog lists under this slice. |
| FR-CMD-08 | S-13 | src/messages.rs (cancelled); src/app/pickers.rs | src/main.rs (cancelled test); tests/messages_verbatim.rs; tests/tui_project_picker.rs | covered | Escape in the project and history pickers gives Outcome::Cancelled, printed as Cancelled. by report. The nested profile picker cancel is S-15. |
| FR-CMD-09 | S-17 |  |  | open | No runtime program besides tmux uses this message; helper only (proposal P-2). |
| FR-CMD-10 | S-17 |  |  | open |  |
| FR-CMD-11 | S-17 |  |  | open |  |
| FR-CMD-12 | S-13 | src/app/pickers.rs | tests/tui_project_picker.rs | covered | Empty at start: no history file, empty history file and no projects exit without a window. Empty after an action: the switcher and trees (later slices); no S-13 action empties a list. |
| FR-LAUNCH-01 | S-12 | src/app/launch.rs; src/app/open.rs (open_dir_with_editor) | tests/component_open.rs | covered | Launch only when `ensure_session` reports Created: reuse, and a plain open without an editor, start nothing (FakeProcess). The pickers, `bm tree`, `ss` and `ss tree` open paths are not wired yet (later slices). |
| FR-LAUNCH-02 | S-12 | src/app/launch.rs; src/ports/process.rs; src/adapters/process.rs | tests/component_open.rs; tests/messages_verbatim.rs | manual | Automated: `code`, `idea` and `--editor` argv (split on whitespace, directory last) on FakeProcess. Manual: the GUI editor really opens the directory; reason: needs a desktop session with VS Code or IntelliJ IDEA installed, which the gate does not have (testing.md, manual list). The `--editor` value parsing and the `--code`/`--idea`/`--editor` option grammar are S-17 (FR-CMD-06). |
| FR-LAUNCH-03 | S-12 | src/app/launch.rs; src/adapters/process.rs (spawn_detached) | tests/component_open.rs | covered | Detached spawn checked on FakeProcess argv: `setsid -f <cmd> <dir>` with setsid present, direct start with the setsid warning without it. The process outliving ledge is the setsid mechanism itself and is not run by the gate. |

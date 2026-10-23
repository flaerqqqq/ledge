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

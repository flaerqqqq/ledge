# Decisions

STATUS: APPROVED

Resolves "Decisions still open" in `requirements.md`. Statuses: `Proposed` (analyst's suggestion,
awaiting the user), `Accepted`, `Rejected`. Only the user sets `Accepted`/`Rejected`.
The requirement-analyst may refine the proposed text and rationale but not the status.
Finding numbers (F-nn) refer to `docs/analysis.md`.

## D-01 Command groups
Question: keep `project`, `bm`, `ss` names or rename?
Proposed: keep `project`, `bm`, `ss`. Messages already quote them ("ledge project pick",
"ledge bm add", "bm tree", "ss tree"), so renaming would mean rewriting those messages too.
Keeping the names does not settle the full command list. That is still missing (F-01, blocker):
every sub-command and option must be listed, including where `history`, `kill`, `doctor`
and `help` sit.
Status: Accepted (user, gate A, 2026-10-24)

## D-02 Session naming compatibility
Question: byte-for-byte compatible with earlier sessions, or migrate?
Proposed: byte-for-byte compatible (matches FR-NAME-02). No migration code. For this to be testable,
the user supplies: the exact digest input (the cleaned absolute path, with or without a trailing newline),
the hex case, how characters other than "." that tmux rejects are handled (e.g. ":"), the tmux option names
and values of the managed marker and the profile tag, and at least two reference pairs
(directory -> session name) from the earlier tool. These become fixed unit-test vectors. The 16-bit
suffix collision risk (F-33) is accepted as known.
Rationale: users keep long-lived sessions. Migrating would mean renaming live sessions, which risks
breaking clients attached to them and gains nothing.
Status: Accepted (user, gate A, 2026-10-24)

## D-03 Default search path
Question: keep "~/projects" or ask on first run?
Proposed: keep (matches FR-CONF-01). Write "~/projects" literally into the created config. Do not
create the `~/projects` directory. If it does not exist, the project picker shows no projects from
it and `doctor` warns (FR-DOC-03).
Rationale: a first-run prompt would block non-interactive use and tests, and duplicates the job of
editing `config.json`.
Status: Accepted (user, gate A, 2026-10-24)

## D-04 Missing recorded profile in the session switcher rebuild
Question: warn or fail?
Proposed: warn and use the default profile. FR-PROF-15 already says this, so this decision only
confirms it. It applies to every rebuild path (ctrl-r and alt-r in every picker), not only the
switcher. If the default profile is also missing or invalid, print the FR-PROF-12 warning as well
and use the single-window layout ("basic"). If no default_profile is set, use "basic" directly
after the FR-PROF-15 warning. Each warning is printed once per rebuild action.
Status: Accepted (user, gate A, 2026-10-24)

## D-05 Atomic writes for history, bookmarks, bm-edit-open
Question: write atomically?
Proposed: yes, for every file ledge writes (history, bookmarks, bm-edit-open). Write the complete
new contents to a temporary file in the same directory, then rename it over the target. A crash
leaves either the old file or the new one, never a truncated file. When two ledge processes write
at once, the last writer wins; no locking. `config.json` is written only when it is created and is
never rewritten.
Status: Accepted (user, gate A, 2026-10-24)

## D-06 Bookmark paths differing by trailing slash
Question: behaviour for the same path with different trailing slashes?
Proposed: one bookmark. Every path that ledge stores or compares (bookmarks, history, bm-edit-open,
the session-name input of FR-NAME-01, and "inside a project" checks) is first made absolute against
the current directory and then cleaned lexically: trailing slashes removed, "." and ".." resolved,
repeated "/" collapsed. Symlinks are not resolved, so a bookmark to a directory that no longer exists
can still be listed and removed. Entries read from a hand-edited file are cleaned and deduplicated
in memory, keeping the first occurrence. "Inside" is compared by whole path components (`/p/app` is
not inside `/p/ap`).
Status: Accepted (user, gate A, 2026-10-24)

## D-07 Session switcher scope
Question: show sessions outside any search path?
Proposed: the question mixes two things (F-38). Managed or not: only managed sessions, never
unmarked ones (FR-SESS-01, FR-NAME-04). Location: show every managed session wherever its directory is,
including directories outside every search path (e.g. opened with `ledge /tmp/x`). In the switcher
list and `ss list`, these rows show an empty project column. In the session tree they go under one
extra root labelled with their own path (this needs confirming with F-20).
Status: Accepted (user, gate A, 2026-10-24)

## D-08 Wording "No ledge sessions."
Question: acceptable given the product name?
Proposed: keep "No ledge sessions." It matches the product name and the sibling message
"'<name>' is not a ledge session.". When accepted, remove the "(Wording to be confirmed ...)" note
from Messages.
Status: Accepted (user, gate A, 2026-10-24)

## D-09 Bookmark edit window expanded state
Question: per project or global?
Proposed: global, in one `bm-edit-open` file. Paths are absolute, so entries from different projects
cannot collide, and opening project A does not affect B. When the file is written, entries whose
directory no longer exists are dropped. Collapsing a folder removes only that folder's entry; the
expanded state of its descendants is kept for the next time it opens.
Status: Accepted (user, gate A, 2026-10-24)

## D-10 Fuzzy search rules
Question: match the previous external picker's ranking/highlighting or use own rules?
Proposed: ledge's own rules. Case-insensitive subsequence match, with a score that rewards
consecutive matches and matches at word boundaries ("/", "-", "_", ".", space, camelCase).
Ties keep the original row order, and matched characters are highlighted (underlined or bold when
there is no color, see F-11). An empty query shows all rows in their original order with the
preselection kept (FR-PICK-04). When the query changes, the selection moves to the best match. The
search column is set per picker (F-09). In trees, a matching node keeps its ancestors visible so the
tree stays readable. This is a pure function with full unit tests.
Rationale: matching an external tool's undocumented ranking cannot be specified or tested exactly.
Status: Accepted (user, gate A, 2026-10-24)

---

# Gate A resolutions (user: "accept defaults", 2026-10-24)

The user accepted the analyst's recommended resolutions for every finding in `docs/analysis.md`.
Where a finding gives a "Suggested"/"Resolution", that is now binding. The blockers are made concrete
below. These entries override vaguer wording in `requirements.md`. Anything still unresolved is
decided by the architect and recorded in `docs/architecture.md` §13 "Assumptions".

## D-11 Command line (F-01)
Status: Accepted
```
ledge [DIR] [OPEN-OPTS]                 open DIR or cwd (implicit `project open`; options may follow)
ledge project open [DIR] [OPEN-OPTS]
ledge project pick [OPEN-OPTS]          project picker
ledge history list | pick [OPEN-OPTS] | clean
ledge bm add [DIR] | remove DIR | list | pick [OPEN-OPTS]
ledge bm edit [DIR] [--session-scope]
ledge bm tree [DIR] [--session-scope] [OPEN-OPTS]
ledge ss [OPEN-OPTS]                    session switcher (bare `ss` IS the switcher)
ledge ss list | tree [OPEN-OPTS] | kill NAME|DIR
ledge doctor
ledge help | -h | --help
OPEN-OPTS: --profile NAME | --no-profile ; --code | --idea | --editor CMD
```
- First word is a command if it matches a command word; otherwise it is DIR for the implicit open.
  `./bm` forces the directory reading.
- `project`, `history`, `bm` without a sub-command print group usage, exit 0 (FR-CMD-02).
- FR-CMD-11: for explicit commands, flags only after the command word(s); for the bare form, flags may
  appear anywhere after `ledge`.
- `--profile` with `--no-profile`: error "--profile cannot be combined with --no-profile.".
- Only one editor option; a second one is an error "only one of --code, --idea, --editor may be given.".
- An option not accepted by a command: FR-CMD-04 message.
- Help and usage text are written by the architect/implementer; tests assert they list every command.

## D-12 Picker open/close behaviour (F-02)
Status: Accepted
| Picker | Empty at start | An action empties it |
|---|---|---|
| project | "No projects found in search paths." (info, exit 0) | close, exit 0, no output |
| history | file missing: "No history cache found."; empty file: "No history." (info, exit 0) | close, exit 0 |
| bookmark picker | "No bookmarks. Use 'ledge bm add' to add one." (info, exit 0) | close, exit 0 |
| session switcher | "No ledge sessions." (info, exit 0) | STAYS OPEN (empty list) |
| session tree | "No ledge sessions." (info, exit 0) | STAYS OPEN |
| bookmark tree | "No bookmarks." (info, exit 0) | STAYS OPEN (FR-TREE-12) |
| bm edit | FR-EDIT-11: exit 0, no output | n/a |
| profile picker | always has "basic" | n/a |
"Reopen the list" and "stays open" are the same: the list is refreshed, the filter text is kept,
the selection stays on the same row index clamped to the list. "Cancelled." is printed only on
escape / ctrl-c / enter with no matching row.

## D-13 Profile format (F-03)
Status: Accepted
- File: `<config>/profiles/<name>.json`. Name must match `[A-Za-z0-9_-]+`; otherwise the
  "not found" error.
- `{"windows": [{"name": "editor", "command": "nvim"}, {"name": "shell"}]}`
- `windows`: non-empty array of objects. `name`: required non-empty string. `command`: optional
  string, sent to the window as typed keys followed by Enter. Unknown fields are ignored. Duplicate
  names are allowed.
- Every window starts in the session directory.
- Missing file: "profile '<name>' not found at <path>". JSON syntax error:
  "profile '<name>' is invalid JSON: <detail>". Any shape problem (missing/empty/non-array `windows`,
  bad window entry): "profile '<name>' is invalid (expected a 'windows' array)".
- "basic" is built in and reserved; `profiles/basic.json` is ignored (doctor warns).
- Precedence (F-18): `--no-profile` > `--profile` > recorded profile (rebuild only) > default_profile
  > basic. alt-r uses each session's recorded profile even when `--profile` was given (FR-ACT-08);
  alt-p asks. ctrl-r uses `--profile` if given, else the recorded profile.

## D-14 Session name and tags (F-04)
Status: Accepted (assumption; the user may correct it later from the old tool)
- name = basename(clean_abs_path) with every "." and ":" replaced by "_", then "-", then the first 4
  characters of the lowercase hex MD5 of the clean absolute path bytes (UTF-8, NO trailing newline).
- Example: `/home/u/projects/my.app` -> `my_app-` + md5("/home/u/projects/my.app")[0..4].
- tmux session options: `@ledge_managed` = `1` (managed marker; a user adopts a session by setting
  it), `@ledge_profile` = profile name.
- Collisions (F-33): accepted. If a managed session with the name exists but its recorded directory
  (`@ledge_dir` option, also set at creation) differs, open fails with
  "could not create session '<name>': name already used by <dir>".
- Unmanaged session with the computed name (F-17): open attaches to it without marking it;
  rebuild/kill refuse with "'<name>' is not a ledge session.".

## D-15 Bookmark edit window keys (F-05)
Status: Accepted
- right or ctrl-l: expand. left or ctrl-h: collapse if expanded, otherwise move to parent.
  enter: toggle bookmark. escape: close. up/down/page keys move.
- Footer: "enter toggle bookmark · →/^l open · ←/^h close · esc done".
- The project root is shown as the top row; enter on it does nothing (the project itself cannot be
  bookmarked, FR-BM-04).
- No type-to-filter in the edit window (exception to FR-TUI-02).

## D-16 Message classes (F-06, F-07, F-08)
Status: Accepted
- error ("Error: " prefix, stderr, exit 1): every "fail"/"reject" message, argument errors,
  "'<name>' is not a ledge session.", "'<path>' is not a bookmark.", bookmark refusals (FR-BM-03/04).
- info (stdout, exit 0, no prefix): "No history.", "No history cache found.", "No bookmarks...",
  "Bookmarked", "Already bookmarked", "Removed bookmark", "Killed session", "No ledge sessions.",
  "Project history cache clear successfully.", "No projects found in search paths.".
- warning ("Warning: " prefix, stderr, does not change the exit code).
- tmux is checked only for commands that need it; missing tmux uses FR-CMD-10's message. Doctor
  reports tmux as ERR instead of failing at start. Other missing programs use FR-CMD-09's message.
- `--session-scope` outside a tmux session: error "--session-scope requires running inside a session.".
- A directory argument on the project picker (FR-PICK-11): "project pick takes no directory argument.".

## D-17 Current-session safety (F-15, F-16)
Status: Accepted
- ctrl-x / ctrl-d on the row of the session the user is in: no kill; the picker shows a status line
  "cannot kill the session you are in." (the bookmark is still deleted for ctrl-d).
- Rebuilding the current session (FR-ACT-05): build `<name>-rebuild` with the new profile, switch the
  client to it, kill the old session, rename the new one to `<name>`. On build failure the original
  is untouched and the error is shown.

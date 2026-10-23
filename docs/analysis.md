# Requirements analysis

STATUS: DRAFT

Audit of `requirements.md` by the requirements-analyst (Phase 0, step 2). `requirements.md` was not
edited. Each finding has FR-IDs, a severity (blocker / should-fix / note), the question, and a
recommended resolution. Blockers stop the architect or test-engineer from working on part of the
product until they are answered. Proposed answers to "Decisions still open" are in `docs/decisions.md` (D-01..D-10).

Counts: 5 blocker, 25 should-fix, 8 note.

## Questions for the user

Blockers:

1. (F-01) What is the full command line? List every command word, sub-command and option, e.g.
   `ledge [DIR]`, `ledge project open|pick`, `ledge history list|pick|clean`, `ledge bm add|remove|list|pick|edit|tree`,
   `ledge ss [list|tree|kill]`, `ledge doctor`, `ledge help`. Give the spelling of the profile, no-profile,
   session-scope, code, idea and editor options. Say how `ledge --profile X` (open the current directory
   with a profile) fits with FR-CMD-11 "flags only after the command word".
2. (F-02) Which pickers stay open when they have no rows left, and which close? When a picker closes
   because it is empty, what does ledge print and with what exit code? What happens when a picker
   starts with zero rows?
3. (F-03) What is the profile file format: file name and extension, the fields of a window (name,
   command, others?), the working directory of each window, and whether an empty `windows` list is
   valid? Is "basic" a reserved name?
4. (F-04) For compatibility with earlier sessions: give the exact MD5 input (the cleaned absolute path?
   With a trailing newline, as `echo "$path" | md5sum` would produce?), the letter case of the hex,
   and the tmux option names and values of the managed marker and the profile tag. One or two
   reference pairs (directory -> session name) from the earlier tool would settle it.
5. (F-05) Which keys does the bookmark edit window use? The key table says right and left arrows. The
   footer says "^l open · ^h close". The left arrow does "Collapse the folder, or move to the parent",
   but FR-EDIT-03 only says it collapses.

Should-fix (the answers most likely to change design):

6. (F-06) What exit code and output stream (stdout or stderr) apply to "print and stop" outcomes
   such as "No bookmarks. Use 'ledge bm add' to add one.", "No history cache found." and
   "ss takes no directory argument."?
7. (F-08) Should `help`, `doctor` and the file-only commands (history, bm add/list/remove) run when
   tmux is missing? Which message is used for a missing tmux: FR-CMD-09 or FR-CMD-10?
8. (F-09) Which column does fuzzy search match? The first visible column of the project picker is
   the marker, and in the trees it is the tree drawing.
9. (F-15, F-16) What happens on ctrl-x / ctrl-d on the session the user is in? What exactly should
   happen when the user rebuilds the session they are in (FR-ACT-05)?
10. (F-17) If a session with the computed name exists but is not managed, should open attach to it,
    fail, or recreate it?
11. (F-18) Does `--profile` given on the command line override the recorded profile for alt-r (FR-ACT-08)?
    Can `--profile` and the no-profile option be combined?
12. (F-20) What does the session tree contain: every project under the search paths, or only the
    ancestors of sessions? Can a gray entry in the session tree be opened? The screen table says it cannot.
13. (F-13) What are the boundaries of the age text: "just now" under 60 s, then whole minutes rounded
    down, "1h ago" from 60 min, "1d ago" from 24 h?
14. (F-26) Which commands accept the editor options? Does a rebuild count as "a session is created"
    for FR-LAUNCH-01? If the editor or setsid is missing, should ledge fail or warn?
15. (F-30) Should the Installation section get FR-IDs, and is the uninstall option part of the
    install script or of the ledge binary?

---

## Blockers

### F-01 The command-line surface is not defined
FR-IDs: FR-OPEN-01, FR-OPEN-04, FR-PICK-11, FR-PICK-12, FR-HIST-03..07, FR-BM-07..14, FR-EDIT-09,
FR-TREE-06, FR-SESS-05, FR-SESS-09, FR-SESS-10, FR-PROF-09, FR-PROF-10, FR-DOC-*, FR-CMD-01..06,
FR-CMD-11, FR-LAUNCH-02. Severity: blocker.

The requirements refer to commands by role ("the clean command", "the history picker", "the
switcher", "the session-scope option", "the no-profile option", "the code option", "the help flags")
and never name them. The only names are hints in messages: "project open", "ledge project pick",
"ledge bm add", "bm pick", "bm tree", "ss", "ss tree", "kill", "bookmark remove", "--profile",
"--editor", "--session-scope". Without names there can be no tests for FR-CMD-02/03/04 (usage text,
unknown command, unknown option).

Contradictions:
- FR-OPEN-01 "open the session for the current directory when run with no command word" and
  FR-CMD-11 "accept flags only after the command word". Together these rule out
  `ledge --profile dev` and `ledge --code`. Is that intended?
- FR-OPEN-02 "the directory given as its single argument". Is `ledge ~/x` valid, or only
  `ledge project open ~/x` (the wording of FR-OPEN-04's message)? If a word is both a command and a
  directory name (e.g. `./bm`), which reading wins?
- FR-CMD-02 "print the group usage text for a group command given without a sub-command". Does
  `ledge project` print usage, or open the current directory? Does `ledge ss` print usage, or open the
  switcher (FR-SESS-05's message "ss takes no directory argument." suggests `ss` is itself the switcher)?
- The usage and help texts themselves are not given, so FR-CMD-01/02 can only be tested as "prints
  something, exit 0".
- No message is given for flags before the command word (FR-CMD-11), for an option on a command
  that does not accept it, or for repeated options.

Resolution: add a command table to `requirements.md` with each command, its arguments and options,
its FR-IDs, and its exit codes. State the rule for the bare `ledge [DIR]` form.

### F-02 When pickers stay open or close is circular and contradictory
FR-IDs: FR-TUI-10, FR-CMD-12, FR-TREE-12, FR-BM-12, FR-HIST-05, FR-PICK-09, FR-EDIT-11, the screen
tables. Severity: blocker.

- FR-TUI-10: "keep a picker open after an action that leaves it with rows, and close it when it has none, unless FR-CMD-12 applies".
- FR-CMD-12: "exit a picker without a window when it has no rows to show, unless the picker is meant to stay open".
  Each one defers to the other, and "meant to stay open" is never defined.
- Session switcher screen: "The list stays open after the list is empty." FR-TREE-12: "keep the tree
  window open when its last entry is removed." These are the only stated exceptions. Does the session
  tree also stay open?
- Project picker ctrl-x: "Kill the session and reopen the list". Bookmark picker ctrl-d: "reopen the
  list". Session switcher ctrl-x: "the list stays open". Are "reopen" and "stays open" different
  behaviours that a user can see (filter text kept? selection kept?)?
- At start with zero rows: FR-BM-12 prints a message, and FR-HIST-05 prints a message when the file
  is missing. But what happens when the history file exists and is empty, when the project picker
  finds no projects, or when the switcher starts with no managed sessions?
- When a picker closes because an action emptied it: is "Cancelled." printed? What is the exit code?

Resolution: one table, one row per picker, with three columns: behaviour on empty at start (message,
exit code), behaviour when an action empties it (stay / close plus output), and what "reopen" keeps
(filter text, selection).

### F-03 Profile file format is not specified
FR-IDs: FR-PROF-01..07, FR-PROF-11, FR-DOC-05, Data table. Severity: blocker.

- FR-PROF-01 "a file named by the profile": is it `profiles/<name>.json` or `profiles/<name>`? The
  message "profile '<name>' not found at <path>" requires a defined path.
- FR-PROF-02 / Messages "expected a 'windows' array": the fields of each window are not given.
  FR-PROF-04 implies an optional `command`. Does a window have a `name`? What if the name is missing
  or duplicated? Are unknown fields allowed?
- The working directory of each window is not stated. (Presumably the project directory.)
- Is `"windows": []` valid? FR-PROF-05 needs a first window.
- Is the profile name validated (e.g. `../x`, an empty string)? Is "basic" reserved (FR-PROF-07, FR-PROF-13)?
  If `profiles/basic.json` exists, which one wins?
- What does "invalid" mean when the window entries have the wrong type (e.g. `"windows": "x"` or
  `[1,2]`)? Which of the three profile messages applies?

Resolution: give the JSON schema with one valid example and one example per error message.

### F-04 The session name and marker contract with earlier sessions is not specified exactly
FR-IDs: FR-NAME-01, FR-NAME-02, FR-NAME-03, FR-NAME-05, FR-OPEN-05, D-02. Severity: blocker.

FR-NAME-02 promises that "existing sessions are recognised". That can only be tested against
reference values, and FR-NAME-01 leaves several choices open:
- MD5 of which bytes? The cleaned absolute path, the path with symlinks resolved, or the path plus a
  trailing newline? Shell-based tools often hash `echo "$PWD"`, which includes the newline.
- Lower- or upper-case hex?
- Only dots are replaced. tmux also rejects `:` in session names, and spaces or other characters may
  need handling. What happens to them?
- FR-NAME-03 / FR-NAME-05: the managed marker and the profile tag are user-visible (the user sets the
  marker by hand to adopt a session), yet their tmux option names and values are not given. If
  earlier sessions must be recognised, these names must also match the earlier tool's.

Resolution: give the exact algorithm plus at least two reference pairs (one with a dot in the
basename), and the marker and profile option names and values. Record them in D-02.

### F-05 Bookmark edit window keys contradict each other
FR-IDs: FR-EDIT-03, FR-EDIT-07, the edit-window screen. Severity: blocker for the edit-window slice.

- Key table: right arrow expands and left arrow collapses. Footer: "enter toggle bookmark · ^l open · ^h close · esc done".
  ctrl-l and ctrl-h have no rows in the table. "open" might mean expand or open a session.
- Key table, left arrow: "Collapse the folder, or move to the parent". FR-EDIT-03 says only "collapse
  it with the left-arrow key".
- Is the project root a row? FR-EDIT-02 "toggle the bookmark of the selected directory", but
  FR-BM-04 refuses a project itself. What does enter do on the root row?
- Does typing filter the tree (FR-TUI-02 applies to "every picker")?

Resolution: one key table that the footer copies exactly. Say whether ctrl-l/ctrl-h are aliases for
right/left. Define enter on the root row.

## Should-fix

### F-06 Exit code and stream for "print and stop" outcomes
FR-IDs: FR-HIST-03, FR-HIST-05, FR-HIST-07, FR-BM-06, FR-BM-08, FR-BM-12, FR-SESS-05, FR-SESS-09,
FR-SESS-11, FR-EDIT-11, FR-PICK-11.

The Messages section gives a stream and exit code only for "Error:", "Warning:" and "Cancelled.".
Other wording differs: FR-SESS-05 "print ... and stop", FR-SESS-11 "print", FR-BM-14 "reject",
FR-OPEN-03 "fail". Is "'x' is not a ledge session." an error ("Error: " prefix, stderr, exit 1)? Is
"No bookmarks. Use 'ledge bm add' to add one." stdout with exit 0? Is "Already bookmarked: <path>"
exit 0 or 1? Does FR-EDIT-11 "exit without a window" print anything?
Resolution: in Messages, mark each line as error, warning or info, which sets its stream, prefix and exit code.

### F-07 Messages with no triggering FR, and FRs with no message
- Messages with no FR: "ss tree takes no directory argument.", "bookmark remove requires a directory.",
  "bm add takes at most one directory.", "could not create session '<name>': <detail>",
  "could not attach to '<name>': <detail>".
- FRs with no message: FR-PICK-11 (directory argument on the project picker), FR-EDIT-09 and
  FR-TREE-06 when the session-scope option is used outside a session, FR-EDIT-11, FR-CMD-11, FR-SESS-10
  when both a name and a directory are given.
- FR-CMD-10 "Required tool 'tmux' is not installed." is not in the Messages section.
- FR-PROF-11 "fail with the profile's error" and FR-PROF-12 "warn" should name their messages.
Resolution: add the missing FRs and messages so that the mapping is one-to-one.

### F-08 tmux-missing check conflicts with doctor and with commands that do not need tmux
FR-IDs: FR-CMD-09, FR-CMD-10, FR-DOC-01, FR-DOC-08.
FR-CMD-10 "fail ... when tmux is missing at start". If that runs first, `ledge doctor` can never
print "ERR tmux", and `help`, `bm add` and `history list` fail for no reason. FR-CMD-09 gives a
second, generic message for the same case.
Resolution: check tmux only for commands that use it. Doctor reports it as ERR. Pick one message.

### F-09 The fuzzy-search column is the marker or the tree drawing
FR-IDs: FR-TUI-02, Shared rules, the project picker columns, the trees.
"matching fuzzily over the first visible column unless noted". The first project picker column is
"marker", and tree rows start with tree glyphs. "unless noted" is never noted anywhere.
Resolution: name the search column per picker (e.g. project basename; session name; bookmark path;
the node name without glyphs).

### F-10 Picker chrome is incomplete or self-contradictory
FR-IDs: FR-TUI-04, FR-TUI-07, the screens.
- History picker: "Same keys as the project picker. Footer: 'enter open'". FR-TUI-04 requires the
  footer to list its keys. Which is right?
- Border labels are missing for the history picker, session switcher and bookmark picker. The icon is
  not given for any picker. The bookmark picker has no columns. The header row text is not given.
- The session switcher table has no escape row (the shared rules probably cover it).
Resolution: give label, icon, header, columns and footer literally for every picker.

### F-11 Dimming without color
FR-IDs: FR-TUI-08, FR-TUI-09, FR-BM-11, FR-TREE-02.
"readable" cannot be tested. Dimmed rows and gray connectors give no visible cue on a terminal
without color. How does ledge detect that color is unavailable (`NO_COLOR`, `TERM=dumb`, not a tty)?
Resolution: name a text cue per state (e.g. a "(missing)" suffix, a different connector glyph) and
the detection rule.

### F-12 "fit" and truncation are not defined
FR-IDs: FR-TUI-05, Quality ("column alignment and truncation").
Which columns shrink first? Is there an ellipsis character? Is there a minimum terminal size, and
what is shown below it? Are wide (CJK) characters counted by display width?
Resolution: give the column priority, the truncation marker and the minimum size.

### F-13 Age text boundaries and activity order
FR-IDs: FR-SESS-02, FR-SESS-03.
Is 59 s "just now"? Is 90 s "1m ago"? Is 60 min "1h ago" or "60m ago"? Is 24 h "1d ago"? Is there an
upper limit (weeks)? How are ties in activity ordered?
Resolution: suggested rule: <60 s "just now"; <60 min "<n>m ago"; <24 h "<n>h ago"; otherwise
"<n>d ago", all rounded down. Ties are broken by session name.

### F-14 Cancel semantics are incomplete
FR-IDs: FR-CMD-08, Shared rules, FR-ACT-04, FR-ACT-09.
- "escape or an empty choice cancels": what is an "empty choice"? Enter while the filter matches no
  rows?
- Escape in the profile picker opened by ctrl-p: does it return to the parent picker, or exit with "Cancelled."?
- ctrl-c: same as escape (exit 0) or an interrupt (FR-TUI-06; exit code)?
Resolution: state each case.

### F-15 Scope of kill and delete keys; killing the session the user is in
FR-IDs: FR-ACT-01, FR-ACT-07, FR-ACT-11, FR-BM-13, the screens.
- FR-ACT-01 "on the kill key, in every picker". The profile picker and the edit window have no kill
  key. Limit this to the listed pickers.
- FR-ACT-07 protects the current session only from alt-keys. What happens on ctrl-x or ctrl-d on the
  session the user is in? Killing it also kills the pane that runs ledge.
- Project picker ctrl-x has the condition "Session is managed", but the bookmark picker uses "Always".
  What happens on a row with no session?
Resolution: either refuse to kill the current session (with a message) or define the outcome. State
that the key does nothing on rows without a managed session.

### F-16 Rebuilding the session the user is in
FR-IDs: FR-ACT-05, FR-ACT-03.
"build a copy of the session ..., switch to the copy, and give it the original name". The temporary
name, when the old session is killed, and what happens if the build fails part-way are not stated.
Resolution: give the observable result. Afterwards exactly one managed session has the original
name, the client is on it, and it has the new profile. On failure the original session is untouched
and an error is printed.

### F-17 A session with the computed name exists but is not managed
FR-IDs: FR-OPEN-07, FR-NAME-04, FR-ACT-02, FR-ACT-13.
FR-OPEN-07 creates a session "only when no session of that name exists". FR-NAME-04 says only
marked sessions count "in every ... check". If the user's own session has the name `proj-1a2b`,
does open attach to it, fail, or recreate it? Can ctrl-r replace it?
Resolution: suggested: open attaches without marking it, and rebuild/kill refuse with
"'<name>' is not a ledge session.". The user decides.

### F-18 Profile precedence conflicts
FR-IDs: FR-ACT-08, FR-ACT-12, FR-PROF-09, FR-PROF-10, FR-PROF-12, FR-PROF-14, FR-PROF-15, FR-PICK-13.
- FR-ACT-12 says the profile option takes effect in rebuild actions. FR-ACT-08 says alt-r uses "the
  profile recorded on that session". Which wins when `--profile` was given?
- FR-PROF-15 falls back to "the default profile". If that is missing or invalid too, does FR-PROF-12
  apply (warn and use basic)? Is the warning printed twice? If no default_profile is set, does
  "the default profile" mean basic?
- Can the profile option and the no-profile option be combined?
- FR-PICK-13 "when a non-basic profile applies": does default_profile count, or only `--profile`?
Resolution: one precedence list: no-profile > `--profile` > recorded (rebuild only) > default_profile > basic,
with a stated exception for alt-r.

### F-19 Profile picker contents
FR-IDs: FR-PROF-13, FR-ACT-04, FR-ACT-09.
Is the current profile listed again among "every other profile"? Is "[Default: <name>]" left out
when no default is set? Where is "basic" placed when it is the current or default profile? Which row
is preselected? How are the profiles in the rest of the list ordered? For alt-p (several sessions)
there is no single "Current".
Resolution: give the ordering, dedupe and preselection rules.

### F-20 Session tree content; gray entries; trees outside projects
FR-IDs: FR-SESS-06, FR-SESS-07, FR-SESS-08, FR-TREE-01..12, the tree screen.
- FR-SESS-06 "gray entries for project folders that have no session": every project under every
  search path, or only folders between a root and a session?
- FR-SESS-07/08 suggest a gray folder can be opened (it shows "the session name it would get").
  The shared tree table says enter works only when the "Entry is not a gray connector". Is a gray
  folder in the session tree a "connector"?
- Where does a managed session whose directory is outside every project appear in the session tree?
  (FR-TREE-04 drops such bookmarks; nothing is said for sessions.)
- FR-TREE-07 "show only the root directory when the given directory is outside every tree": which
  root? The project containing the directory? What if the directory is outside every project as well?
- FR-TREE-02 vs FR-TREE-03: is a gray "shared parent" different from a gray "connector"? Is the root
  gray when it is not bookmarked?
Resolution: define the node set per tree and which node types accept enter, ctrl-*, alt-*.

### F-21 "inside" is used both inclusively and exclusively
FR-IDs: FR-BM-02, FR-EDIT-08, FR-EDIT-10, FR-SCOPE-01, FR-SCOPE-02, FR-CONF-07.
FR-BM-02 excludes the project itself ("refuse a project itself"). FR-SCOPE-02 includes it ("is the
group entry or inside it"). FR-EDIT-08 is unclear: does `bm edit <project>` work? The comparison is
also unclear: `/p/app` must not count as inside `/p/ap`.
Resolution: define "inside" once (the path equals the base, or descends from it by whole components)
and say where the project itself is excluded.

### F-22 Form of path placeholders in messages
FR-IDs: FR-OPEN-03, FR-PICK-09, FR-BM-03..10, FR-EDIT-10, Messages.
Is "<dir>" / "<path>" shown as typed, as the cleaned absolute path, or with "~"? Example:
`ledge bm add .` then "Bookmarked ." or "Bookmarked /home/u/p/app/src"? FR-HIST-03 uses "~" only for
the history list.
Resolution: suggested: the cleaned absolute path everywhere except FR-OPEN-03, which echoes what
the user typed.

### F-23 Project picker current-directory row and "Submodules"
FR-IDs: FR-PICK-04, FR-PICK-05, FR-PICK-07, FR-PICK-13.
- "mark the current directory with '*' and show its path when it is not a project". Is "*" always
  shown, and the path only for non-projects? Does "*" replace or sit beside the filled or empty
  marker? If the current directory is a project, is it shown twice (first, and in sort order)?
- What if the current directory is inside a project, or is the home directory?
- "Projects & Submodules": submodules are never defined. Are git submodules listed?
Resolution: give a worked example of the picker rows for each case. Remove or define "Submodules".

### F-24 Search path and configuration edge cases
FR-IDs: FR-PICK-01, FR-CONF-01..07.
A search path that does not exist (skip silently?), duplicate or overlapping search paths
(deduplicate projects?), symlinked directories (count as projects?), an undefined environment
variable in a path (empty, kept literally, or error?), the variable syntax (`$VAR`, `${VAR}`), and
valid JSON with wrong types (`"search_paths": "~/p"`). Which message applies? Are unknown keys
allowed? If config creation fails (read-only home), what happens? Does `help` create the config?
Resolution: state each case. Suggested: skip missing paths silently in pickers (doctor warns),
deduplicate by cleaned path, follow symlinks, treat undefined variables as an error, and report
wrong types as an invalid-config error with its own message.

### F-25 Meaning of "configuration home" and "cache home" environment settings
FR-IDs: FR-PLAT-02, FR-CONF-08, Configuration.
Does the variable name ledge's folder itself (e.g. `LEDGE_CONFIG_DIR=/x`, so config is at `/x/config.json`)
or the XDG base (`XDG_CONFIG_HOME=/x`, so config is at `/x/ledge/config.json`)? Is an empty or
relative value ignored, as the XDG spec requires?
Resolution: name the variables and the resulting paths.

### F-26 Editor launch is under-specified
FR-IDs: FR-LAUNCH-01..03, FR-CMD-06, FR-CMD-09, External dependencies.
- The section heading says "(FR-EDIT-LAUNCH)" but the IDs are "FR-LAUNCH-xx". "FR-EDIT" is already
  used by the edit window. Fix the heading.
- Which commands accept code/idea/editor (open only, or pickers too)? Can several be given?
- Does a rebuild (ctrl-r) count as "a session is created"?
- How is the `--editor` value split (shell words, or a single program)? Is the directory appended as
  the last argument?
- If the editor program is missing: fail (FR-CMD-09, exit 1, after the session was already
  created?) or warn? If setsid is missing (it is "optional"): fall back to some other detaching
  method, or warn?
Resolution: state each case. Suggested: a missing editor gives a warning and the session still opens.

### F-27 Doctor check text and severity of optional programs
FR-IDs: FR-DOC-01..08, Messages ("with the check text in the Diagnostics requirements").
The Diagnostics requirements contain no check text, so no line can be asserted exactly. Missing
optional programs (setsid, code, idea): WARN or ERR? Is the tmux version (3.2 or later) checked?
Does doctor create a missing config (FR-CONF-01) or report ERR? Is the order of checks fixed?
Resolution: list each check line verbatim with its OK/WARN/ERR condition.

### F-28 Bookmark picker and bm add details
FR-IDs: FR-BM-02, FR-BM-07, FR-BM-11, FR-BM-13, bookmark picker screen.
- "enter | Open the project": does this open the bookmarked directory's own session (FR-OPEN-05
  naming) or the containing project's session?
- `bm add` on a path that does not exist: refuse? With what message?
- "its session" (FR-BM-13, FR-ACT-10): the session named from the bookmark directory, and only if it
  is managed?
Resolution: state each case.

### F-29 Kill target: name or directory
FR-IDs: FR-SESS-10, FR-SESS-11, FR-SESS-13.
"kill a managed session given by exact name, or the session of a given directory". If an argument
is both an existing relative directory and a session name, which reading wins? Is a directory whose
session is not running reported with FR-SESS-11's message?
Resolution: suggested: try the exact session name first, then the directory. Use FR-SESS-11 for
both misses.

### F-30 Installation section has no FR-IDs and its messages are not in Messages
FR-IDs: none (Installation and removal).
The statements cannot be traced. "Removed <path>", "Not removed: ...", "Nothing to remove: ..." and
"Error: build toolchain is required to build ledge." are not in Messages. Is "the uninstall option"
an option of the install script or of the ledge binary? If it belongs to the script, is the script in
scope for the Rust test suite?
Resolution: add FR-INST-xx IDs, say which program owns uninstall, and add the messages.

## Notes

### F-31 FRs that need a tmux or terminal seam
These cannot be tested without tmux or a real terminal, so the architect needs a backend trait or
seam for them: FR-OPEN-07/08/09, FR-SESS-01..04, FR-SESS-09..13, FR-ACT-01..10, FR-ACT-13,
FR-PROF-03..07, FR-NAME-03..05, FR-DOC-01, FR-DOC-07, FR-LAUNCH-01..03 (process spawn), FR-TUI-05 and
FR-TUI-06 (terminal size, raw mode, signals), and FR-PICK-07 (live-session marker). Inside or outside a
session (FR-OPEN-08, FR-SESS-04) also needs a seam over the environment.

### F-32 Overlapping requirements
FR-PLAT-02 / FR-CONF-08; FR-PICK-01 / FR-CONF-07; FR-TUI-04 / Shared rules; FR-OPEN-08 / FR-SESS-04;
FR-NAME-04 / FR-SESS-01 / FR-ACT-13; FR-TREE-02 / FR-TREE-03; FR-BM-13 / FR-ACT-10 / FR-ACT-11. These
are harmless if they agree, but each should cite the other so that a later edit does not leave them
out of step.

### F-33 A four-hex-digit suffix cannot guarantee FR-OPEN-06
FR-OPEN-06 "give different sessions to two directories that share a basename". FR-NAME-01 uses 16
bits of MD5, so a collision is possible (about 1 in 65,536 per pair). Accept this as known, or define
what happens on a collision (e.g. ledge detects that the managed session's directory differs and
refuses). This is tied to D-02.

### F-34 Wording "Project history cache clear successfully."
FR-HIST-06 says "(exact wording in Messages)", so the ungrammatical text looks deliberate
(compatibility with the earlier tool?). Please confirm. It will be asserted byte for byte.

### F-35 Quality section puts picker keys in manual-only tests
"Verified ... only in manual or integration runs: picker keys and layouts". Picker key handling and
layout can be tested headless against a fake terminal and backend. Recommend moving them into the
automated set, leaving only real-tmux effects as integration tests.

### F-36 History edge cases
FR-OPEN-10 records the directory before the open, so a failed open still enters history. Is that
intended? FR-HIST-04 on a history entry whose directory is missing: does FR-PICK-09's
"Missing directory" apply? Are missing directories dimmed (FR-TUI-08)? Does the 50-entry limit
(FR-HIST-01) apply when reading a longer file that the user edited?

### F-37 Navigation details
FR-TUI-03: home/end, wrap-around at the ends, mouse? After the filter changes, does the selection
reset to the first match or stay on the same row if it still matches? Is the preselection of
FR-PICK-04 kept?

### F-38 Two open decisions are already answered or conflated
"warn or fail when a profile's recorded session profile is missing during the session switcher's
rebuild" is already answered by FR-PROF-15 (warn). "show sessions outside any search path, or only
managed ones" mixes two questions (managed or not; inside a search path or not). See D-04 and D-07.

# ledge requirements

## Purpose and users

ledge opens a project directory as a named session, builds a window layout for it from a
profile, and lets the user pick, rebuild, switch to and kill those sessions from its own
terminal interface. Users are developers who keep many project directories and want one
command to reach a working terminal layout for any of them. ledge runs in a terminal, inside
or outside an existing session.

## Scope and platforms

- ledge runs from a terminal on Linux.
- ledge draws all of its pickers itself and needs no external picker program.
- ledge is a single program with no runtime dependency on its build location.
- Optional integrations: VS Code, IntelliJ IDEA, or any command given
  by the user (opened on a directory, detached).
- Not applicable: graphical interfaces, multi-user sharing, remote hosts.

## Functional requirements

### Platforms (FR-PLAT)

- FR-PLAT-01: ledge shall run on Linux with tmux as its session backend.
- FR-PLAT-02: ledge shall store configuration and cache in the standard per-user folders of Linux.

### Terminal interface (FR-TUI)

- FR-TUI-01: ledge shall draw every picker in its own terminal interface, without an external picker program.
- FR-TUI-02: ledge shall filter the rows of a picker as the user types, matching fuzzily over the first visible column unless noted.
- FR-TUI-03: ledge shall move the selection with the arrow keys and the page keys.
- FR-TUI-04: ledge shall show each picker with a header row, a border label with an icon, a footer listing its keys, and a prompt.
- FR-TUI-05: ledge shall redraw each picker to fit the terminal size and keep the selected row visible when the terminal is resized.
- FR-TUI-06: ledge shall restore the terminal to its previous state on exit, including after an error or an interrupt.
- FR-TUI-07: ledge shall use the same key for the same action in every picker where that action exists.
- FR-TUI-08: ledge shall dim rows whose directory is missing.
- FR-TUI-09: ledge shall keep its markers and labels readable in terminals without color support.
- FR-TUI-10: ledge shall keep a picker open after an action that leaves it with rows, and close it when it has none, unless FR-CMD-12 applies.

### Open (FR-OPEN)

- FR-OPEN-01: ledge shall open the session for the current directory when run with no command word.
- FR-OPEN-02: ledge shall open the session for the directory given as its single argument.
- FR-OPEN-03: ledge shall fail with "Directory '<dir>' not found." when the given directory does not exist or is not a directory.
- FR-OPEN-04: ledge shall reject more than one directory argument with "project open takes at most one directory argument."
- FR-OPEN-05: ledge shall name each session from its directory so that the same directory always yields the same session name.
- FR-OPEN-06: ledge shall give different sessions to two directories that share a basename.
- FR-OPEN-07: ledge shall create the session, with its windows, only when no session of that name exists.
- FR-OPEN-08: ledge shall attach to the session when run outside a session and switch the client to it when run inside one.
- FR-OPEN-09: ledge shall start the session backend's server with the new session when no server is running.
- FR-OPEN-10: ledge shall record the opened directory in the history before opening its session.
- FR-OPEN-11: ledge shall continue opening the session when the history cannot be written, and shall print a warning.

### Project picker (FR-PICK)

- FR-PICK-01: ledge shall list every visible directory directly under each search path as a project.
- FR-PICK-02: ledge shall hide directories whose names start with a dot.
- FR-PICK-03: ledge shall sort projects by full path.
- FR-PICK-04: ledge shall place the current directory first and preselect it.
- FR-PICK-05: ledge shall mark the current directory with "*" and show its path when it is not a project.
- FR-PICK-06: ledge shall show each project with its basename, its session name, and its profile when a session exists.
- FR-PICK-07: ledge shall show a filled marker for projects with a live session and an empty marker otherwise.
- FR-PICK-08: ledge shall open the chosen project on enter.
- FR-PICK-09: ledge shall show "Missing directory: <path>" and keep the picker open when the chosen directory no longer exists.
- FR-PICK-10: ledge shall exit with "No search paths defined in <config file>." when no search path is configured.
- FR-PICK-11: ledge shall reject a directory argument on the project picker.
- FR-PICK-12: ledge shall accept a profile option that applies to the opened session.
- FR-PICK-13: ledge shall show the border label "Projects (Profile: <name>)" when a non-basic profile applies, otherwise "Projects & Submodules".

### History (FR-HIST)

- FR-HIST-01: ledge shall keep the most recently opened directories, newest first, up to 50 entries.
- FR-HIST-02: ledge shall move a directory to the top of the history when it is opened again, without duplicating it.
- FR-HIST-03: ledge shall list history entries one per line, with the home directory shown as "~", and print "No history." when empty.
- FR-HIST-04: ledge shall offer a picker over history entries that opens the chosen directory on enter.
- FR-HIST-05: ledge shall print "No history cache found." and stop when the history picker is asked for and no history file exists.
- FR-HIST-06: ledge shall delete the history file on the clean command and print "Project history cache clear successfully." (exact wording in Messages).
- FR-HIST-07: ledge shall print "No history cache found." when clearing and no history file exists.
- FR-HIST-08: ledge shall leave bookmarks untouched when the history is cleared.

### Bookmarks (FR-BM)

- FR-BM-01: ledge shall store bookmarks as absolute directory paths, one per line, in insertion order.
- FR-BM-02: ledge shall accept a bookmark only for a directory that sits inside a project, and refuse a project itself.
- FR-BM-03: ledge shall refuse a bookmark outside every project with "<path> is not inside a project. Bookmarks go inside a directory from search_paths."
- FR-BM-04: ledge shall refuse a project with "<path> is a project; open it with 'ledge project pick'."
- FR-BM-05: ledge shall print "Bookmarked <path>" when a bookmark is added.
- FR-BM-06: ledge shall print "Already bookmarked: <path>" and change nothing when the directory is already bookmarked.
- FR-BM-07: ledge shall default the bookmark add target to the current directory.
- FR-BM-08: ledge shall list bookmarks with their path and a status of "ok", "missing", or "outside project", and print "No bookmarks." when empty.
- FR-BM-09: ledge shall remove a bookmark on request and print "Removed bookmark <path>".
- FR-BM-10: ledge shall fail with "'<path>' is not a bookmark." when removing a directory that is not bookmarked.
- FR-BM-11: ledge shall show a bookmark picker that lists bookmarks in stored order, dimming those whose directory is missing.
- FR-BM-12: ledge shall print "No bookmarks. Use 'ledge bm add' to add one." and stop the bookmark picker when there are no bookmarks.
- FR-BM-13: ledge shall delete a bookmark from the bookmark picker and kill its session, on the delete key.
- FR-BM-14: ledge shall reject a directory argument on the bookmark picker with "bm pick takes no directory argument; use 'ledge bm add DIR'."

### Bookmark edit window (FR-EDIT)

- FR-EDIT-01: ledge shall open a window listing the subdirectories of a project, expandable as a tree, hidden directories skipped.
- FR-EDIT-02: ledge shall toggle the bookmark of the selected directory on enter, without closing the window.
- FR-EDIT-03: ledge shall expand a folder with the right-arrow key and collapse it with the left-arrow key.
- FR-EDIT-04: ledge shall show a folder with subfolders with "+" when collapsed and "-" when expanded.
- FR-EDIT-05: ledge shall remember which folders are expanded between runs.
- FR-EDIT-06: ledge shall mark bookmarked directories with the bookmark marker in a column of their own.
- FR-EDIT-07: ledge shall close the window on escape.
- FR-EDIT-08: ledge shall choose the project as the project containing the current directory, or the project containing a given directory.
- FR-EDIT-09: ledge shall use the directory of the current session when the session-scope option is given.
- FR-EDIT-10: ledge shall fail with "<path> is not inside a project from search_paths." when the chosen directory is outside every project.
- FR-EDIT-11: ledge shall exit without a window when the project has no subdirectories.

### Bookmark tree (FR-TREE)

- FR-TREE-01: ledge shall show one root per project that contains at least one bookmark, with its bookmarks nested under their nearest shown ancestor.
- FR-TREE-02: ledge shall show directories between a root and a bookmark as gray connector entries.
- FR-TREE-03: ledge shall show a shared parent of two bookmarks as a gray entry.
- FR-TREE-04: ledge shall leave out bookmarks that are outside every project.
- FR-TREE-05: ledge shall show only the tree that contains a given directory, with the cursor on its nearest node.
- FR-TREE-06: ledge shall show only the tree of the directory of the current session when the session-scope option is given.
- FR-TREE-07: ledge shall show only the root directory when the given directory is outside every tree.
- FR-TREE-08: ledge shall show all trees when no directory and no session-scope option is given.
- FR-TREE-09: ledge shall reject two directory arguments with "bm tree takes at most one directory."
- FR-TREE-10: ledge shall reject combining the session-scope option with a directory with "--session-scope cannot be combined with a directory."
- FR-TREE-11: ledge shall open the chosen entry on enter, but not a gray connector entry.
- FR-TREE-12: ledge shall keep the tree window open when its last entry is removed.

### Session switcher (FR-SESS)

- FR-SESS-01: ledge shall list managed sessions only, never sessions it did not create.
- FR-SESS-02: ledge shall list the session the user is in first, then the rest by most recent activity.
- FR-SESS-03: ledge shall show for each session its name, project, profile, and last activity as "just now", "<n>m ago", "<n>h ago", or "<n>d ago".
- FR-SESS-04: ledge shall switch the client to the chosen session on enter inside a session, and attach to it outside one.
- FR-SESS-05: ledge shall print "ss takes no directory argument." and stop when a directory is given to the switcher.
- FR-SESS-06: ledge shall list managed sessions as a tree of their directories, with gray entries for project folders that have no session.
- FR-SESS-07: ledge shall show a gray folder entry with the session name it would get.
- FR-SESS-08: ledge shall open the directory of the chosen session tree entry on enter.
- FR-SESS-09: ledge shall list managed sessions in a plain table with session name, windows, state (attached or detached), and path, and print "No ledge sessions." when none exist.
- FR-SESS-10: ledge shall kill a managed session given by exact name, or the session of a given directory.
- FR-SESS-11: ledge shall print "'<name>' is not a ledge session." when the target is not a managed session.
- FR-SESS-12: ledge shall print "Killed session '<name>'." after killing a session.
- FR-SESS-13: ledge shall fail with "kill requires a session name or directory." when no target is given.

### Session actions (FR-ACT)

- FR-ACT-01: ledge shall kill the session of the selected row on the kill key, in every picker.
- FR-ACT-02: ledge shall rebuild the session of the selected row on the rebuild key, creating it if it is missing.
- FR-ACT-03: ledge shall rebuild a session by killing it and building it again from its profile, which stops everything running in it.
- FR-ACT-04: ledge shall offer a profile choice on the profile key and rebuild the selected session with the chosen profile.
- FR-ACT-05: ledge shall build a copy of the session when the user rebuilds the session they are in, switch to the copy, and give it the original name.
- FR-ACT-06: ledge shall apply the alt-key action to every managed session under the selected tree node.
- FR-ACT-07: ledge shall never rebuild or kill the session the user is in through an alt-key action.
- FR-ACT-08: ledge shall rebuild each session under a node with the profile recorded on that session, when the alt-rebuild key is used.
- FR-ACT-09: ledge shall ask for one profile and rebuild every session under a node with it, when the alt-profile key is used.
- FR-ACT-10: ledge shall delete every bookmark under a node and kill their sessions, when the alt-delete key is used in the bookmark tree.
- FR-ACT-11: ledge shall apply the delete key only to bookmarks, and do nothing on other rows.
- FR-ACT-12: ledge shall pass the profile option to the session-rebuild actions, so that a chosen profile takes effect.
- FR-ACT-13: ledge shall apply rebuilds and kills to managed sessions only.

### Profiles (FR-PROF)

- FR-PROF-01: ledge shall read each profile as a file named by the profile, in the profiles folder of the configuration.
- FR-PROF-02: ledge shall require every profile to contain a list of windows.
- FR-PROF-03: ledge shall create the session's windows in the order the profile lists them.
- FR-PROF-04: ledge shall run the command of each window in it, when a command is given.
- FR-PROF-05: ledge shall use the first window as the session's first window and select it after the build.
- FR-PROF-06: ledge shall build a single window named "Terminal" when no profile applies.
- FR-PROF-07: ledge shall record the profile name on each session it creates, using "basic" for the single-window layout.
- FR-PROF-08: ledge shall apply a profile only when a session is created or rebuilt, so an edited profile has no effect on a running session until it is rebuilt.
- FR-PROF-09: ledge shall use the profile given with the profile option over the default profile.
- FR-PROF-10: ledge shall use the single-window layout with the no-profile option, ignoring profiles.
- FR-PROF-11: ledge shall fail with the profile's error when the profile given with the profile option is missing or invalid.
- FR-PROF-12: ledge shall warn and use the single-window layout when the default profile is missing or invalid.
- FR-PROF-13: ledge shall list profiles in the profile picker as "[Current: <name>]" when rebuilding an existing session, then "[Default: <name>]", then every other profile and "basic".
- FR-PROF-14: ledge shall reuse the profile recorded on a session when a rebuild is requested without a profile option.
- FR-PROF-15: ledge shall warn "session profile '<name>' is missing or invalid; using the default profile." when a session's recorded profile cannot be loaded during a rebuild.

### Configuration and files (FR-CONF)

- FR-CONF-01: ledge shall create the configuration file with the default search path "~/projects" on first run when it does not exist.
- FR-CONF-02: ledge shall read search paths and the default profile from the configuration file.
- FR-CONF-03: ledge shall expand "~" and environment variables in search paths.
- FR-CONF-04: ledge shall skip empty search path entries.
- FR-CONF-05: ledge shall fail with "config is not valid JSON: <file>" when the configuration file cannot be parsed.
- FR-CONF-06: ledge shall create the profiles folder when it is missing.
- FR-CONF-07: ledge shall treat a directory as a project only when it sits directly under a search path.
- FR-CONF-08: ledge shall honour the configuration and cache folder environment settings, falling back to the standard per-user folders of Linux.

### Session naming and tagging (FR-NAME)

- FR-NAME-01: ledge shall name a session as the directory basename, with dots replaced by underscores, followed by a dash and the first four hex characters of an MD5 digest of the absolute path.
- FR-NAME-02: ledge shall keep this naming stable across versions so that existing sessions are recognised.
- FR-NAME-03: ledge shall mark each session it creates with a managed marker and the profile name.
- FR-NAME-04: ledge shall consider only marked sessions in every list, picker, kill, label and check.
- FR-NAME-05: ledge shall let a user adopt an unmarked session by setting the managed marker on it manually.

### Grouping and scope (FR-SCOPE)

- FR-SCOPE-01: ledge shall group sessions and bookmarks by the project that contains them.
- FR-SCOPE-02: ledge shall apply a group action to every managed session whose directory is the group entry or inside it.

### Diagnostics (FR-DOC)

- FR-DOC-01: ledge shall check that each external program it needs on the current platform is installed and print "OK" or "ERR" for each.
- FR-DOC-02: ledge shall check that the configuration file exists and parses.
- FR-DOC-03: ledge shall warn when no search path is configured or a search path does not exist.
- FR-DOC-04: ledge shall warn when a bookmark's directory is missing or sits outside every project.
- FR-DOC-05: ledge shall check that every profile file contains a list of windows.
- FR-DOC-06: ledge shall check that the default profile is valid.
- FR-DOC-07: ledge shall warn for each managed session whose directory is missing or whose profile no longer exists.
- FR-DOC-08: ledge shall print one line per check, prefixed with "OK", "WARN" or "ERR", and exit 1 when any check is "ERR".

### Commands and exit behaviour (FR-CMD)

- FR-CMD-01: ledge shall print its help text for the help command and the help flags.
- FR-CMD-02: ledge shall print the group usage text for a group command given without a sub-command.
- FR-CMD-03: ledge shall reject an unknown command with "Unknown <group> command: <word> (run 'ledge <group>' for the list)."
- FR-CMD-04: ledge shall reject an unknown option with "Unknown option: <option> (run 'ledge help' for usage)."
- FR-CMD-05: ledge shall reject a profile option given without a value with "--profile requires a profile name."
- FR-CMD-06: ledge shall reject an editor option given without a command with "--editor requires a command."
- FR-CMD-07: ledge shall exit with status 1 on every error, and with status 0 on cancel.
- FR-CMD-08: ledge shall print "Cancelled." and exit 0 when a picker is closed without a choice.
- FR-CMD-09: ledge shall fail with "<name> is required for this command." when a required external program is missing.
- FR-CMD-10: ledge shall fail with "Required tool 'tmux' is not installed." when tmux is missing at start on Linux.
- FR-CMD-11: ledge shall accept flags only after the command word.
- FR-CMD-12: ledge shall exit a picker without a window when it has no rows to show, unless the picker is meant to stay open.

### Editor launch (FR-EDIT-LAUNCH)

- FR-LAUNCH-01: ledge shall launch the chosen editor on the directory only when a session is created, not when an existing one is opened.
- FR-LAUNCH-02: ledge shall launch VS Code with the code option, IntelliJ IDEA with the idea option, and any command with the editor option.
- FR-LAUNCH-03: ledge shall start the editor detached so that it outlives ledge.

## Screens and interactions

Shared rules: every picker has a header row, a border label with an icon, a footer listing its keys, a prompt "> ", and fuzzy search over the first visible column unless noted. Enter selects the row, and escape or an empty choice cancels.

### Project picker

| Key | Action | Condition |
| --- | --- | --- |
| enter | Open the project | Directory exists |
| ctrl-r | Rebuild the session, creating it if missing | Always |
| ctrl-p | Pick a profile, then rebuild | Always |
| ctrl-x | Kill the session and reopen the list | Session is managed |
| escape | Cancel | Always |

Columns: marker, project, session, profile. Footer: "enter open · ^r rebuild · ^p profile · ^x kill".

### History picker

Same keys as the project picker. Footer: "enter open". Columns: project, session, profile.

### Session switcher

| Key | Action | Condition |
| --- | --- | --- |
| enter | Switch client (inside a session) or attach (outside) | Always |
| ctrl-r | Rebuild the session in its directory | Always |
| ctrl-p | Pick a profile, then rebuild | Always |
| ctrl-x | Kill the session; the list stays open | Always |

Columns: session, project, profile, last activity. Footer: "enter switch · ^r rebuild · ^p profile · ^x kill". The list stays open after the list is empty.

### Bookmark picker

| Key | Action | Condition |
| --- | --- | --- |
| enter | Open the project | Directory exists |
| ctrl-r, ctrl-p, ctrl-x | As in the project picker | Always |
| ctrl-d | Delete the bookmark, kill its session, reopen the list | Row is a bookmark |

Footer: "enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark". Missing directories are shown dimmed.

### Bookmark tree and session tree

Rows are tree-drawn. Border label "Bookmark tree" or "Session tree". The list opens with the tree top-down and does not sort.

| Key | Action | Condition |
| --- | --- | --- |
| enter | Open the entry | Entry is not a gray connector |
| ctrl-r, ctrl-p, ctrl-x | Act on the selected session | Row has a session |
| ctrl-d | Delete the selected bookmark and kill its session | Bookmark tree only; row is a bookmark |
| alt-r | Rebuild each session under the node with its own profile | Node has sessions |
| alt-p | Pick one profile, then rebuild every session under the node | Node has sessions |
| alt-x | Kill every session under the node | Node has sessions |
| alt-d | Delete every bookmark under the node and kill their sessions | Bookmark tree only |
| escape | Cancel | Always |

Footer (bookmark tree): "enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node". Footer (session tree): "enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node". The alt-keys never act on the session the user is in.

### Bookmark edit window

| Key | Action | Condition |
| --- | --- | --- |
| enter | Toggle the bookmark of the row | Always |
| right arrow | Expand the folder | Folder has subfolders |
| left arrow | Collapse the folder, or move to the parent | Always |
| escape | Close the window | Always |

Columns: bookmark marker, directory tree. Footer: "enter toggle bookmark · ^l open · ^h close · esc done". Border label: "Bookmarks in <project basename>".

### Profile picker

Rows are profile names; enter chooses one; escape cancels. Border label "Profiles". Footer: none required.

## Data and storage

| Item | Identifier | Storage | Format | Who changes it |
| --- | --- | --- | --- | --- |
| Configuration | Fixed file name | Configuration folder | JSON object with search paths and default profile | User |
| Profile | Profile name | Profiles folder in configuration folder | JSON object with a list of windows | User |
| Session | Session name (see FR-NAME-01) | Session backend | Session with windows and managed marker | ledge, user (kill, rebuild) |
| History | Absolute directory path | Cache folder, file "history" | One path per line, newest first, at most 50 | ledge (add, clean) |
| Bookmarks | Absolute directory path | Cache folder, file "bookmarks" | One path per line, insertion order | ledge (add, remove, edit), user |
| Bookmark edit state | Absolute directory path | Cache folder, file "bm-edit-open" | One path per line, expanded folders | ledge |

Paths are stored absolute and cleaned. A missing data file means no entries.

## Configuration

- Configuration folder: "~/.config/ledge" by default, or the folder set by the configuration home environment setting.
- Cache folder: "~/.cache/ledge" by default, or the folder set by the cache home environment setting.
- Configuration file: "config.json" in the configuration folder, created on first run.
- Default configuration content: search paths "~/projects", no default profile.
- Keys: "search_paths" (list of directories whose immediate children are projects), "default_profile" (profile name, optional).
- Profiles folder: "profiles" inside the configuration folder.
- Environment variables that move the folders: the configuration home and cache home settings.
- Test isolation: ledge shall honour a separate session server name given by an environment variable, so tests never touch the user's server.

## External dependencies

- tmux 3.2 or later (Linux): ledge shall create, list, attach, switch, rename, kill and option-tag sessions and windows, and shall start a server when none runs.
- setsid (optional, Linux only, for editor launch): ledge shall use it to start editors detached.
- VS Code, IntelliJ IDEA, or any user command (optional): ledge shall run it with the directory as argument.
- Build toolchain (build time only): ledge shall be built from source by a provided install script.

## Messages

Exact wording, quoted, with the situation that triggers it. Placeholders are in angle brackets.

- "Error: <message>" on stderr, exit 1: any fatal error.
- "Warning: <message>" on stderr: a non-fatal problem.
- "Cancelled." on stdout, exit 0: a picker closed without a choice.
- "Directory '<dir>' not found.": open target is missing or not a directory.
- "project open takes at most one directory argument.": two directories given to open.
- "Missing directory: <path>": picked project no longer exists; picker stays open.
- "No search paths defined in <config file>.": project picker with no search paths.
- "<name> is required for this command.": required program missing.
- "Unknown option: <option> (run 'ledge help' for usage).": unrecognised flag.
- "--profile requires a profile name.": profile option without value.
- "--editor requires a command.": editor option without value.
- "Unknown <group> command: <word> (run 'ledge <group>' for the list).": unknown sub-command.
- "profile '<name>' not found at <path>": named profile missing.
- "profile '<name>' is invalid JSON: <detail>": profile not parsable.
- "profile '<name>' is invalid (expected a 'windows' array)": profile lacks windows.
- "default_profile '<name>' is missing or invalid; using single-window layout.": warning at start.
- "session profile '<name>' is missing or invalid; using the default profile.": warning during rebuild.
- "config is not valid JSON: <file>": configuration cannot be parsed.
- "could not create session '<name>': <detail>": session build failure.
- "could not attach to '<name>': <detail>": attach failure.
- "could not update history: <detail>": history write failure, warning.
- "No history.": history list empty.
- "No history cache found.": history picker or clean with no file.
- "Project history cache clear successfully.": history cleared.
- "No bookmarks.": bookmark list empty.
- "No bookmarks. Use 'ledge bm add' to add one.": bookmark picker empty.
- "Bookmarked <path>": bookmark added.
- "Already bookmarked: <path>": bookmark already present.
- "Removed bookmark <path>": bookmark removed.
- "'<path>' is not a bookmark.": removal of unknown bookmark.
- "bookmark remove requires a directory.": remove without argument.
- "bm add takes at most one directory.": two directories given to add.
- "bm pick takes no directory argument; use 'ledge bm add DIR'.": directory given to bookmark picker.
- "<path> is a project; open it with 'ledge project pick'.": adding a project as bookmark.
- "<path> is not inside a project. Bookmarks go inside a directory from search_paths.": bookmark outside projects.
- "<path> is not inside a project from search_paths.": edit window outside projects.
- "bm tree takes at most one directory.": two directories given to tree.
- "--session-scope cannot be combined with a directory.": conflicting scope options.
- "ss takes no directory argument.": directory given to switcher.
- "ss tree takes no directory argument.": directory given to session tree.
- "No ledge sessions.": session list empty. (Wording to be confirmed; see Decisions still open.)
- "'<name>' is not a ledge session.": kill target not managed.
- "Killed session '<name>'.": kill succeeded.
- "kill requires a session name or directory.": kill without target.
- "[Current: <name>]", "[Default: <name>]": profile picker row labels.
- Doctor lines: "OK    <check>", "WARN  <check>", "ERR   <check>" with the check text in the Diagnostics requirements.

## Installation and removal

- ledge shall be built and installed by a provided script into the user's local binary folder, which must be on the PATH.
- ledge shall warn when the local binary folder is not on the PATH after install.
- ledge shall record the source folder of the install so that removal only deletes a binary it installed.
- ledge shall remove its binary on the uninstall option, when that binary came from the same source folder, and report "Removed <path>".
- ledge shall refuse to remove a binary installed from elsewhere, with "Not removed: <path> was not installed from <source>", exit 1.
- ledge shall report "Nothing to remove: <path> does not exist" when no binary is present.
- ledge shall fail with "Error: build toolchain is required to build ledge." when no build toolchain is present.
- ledge shall leave the configuration, profiles, history and bookmarks in place on removal.

## Quality expectations

- Testable without any session backend or external program: session naming, configuration loading and defaults, profile validation, history add/read/limit, bookmark add/remove/read and normalisation, tree construction and scope, column alignment and truncation, relative age text, rebuild skip rules, picker filtering and selection.
- Verified against the Linux session backend only in manual or integration runs: picker keys and layouts, session creation, attach and switch, rebuild, kill, editor launch.
- Integration runs shall use a separate session server name and never the user's default server.
- Automated checks: linting, formatting, and the automated test suite must pass before a release.

## Decisions still open

- Command groups: should the command groups (project, bm, ss) keep their current names, or be renamed?
- Should the session naming scheme stay byte-for-byte compatible with earlier sessions, or may ledge migrate them?
- Should the default search path "~/projects" be kept, or should first run ask the user?
- Should ledge warn or fail when a profile's recorded session profile is missing during the session switcher's rebuild?
- Should history and bookmark files be written atomically to avoid corruption on interruption?
- What is the expected behaviour when two bookmarks share a path with different trailing slashes?
- Should the session switcher show sessions outside any search path, or only managed ones?
- Is "No ledge sessions." acceptable wording, given the product name?
- Should the bookmark edit window's expanded-folder state be per project or global?
- Should the built-in fuzzy search match the ranking and highlighting of the previous external picker, or use its own rules?

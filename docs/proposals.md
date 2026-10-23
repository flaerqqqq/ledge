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

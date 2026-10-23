# Conventions

STATUS: APPROVED
Owner: user. Refined by the architect in Phase 0 (gate A, decisions D-01..D-17); gate B waived (autonomous
mode). Layering and module names are in `docs/architecture.md` sections 2 and 3.

## Rust toolchain and crates
- Edition 2021. `rust-version` in `Cargo.toml` is set in S-00 to the lowest toolchain that builds the pinned
  crates (not below 1.80). Raise it only when a pinned crate requires it, and note the change in
  architecture section 10.
- `Cargo.lock` is committed (binary crate, `--locked` in the install script and the gate).
- Runtime dependencies are the four listed in architecture section 10 (`ratatui`, `serde_json`, `md-5`,
  `signal-hook`). No dev-dependencies. Adding a crate needs user approval (proposal first).
- `cargo fmt` with the default config. `cargo clippy --all-targets -- -D warnings` must be clean.
- Lints in `Cargo.toml`:
  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  [lints.clippy]
  unwrap_used = "deny"
  expect_used = "deny"
  panic = "deny"
  ```
  Test code opts out with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the top of
  each test file and `#[cfg(test)]` modules that need it. Non-test code uses `?`, `match`, `unwrap_or`,
  `unwrap_or_default` and `get`. There is no `expect` in non-test code; a state that "cannot happen" is
  returned as `LedgeError::Fatal` with a message from `messages`.
- `#![forbid(unsafe_code)]` is in `lib.rs` and `main.rs`. No exception is justified in the architecture.
- Indexing that can panic (`v[i]`, slicing) is avoided in non-test code; use `get`, iterators, or
  `split_first`.

## Errors
- One error type: `LedgeError::Fatal(String)` (architecture section 8.1). Hand-written `Display`.
- User-facing text is defined only in `src/messages.rs`. It matches the "Messages" section of
  `requirements.md` and D-11..D-17 character for character. A test (`tests/messages_verbatim.rs`) holds the
  literal strings.
- Stream and prefix follow the classes in architecture section 8.2: error (stderr, `Error: `, exit 1),
  warning (stderr, `Warning: `, exit unchanged), info (stdout, exit 0), cancel (`Cancelled.`, stdout, exit 0).
- `main` is the only place that calls `std::process::exit`.
- An `io::Error` is never shown raw; the caller wraps it with the message that names the situation.

## Structure
- Pure logic lives in `domain/` and has no I/O (no `std::fs`, `std::env`, `std::process`, `std::io`).
- I/O sits behind the traits in `ports/` and `tui/terminal.rs`; adapters implement them. `app/` receives the
  ports through `app::Ctx` and never names an adapter.
- Dependency rules are the table in architecture section 3. `scripts/check_layers.sh` enforces them in the gate.
- One responsibility per module. A module over about 400 lines is split.
- No global mutable state. Pass dependencies explicitly. `static` is only for `const` data.
- Path values use `AbsPath` (cleaned, absolute, UTF-8). Non-UTF-8 directory names are skipped with no message.
- Symlinks are followed for directory checks (D-06, F-24). Lexical cleaning does not resolve links.

## Naming
- Types `UpperCamelCase`, functions and modules `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Message functions are named by role (`no_bookmarks()`, `bookmarked(path)`), never by number.
- Test functions describe behaviour: `rejects_bookmark_outside_project`, `keeps_selection_on_same_row_after_reload`.
- Files and folders: `snake_case`. Shell scripts: `kebab-case.sh` under `scripts/`.
- Tmux session names are `<basename with . and : replaced by _>-<4 hex>` (FR-NAME-01, D-14). Options are
  `@ledge_managed`, `@ledge_profile`, `@ledge_dir`. Those names are constants in `domain/naming.rs` and
  `adapters/tmux.rs`; nothing else writes them.

## Requirement references
- Public items implementing a requirement carry the FR-ID in the doc comment: `/// FR-BM-03: ...`.
- Each test has the FR-ID in a comment line directly above it: `// FR-BM-03`. A test that checks a message
  says so in the comment: `// FR-BM-03, message`.
- Tests that cover a decision cite the decision: `// D-14`.

## Comments
- Comment WHY, not WHAT. No commented-out code.
- TODO and FIXME name a backlog slice: `// TODO(S-12): ...`. A TODO without a slice is rejected in review.

## Shell (scripts/)
- Bash, `#!/usr/bin/env bash`, `set -euo pipefail`. `shellcheck` clean when available (the gate runs it if it is
  installed and reports a skip otherwise).
- Messages in scripts are written exactly as in architecture section 11 and 8.3.
- Scripts never touch the real `~/.local` or `~/.config` when `LEDGE_INSTALL_BIN_DIR`, `LEDGE_INSTALL_STATE_DIR`
  or the test wrapper set their folders.

## Git
- Branch `slice/<id>-<name>` (for example `slice/S-03-history-bookmarks`). Commit message: `<slice id>: <summary>`,
  then a body with the FR-IDs of the slice.
- One slice, one branch, one reviewable diff. No unrelated refactors. No commits to `main`, no pushes.
- `docs/traceability.md` is updated in the same slice (tests and code columns).

## Dependencies
- Minimal. The list and reasons are in architecture section 10. Changes need a proposal.

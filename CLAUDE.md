# ledge

ledge is a Rust tmux project/session manager with its own TUI pickers. The product is defined by
`requirements.md` (the source of truth for WHAT). This repo is built by an agent "factory": a
coordinator (the main Claude session) drives specialised worker subagents through small slices.

## Read before acting (every agent, every time)

1. `requirements.md` - what to build. FR-IDs are the unit of traceability.
2. `docs/decisions.md` - resolved product decisions. Do not re-litigate `Accepted` ones.
3. `docs/architecture.md` - modules, layering, trait boundaries.
4. `docs/conventions.md` - Rust style, errors, naming, layout.
5. `docs/testing.md` - what is tested how.
6. `docs/backlog.md` and `docs/traceability.md` - current slice state and FR coverage.
7. `docs/workflow.md` - roles, gates, escalation rules.

If a doc is still a skeleton marked `STATUS: DRAFT`, the factory is in Phase 0: do not write
product code until the user has approved the docs (see `docs/workflow.md`).

## Commands

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format check: `cargo fmt --check`
- Gate (all four must pass before a slice is done): fmt, clippy, build, test.
- Integration tests that touch tmux must use a separate server name via the env var defined in
  `docs/testing.md`. Never touch the user's default tmux server.

## Factory entry points (skills)

- `/phase0` - run the Phase 0 planning pipeline (analyst -> architect -> user approval).
- `/next-slice` - coordinator loop for the next ready slice (test -> implement -> review -> gate).
- `/factory-status` - report backlog, traceability coverage and pending escalations.

## Hard rules

- Only the user changes `requirements.md`, `docs/architecture.md`, `docs/conventions.md` and
  `docs/decisions.md`. Agents PROPOSE changes in `docs/proposals.md`; the coordinator escalates.
- Error and message wording must match the "Messages" section of `requirements.md` exactly.
- Every code change names the FR-IDs it covers; `docs/traceability.md` is updated in the same slice.
- No `unwrap()`/`expect()` in non-test code unless justified by a comment (see conventions).
- No git push, no force operations, no work on `main`. Slices live on branches `slice/<id>-<name>`.
  Commits are made only after reviewer approval and only with the user's go-ahead
  (supervised mode) or the standing authorisation in autonomous mode (see `docs/workflow.md`).
- Do not add dependencies without an entry in `docs/architecture.md` (architect proposes, user approves).

# Factory workflow

STATUS: ACTIVE

## Roles

| Role | Kind | Writes | Never touches |
|---|---|---|---|
| coordinator | main session (skills `/phase0`, `/next-slice`) | `docs/backlog.md` status fields, `docs/proposals.md` triage notes | code, tests, requirements, architecture |
| requirements-analyst | subagent | `docs/analysis.md`, proposed decisions in `docs/decisions.md` (status `Proposed`) | code |
| architect | subagent | `docs/architecture.md`, `docs/conventions.md`, `docs/testing.md`, `docs/backlog.md` (slice definitions) | code |
| test-engineer | subagent | tests, test fakes, `docs/traceability.md` (tests column) | production code |
| implementer | subagent | production code, `docs/traceability.md` (code column) | tests (except fixing compile breaks), docs owned by user |
| reviewer | subagent, read-only | nothing; returns a verdict | everything |

Subagents cannot spawn subagents, so the coordinator must be the main session.

## Phase 0: planning (user in the loop)

1. requirements-analyst audits `requirements.md`, writes `docs/analysis.md`, proposes answers for
   every item under "Decisions still open" into `docs/decisions.md`.
2. User reviews: marks each decision `Accepted` / `Rejected` (or edits). Requirement edits are the user's.
3. architect writes `architecture.md`, `conventions.md`, `testing.md`, and the slice backlog.
4. User reviews and approves: change `STATUS: DRAFT` to `STATUS: APPROVED` in each doc.
   No slice starts until architecture, conventions, testing and backlog are `APPROVED`.

## Slice loop (per slice)

A slice is small (target: one reviewable PR, a few hundred lines), covers a defined set of FR-IDs,
and lists its dependencies in `docs/backlog.md`.

1. coordinator picks the first `ready` slice whose dependencies are `done`, creates branch
   `slice/<id>-<name>`, sets status `in-progress`.
2. test-engineer writes failing tests derived from the slice's FR-IDs (and the exact messages in
   requirements), not from any implementation. Confirms they fail for the right reason.
3. implementer makes the tests pass with minimal, convention-following code. Does not weaken tests.
   If a test looks wrong, it reports it instead of editing it.
4. coordinator runs the gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo build`, `cargo test`.
5. reviewer checks the diff against requirements, architecture, conventions, testing, traceability.
   Verdict: `APPROVE` or `CHANGES_REQUIRED` with numbered findings.
6. On `CHANGES_REQUIRED`: back to implementer (or test-engineer if findings are about tests) with
   the findings. Max 3 rounds, then escalate.
7. On `APPROVE` plus green gate: slice status -> `done`; coordinator reports to the user.

## Escalation (coordinator stops and asks the user)

- Any proposal in `docs/proposals.md` to change requirements, architecture, conventions or dependencies.
- A requirement that is ambiguous, contradictory, or untestable as written.
- 3 failed review rounds, or the gate fails twice with no clear fix.
- A slice that turns out to be too large (split it; architect redefines).
- Anything that would need `main`, push, or destructive git operations.

Escalation message format: slice id, what blocked, options with a recommendation, what you need decided.

## Modes

- **Supervised (default):** stop after every slice, show summary and diff stat, ask the user whether
  to commit and continue.
- **Autonomous:** only if the user says so explicitly for the session (e.g. "run 3 slices
  autonomously"). The coordinator may commit on slice branches (never `main`, never push) and
  continue to the next ready slice, stopping at any escalation trigger or after the requested count.

## Handoff briefs

The coordinator gives each worker a brief, not a retelling of the docs:
slice id, FR-IDs, branch, files/modules in scope, links to docs to read, the previous step's output
path or summary, and the expected output format. Workers read the docs themselves.

## Proposals

Anyone who wants to change a user-owned doc appends to `docs/proposals.md`:
`### P-<n> <title>` / who / affected doc / change / why. The coordinator escalates open proposals.

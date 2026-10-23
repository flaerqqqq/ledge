---
name: factory-status
description: Report the ledge factory state - doc approval status, backlog progress, requirement coverage, open proposals and decisions. Read-only.
disable-model-invocation: true
---

Read-only. Do not spawn agents or modify files.

Read `docs/*.md` and report concisely:

1. **Docs:** each of decisions, architecture, conventions, testing, backlog with its `STATUS`.
2. **Decisions:** counts of Proposed / Accepted / Rejected.
3. **Backlog:** counts per status; the next `ready` slice; any `escalated` slice and why.
4. **Coverage:** from `docs/traceability.md`, number of FR-IDs `open` / `covered` / `manual` out of
   the total in `requirements.md` (flag any FR-ID missing from the matrix).
5. **Proposals:** open items from `docs/proposals.md`, one line each.
6. **Git:** current branch, uncommitted changes, `slice/*` branches.
7. **Next step:** one line - `/phase0`, resolve X, or `/next-slice`.

Keep it to a short table or list. Do not repeat doc contents beyond what is asked.

# Traceability

STATUS: APPROVED
Updated by test-engineer (tests column) and implementer (code column) in the same slice.
The reviewer rejects a slice whose FR-IDs are not reflected here.

Status: `open` | `covered` | `manual` (needs a reason in the Note column).
Seeded by the architect in Phase 0 from `docs/backlog.md`: one row per FR-ID of `requirements.md` (160 rows),
each in exactly one slice. Installation statements have no FR-IDs yet (proposal P-1).

| FR-ID | Slice | Code | Tests | Status | Note |
|---|---|---|---|---|---|
| FR-PLAT-01 | S-09 |  |  | open | Integration (ignored) against isolated tmux server. |
| FR-PLAT-02 | S-01 | domain/location.rs; adapters/env.rs; ports/env.rs | domain/location.rs; tests/component_location.rs | covered |  |
| FR-TUI-01 | S-08 |  |  | open |  |
| FR-TUI-02 | S-06 |  |  | open |  |
| FR-TUI-03 | S-07 |  |  | open |  |
| FR-TUI-04 | S-07 |  |  | open |  |
| FR-TUI-05 | S-07 |  |  | open |  |
| FR-TUI-06 | S-08 |  |  | open | Restore on error checked with fake terminal; real signal path is manual. |
| FR-TUI-07 | S-07 |  |  | open |  |
| FR-TUI-08 | S-07 |  |  | open | Text cues automated; color appearance is manual. |
| FR-TUI-09 | S-07 |  |  | open | Text cues automated; color appearance is manual. |
| FR-TUI-10 | S-07 |  |  | open |  |
| FR-OPEN-01 | S-17 |  |  | open |  |
| FR-OPEN-02 | S-17 |  |  | open |  |
| FR-OPEN-03 | S-11 |  |  | open |  |
| FR-OPEN-04 | S-17 |  |  | open |  |
| FR-OPEN-05 | S-01 | domain/naming.rs; domain/path.rs | domain/naming.rs | covered |  |
| FR-OPEN-06 | S-01 | domain/naming.rs | domain/naming.rs | covered |  |
| FR-OPEN-07 | S-10 |  |  | open |  |
| FR-OPEN-08 | S-11 |  |  | open | Expected manual: attach and switch need a real client and tty; argv covered by adapter test. |
| FR-OPEN-09 | S-09 |  |  | open | Integration (ignored) against isolated tmux server. |
| FR-OPEN-10 | S-11 |  |  | open |  |
| FR-OPEN-11 | S-11 |  |  | open |  |
| FR-PICK-01 | S-04 |  |  | open |  |
| FR-PICK-02 | S-04 |  |  | open |  |
| FR-PICK-03 | S-04 |  |  | open |  |
| FR-PICK-04 | S-04 |  |  | open |  |
| FR-PICK-05 | S-04 |  |  | open |  |
| FR-PICK-06 | S-13 |  |  | open |  |
| FR-PICK-07 | S-13 |  |  | open | Live marker from fake backend; real tmux via integration. |
| FR-PICK-08 | S-13 |  |  | open |  |
| FR-PICK-09 | S-13 |  |  | open |  |
| FR-PICK-10 | S-13 |  |  | open |  |
| FR-PICK-11 | S-17 |  |  | open |  |
| FR-PICK-12 | S-13 |  |  | open |  |
| FR-PICK-13 | S-13 |  |  | open |  |
| FR-HIST-01 | S-03 | domain/history.rs; app/store.rs | domain/history.rs; tests/component_store.rs | covered |  |
| FR-HIST-02 | S-03 | domain/history.rs; app/store.rs | domain/history.rs; tests/component_store.rs | covered |  |
| FR-HIST-03 | S-18 |  |  | open |  |
| FR-HIST-04 | S-13 |  |  | open |  |
| FR-HIST-05 | S-13 |  |  | open |  |
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
| FR-TREE-01 | S-05 |  |  | open |  |
| FR-TREE-02 | S-05 |  |  | open |  |
| FR-TREE-03 | S-05 |  |  | open |  |
| FR-TREE-04 | S-05 |  |  | open |  |
| FR-TREE-05 | S-24 |  |  | open |  |
| FR-TREE-06 | S-24 |  |  | open |  |
| FR-TREE-07 | S-24 |  |  | open |  |
| FR-TREE-08 | S-24 |  |  | open |  |
| FR-TREE-09 | S-17 |  |  | open |  |
| FR-TREE-10 | S-17 |  |  | open |  |
| FR-TREE-11 | S-05 |  |  | open |  |
| FR-TREE-12 | S-22 |  |  | open |  |
| FR-SESS-01 | S-16 |  |  | open |  |
| FR-SESS-02 | S-16 |  |  | open |  |
| FR-SESS-03 | S-16 |  |  | open |  |
| FR-SESS-04 | S-16 |  |  | open | Expected manual: attach and switch need a real client and tty; argv covered by adapter test. |
| FR-SESS-05 | S-17 |  |  | open |  |
| FR-SESS-06 | S-05 |  |  | open |  |
| FR-SESS-07 | S-05 |  |  | open |  |
| FR-SESS-08 | S-23 |  |  | open |  |
| FR-SESS-09 | S-25 |  |  | open |  |
| FR-SESS-10 | S-25 |  |  | open |  |
| FR-SESS-11 | S-25 |  |  | open |  |
| FR-SESS-12 | S-25 |  |  | open |  |
| FR-SESS-13 | S-25 |  |  | open |  |
| FR-ACT-01 | S-15 |  |  | open |  |
| FR-ACT-02 | S-14 |  |  | open |  |
| FR-ACT-03 | S-14 |  |  | open |  |
| FR-ACT-04 | S-15 |  |  | open |  |
| FR-ACT-05 | S-14 |  |  | open |  |
| FR-ACT-06 | S-22 |  |  | open |  |
| FR-ACT-07 | S-14 |  |  | open |  |
| FR-ACT-08 | S-22 |  |  | open |  |
| FR-ACT-09 | S-22 |  |  | open |  |
| FR-ACT-10 | S-24 |  |  | open |  |
| FR-ACT-11 | S-19 |  |  | open |  |
| FR-ACT-12 | S-15 |  |  | open |  |
| FR-ACT-13 | S-14 |  |  | open |  |
| FR-PROF-01 | S-02 | app/store.rs; domain/profile.rs | tests/component_config.rs | covered |  |
| FR-PROF-02 | S-02 | domain/profile.rs | domain/profile.rs; tests/component_config.rs | covered |  |
| FR-PROF-03 | S-10 |  |  | open |  |
| FR-PROF-04 | S-10 |  |  | open |  |
| FR-PROF-05 | S-10 |  |  | open |  |
| FR-PROF-06 | S-02 | domain/profile.rs | domain/profile.rs; tests/component_config.rs | covered |  |
| FR-PROF-07 | S-10 |  |  | open |  |
| FR-PROF-08 | S-10 |  |  | open |  |
| FR-PROF-09 | S-11 |  |  | open |  |
| FR-PROF-10 | S-11 |  |  | open |  |
| FR-PROF-11 | S-11 |  |  | open |  |
| FR-PROF-12 | S-11 |  |  | open |  |
| FR-PROF-13 | S-15 |  |  | open |  |
| FR-PROF-14 | S-14 |  |  | open |  |
| FR-PROF-15 | S-14 |  |  | open |  |
| FR-CONF-01 | S-02 | app/store.rs; domain/config.rs | domain/config.rs; tests/component_config.rs | covered | Default text compared literally; the trailing newline after the architecture 9.2 block is an assumption. |
| FR-CONF-02 | S-02 | domain/config.rs; app/store.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-03 | S-02 | domain/config.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-04 | S-02 | domain/config.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-05 | S-02 | domain/config.rs; app/store.rs | domain/config.rs; tests/component_config.rs | covered |  |
| FR-CONF-06 | S-02 | app/store.rs | tests/component_config.rs | covered |  |
| FR-CONF-07 | S-04 |  |  | open |  |
| FR-CONF-08 | S-01 | domain/location.rs; adapters/env.rs; ports/env.rs | domain/location.rs; tests/component_location.rs | covered |  |
| FR-NAME-01 | S-01 | domain/naming.rs; domain/path.rs | domain/naming.rs | covered | Vectors provisional until user supplies reference pairs (D-02). |
| FR-NAME-02 | S-01 | domain/naming.rs | domain/naming.rs | open | Vectors provisional until user supplies reference pairs (D-02); stays open until those pairs are supplied. |
| FR-NAME-03 | S-09 |  |  | open | Integration (ignored) against isolated tmux server. |
| FR-NAME-04 | S-09 |  |  | open | Integration (ignored) against isolated tmux server. |
| FR-NAME-05 | S-09 |  |  | open | Integration (ignored) against isolated tmux server. |
| FR-SCOPE-01 | S-04 |  |  | open |  |
| FR-SCOPE-02 | S-04 |  |  | open |  |
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
| FR-CMD-08 | S-13 | src/messages.rs (cancelled) | src/main.rs (cancelled test); tests/messages_verbatim.rs | open | S-00 implements only the "Cancelled." text (messages::cancelled, report). The rest of FR-CMD-08 is S-13; status stays open until then. |
| FR-CMD-09 | S-17 |  |  | open | No runtime program besides tmux uses this message; helper only (proposal P-2). |
| FR-CMD-10 | S-17 |  |  | open |  |
| FR-CMD-11 | S-17 |  |  | open |  |
| FR-CMD-12 | S-13 |  |  | open |  |
| FR-LAUNCH-01 | S-12 |  |  | open |  |
| FR-LAUNCH-02 | S-12 |  |  | open | Argv via FakeProcess; GUI editor start is manual. |
| FR-LAUNCH-03 | S-12 |  |  | open | Detached spawn checked through FakeProcess argv. |

# ledge UI design

STATUS: PROPOSED (designer, slice D-01). Written 2026-10-24 on branch `slice/D-01-design`.
Scope: the terminal interface only. Behaviour stays as in `requirements.md`; the requirement changes
this design needs are in `docs/proposals.md` (P-58, P-59, P-60). Theme palette tables: `docs/design/themes.md`.

How to read the mock-ups: they show the target design. Section 1 lists what differs from the code today.
Each mock-up is the frame exactly as it renders in that many cells: `│` border, `▌` selection bar,
`(missing)` text cue. Reverse video and colour are not visible in plain text, so the cues that carry
meaning are all text.

---


## 0. Decision record

- 2026-10-24: the user asked for implementation of the design. Recommendations in section 8 are taken as
  accepted: P-58 (confirm destructive keys), P-59 (F1 panel, marker legend, footer fit, no-op messages),
  P-60 (`theme` and `theme_colors` config keys). Item 4 (pearl contrast): Kanso values kept, bold for the
  low-contrast states. Proposals stay Open in docs/proposals.md until the user moves them into requirements.md.

## 1. Problems today

What the user sees now, from `src/tui/render.rs`, `columns.rs`, `picker.rs`, `keys.rs` and the windows in
`src/app/pickers.rs` and `src/app/bookmark_cmd.rs`. Each point names the screen and the code behind it.

1. **Keys are learned by accident.** The footer is the only key help, and it is cut mid-phrase when the
   window is narrow (`render.rs`: `cut_end` on the footer). At 60 cells the bookmark picker footer reads
   `enter open · ^r rebuild · ^p profile · ^x kill · ^d delete …`, so the delete key is cut; at 50 cells
   the bookmark tree footer keeps only `alt-r/alt-p/a…`, and at 40 cells the project picker footer loses `^x kill`.
   The keys that matter most (kill, delete) are the ones cut first, and nothing says that more keys exist.
2. **Letter keys do nothing in the pickers that most users open.** In every picker with a filter (project,
   history, bookmark, switcher, both trees, profile) `j`, `k`, `h` and `l` are typed into the filter
   (`picker.rs` `apply`: `Type('j') if !has_filter()`). The footer does not mention `ctrl-j`/`ctrl-k`, the
   only letter moves there. The user types `j` to move, sees the filter change, and learns nothing.
3. **The selected row is hard to see and has holes.** Reverse video is put on the cell spans only; the gap
   between two cells is a plain space (`render.rs` `join` adds `Span::raw`), so the highlight is broken at
   every column boundary. Without colour (`NO_COLOR`, `TERM=dumb`) the reverse video is the only cue, and
   it is cut by those gaps.
4. **Missing directories are hidden without colour.** The `(missing)` text cue exists only for the project,
   history and bookmark pickers (`pickers.rs`: `missing_column` is set in `project_chrome`,
   `history_chrome`, `bookmark_chrome` and nowhere else). The session switcher, the session tree and the
   bookmark tree dim a missing row with the `DIM` style alone, and `DIM` is applied only when colour is on
   (`render.rs` `row_line`). In a colourless terminal a dead session looks like a live one (FR-TUI-08
   and FR-TUI-09 together).
5. **Long names lose the part that tells them apart.** Session names are the directory basename plus a
   four-character hash (FR-NAME-01). The Session column is cut at the end (`ColumnSpec::text`), so
   `api-gateway-91bc` and `api-gateway-4e0a` show as `api-gateway-…` when the column is narrow. The hash
   is the only distinguishing part.
6. **The Session column repeats the Project column.** In the project picker, `ledge` is followed by
   `ledge-3f2a`, and in the switcher the first two columns say the same thing with a suffix. Two of the
   four project picker columns carry little information on most rows, and the Profile column is empty for
   every project without a session.
7. **Matches are not shown on long rows.** When a row is wider than its column, `cell_spans` draws the whole
   cell plain ("highlight positions would no longer line up"). The row that matched is the one that loses
   its highlight, so the user cannot see why it is in the list.
8. **Markers have no legend.** The project picker draws `*` (current directory), `●` (live session) and
   `○` (none) in a 2-cell column. The footer does not explain them, and nothing on screen does. The edit
   window's `★` (bookmarked), `+` and `-` (folder collapsed and expanded) have no legend either.
9. **Destructive keys act at once.** `ctrl-x` kills a session, `ctrl-d` deletes a bookmark and kills its
   session, `alt-x` kills every session under a node and `alt-d` deletes every bookmark under a node
   (`pickers.rs` `kill_row`, `delete_bookmark`, `node_step(kill_node(...))`). None asks first. One key next to
   `^r` (rebuild) and `^p` (profile) ends a running shell in a session with no undo.
10. **Keys that do nothing say nothing.** `enter` on a gray connector row, `ctrl-d` on a project row
    (`dir_action` returns `no_action`), `ctrl-x` on a project with no session and `enter` on the edit
    window's project root all return with no status message. The user cannot tell the key was read.
11. **Trees are drawn by indentation only.** A tree row is two spaces per level in the first column
    (`render.rs` `cell_parts`: `"  ".repeat(depth)`), with no branch glyphs. At depth 3 the name starts at
    column 6, and a long name pushes the Session or Project cell out of view. The bookmark tree and the
    session tree cannot be collapsed (`TreeHandler::act` has no Left or Right case), so a project with many
    bookmarks is one long list. The edit window does collapse, with `+` and `-`, but the trees beside it do not.
12. **Gray connectors look disabled.** A connector row is drawn as `(name)` with gray parentheses. The
    parentheses are a text cue (good), but the row is not explained, and a click-through does nothing
    (point 10).
13. **Session tree placeholders are hard to scan.** A project with no running session is the row
    `acme-web (no session)` in the first column, the same column as session names. The text `(no session)`
    is part of the name, so the name column mixes two kinds of value.
14. **Empty windows keep their keys.** The switcher with no sessions shows `No ledge sessions.` in the
    list and still shows `enter switch · ^r rebuild · ^p profile · ^x kill`, all of which do nothing.
15. **Status line messages persist until the next action.** `set_status` is replaced on every `Continue`
    step, so a `Missing directory: ...` message stays through cursor movement and typing. It has no colour
    and no severity cue; it sits on the row above the footer, where it is cut at the end
    (`cut_end(status_text)`), which hides the path tail the user needs.
16. **The bookmark edit window has no filter.** It is the only window without a search (D-15), and its
    search bar (P-53, `/`) is not in its footer. Its footer names `→/^l open` and `←/^h close` as two keys
    each, which reads as four keys.

---

## 2. Principles

1. **The selected row is the most visible thing on screen.** It gets a left bar `▌` and reverse video across
   the whole row, gaps included, so it is visible without colour. Why: it is where Enter goes (FR-TUI-03).
2. **Every key in a footer works in that window, and a key means the same thing in every window.** Why:
   FR-TUI-07 and the footers listed in the requirements.
3. **A key that ends a running session asks first.** Why: a kill has no undo and the session holds running
   work. Until proposal P-58 is accepted, the current immediate behaviour stays (FR-ACT-01).
4. **Every state has a text cue and a colour.** Why: FR-TUI-09 requires the markers and labels to be readable
   without colour.
5. **The name comes first, the path second.** Why: the user thinks in project names; full paths are for
   the rare row that is not a project (FR-PICK-05, P-55, P-57).
6. **A key that does nothing says so in the status line.** Why: a silent key looks broken. The message text
   needs P-59.
7. **Each window shows its own help.** Why: the footer names the keys that work now, in the order a user
   needs them, and the full key list is one key away (P-59).

---

## 3. Layout system

### 3.1 The frame

Every picker has the same frame, top to bottom. Heights are for a picker with a filter (the prompt row is
present); the edit window has no prompt row unless its search bar is open.

| Row | Content | Style |
| --- | --- | --- |
| 1 | Border top, label `icon label` (FR-TUI-04) | `border`; icon and label in `accent` |
| 2 | Prompt `> filter`, cursor at the end (FR-TUI-02) | `text`; `> ` in `text_muted` |
| 3 | Column titles | `text_muted`, bold |
| 4..n-3 | List. Selection bar `▌` in column 0, the row in reverse video | see section 5 |
| n-2 | Status line: the last message, or empty | `error` or `warning` for errors and warnings, `text` otherwise |
| n-1 | Footer: the keys that work here | `text_muted`; each key in `text` |
| n | Border bottom | `border` |

The list height is the terminal height minus 6 with a prompt, minus 5 without (FR-TUI-05). The selection
bar takes column 0, so the cell widths are the inner width minus 1 and the gaps are unchanged.

### 3.2 Column rules

- **Primary column.** The first text column after the marker is the primary one: the project name or
  path in the project picker, the bookmark path in the bookmark picker, the session name in the switcher,
  the directory in the bookmark tree and the session in the session tree. It is the column the filter searches
  (FR-TUI-02) and the column the selection is read from.
- **Widths.** Keep the existing shrink order in `columns.rs` (A-06): last column first, then the search
  column, then the others from right to left. Marker is 2 cells and fixed. Gap is 1 cell.
- **Truncation.** Paths: cut from the left, keeping the tail (`…/projects/ledge`); home paths keep `~`
  (P-57). Session names: cut in the middle, keeping the start and the four-character hash (`api-gate…-91bc`).
  This is a new function `cut_middle` in `columns.rs`. Other text: cut at the end with `…`.
  Why: the hash is the only part that tells two sessions with one basename apart (FR-NAME-01).
- **Missing text.** A row whose directory is missing ends its primary cell with ` (missing)`, cut from the
  start of the path, so the cue is never lost (FR-TUI-08, FR-TUI-09). Every window that has a directory
  column sets `missing_column` to that column, including the switcher and both trees.
- **Empty states.** A window with no rows shows its message in `text_muted` in the list area and keeps the
  frame. The footer then shows only `esc close` (principle 2). The switcher and both trees stay open
  (FR-TUI-10, D-12).
- **Highlight after truncation.** A truncated cell keeps its match highlight: the highlighted characters are
  mapped into the visible part, and the cut is marked with `…`. Why: the user sees why a row matched.

### 3.3 Project picker

Label `Projects & Submodules` (`Projects (Profile: <name>)` with a profile, FR-PICK-13). Columns: marker,
Project, Session, Profile. Marker: `*` current directory, `●` live session, `○` none (FR-PICK-05, FR-PICK-07).
The legend for the markers is on the footer line when the footer has room: `* here ● live ○ none` is
added in front of the keys at 100 cells and wider (P-59).

80x24

```text
┌▸ Projects & Submodules ──────────────────────────────────────────────────────┐
│>                                                                             │
│    Project                 Session          Profile                          │
│▌*● ledge                   ledge-3f2a       dev                              │
│  ● api-gateway             api-gateway-91bc dev                              │
│  ● infra                   infra-7aa0       basic                            │
│  ○ docs-site                                                                 │
│  ○ acme-web                                                                  │
│  ○ notes-scratch                                                             │
│  ○ old-prototype (missing)                                                   │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter open · ^r rebuild · ^p profile · ^x kill                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌▸ Projects & Submodules ──────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│    Project                 Session          Profile                                                                  │
│▌*● ledge                   ledge-3f2a       dev                                                                      │
│  ● api-gateway             api-gateway-91bc dev                                                                      │
│  ● infra                   infra-7aa0       basic                                                                    │
│  ○ docs-site                                                                                                         │
│  ○ acme-web                                                                                                          │
│  ○ notes-scratch                                                                                                     │
│  ○ old-prototype (missing)                                                                                           │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter open · ^r rebuild · ^p profile · ^x kill                                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

The status line shows `Missing directory: ~/projects/old-prototype` after a missing row is chosen
(FR-PICK-09). The frame shows the empty status line here.

### 3.4 History picker

Label `History`, icon `↺`. Columns: Project (path, primary), Session, Profile. No marker column, because
history has no current-directory mark. Same keys as the project picker (FR-HIST-04).

80x24

```text
┌↺ History ────────────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Project                            Session          Profile                  │
│▌~/projects/ledge                   ledge-3f2a       dev                      │
│ ~/projects/api-gateway             api-gateway-91bc dev                      │
│ ~/projects/infra                   infra-7aa0       basic                    │
│ ~/projects/old-prototype (missing)                                           │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter open · ^r rebuild · ^p profile · ^x kill                                │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌↺ History ────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Project                            Session          Profile                                                          │
│▌~/projects/ledge                   ledge-3f2a       dev                                                              │
│ ~/projects/api-gateway             api-gateway-91bc dev                                                              │
│ ~/projects/infra                   infra-7aa0       basic                                                            │
│ ~/projects/old-prototype (missing)                                                                                   │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter open · ^r rebuild · ^p profile · ^x kill                                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.5 Session switcher

Label `Sessions`, icon `◉`. Columns: Session (primary), Project, Profile, Activity. The current session
is first and is marked `*` in front of its name, the same mark as the project picker uses for the current
directory (FR-SESS-02). `old-prototype (missing)` shows the missing cue in the Project column, because
the session has no directory that exists.

80x24

```text
┌◉ Sessions ───────────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Session            Project                 Profile Activity                  │
│▌*ledge-3f2a        ledge                   dev     just now                  │
│ api-gateway-91bc   api-gateway             dev     12m ago                   │
│ infra-7aa0         infra                   basic   2h ago                    │
│ old-prototype-0c4e old-prototype (missing)         3d ago                    │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter switch · ^r rebuild · ^p profile · ^x kill                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌◉ Sessions ───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Session            Project                 Profile Activity                                                          │
│▌*ledge-3f2a        ledge                   dev     just now                                                          │
│ api-gateway-91bc   api-gateway             dev     12m ago                                                           │
│ infra-7aa0         infra                   basic   2h ago                                                            │
│ old-prototype-0c4e old-prototype (missing)         3d ago                                                            │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter switch · ^r rebuild · ^p profile · ^x kill                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.6 Session tree

Label `Session tree`, icon `◉`. Columns: Session (tree column, primary), Project, Profile, Activity
(FR-SESS-06, P-54, P-55, P-56). Each session is a child of the session with the nearest ancestor
directory. A project with no running session is a gray row `name (no session)` with no activity.
Its children are listed under it. `(missing)` marks a session whose directory is gone.

80x24

```text
┌◉ Session tree ───────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Session               Project                 Profile Activity               │
│▌*ledge-3f2a           ledge                   dev     just now               │
│   ledge-docs-b9e3     ledge                           1h ago                 │
│ acme-web (no session) acme-web                                               │
│ old-prototype-0c4e    old-prototype (missing)         3d ago                 │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌◉ Session tree ───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Session               Project                 Profile Activity                                                       │
│▌*ledge-3f2a           ledge                   dev     just now                                                       │
│   ledge-docs-b9e3     ledge                           1h ago                                                         │
│ acme-web (no session) acme-web                                                                                       │
│ old-prototype-0c4e    old-prototype (missing)         3d ago                                                         │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter open · ^r/^p/^x selected · alt-r/alt-p/alt-x whole node                                                         │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.7 Bookmark picker

Label `Bookmarks`, icon `★`. Columns: Bookmark (path, primary), Session, Profile (FR-BM-11). Stored order.
Long bookmark paths are cut from the left so the tail, the folder the user wants, stays visible.

80x24

```text
┌★ Bookmarks ──────────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Bookmark                             Session          Profile                │
│▌~/projects/ledge/src                 ledge-3f2a       dev                    │
│ ~/projects/api-gateway/services/auth api-gateway-91bc dev                    │
│ ~/projects/infra/terraform (missing)                                         │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark           │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌★ Bookmarks ──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Bookmark                             Session          Profile                                                        │
│▌~/projects/ledge/src                 ledge-3f2a       dev                                                            │
│ ~/projects/api-gateway/services/auth api-gateway-91bc dev                                                            │
│ ~/projects/infra/terraform (missing)                                                                                 │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter open · ^r rebuild · ^p profile · ^x kill · ^d delete bookmark                                                   │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.8 Bookmark tree

Label `Bookmark tree`, icon `★`. Columns: Directory (tree column, primary), Session, Profile
(FR-TREE-01..05). Roots are projects with a bookmark. Gray `(name)` rows are connectors (FR-TREE-02). The
connector parentheses stay as the text cue (FR-TREE-11). The tree keeps the two-cell indent per level of
today; branch glyphs are open question 11. The `(missing)` cue is added to the Directory column (new, section 3.2).

80x24

```text
┌★ Bookmark tree ──────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Directory           Session          Profile                                 │
│ (ledge)                                                                      │
│▌  src               ledge-3f2a       dev                                     │
│ (api-gateway)                                                                │
│   (services)                                                                 │
│     auth            api-gateway-91bc dev                                     │
│ terraform (missing)                                                          │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node        │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌★ Bookmark tree ──────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Directory           Session          Profile                                                                         │
│ (ledge)                                                                                                              │
│▌  src               ledge-3f2a       dev                                                                             │
│ (api-gateway)                                                                                                        │
│   (services)                                                                                                         │
│     auth            api-gateway-91bc dev                                                                             │
│ terraform (missing)                                                                                                  │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter open · ^r/^p/^x/^d selected · alt-r/alt-p/alt-x/alt-d whole node                                                │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.9 Bookmark edit window

Label `Bookmarks in <project>` (FR-EDIT-01). Columns: marker (`★`, 2 cells, fixed), Directory (tree).
Folders show `+` collapsed and `-` expanded (FR-EDIT-04). The project root is `- <project>` and is always
expanded (P-38). No prompt row unless the search bar is open (P-53, FR-EDIT-13). The footer keys are in
section 4.

80x24

```text
┌★ Bookmarks in ledge ─────────────────────────────────────────────────────────┐
│    Directory                                                                 │
│    - ledge                                                                   │
│▌★    - src                                                                   │
│      - api-gateway                                                           │
│        + services                                                            │
│ ★          auth                                                              │
│      + docs                                                                  │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter toggle bookmark · →/^l open · ←/^h close · esc done                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌★ Bookmarks in ledge ─────────────────────────────────────────────────────────────────────────────────────────────────┐
│    Directory                                                                                                         │
│    - ledge                                                                                                           │
│▌★    - src                                                                                                           │
│      - api-gateway                                                                                                   │
│        + services                                                                                                    │
│ ★          auth                                                                                                      │
│      + docs                                                                                                          │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter toggle bookmark · →/^l open · ←/^h close · esc done                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.10 Profile picker

Label `Profiles`, icon `≡`. One column, Profile. Rows are profile names, with `[Current: <name>]` and
`[Default: <name>]` in front when they apply (FR-PROF-13). Filter on the name.

80x24

```text
┌≡ Profiles ───────────────────────────────────────────────────────────────────┐
│>                                                                             │
│ Profile                                                                      │
│▌[Current: dev] [Default: dev]                                                │
│ api                                                                          │
│ basic                                                                        │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│enter choose · esc cancel                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

120x36

```text
┌≡ Profiles ───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│>                                                                                                                     │
│ Profile                                                                                                              │
│▌[Current: dev] [Default: dev]                                                                                        │
│ api                                                                                                                  │
│ basic                                                                                                                │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│                                                                                                                      │
│enter choose · esc cancel                                                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.11 Doctor report

Not a picker: plain text on stdout, no frame, no key footer (FR-DOC-08). Each line is a tag padded to six
cells, then the check text. The tag is the text cue; `OK`, `WARN` and `ERR` also take the `success`,
`warning` and `error` tokens when colour is on. The exit code is 1 when any line is `ERR`.

80 columns, plain text

```text
OK    tmux 3.4
OK    setsid
WARN  code is not installed (needed for --code)
OK    config file ~/.config/ledge/config.json
OK    search path ~/projects
OK    default profile dev
WARN  bookmark ~/projects/infra/terraform is missing
OK    bookmarks (3)
OK    sessions (3)
WARN  session 'old-prototype-0c4e' directory is missing: ~/projects/old-prototype
```

---

## 4. Interaction

### 4.1 Key map

Keys are the same in every window where the action exists (FR-TUI-07). `^` is ctrl, `alt-` is alt.

| Key | Action | Project | History | Switcher | Session tree | Bookmark picker | Bookmark tree | Edit window | Profile |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| enter | open or choose | open | open | switch or attach | open the entry (not connector) | open | open (not connector) | toggle bookmark | choose |
| esc, ^c | cancel (edit window: close, or clear search bar) | cancel | cancel | cancel | cancel | cancel | cancel | close | cancel |
| up, down, PgUp, PgDn, Home, End | move | yes | yes | yes | yes | yes | yes | yes | yes |
| ^j, ^k | move (filter windows) | yes | yes | yes | yes | yes | yes | n/a | yes |
| j, k | move (no filter windows only) | n/a | n/a | n/a | n/a | n/a | n/a | yes | n/a |
| h, l, ←, → | collapse / move to parent, expand | n/a | n/a | n/a | n/a | n/a | n/a | yes | n/a |
| / | open search bar | n/a | n/a | n/a | n/a | n/a | n/a | yes | n/a |
| typed text | filter | yes | yes | yes | yes | yes | yes | search bar only | yes |
| ^r | rebuild the row's session | yes | yes | yes | yes | yes | yes | n/a | n/a |
| ^p | pick a profile, then rebuild | yes | yes | yes | yes | yes | yes | n/a | n/a |
| ^x | kill the row's session (confirm: P-58) | yes | yes | yes | yes | yes | yes | n/a | n/a |
| ^d | delete the bookmark and kill its session (confirm: P-58) | n/a | n/a | n/a | n/a | yes | yes | n/a | n/a |
| alt-r, alt-p | rebuild / pick profile for every session under the node | n/a | n/a | n/a | yes | n/a | yes | n/a | n/a |
| alt-x | kill every session under the node (confirm: P-58) | n/a | n/a | n/a | yes | n/a | yes | n/a | n/a |
| alt-d | delete every bookmark under the node (confirm: P-58) | n/a | n/a | n/a | n/a | n/a | yes | n/a | n/a |
| F1 | full key list in a panel (P-59) | yes | yes | yes | yes | yes | yes | yes | yes |

Why the alt keys stay: they are in requirements.md (FR-ACT-06..10) and the node is a visible selection.
Why `j`/`k` do not move in filter windows: a filter needs every letter (FR-TUI-02); `^j`/`^k` are the
letter moves there, and the footer does not say so, which is why P-59 adds F1.

### 4.2 Footers

The footer is built from key and label pairs, in this order: the Enter action, then the row keys, then the
node keys, then the window's own keys. The footer text is the same as the requirements (for example
`enter open · ^r rebuild · ^p profile · ^x kill`). When the footer is wider than the window, whole pairs are
dropped from the end and never cut mid-word; the dropped keys are in F1 (P-59). Why: a cut footer
shows a half key and hides the most destructive ones.

The edit window footer keeps its requirement text (`enter toggle bookmark · →/^l open · ←/^h close · esc done`)
until P-59 is accepted; the pairs `→/^l` and `←/^h` are one key each, shown as in the requirement.

### 4.3 Focus and selection

- One row is selected at a time. The selection starts on the preselected row (FR-PICK-04): the current
  directory in the project picker, the session the user is in in the switcher, the cursor row in the trees.
- Movement never wraps (A-28). Page movement is the list height minus one.
- After a filter edit, the selection goes to the best match (D-10). An empty filter keeps the row that was
  selected while filtering when it is still visible.
- Escape clears the search bar in the edit window and keeps the match on enter (FR-EDIT-13).

### 4.4 Confirmations

Proposal P-58 asks for a confirmation on `^x`, `^d`, `alt-x` and `alt-d`. The confirmation is one line in
the status row, with the target name and the keys:

```text
Kill session ledge-3f2a and its running processes? y yes · n or esc no
```

Only `y` confirms; `n`, `esc` and any other key cancel. Enter does not confirm, so a stray Enter cannot
kill. For a node action the line names the count: `Kill 4 sessions under ~/projects/clients? y yes · n or esc no`.
Until P-58 is accepted, these keys act at once, as FR-ACT-01 and FR-BM-13 say.

### 4.5 Status messages

- Shown in the status row, above the footer.
- Errors (for example `could not kill session 'ledge-3f2a': ...`) use the `error` token and bold. Warnings
  use `warning`. Information (`Killed session '...'.`) uses `text`.
- A message stays until the next action that changes the status. Cursor movement and typing keep it, as
  today. It is not timed, so no message disappears while the user is reading it.
- A message longer than the row is cut at the end. Paths in messages use `~` (P-57), so most fit. Message
  text is exact per requirements.md, so no message is reworded for the screen.
- A key with no effect on this row sets a message (principle 6, P-59).

### 4.6 Session and bookmark actions in the tree

Only the selected row's session is acted on by `^r`, `^p`, `^x` (FR-TUI-07 and architecture 6.3). The node
keys act on every managed session under the selected node and never on the session the user is in
(FR-ACT-07). A connector row has no session and no bookmark, so the ctrl keys do nothing on it and say so
in the status row (principle 6).

---

## 5. Visual states

Every state has a text cue and a colour token. Without colour only the text cue and the selection bar and
reverse video remain (section 6.4).

| State | Where | Text cue | Token | Without colour |
| --- | --- | --- | --- | --- |
| Selected | every list row | `▌` in column 0, reverse video over the whole row | `selection_bg`, `selection_fg` | `▌` and reverse video |
| Dimmed, directory missing | every list row with a directory | ` (missing)` at the end of the primary cell | `missing` | the text cue |
| No session | project picker, bookmark rows, session tree placeholder | empty session cell, or `(no session)` in the session tree | `session_none` | the text cue |
| Current session | session switcher and session tree | `*` before the session name | `accent` | the `*` |
| Current directory | project picker | `*` in the marker column | `accent` | the `*` |
| Live session | project picker marker, switcher | `●` in the marker column (picker); session name present | `session_active` | the `●` |
| Connector (gray tree entry) | bookmark tree, session tree | `(name)` with parentheses | `connector` | the parentheses |
| Bookmarked | edit window | `★` in column 0 | `accent` | the `★` |
| Folder with subfolders | edit window | `+` collapsed, `-` expanded | `text` | the sign |
| Error | status row | text as the requirement gives it | `error`, bold | text |
| Warning | status row, doctor | text, or `WARN` tag | `warning` | text or the tag |
| Doctor OK / WARN / ERR | doctor report | `OK`, `WARN`, `ERR` tag | `success`, `warning`, `error` | the tag |
| Empty window | any list | the message in `text_muted` | `text_muted` | the text |

Why one cue per state: FR-TUI-09 says markers and labels must read without colour, and the table makes
each state's cue a character, not a colour.

---

## 6. Theming

### 6.1 Tokens

A theme is a set of named semantic tokens. Code asks for a token; it never names a colour. The tokens
are:

| Token | Used for |
| --- | --- |
| `background` | the window background (dark themes set it; `default` leaves the terminal's) |
| `text` | list text, prompt text, footer key names |
| `text_muted` | column titles, footer labels, empty-state messages, `> ` prompt |
| `accent` | border icon and label, `▌` bar, `*`, `★`, matched characters |
| `border` | frame lines |
| `selection_bg`, `selection_fg` | the selected row |
| `success` | `OK` tag, `●` marker |
| `warning` | `WARN` tag, warnings in the status row |
| `error` | `ERR` tag, errors in the status row |
| `session_active` | live session name in the switcher and session tree |
| `session_none` | `(no session)` placeholder, empty session cell |
| `path` | directory and bookmark paths |
| `missing` | the `(missing)` cue and the directory of a missing row |
| `connector` | gray tree connector rows |

Values for every built-in theme, with contrast ratios and 256-colour indexes, are in `docs/design/themes.md`.

### 6.2 Built-in themes

| Name | Kind | Source of the values |
| --- | --- | --- |
| `default` | dark or light, the terminal decides | named ANSI colours, so the user's terminal palette applies |
| `ink` | dark | Kanso ink, verified (section 10, sources 10b and 10c) |
| `mist` | dark | Kanso mist, verified |
| `zen` | dark | Kanso zen, verified |
| `pearl` | light | Kanso pearl, verified |

`default` uses the ANSI names: `Cyan` for `accent`, `Green`, `Yellow` and `Red` for the three states,
`DarkGray` for `text_muted`, `missing`, `session_none` and `connector`, and the terminal's own foreground
and background for `text` and `background`. Why: the user's own palette keeps the picker readable in any
terminal, and the 16 ANSI names are the most widely supported colours.

### 6.3 Choosing a theme and overriding a token

The theme is set in `config.json`, beside `search_paths` and `default_profile` (FR-CONF-02). Proposal P-60
adds the two keys. An example:

```json
{
  "search_paths": ["~/projects"],
  "theme": "ink",
  "theme_colors": {
    "accent": "#7fb4ca",
    "selection_bg": "#2a2c35"
  }
}
```

- `theme`: one of `default`, `ink`, `mist`, `zen`, `pearl`. Missing means `default`.
- `theme_colors`: token name to hex value (`#rrggbb`). Each entry replaces one token of the chosen theme;
  the rest keep the theme's values. An unknown token name or a value that is not `#rrggbb` is ignored with
  a warning on stderr: `Warning: theme_colors '<name>' is not a token; ignored.` (P-60 adds the text).
- Why a config key and not an environment variable: the config is already the user's one place for
  settings, and FR-CONF-02 reads it at every start. An environment variable for a single run can follow
  later (section 8, question 3).

Hex values in `theme_colors` are used as truecolor. On a 256-colour terminal the nearest index is used, and
on a 16-colour terminal the token falls back to the `default` ANSI name for its role.

### 6.4 Colour off

Colour is off when any of these holds:

- `NO_COLOR` is set and not empty (no-color.org: "when present and not an empty string (regardless of its
  value), prevents the addition of ANSI color").
- `TERM` is `dumb`.
- stdout is not a terminal (the picker cannot run without one; the implementer checks what the open flow
  does then).

`render::color_enabled` already implements the first two (FR-TUI-09). With colour off, every token is
ignored, the selection keeps its `▌` and reverse video, and every state keeps its text cue (section 5).
Why: colour is never the only signal.

---

## 7. Terminal support

- **Minimum size.** The picker draws at 40x8 (A-06, `MIN_WIDTH` and `MIN_HEIGHT` in `render.rs`); below it
  the message `Terminal too small: need at least 40x8.` is the only line. That message is in the Messages
  section and stays. Recommended size is 80x24. At 80x24 every footer in section 3 fits, and the
  mock-ups in section 3 are all 80 cells wide.
- **Colour levels.** `COLORTERM` `truecolor` or `24bit` gives truecolor. A `TERM` with `256color` gives the
  256-colour indexes. Anything else gives the 16 ANSI names of `default`. The code I read does not detect
  these levels (implementation note 1).
- **Box drawing.** The frame uses `┌ ─ ┐ │ └ ┘`. The icons (`▸ ↺ ◉ ★ ≡`) and
  the markers (`▌ ● ○ ★`) are single-cell characters in the common fonts. The `…` cut mark is one cell.
- **ASCII fallback.** When the locale is not UTF-8 (`LANG` and `LC_ALL` without `UTF-8`), the frame uses
  `+ - |`, `>` replaces `▌`, `...` replaces `…`, and the icons are dropped
  (the label alone). This is not in the code I read; section 8, question 5.
- **Resize.** The layout is recomputed on every draw (FR-TUI-05). The selection stays visible.

---

## 8. Open questions

1. **Confirm destructive keys (P-58).** Recommendation: yes, a one-line `y`/`n` confirmation on `^x`,
   `^d`, `alt-x` and `alt-d` (section 4.4). Why: one key ends a running session with no undo. Cost: one
   keystroke more for a kill the user meant.
2. **F1 key panel and marker legend (P-59).** Recommendation: yes. F1 opens a panel listing every key of
   the window and the markers. The footer keeps the short form. Why: the footer cannot hold every key
   at 80 cells, and the current footer hides the destructive keys first.
3. **Theme keys in config (P-60).** Recommendation: `theme` and `theme_colors` in `config.json`, as in
   section 6.3. Alternative: an environment variable `LEDGE_THEME` for one run. Recommendation is the
   config key only, for now.
4. **Pearl text contrast.** Four pearl tokens used for text fall below the 4.5:1 that is usual for text:
   `success` (`pearlGreen`, 3.47:1), `path` (`pearlTeal1`, 3.32:1), `error` (`pearlRed`, 4.31:1) and
   `warning` (`pearlYellow`, 4.41:1). The dark Kanso themes fail for `error` (`red`, 2.99 to 3.82:1). Recommendation:
   keep the Kanso values (they are the theme), draw these states in bold, and rely on the text cues.
   Alternative: the user picks other values. The user decides.
5. **ASCII fallback for non-UTF-8 locales.** Recommendation: yes, as in section 7. Cost: a second set of
   glyphs in `render.rs`.
6. **Minimum size.** Recommendation: keep 40x8 and the existing message; state 80x24 as the supported size
   in the README. A change of the minimum needs its message text in P-59 or a new proposal.
7. **Session names in the switcher.** Recommendation: keep the full session name with the hash, cut in the
   middle (section 3.2). Alternative: a short name without the hash. Rejected for now: FR-PICK-06 and
   FR-SESS-03 name the session, and the hash is how two sessions are told apart.
8. **Collapsible bookmark and session trees.** Recommendation: no. FR-TREE-08 shows all trees, and the tree
   fits a 24-row screen for a normal number of bookmarks. The edit window stays the place to collapse folders.
9. **Status message timing.** Recommendation: no timer. A message stays until the next action that changes
   it (section 4.5). Why: a timed message disappears while the user is reading the path it names.
10. **Current-session mark.** Recommendation: `*` before the session name (section 3.5), the same mark as
    the project picker. Alternative: a separate marker column. Rejected: it costs two cells on every row.
11. **Branch glyphs in trees.** Recommendation: yes, `├─`, `└─` and `│` in place of the two-space indent
    in the bookmark and session trees, two cells per level. Why: depth is hard to read at a glance with
    the indent alone (section 1, point 11). Cost: the name column gets two cells narrower per level. The
    mock-ups in section 3 show the indent only until this is decided.

Eleven open questions; each has a recommendation above.

---

## 9. Implementation notes

Order of work. Each step keeps the requirements behaviour (section 9.3).

1. **`src/tui/theme.rs` (new).** A `Theme` struct with the tokens of section 6.1, the built-ins, and the
   `theme_colors` override with the colour-level mapping. Pure functions, unit tests for every built-in and
   for an override. No new dependency. Architecture 6.2 needs a line for this module (architect).
2. **`src/tui/render.rs`.**
   a. Selection bar `▌` in column 0 and reverse video across the whole row, including the gaps (`join`
      gives the gap spans the row's style). Inner widths become `inner - 1` for the rows.
   b. Colours from the theme's tokens; `color_enabled` stays as it is.
   c. Footer: pairs dropped from the end when too wide (new function in `columns.rs`, section 4.2).
   d. Status row: error and warning tokens, bold for errors.
   e. Highlight kept on truncated cells (section 3.2).
3. **`src/tui/columns.rs`.** `cut_middle` for session names; a `fit_pairs` helper for the footer; tests
   for both, with the CJK cases the file already has.
4. **`src/app/pickers.rs`.**
   a. `switcher_chrome`: `missing_column = Some(1)`.
   b. `tree_chrome`: `missing_column = Some(0)` for the bookmark tree and `Some(1)` for the session tree.
   c. `switcher_rows` and `tree_item`: the `*` prefix on the current session's name.
   d. Empty switcher and tree: footer `esc close` (a second footer constant in `messages.rs`, P-59).
   e. No-op messages for `^x`, `^d`, `^r`, `^p` on a row with no session, and `enter` on a connector
      (P-59 text).
   f. Confirmation state for the four destructive keys (P-58). Until P-58 is accepted, do not add it.
5. **`src/app/bookmark_cmd.rs`.** The edit window footer stays. The `/` search bar is already in
   `render.rs`; the bar's prompt `/` is `messages::edit_search_prompt`.
6. **`src/app/store.rs` and `src/domain/config.rs`.** Read `theme` and `theme_colors` (P-60). Unknown or
   invalid values warn and use `default`.
7. **Tests.** `tests/tui_engine.rs` with `TestBackend`: the selection bar and gap highlight, the `(missing)`
   cue in each window with colour off, and the footer dropping whole pairs at 60 cells.

Must not change:

- Every message text in `messages.rs` and in `requirements.md` "Messages" (exact wording). New texts only
  through P-59 and P-60.
- The key map in `keys.rs` until P-58 and P-59 are accepted, and then only the keys they name.
- Row order and tree rules (FR-PICK-03, FR-SESS-02, FR-TREE-01..08, P-54, P-56).
- Which rows are openable, and which keys act on which rows (FR-ACT-06..11, FR-TREE-11).
- Exit codes and the `Cancelled.` behaviour (FR-CMD-07, FR-CMD-08).
- The 40x8 minimum and its message (A-06).

---

## 10. Research

### 10.1 Terminal tools

What was read, and what the sources say. Where a source was not read, the row says so.

| Tool | Read | Density and layout | Selected row, filter, footer | Status and errors | Trees | Colour-free | Themes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| fzf | 10k | Fullscreen by default; `--layout reverse`, `--height`, `--border`, `--preview` with `--preview-window` (position and size) | `--prompt` and `--header` in examples; no footer option in the README; match count is `$FZF_MATCH_COUNT` | not in the README | none | not mentioned in the README | `--color` takes `key:value` pairs (`fg`, `fg+`, `bg`, `border`, `label`, `header-border` and others) |
| lazygit | 10d | a bottom line with keybinding help (`showBottomLine`, default true); panel layout not read | the bottom line is the key help; `optionsTextColor` colours it | a loader is shown in the bottom line during an action | not read | not in the doc | `gui.theme` keys (`activeBorderColor`, `selectedLineBgColor`, `selectedLineFgColor`); values are colour names, hex, or modifiers; `gui.darkTheme` and `gui.lightTheme` override fields; no named built-in themes in the doc |
| gitui | 10e | not read | keys are configured per action in `key_bindings.ron`; the page does not say whether key hints are shown | not read | not read | not read | `theme.ron` not documented on the page read |
| yazi | 10f | not read (theme file only) | selection style `select_main` in `[mode]` | sections `[notify]` and `[confirm]` exist; their use not read | not read | not read | `theme-dark.toml` preset; sections `[mgr]`, `[status]`, `[which]`, `[input]`, `[mode]`, `[pick]`; named colours and inline style tables (`{ fg = "cyan" }`, `{ bg = "red", bold = true }`) |
| zellij | 10g | session manager is a plugin | not read | not read | sessions as a list | not read | `default.kdl`: named bundled themes `default`, `default-light`, `classic`; a custom theme is a `themes` block with `fg`, `bg`, `red`, `green`, `yellow`, `blue`, `magenta`, `orange`, `cyan`, `black`, `white`, RGB values; the file does not say whether omitted keys inherit |
| tmux choose-tree | 10h | one line per session, window or pane, with a shortcut key in brackets; `-s` and `-w` start collapsed; `+`/`-` expand and collapse | `Enter` chooses; `x` kills, `X` kills tagged; `C-s` searches by name; `f` filter, `O` sort; `v` preview | not read | expand and collapse per item and for all (`M-+`, `M--`) | not read | not themable in the man page |
| k9s | 10i | header, table, footer hints (the README does not say) | not read | not read | none | the README says nothing on colour-free output | YAML skins under `k9s:` with `body`, `frame.border`, `frame.menu`, `views.table` keys; selected in `ui.skin` or `K9S_SKIN` |
| television | not read (docs site returned 404; README has no theme section) | not read | not read | not read | not read | not read | not verified |
| bottom | not read (theme docs path returned 404) | not read | not read | not read | not read | not read | not verified |

Findings that shape the design:

- lazygit shows the keys of the current window in one bottom line (verified in 10d). ledge does the same and
  adds F1 for the full list (P-59). The footers of the other tools were not verified.
- Themes are named colour roles in one file with an optional override block (lazygit `gui.theme`, zellij
  `themes`, yazi `[mode]`). ledge takes the token approach (section 6) because the same tokens also drive
  the 256-colour fallback.
- tmux choose-tree gives its tree the `x` and `X` pair (one row, all tagged rows). The man page lists no
  confirmation for `x`. That is the pattern P-58 changes for ledge.
- The sources do not say how these tools behave with `NO_COLOR`, except the no-color.org spec (10j). The
  design therefore does not copy any tool's colour-free mode; it sets its own rule (section 6.4).

### 10.2 Kanso

Kanso is a theme family for Neovim with four variants, Zen, Ink, Mist and Pearl (10a). The palette names
and values are in `lua/kanso/colors.lua` (10b). The editor background, foreground, visual selection,
cursor line and comment colour of each variant are in `lua/kanso/themes.lua` (10c), and the values in
section 6.2 and `docs/design/themes.md` come from those two files.

What was verified: the palette lines quoted in `docs/design/themes.md`, read from the raw file on 2026-10-24 through WebFetch. The
values are quoted by the fetch tool, not checked against a local clone, and the commit was not pinned.
What was not verified: the 256-colour indexes are not Kanso's; they are the nearest xterm 256 colour to each
hex, computed in `docs/design/themes.md`.

### 10.3 Source list

- 10a. https://github.com/webhooked/kanso.nvim (README: four variants, no hex values in the README)
- 10b. https://raw.githubusercontent.com/webhooked/kanso.nvim/main/lua/kanso/colors.lua (palette)
- 10c. https://raw.githubusercontent.com/webhooked/kanso.nvim/main/lua/kanso/themes.lua (variant mapping)
- 10d. https://raw.githubusercontent.com/jesseduffield/lazygit/master/docs/Config.md (lazygit config)
- 10e. https://raw.githubusercontent.com/gitui-org/gitui/master/KEY_CONFIG.md (gitui keys)
- 10f. https://raw.githubusercontent.com/sxyazi/yazi/main/yazi-config/preset/theme-dark.toml (yazi theme)
- 10g. https://raw.githubusercontent.com/zellij-org/zellij/main/zellij-utils/assets/config/default.kdl (zellij defaults)
- 10h. https://raw.githubusercontent.com/tmux/tmux/master/tmux.1 (tmux manual, choose-tree)
- 10i. https://github.com/derailed/k9s (README: skins)
- 10j. https://no-color.org/ (NO_COLOR)
- 10k. https://raw.githubusercontent.com/junegunn/fzf/master/README.md (fzf)
- Not read: television (https://alexpasmantier.github.io/television/docs/Users/themes/ returned 404; the README at
  https://github.com/alexpasmantier/television has no theme section), bottom (theme page returned 404).

Sources cited: 11 (10a to 10k). Each was read through WebFetch on 2026-10-24; the page text was summarised by the fetch tool, so a value
that matters for an implementation should be checked against the page before it is copied.

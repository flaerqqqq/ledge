# ledge bookmark edit window: three options

STATUS: PROPOSED (designer, OPTIONS mode). Written 2026-10-24 on branch `main`. Scope: the bookmark edit window only
(`ledge bm edit`, FR-EDIT-01..13, `src/app/bookmark_cmd.rs` edit rows, `src/tui/render.rs`). No code changed, no
requirement changed. The window keeps the visual system of the framed design (`docs/design.md` sections 3.1, 3.2 and
6; `docs/design-options.md` option B) with the zen palette (`docs/design/themes.md`, `src/tui/theme.rs`).

Mock-ups are the frame exactly as it renders in that many cells. Colour and reverse video are not visible in plain
text: the selected row is `api-gateway` in every mock-up, drawn in reverse video in the window, and the cues that carry
meaning (`★`, `+`, `-`, `live`, `none`) are text.

---

## 1. What the window shows today

Problems, from `src/app/bookmark_cmd.rs` (`edit_chrome`, `edit_row`) and `src/tui/render.rs` (`draw`):

1. **One data column.** The window has a marker column (2 cells, holds `★`) and a tree column. The header names only
   the tree column (`Directory`). Nothing else is on the row, so an 80x24 window shows a thin tree in the top corner
   and the rest is empty.
2. **No detail for the selected folder.** The shared renderer draws a Details section, but `edit_row` never sets
   `details`, so `shows_details` is false and nothing is drawn at 120 cells either.
3. **An empty status row.** Toggling a bookmark or expanding a folder sets no status (`Step::Continue { status: None }`),
   so the row between the list and the footer is always blank. Why it matters: the user cannot see that a change was
   saved, or how many bookmarks the project has.
4. **The bookmark mark sits before the name.** The `+` and `-` already follow the name (S-41). The `★` is still in
   column 0; section 2.3 moves it after the name, as the brief asks (open question 1).
5. **The session state is not shown.** A folder with a live session looks the same as one without (the window does not
   load the session list today).

Footer today: `enter toggle bookmark · →/^l open · ←/^h close · esc done`. The new footer (section 2.2) adds `/ search`,
which is already in the window (FR-EDIT-13).

---

## 2. Shared rules for the three options

### 2.1 Fixture used in every mock-up

Project `ledge`, `ledge` root expanded, `src` and `docs` expanded, `api-gateway` expanded, `services` collapsed. Bookmarks:
`src`, `auth`, `guides`, and `ci` (inside the collapsed `services`), so the project has 4 bookmarks. Live sessions:
`ledge-3f2a` on the project root and `api-gateway-91bc` on `api-gateway`. The list has 12 visible rows. The selected
row is `api-gateway`. Subfolder counts: ledge 4, src 3, api-gateway 2, services 1, docs 2, all others 0.

### 2.2 Frame, header, status and footer (all options)

- The frame is the one of every other window: the top edge carries ` Bookmarks in ledge ` (FR-EDIT-01), bold in `text`.
  The mock-ups draw corners as `┌ ┐ └ ┘`; the window draws the rounded corners (`BorderType::Rounded`).
- No prompt row (D-15). The `/` search bar appears only when it is open (FR-EDIT-13), above the header, as now.
- Header row on the `surface` band, titles in `text_muted`, bold. A `border` rule runs under it.
- Status row above the bottom edge on the `surface` band: the count on the left in `text_muted`, the last message on the
  right (`error` for a failed change, bold; `text_muted` otherwise). Why: the window shows its state without a key press
  (principle 4 in `docs/design.md`).
- Footer in the bottom edge, keys in `text` bold and labels in `text_muted`:

  `enter toggle bookmark · →/l open · ←/h close · / search · esc done`

  Every key exists: enter toggles (FR-EDIT-02), right and l expand (FR-EDIT-03), left and h collapse or move to the parent
  (D-15), `/` opens the search bar (FR-EDIT-13), esc closes (design 4.1). The footer is 66 cells, so it fits at 80.
  Narrower, whole pairs drop from the end (`docs/design.md` 4.2). The requirement text `→/^l` and `←/^h` is replaced by
  `→/l` and `←/h` as the user named them; the F1 panel (P-59) lists the rest.

### 2.3 Markers and text cues (all options)

- `★` (bookmarked): `accent`. In options A and C it follows the name: `src - ★`. In option B it is in its own column.
  Without UTF-8 it becomes `b`. The `*` marks stay removed (S-41) and the selection bar `▌` is not used.
- `+` and `-` (FR-EDIT-04) follow the name after one space: `-` expanded, `+` collapsed, nothing for a folder without
  subfolders. `text_muted`.
- Branch glyphs `├─`, `└─` and `│` come from `render::tree_guides` (P-61), `text_muted`, with the same ASCII fallback
  as today.
- Selected row: `selection_bg` and `selection_fg` across the whole inner width, bold text, no bar. The row keeps the
  reverse video when the theme sets no selection colour, as now.
- Colour off (`NO_COLOR`, `TERM=dumb`): every cue above is text, so nothing is lost (FR-TUI-09).

### 2.4 Palette: zen (unchanged)

The values are the `zen` theme of `src/tui/theme.rs` and `docs/design/themes.md` (Kanso zen, S-42). The mist values in
`docs/design-options.md` 2.3 are not used here.

| Token | Hex | 256 | Used in this window for |
| --- | --- | --- | --- |
| background | #090E13 | 233 | the window background |
| surface | #1C1E25 | 234 | header row and status row (and the path strip in option C) |
| text | #C5C9C7 | 251 | folder names, values, footer keys, the top-edge label |
| text_muted | #909398 | 246 | column titles, `+` and `-`, connectors, fact labels, the count, footer labels, `none` |
| accent | #7FB4CA | 110 | `★` |
| border | #393B44 | 237 | frame lines and rules |
| selection_bg | #393B44 | 237 | the selected row (text on it: selection_fg #C5C9C7, 6.67:1) |
| success, session_active | #98BB6C | 107 | the `live` session state |
| warning | #DCA561 | 179 | warnings in the status row |
| error | #E46876 | 168 | failed changes in the status row |
| path | #8ea4a2 | 247 | paths |
| session_none | #909398 | 246 | `none` |

---

## 3. Option A: Tree plus details

**Idea.** The tree keeps its place and the selected folder gets a details block with four facts: its path, whether it
is bookmarked, how many subfolders it has, and its managed session (name, profile, live state). At 80 cells the block sits
under the list, as the Details section does in the other windows. At 120 cells the tree moves to the left and the
details to the right. It suits a user who acts on one folder at a time and wants to know what the folder is before
toggling it.

### 3.1 Mock-ups

80x24

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────┐
│ Folder                                                                       │
├──────────────────────────────────────────────────────────────────────────────┤
│ ledge -                                                                      │
│ ├─ src - ★                                                                   │
│ │  ├─ api-gateway -                                                          │
│ │  │  ├─ routes                                                              │
│ │  │  └─ services +                                                          │
│ │  ├─ auth ★                                                                 │
│ │  └─ tui                                                                    │
│ ├─ docs -                                                                    │
│ │  ├─ adr                                                                    │
│ │  └─ guides ★                                                               │
│ ├─ tests                                                                     │
│ └─ notes                                                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│ Details · api-gateway                                                        │
│ Path        ~/dev/projects/ledge/src/api-gateway                             │
│ Bookmarked  no   (enter adds a bookmark)                                     │
│ Subfolders  2   routes, services                                             │
│ Session     api-gateway-91bc · profile dev · live                            │
├──────────────────────────────────────────────────────────────────────────────┤
│ 12 folders                                                                   │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────┘
```

Caption: the header row names the tree (`Folder`). The details block is 7 rows: a rule, the title `Details · <name>`
in `text` bold, four facts, and a closing rule. The list has 12 rows, so the whole fixture shows; a longer tree scrolls.

120x36

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Folder                                                   │ Details · api-gateway                                     │
├──────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────┤
│ ledge -                                                  │ Path        ~/dev/projects/ledge/src/api-gateway          │
│ ├─ src - ★                                               │ Bookmarked  no   (enter adds a bookmark)                  │
│ │  ├─ api-gateway -                                      │ Subfolders  2   routes, services                          │
│ │  │  ├─ routes                                          │ Session     api-gateway-91bc · profile dev · live         │
│ │  │  └─ services +                                      │                                                           │
│ │  ├─ auth ★                                             │                                                           │
│ │  └─ tui                                                │                                                           │
│ ├─ docs -                                                │                                                           │
│ │  ├─ adr                                                │                                                           │
│ │  └─ guides ★                                           │                                                           │
│ ├─ tests                                                 │                                                           │
│ └─ notes                                                 │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│                                                          │                                                           │
│ 12 folders                                                                                                           │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────────────────────────────────────────────┘
```

Caption: the header band is split: `Folder` over the tree, `Details · <name>` over the facts, with `┼` where the rule
crosses the divider. The tree is 58 cells, the facts 59. The list rows and the facts share the same body rows.

### 3.2 Palette, option A

- Fact labels (`Path`, `Bookmarked`, `Subfolders`, `Session`) `text_muted`, 12 cells wide. Values `text`.
- The path value in `path`. `no` in `text` and the hint in parentheses in `text_muted`. `live` in `session_active`, `none`
  in `session_none`.
- The details title `Details · <name>` in `text`, bold. The divider and the rules in `border`.

### 3.3 Row and header rules, option A

- Tree rows: one margin cell, the tree prefix, the name, then ` -` or ` +` if the folder has subfolders, then ` ★` if it is
  bookmarked. Names cut at the end with `…`.
- Facts: label padded to 12 cells, then the value. Paths keep the tail and cut from the left (`…/src/api-gateway`),
  home paths keep `~` (P-57). A session value reads `<name> · profile <profile> · live` or `none`.
- The details block shows the selected row only. For a folder that is not bookmarked the `Bookmarked` line reads
  `no   (enter adds a bookmark)`.
- At 80 cells the details block sits under the list; at 120 cells and up the split is used. Why: 80x24 is the supported
  size (`docs/design.md` 7), so the facts must show there too.

### 3.4 Keys footer, option A

The shared footer (section 2.2). Nothing new is needed: the details block shows facts, not keys.

### 3.5 Trade-offs, option A

- For: the selected folder is fully described. The other facts need no scanning.
- Against: only one row is described. The other eleven rows still show a name and nothing else.
- Against: at 80x24 the block takes 7 of the 24 rows, so the list is 12 rows high.
- Against: the 120 split is a layout the other windows do not use. They put the Details section under a full-width list
  (`docs/design.md` 3.3). This window would differ.
- Cost: the window must load the session list (the project picker already does, `live_session` in `src/app/pickers.rs`).

### 3.6 Files that change, option A

- `src/tui/picker.rs`: `Details` has fixed fields (`path`, `session`, `windows`, `profile`). It must take labelled facts,
  so the window can show `Bookmarked` and `Subfolders`. `shows_details` takes a flag that allows 80 cells for this window.
- `src/tui/render.rs`: `draw_details` takes its label list from the model; `draw` gets a split layout for this window at
  120 cells (tree left, details right, `┼` in the rule). `DETAILS_ROWS` stays 7.
- `src/app/bookmark_cmd.rs`: `edit_row` fills `details` (path, bookmarked, subfolder count, session); `edit_chrome`
  sets the flag and the split.
- `src/domain/bm_edit.rs`: `EditRow` carries `subfolders: usize`, counted from the same `children` closure that
  `visible_rows` uses.
- `src/app/pickers.rs`: `live_session` and `session_fact` are shared with `bookmark_cmd.rs`.
- `src/messages.rs`: the fact labels and the text `enter adds a bookmark`.
- `tests/tui_edit_window.rs`, `tests/tui_engine.rs`: 80x24 and 120x36 layouts with `TestBackend`.
- `docs/design.md` section 3.9, when the option is accepted.

---

## 4. Option B: Tree with columns

**Idea.** Each folder is a table row. The tree and the name stay in the first column, and the bookmark, the session state and
the child count sit in their own columns to the right, under a header row on the surface band. At 120 cells a Path column
is added at the end. It is the table of the project picker and the switcher, with the tree as the primary column, so the
three windows read as one family. It suits a user who scans a project for bookmarks and live sessions.

### 4.1 Mock-ups

80x24

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────┐
│ Folder                             Bookmark   Session      Subfolders        │
├──────────────────────────────────────────────────────────────────────────────┤
│ ledge -                                       live                  4        │
│ ├─ src -                           ★          none                  3        │
│ │  ├─ api-gateway -                           live                  2        │
│ │  │  ├─ routes                               none                  0        │
│ │  │  └─ services +                           none                  1        │
│ │  ├─ auth                         ★          none                  0        │
│ │  └─ tui                                     none                  0        │
│ ├─ docs -                                     none                  2        │
│ │  ├─ adr                                     none                  0        │
│ │  └─ guides                       ★          none                  0        │
│ ├─ tests                                      none                  0        │
│ └─ notes                                      none                  0        │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│ 12 folders                                                                   │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────┘
```

Caption: four columns. `Folder` (34 cells, tree, name, `+` or `-`), `Bookmark` (10 cells, `★` or blank), `Session`
(12 cells, `live` or `none`), `Subfolders` (10 cells, right-aligned count). The empty rows below the twelfth are the list
area; the window has no filler text.

120x36

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Folder                             Bookmark   Session                Subfolders Path                                 │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ledge -                                       ledge-3f2a live                 4 ~/dev/projects/ledge                 │
│ ├─ src -                           ★          none                            3 ~/dev/projects/ledge/src             │
│ │  ├─ api-gateway -                           api-gateway-91bc live           2 ~/dev/projects/ledge/src/api-gateway │
│ │  │  ├─ routes                               none                            0 …ojects/ledge/src/api-gateway/routes │
│ │  │  └─ services +                           none                            1 …ects/ledge/src/api-gateway/services │
│ │  ├─ auth                         ★          none                            0 ~/dev/projects/ledge/src/auth        │
│ │  └─ tui                                     none                            0 ~/dev/projects/ledge/src/tui         │
│ ├─ docs -                                     none                            2 ~/dev/projects/ledge/docs            │
│ │  ├─ adr                                     none                            0 ~/dev/projects/ledge/docs/adr        │
│ │  └─ guides                       ★          none                            0 ~/dev/projects/ledge/docs/guides     │
│ ├─ tests                                      none                            0 ~/dev/projects/ledge/tests           │
│ └─ notes                                      none                            0 ~/dev/projects/ledge/notes           │
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
│ 12 folders                                                                                                           │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────────────────────────────────────────────┘
```

Caption: `Session` widens to 22 cells and shows the session name with its state (`api-gateway-91bc live`). `Path` is the
last column, 36 cells, cut from the left when longer (`…/projects/ledge/src/api-gateway` is cut to its tail).

### 4.2 Palette, option B

- Header titles `text_muted`, bold, on the `surface` band. The rule under it in `border`.
- Folder names and the `-` and `+` markers: `text` and `text_muted`.
- `★` in `accent`, in the Bookmark column.
- `live` in `session_active` (with the name at 120 cells), `none` in `session_none`.
- Subfolder counts in `text`, `0` included: a leaf shows `0`, so the column reads as a count for every row.
- Path column in `path`.

### 4.3 Row and header rules, option B

- Columns, 80 cells: Folder 34, Bookmark 10, Session 12, Subfolders 10, one cell between columns and one cell of margin
  on the left. They take 70 of the 78 inner cells.
- Columns, 120 cells: Folder 34, Bookmark 10, Session 22, Subfolders 10, Path 36. They take 117 of the 118 inner cells.
- Shrink order when a column does not fit (`src/tui/columns.rs`, A-06): Path first (last column), then the Session
  name drops to its state, then Folder never goes below 24 cells. The Subfolders and Bookmark columns never shrink.
- Titles: `Folder`, `Bookmark`, `Session`, `Subfolders`, `Path`. The `Subfolders` title is right-aligned over its numbers.
- Subfolders counts the direct subfolders only, not the whole subtree. Why: it matches what `+` and `-` mean.
- The `Bookmark` column carries the bookmark mark as a text cue (`★`) and nothing else, so colour off keeps it.

### 4.4 Keys footer, option B

The shared footer (section 2.2).

### 4.5 Trade-offs, option B

- For: every folder shows its bookmark, session and count without selecting. The 80 cells are used by data, not by
  empty space.
- For: the window matches the project picker and the switcher (`docs/design-options.md` option B 2.1 and 2.2), so the
  user learns one table.
- For: the `★` moves to a column of its own after the name. FR-EDIT-06 asks for "a column of their own" and does not
  name the position, so no requirement changes.
- Against: the `Session` column reads `none` on most rows. A project with no sessions shows a column of `none`. Keep it:
  the column width stays the same from one run to the next.
- Against: at 120 the Path column repeats the tree. It is useful only when two folders share a name. It can be dropped
  without other changes.
- Against: the `Subfolders` count and the `+` marker say the same thing for a folder with children. The count adds the
  number; the marker stays for expand and collapse (FR-EDIT-04).
- Cost: the session list is read once when the window opens (as the project picker does).

### 4.6 Files that change, option B

- `src/tui/columns.rs`: a right-aligned column for the count (a `ColumnSpec` alignment, or a `count` constructor), and
  the shrink order of the Path and Session columns (A-06).
- `src/tui/render.rs`: `cell_parts` and `display_text` pad a right-aligned cell on the left. Header titles already draw
  (the header row exists), so no new header code.
- `src/domain/bm_edit.rs`: `EditRow` carries `subfolders: usize` and `session` (the live session of its path, if any).
- `src/app/bookmark_cmd.rs`: `edit_row` returns one cell per column (folder, bookmark, session, count, and path at 120).
  `edit_chrome` lists the columns and sets `session_column` (the session colour, as in the switcher).
- `src/app/pickers.rs`: the session lookup (`live_session`, `session_text`) is shared with `bookmark_cmd.rs`.
- `src/messages.rs`: the titles `Folder`, `Bookmark`, `Session`, `Subfolders`, `Path`.
- `tests/tui_edit_window.rs`: the column titles, the `★` column, the count column, the full-width selection, the Path cut
  at 120 cells.
- `docs/design.md` section 3.9, when the option is accepted.

---

## 5. Option C: Tree with a path strip

**Idea.** The tree stays exactly as it is now. Above the footer, two rows of a strip say which folder is selected, in full,
whether it is bookmarked, how many bookmarks the project has, and which key toggles the bookmark. It is the smallest change,
and it answers the question "where am I, and what does enter do here?" that the thin tree leaves open. It suits a user who
is not interested in the other folders' state.

### 5.1 Mock-ups

80x24

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────┐
│ Directory                                                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│ ledge -                                                                      │
│ ├─ src - ★                                                                   │
│ │  ├─ api-gateway -                                                          │
│ │  │  ├─ routes                                                              │
│ │  │  └─ services +                                                          │
│ │  ├─ auth ★                                                                 │
│ │  └─ tui                                                                    │
│ ├─ docs -                                                                    │
│ │  ├─ adr                                                                    │
│ │  └─ guides ★                                                               │
│ ├─ tests                                                                     │
│ └─ notes                                                                     │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ ~/dev/projects/ledge/src/api-gateway                          not bookmarked │
│ 4 bookmarks in ledge                               enter toggles this folder │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────┘
```

Caption: the header keeps the title `Directory` (HEADER_DIRECTORY, unchanged). A `border` rule sits above the strip. Strip
row 1 is the selected folder's full path in `path` on the `surface` band, and `not bookmarked` on the right. Strip row 2
is the project's count on the left and `enter toggles this folder` on the right. The list has 17 rows.

120x36

```text
┌─ Bookmarks in ledge ─────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Directory                                                                                                            │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ledge -                                                                                                              │
│ ├─ src - ★                                                                                                           │
│ │  ├─ api-gateway -                                                                                                  │
│ │  │  ├─ routes                                                                                                      │
│ │  │  └─ services +                                                                                                  │
│ │  ├─ auth ★                                                                                                         │
│ │  └─ tui                                                                                                            │
│ ├─ docs -                                                                                                            │
│ │  ├─ adr                                                                                                            │
│ │  └─ guides ★                                                                                                       │
│ ├─ tests                                                                                                             │
│ └─ notes                                                                                                             │
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
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ~/dev/projects/ledge/src/api-gateway                                                                  not bookmarked │
│ 4 bookmarks in ledge                                                                       enter toggles this folder │
└─ enter toggle bookmark · →/l open · ←/h close · / search · esc done ─────────────────────────────────────────────────┘
```

Caption: the same strip at 120 cells. The path is not cut at this width.

### 5.2 Palette, option C

- Strip on the `surface` band, the rule above it in `border`.
- The path in `path`. `not bookmarked` in `text_muted`; `bookmarked` in `text` with `★` in `accent`.
- The count `4 bookmarks in ledge` in `text`; the right side in `text_muted`, the key name in `text`.
- A status message (an error or a warning) replaces the right side of strip row 2, in `error` or `warning`, until the
  next key. Why: the status row is kept for messages, so the strip does not hide a failed change (`docs/design.md` 4.5).

### 5.3 Row and header rules, option C

- The header and the list follow the current window; the list rows have the same cells as today, with `★` after the name.
- A rule in `border` separates the list from the strip.
- Strip row 1: the full path of the selected folder (`~` for the home path), cut from the left with `…` if it is longer
  than the space left after `not bookmarked`. Why: the tail of a path is the part that tells two same-named folders apart.
- Strip row 2: `N bookmarks in <project>` counts every bookmark of the project, also those under collapsed folders (the
  count is 4 in the fixture, one of them under the collapsed `services`).

### 5.4 Keys footer, option C

The shared footer (section 2.2). `enter toggle bookmark` is in the footer and in the strip. Both stay: the footer lists
every key, the strip names the one that matters on this row. If the user prefers one place, the footer keeps it and the
strip drops its right side.

### 5.5 Trade-offs, option C

- For: the smallest change. The tree, the markers and the footer keep their look.
- For: the full path of the selected folder is visible at all times, and so is the project's bookmark count.
- Against: only the selected row has facts. Other rows show names and `★` only, as today.
- Against: no session state. A folder with a live session looks like one without.
- Against: the two strip rows take two rows of the list at 80x24 (17 rows in place of 19).

### 5.6 Files that change, option C

- `src/tui/picker.rs`: `Chrome` gets a `strip` flag, and `PickerModel` exposes the selected row's path (from `details.path`
  or a new `path` field on `PickerRow`).
- `src/tui/render.rs`: `draw` adds a rule and the two strip rows above the status row when `strip` is set. The path is cut
  from the left with `columns::cut_start`.
- `src/app/bookmark_cmd.rs`: `edit_chrome` sets the strip; the bookmark count of the project is read from the bookmarks
  already loaded in `EditHandler`.
- `src/messages.rs`: `N bookmarks in <project>`, `bookmarked`, `not bookmarked`, `enter toggles this folder`.
- `tests/tui_edit_window.rs`: the strip rows and the path cut at 80 and 120 cells, and the count with collapsed folders.
- `docs/design.md` section 3.9, when the option is accepted.

---

## 6. Comparison

| Question | A. Tree plus details | B. Tree with columns | C. Tree with a path strip |
| --- | --- | --- | --- |
| Facts for the selected folder | path, bookmarked, subfolders, session | the same row only | path, bookmarked, project count |
| Facts for every row | name only | bookmark, session, subfolder count | name and bookmark |
| Visible list rows at 80x24 | 12 | 19 | 17 |
| Chrome added at 80x24 | 7 rows (details block) | 0 rows (header exists) | 3 rows (rule and two strip rows) |
| Split layout at 120 cells | yes, new | no | no |
| Matches the other windows | partly | yes, the table of the picker and the switcher | partly |
| Code change | large | medium | small |
| Requirement change | none | none (FR-EDIT-06 holds) | none |

---

## 7. Recommendation

I recommend option B, because it is the only option that gives every folder its bookmark, session and child count at
both sizes, so the window stops looking empty and reads like the project picker and the switcher. Its code cost sits in
columns that already exist (the frame, the header row, the session colour column and a Path column), and the `+` and `-`
markers stay after the name with the `★` after it, as the brief asks. If the user wants the smallest change instead,
option C is the fallback.

---

## 8. Open questions

1. **Bookmark mark position.** The code puts `★` in column 0, before the name (FR-EDIT-06, "a column of their own"). The
   brief puts it after the name. Recommendation: after the name, as the brief asks. FR-EDIT-06 does not name the position,
   so no requirement change is needed.
2. **Session column when no folder has a session.** Recommendation: keep it, so the layout does not change between runs.
3. **Path column at 120 cells (option B).** Recommendation: keep it for folders that share a name; it can be dropped
   without other changes.

---

## 9. Proposals

None added. Option B needs no requirement change: FR-EDIT-06 holds with a trailing column. The footer wording change
is already covered by P-59 (`docs/proposals.md`).

---

## 10. Sources

No new external sources. The palette values are from `docs/design/themes.md` (Kanso zen, verified there) and
`src/tui/theme.rs`; the terminal-tool research is in `docs/design.md` section 10. Count of new sources: 0.

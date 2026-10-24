# ledge interface: three design options

Status: options for review, not a decision. Companion to `docs/design.md` (the current design: sections 3 layout, 4 interaction,
6 theming). No code is changed by this document. Each option covers the whole interface: frame, header, filter prompt, columns,
selection, footer, status, and colour. The mock-ups show the project picker and the session switcher; the other windows follow
the same rules of the chosen option.

Dates: Kanso values read on 2026-10-24. Sources of the research are in `docs/design.md` section 10 (fzf, lazygit, zellij,
tmux choose-tree, k9s, yazi, gitui, NO_COLOR). Those were read earlier and are not repeated here.

## 0. Rules that all three options share

- Removed items are not used anywhere: no `*` marks, no `●` or `○` markers, no left selection bar. "Here" is written as the word
  `(here)`, a live session as its name, no session as `none`, a missing directory as `(missing)`, and `(current)` marks the
  current session in the switcher.
- The selected row is a full-width fill in `selection_bg` with `selection_fg` text. With colour off it is reverse video over the
  full width (FR-TUI-09). No glyph is drawn at the row start in any option.
- Every state has a text cue as well as a colour: missing, no session, error, warning.
- Footers show the keys of the window only, and drop whole key pairs when the window is narrow (design.md 4.2).
- Colour is off with `NO_COLOR` or `TERM=dumb`; the layout must read the same without colour.
- Palettes are dark-first. Each option has one truecolor palette, a 256-colour fallback (nearest xterm index, the same rule as
  `docs/design/themes.md`), and the 16-colour terminal keeps the `default` theme (ANSI names).
- Text in the mock-ups is plain. Each mock-up is followed by a caption that names the colour role of each part.

Kanso values, verified by WebFetch of the raw palette file on 2026-10-24 (the fetch tool summarised the page, so the values were
checked against the fetched text, not a local clone; the commit is not pinned):
`https://raw.githubusercontent.com/webhooked/kanso.nvim/main/lua/kanso/colors.lua`.
The values used here: `inkBg0` #14171d, `inkBg3` #393B44, `inkBg4` #4b4e57, `mistBg0` #22262D, `mistBg1` #2a2c35,
`mistBg2` #393B44, `mistBg3` #5C6066, `zenBg0` #090E13, `zenBg1` #1C1E25, `zenBg2` #22262D, `zenBg3` #393B44, `fg` #C5C9C7,
`gray3` #909398, `blue` #7FB4CA, `green` #98BB6C, `yellow` #DCA561, `red2` #E46876.

`red2` is used for `error` in all three options. The `red` value #C34043 that the current `ink`, `mist` and `zen` themes use
is 3.53:1 on their background, below the 4.5:1 text rule. `red2` is a Kanso value, so no value is invented here.

## 1. Option A: Quiet

**Idea.** The frame, the rules, the bold header and the coloured borders go. The window is text on the background, with wide
margins, column gaps of four cells and one accent colour used for key letters and the prompt. Structure comes from space alone.
It suits a tool that is opened many times a day and should disappear, and it answers the complaint about spacing and the frame
most directly. The palette is the existing `ink` theme with `red2` for errors, so the theme code barely changes.

### 1.1 Mock-ups: project picker

80x24

```text
   Projects  8                                          rebuilt ledge in 2.1s   
   > type to filter                                                             
                                                                                
   Project                       Session               Profile                  
   ledge (here)                  ledge-3f2a            dev                      
   api-gateway                   api-gateway-91bc      dev                      
   infra                         infra-7aa0            basic                    
   docs-site                     none                  dev                      
   acme-web                      none                  dev                      
   notes-scratch                 none                  basic                    
   old-prototype (missing)       none                  dev                      
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
   enter open    ^r rebuild    ^p profile    ^x kill    ? keys                  
```

Caption: the title row carries the window name in text colour and the count in `text_muted`; a status message sits at the right
of the title row in its own colour (here `success`) and clears after about 3 seconds. The `>` prompt is `accent`; the placeholder
text is `text_muted`. The first row is selected: a full-width band in `selection_bg` from the left margin to the right margin.
The footer is the last row: keys in `accent`, labels in `text_muted`.

120x36

```text
   Projects  8  ·  3 live                                                                       rebuilt ledge in 2.1s   
   > type to filter                                                                                                     
                                                                                                                        
   Project                       Session               Profile     Path                                                 
   ledge (here)                  ledge-3f2a            dev         ~/dev/projects/ledge                                 
   api-gateway                   api-gateway-91bc      dev         ~/dev/projects/api-gateway                           
   infra                         infra-7aa0            basic       ~/dev/projects/infra                                 
   docs-site                     none                  dev         ~/dev/work/docs-site                                 
   acme-web                      none                  dev         ~/dev/work/acme-web                                  
   notes-scratch                 none                  basic       ~/scratch/notes-scratch                              
   old-prototype (missing)       none                  dev         ~/old/prototype                                      
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
   enter open    ^r rebuild    ^p profile    ^x kill    ? keys                                                          
```

Caption: the Path column is added at this width in `text_muted`. Nothing else changes.

### 1.2 Mock-ups: session switcher

80x24

```text
   Sessions  4                                                                  
   > type to filter                                                             
                                                                                
   Session                   Project                     Profile     Activity   
   ledge-3f2a (current)      ledge                       dev         just now   
   api-gateway-91bc          api-gateway                 dev         12m ago    
   infra-7aa0                infra                       basic       2h ago     
   old-prototype-0c4e        old-prototype (missing)     dev         3d ago     
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
                                                                                
   enter switch    ^r rebuild    ^p profile    ^x kill    ? keys                
```

Caption: the current session is `ledge-3f2a (current)` in text colour; a missing directory is `(missing)` in `warning`.

120x36

```text
   Sessions  4  ·  1 missing directory                                                                                  
   > type to filter                                                                                                     
                                                                                                                        
   Session                   Project                     Profile     Activity      Path                                 
   ledge-3f2a (current)      ledge                       dev         just now      ~/dev/projects/ledge                 
   api-gateway-91bc          api-gateway                 dev         12m ago       ~/dev/projects/api-gateway           
   infra-7aa0                infra                       basic       2h ago        ~/dev/projects/infra                 
   old-prototype-0c4e        old-prototype (missing)     dev         3d ago        ~/old/prototype                      
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
                                                                                                                        
   enter switch    ^r rebuild    ^p profile    ^x kill    ? keys                                                        
```

Caption: the Path column is shown at this width.

### 1.3 Colour: option A

Dark. Based on the `ink` background. Columns: the Kanso name (or the rule that gives the value).

| Token | Truecolor | 256-colour | Contrast on background | Kanso source |
| --- | --- | --- | --- | --- |
| background | #14171d | 234 | - | inkBg0 |
| surface | #14171d | 234 | 1.00:1 | same as background (no panels) |
| text | #C5C9C7 | 251 | 10.73:1 | fg |
| text_muted | #909398 | 246 | 5.82:1 | gray3 |
| accent | #7FB4CA | 110 | 7.93:1 | blue |
| border | #4b4e57 | 239 | 2.16:1 | inkBg4 (not drawn: no rules) |
| selection_bg | #393B44 | 237 | 1.61:1 | inkBg3 |
| selection_fg | #C5C9C7 | 251 | 10.73:1 | fg |
| success | #98BB6C | 107 | 8.26:1 | green |
| warning | #DCA561 | 179 | 8.20:1 | yellow |
| error | #E46876 | 168 | 5.59:1 | red2 |

Contrast checks: text on selection 6.67:1; text_muted on surface 5.82:1; error on surface 5.59:1; error on background 5.59:1.

### 1.4 Spacing and typography: option A

- Margins: 3 cells left and right of the window. The title row is row 1 and there is one blank row after it.
- Prompt on row 2; one blank row; header row; one blank row; the list starts at row 6.
- Column gap: 4 cells (`columns.rs` `GAP` is 2 today).
- Header: `text_muted`, normal weight, no rule under it.
- Row height: 1. No zebra stripes, no separators between rows.
- Selection: full-width band from the left margin to the right margin; text in `selection_fg`, normal weight.
- Typography: normal weight for everything, except `error` text (bold). No upper case.
- Footer: the last row, pairs separated by four cells, no rule above it.
- Status: no reserved row. A message appears in the title row for about 3 seconds, then the row is empty.

### 1.5 Trade-offs and code cost: option A

Gains: the cleanest and calmest look; the smallest change; no new colour token; the existing `ink` theme is reused.
Costs: the window has no edge, so it can look unbounded inside a terminal split with other panes; the selection is only a fill,
so it relies on contrast alone (`selection_fg` on `selection_bg` is 6.67:1, enough); status messages are easy to miss, because
there is no status row.

Files that change:
- `src/tui/render.rs`: `draw` drops the `Block` and its `BorderType::Rounded` border and title; `footer_line` moves from the
  bottom edge into the last row; `draw_rule` is removed; `list_lines` and `cell_style` use the margin and the new gap.
- `src/tui/columns.rs`: `GAP` 2 to 4 and the margin is taken from the window area, not the full width.
- `src/tui/engine.rs`: none, if the margin is applied inside `render::draw`.
- `src/tui/theme.rs`: built-in `ink` gets `error` = `#E46876`; no new token.
- `tests/tui_project_picker.rs`, `tests/tui_switcher.rs`, `tests/tui_engine.rs`: assertions on border glyphs and on the column
  positions change.
- `docs/design.md` sections 3.1 (frame), 3.2 (column rules), 4.2 (footers), 6.2 (ink error value).

## 2. Option B: Structured

**Idea.** Each window is a framed panel with its sections marked by rules and labelled headers: the filter, the column header on a
surface band, the list, a details section at the wide size, and a status strip above the bottom edge. Information is dense on
purpose: the path column is always there at 120 cells, and the details of the selected row sit under the list. It suits people
who keep several windows open and want the state of the tool visible without asking for it.

### 2.1 Mock-ups: project picker

80x24

```text
┌─ Projects ───────────────────────────────────────────────────────────────────┐
│  Filter   type to filter                                8 projects · 3 live  │
├──────────────────────────────────────────────────────────────────────────────┤
│  Project                       Session               Profile                 │
│  ledge (here)                  ledge-3f2a            dev                     │
│  api-gateway                   api-gateway-91bc      dev                     │
│  infra                         infra-7aa0            basic                   │
│  docs-site                     none                  dev                     │
│  acme-web                      none                  dev                     │
│  notes-scratch                 none                  basic                   │
│  old-prototype (missing)       none                  dev                     │
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
├──────────────────────────────────────────────────────────────────────────────┤
│  ready · 3 live · no filter                           rebuilt ledge in 2.1s  │
└─ enter open · ^r rebuild · ^p profile · ^x kill · ? keys ────────────────────┘
```

Caption: the top edge carries the window name in `text`, bold. The filter row is on the `surface` band. The column header is bold
`text_muted` on the `surface` band. The first row is selected (`selection_bg`, `selection_fg`, bold). The status strip is the
row above the bottom edge: `surface` band, `text_muted`, with the state on the left and the last message on the right
(`success` for the rebuild message). The footer is in the bottom edge: keys in `text` bold, labels in `text_muted`.

120x36

```text
┌─ Projects ───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  Filter   type to filter                                                                        8 projects · 3 live  │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  Project                       Session               Profile     Path                                                │
│  ledge (here)                  ledge-3f2a            dev         ~/dev/projects/ledge                                │
│  api-gateway                   api-gateway-91bc      dev         ~/dev/projects/api-gateway                          │
│  infra                         infra-7aa0            basic       ~/dev/projects/infra                                │
│  docs-site                     none                  dev         ~/dev/work/docs-site                                │
│  acme-web                      none                  dev         ~/dev/work/acme-web                                 │
│  notes-scratch                 none                  basic       ~/scratch/notes-scratch                             │
│  old-prototype (missing)       none                  dev         ~/old/prototype                                     │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  Details · ledge                                                                                                     │
│  Path       ~/dev/projects/ledge                                                                                     │
│  Session    ledge-3f2a, live, 3 windows, started 3h ago                                                              │
│  Profile    dev (config default)                                                                                     │
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
│  ready · 3 live · no filter                                                                   rebuilt ledge in 2.1s  │
└─ enter open · ^r rebuild · ^p profile · ^x kill · ? keys ────────────────────────────────────────────────────────────┘
```

Caption: the list has the Path column. The Details section shows the selected project; its label is `text` bold, its values
are `text`. The rest of the panel is the same as at 80 cells.

### 2.2 Mock-ups: session switcher

80x24

```text
┌─ Sessions ───────────────────────────────────────────────────────────────────┐
│  Filter   type to filter                                         4 sessions  │
├──────────────────────────────────────────────────────────────────────────────┤
│  Session                   Project                     Profile     Activity  │
│  ledge-3f2a (current)      ledge                       dev         just now  │
│  api-gateway-91bc          api-gateway                 dev         12m ago   │
│  infra-7aa0                infra                       basic       2h ago    │
│  old-prototype-0c4e        old-prototype (missing)     dev         3d ago    │
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
├──────────────────────────────────────────────────────────────────────────────┤
│  warning · old-prototype: directory missing                           ready  │
└─ enter switch · ^r rebuild · ^p profile · ^x kill · ? keys ──────────────────┘
```

Caption: the status strip shows a warning in `warning` bold on the left: `old-prototype: directory missing`. The row for the
missing directory is `text_muted` with `(missing)` in `warning`.

120x36

```text
┌─ Sessions ───────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  Filter   type to filter                                                                                 4 sessions  │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  Session                   Project                     Profile     Activity      Path                                │
│  ledge-3f2a (current)      ledge                       dev         just now      ~/dev/projects/ledge                │
│  api-gateway-91bc          api-gateway                 dev         12m ago       ~/dev/projects/api-gateway          │
│  infra-7aa0                infra                       basic       2h ago        ~/dev/projects/infra                │
│  old-prototype-0c4e        old-prototype (missing)     dev         3d ago        ~/old/prototype                     │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  Details · ledge-3f2a (current)                                                                                      │
│  Path       ~/dev/projects/ledge                                                                                     │
│  Windows    3, panes 5, started 3h ago                                                                               │
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
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  warning · old-prototype: directory missing                                                                   ready  │
└─ enter switch · ^r rebuild · ^p profile · ^x kill · ? keys ──────────────────────────────────────────────────────────┘
```

Caption: at this width the Details section shows the current session's path and window count.

### 2.3 Colour: option B

Dark. Based on the `mist` background, which gives the panels a slightly lighter base than option A.

| Token | Truecolor | 256-colour | Contrast on background | Kanso source |
| --- | --- | --- | --- | --- |
| background | #22262D | 235 | - | mistBg0 |
| surface | #2A2C35 | 236 | 1.09:1 | mistBg1 (header, filter and status strips) |
| text | #C5C9C7 | 251 | 9.08:1 | fg |
| text_muted | #909398 | 246 | 4.93:1 | gray3 |
| accent | #7FB4CA | 110 | 6.71:1 | blue |
| border | #5C6066 | 241 | 2.40:1 | mistBg3 |
| selection_bg | #393B44 | 237 | 1.36:1 | mistBg2 |
| selection_fg | #C5C9C7 | 251 | 9.08:1 | fg |
| success | #98BB6C | 107 | 6.98:1 | green |
| warning | #DCA561 | 179 | 6.94:1 | yellow |
| error | #E46876 | 168 | 4.73:1 | red2 |

Contrast checks: text on selection 6.67:1; text_muted on surface 4.51:1; error on surface 4.33:1; error on background 4.73:1.

### 2.4 Spacing and typography: option B

- Frame: single-line box, `border` colour. The window name is in the top edge. Sections are separated by full-width rules (`├ ┤`).
- Margins: 1 cell inside the frame. Column gap: 2 cells (unchanged).
- Section headers: a label in `text` bold, on the first row of the section (`Filter`, `Details`).
- Column header: bold `text_muted` on the `surface` band, no separate rule under it (the band is the separator).
- Row height: 1. Selection full-width in `selection_bg`, text bold.
- Status strip: always one row above the bottom edge, `surface` band; left = state, right = last message.
- Footer: in the bottom edge; keys bold, labels muted; pairs separated by ` · `.
- Wide layout (120 cells and up): path column in the list; Details section under the list.

### 2.5 Trade-offs and code cost: option B

Gains: the most information per screen; sections are easy to scan; status is always visible.
Costs: it is the closest to the current design, so it may read as "the same frame again" (the user rejected the frame); the
rules and bands add visual weight; the Details section needs data that the picker model may not hold yet (windows count, start
time). That data must be checked in `src/tui/picker.rs` and the store before the section is built.

Files that change:
- `src/tui/render.rs`: section rules and labels; a status strip row; footer in the bottom edge (kept); the Details section
  at 120 cells and up.
- `src/tui/columns.rs`: a path column at 120 cells and up; widths for the name and path columns.
- `src/tui/picker.rs`: the selected row's details (path, session facts, profile) passed to the renderer; a status message kept
  for the strip.
- `src/tui/theme.rs`: new token `surface` (in `Token::ALL`, the names, the config key list); built-in `mist` gets `surface`
  = `#2A2C35` and `error` = `#E46876`.
- `tests/tui_project_picker.rs`, `tests/tui_switcher.rs`, `tests/tui_engine.rs`: new layout assertions for the sections and the
  status strip.
- `docs/design.md` sections 3.1, 3.2, 3.3, 3.5, 4.2, 6.1 (new token), 6.2 (mist values).

## 3. Option C: Modern

**Idea.** One window with one plain border (square corners, low-contrast `border`) and no label in its edge. Inside, a header
band on a subtle `surface` tone carries the title, the count and the last message; a filter field sits under it; the list is
the whole body (no sidebar, no panels). The selected row is a clear full-width band in `zenBg3`. The footer is a row of key
chips: each key sits on a `chip` tone, the label stands beside it. It suits people who want a current, polished look and a
clear answer to "what is selected" and "what can I press".

### 3.1 Mock-ups: project picker

80x24

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│  Projects  8                                          rebuilt ledge in 2.1s  │
│                                                                              │
│  > type to filter                                                            │
│                                                                              │
│  Project                       Session               Profile                 │
│  ledge (here)                  ledge-3f2a            dev                     │
│  api-gateway                   api-gateway-91bc      dev                     │
│  infra                         infra-7aa0            basic                   │
│  docs-site                     none                  dev                     │
│  acme-web                      none                  dev                     │
│  notes-scratch                 none                  basic                   │
│  old-prototype (missing)       none                  dev                     │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│                                                                              │
│  enter  open     ^r  rebuild     ^p  profile     ^x  kill     ?  keys        │
└──────────────────────────────────────────────────────────────────────────────┘
```

Caption: the header band is `surface` across the inside of the border; the title is `text` bold, the count is `text_muted`, and the
rebuild message is `success` at the right. The filter field is text on the window background with its placeholder in `text_muted`.
The first row is selected: `selection_bg` across the full inside width. The footer chips: each key is `text` bold on `chip`;
each label is `text_muted` on the window background; chips are separated by five cells.

120x36

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  Projects  8  ·  3 live                                                                       rebuilt ledge in 2.1s  │
│                                                                                                                      │
│  > type to filter                                                                                                    │
│                                                                                                                      │
│  Project                       Session               Profile     Path                                                │
│  ledge (here)                  ledge-3f2a            dev         ~/dev/projects/ledge                                │
│  api-gateway                   api-gateway-91bc      dev         ~/dev/projects/api-gateway                          │
│  infra                         infra-7aa0            basic       ~/dev/projects/infra                                │
│  docs-site                     none                  dev         ~/dev/work/docs-site                                │
│  acme-web                      none                  dev         ~/dev/work/acme-web                                 │
│  notes-scratch                 none                  basic       ~/scratch/notes-scratch                             │
│  old-prototype (missing)       none                  dev         ~/old/prototype                                     │
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
│  enter  open     ^r  rebuild     ^p  profile     ^x  kill     ?  keys                                                │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Caption: the Path column is added at this width. The chip row is unchanged.

### 3.2 Mock-ups: session switcher

80x24

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│  Sessions  4                                            1 directory missing  │
│                                                                              │
│  > type to filter                                                            │
│                                                                              │
│  Session                   Project                     Profile     Activity  │
│  ledge-3f2a (current)      ledge                       dev         just now  │
│  api-gateway-91bc          api-gateway                 dev         12m ago   │
│  infra-7aa0                infra                       basic       2h ago    │
│  old-prototype-0c4e        old-prototype (missing)     dev         3d ago    │
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
│  enter  switch     ^r  rebuild     ^p  profile     ^x  kill     ?  keys      │
└──────────────────────────────────────────────────────────────────────────────┘
```

Caption: the header band's right side shows the warning `1 directory missing` in `warning`. The missing row carries `(missing)`
in `warning`.

120x36

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  Sessions  4                                                                                    1 directory missing  │
│                                                                                                                      │
│  > type to filter                                                                                                    │
│                                                                                                                      │
│  Session                   Project                     Profile     Activity      Path                                │
│  ledge-3f2a (current)      ledge                       dev         just now      ~/dev/projects/ledge                │
│  api-gateway-91bc          api-gateway                 dev         12m ago       ~/dev/projects/api-gateway          │
│  infra-7aa0                infra                       basic       2h ago        ~/dev/projects/infra                │
│  old-prototype-0c4e        old-prototype (missing)     dev         3d ago        ~/old/prototype                     │
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
│  enter  switch     ^r  rebuild     ^p  profile     ^x  kill     ?  keys                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Caption: the Path column is shown at this width.

### 3.3 Colour: option C

Dark. Based on the `zen` background, the darkest of the Kanso family. The `chip` token is new.

| Token | Truecolor | 256-colour | Contrast on background | Kanso source |
| --- | --- | --- | --- | --- |
| background | #090E13 | 233 | - | zenBg0 |
| surface | #1C1E25 | 234 | 1.16:1 | zenBg1 (header band) |
| chip | #22262D | 235 | 1.28:1 | zenBg2 (footer key chips) |
| text | #C5C9C7 | 251 | 11.58:1 | fg |
| text_muted | #909398 | 246 | 6.29:1 | gray3 |
| accent | #7FB4CA | 110 | 8.57:1 | blue |
| border | #393B44 | 237 | 1.74:1 | zenBg3 |
| selection_bg | #393B44 | 237 | 1.74:1 | zenBg3 |
| selection_fg | #C5C9C7 | 251 | 11.58:1 | fg |
| success | #98BB6C | 107 | 8.91:1 | green |
| warning | #DCA561 | 179 | 8.86:1 | yellow |
| error | #E46876 | 168 | 6.03:1 | red2 |

Contrast checks: text on selection 6.67:1; text_muted on surface 5.40:1; error on surface 5.18:1; error on background 6.03:1.

### 3.4 Spacing and typography: option C

- Border: one line, square corners, `border` colour. No text in the edge.
- Margins: 2 cells inside the border. Column gap: 3 cells.
- Header band: row 1 inside the border, `surface` tone across the full inside width.
- Row after the band: blank; then the filter field; then a blank row; then the column header (`text_muted`, normal weight).
- Row height: 1. Selection full-width in `zenBg3`, text `selection_fg` normal weight.
- Footer chips: the last row inside the border. Key text bold on `chip`; label beside it in `text_muted`; five cells between chips.
- Status: the header band's right side, for about 3 seconds; warnings stay until the state changes.

### 3.5 Trade-offs and code cost: option C

Gains: a clear selected state; footer keys are easy to scan as chips; the subtle tones give depth without a frame of rules.
Costs: chips are harder to make degrade well: without colour the chips become plain text, so the spacing must carry the
separation (the mock-up above is that colour-off case); a new `chip` token is needed; the header band depends on the `surface`
tone, which is close to the background on the dark themes, so the band is subtle on some terminals.

Files that change:
- `src/tui/render.rs`: one `Block` with a plain border; the header band; the filter field; `footer_line` becomes a chip row
  (key and label spans with their own background); `cell_style` keeps the selection full width.
- `src/tui/columns.rs`: `GAP` 2 to 3; margin of 2 cells.
- `src/tui/theme.rs`: new tokens `surface` and `chip` (in `Token::ALL`, the names and the config keys); built-in `zen` gets
  `surface` = `#1C1E25`, `chip` = `#22262D`, `border` = `#393B44`, `error` = `#E46876`.
- `tests/tui_project_picker.rs`, `tests/tui_switcher.rs`, `tests/tui_engine.rs`: assertions for the chip row and the band.
- `docs/design.md` sections 3.1, 3.2, 4.2 (chip row), 6.1 (new tokens), 6.2 (zen values).

## 4. Comparison

| Question | A. Quiet | B. Structured | C. Modern |
| --- | --- | --- | --- |
| Frame | none | section rules, edge labels | one plain border, no label |
| Header | muted text, no rule | bold on a band | band with title and count |
| Selected row | full-width fill | full-width fill, bold | full-width fill, bold text (zenBg3) |
| Footer | last row, muted labels | bottom edge | key chips |
| Status | title row, transient | strip, always visible | header band, transient or warning |
| Information per screen | least | most | medium |
| Colour tokens added | none | `surface` | `surface`, `chip` |
| Code change | smallest | largest | medium |

## 5. Recommendation

I would pick A (Quiet): it removes the frame, the rules and the bold header that the user dislikes, gives the wider spacing the user
asks for, reuses the `ink` theme with one error value, and is the smallest change to `render.rs` and `columns.rs`. Combine it with
C's selected state (the `zen`-family band, `zenBg3`, full width, bold text) and C's footer key chips without the border, since
they make the selection and the keys clearer without adding a frame. Take B's status strip only as a row that appears when a
message is active, and build B's Details section later only if the user asks for more information per row.

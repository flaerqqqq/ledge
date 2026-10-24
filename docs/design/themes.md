# ledge themes: palette tables

Companion to `docs/design.md` section 6. One table per built-in theme. Each token is a hex value for truecolor terminals, the nearest xterm 256-colour index for 256-colour terminals, and a contrast ratio against the theme's `background` (WCAG 2.x relative luminance). Text tokens are expected to reach 4.5:1; the table says which do not.

## Sources of the values

- Kanso palette names and hex values: https://raw.githubusercontent.com/webhooked/kanso.nvim/main/lua/kanso/colors.lua, read through WebFetch on 2026-10-24. Lines quoted verbatim: `zenBg0 = "#090E13"`, `zenBg3 = "#393B44"`, `inkBg0 = "#14171d"`, `inkBg3 = "#393B44"`, `inkBg4 = "#4b4e57"`, `mistBg0 = "#22262D"`, `mistBg2 = "#393B44"`, `mistBg3 = "#5C6066"`, `fg = "#C5C9C7"`, `gray3 = "#909398"`, `gray4 = "#75797f"`, `pearlWhite0 = "#f2f1ef"`, `pearlWhite2 = "#dddddb"`, `pearlBlack0 = "#22262D"`, `pearlBlack1 = "#545464"`, `pearlBlue4 = "#4d699b"`, `pearlGreen = "#6f894e"`, `pearlRed = "#c84053"`, `pearlYellow = "#77713f"`, `pearlOrange = "#cc6d00"`, `pearlTeal1 = "#4e8ca2"`, `pearlGray3 = "#6D6D69"`, `blue = "#7FB4CA"`, `green = "#98BB6C"`, `red = "#C34043"`, `yellow = "#DCA561"`, `aqua = "#8ea4a2"`.
- Which palette name each token uses per variant: https://raw.githubusercontent.com/webhooked/kanso.nvim/main/lua/kanso/themes.lua (`ui.bg`, `ui.fg`, `ui.bg_visual`, `ui.bg_p2`, `syn.comment`). The table reads: zen `zenBg0` / `palette.fg` / `inkBg3`; ink `inkBg0` / `palette.fg` / `inkBg3`; mist `mistBg0` / `palette.fg` / `mistBg2`; pearl `pearlWhite0` / `pearlBlack0` / `pearlWhite2`.
- Not Kanso values, chosen by the designer from the same palette: `text_muted`, `session_none`, `missing` and `connector` on the dark themes use `gray3`, because `gray4` (the comment colour) is 4.10:1 on `inkBg0`, below 4.5:1. The zen `border` uses `zenBg3`, the zen family's own border shade. `pearl` `border` uses `pearlBlack1`; `warning` uses `pearlYellow` (4.41:1), not `pearlOrange` (3.23:1).
- The Kanso repository has no per-token names for `success`, `warning`, `error`, `path` or `missing`. They are mapped to the closest palette colour of the role, and each mapping is listed in the tables.
- The 256-colour column is not Kanso's. It is the nearest xterm 256-colour index to each hex: the cube index (16 + 36r + 6g + b, levels 0, 95, 135, 175, 215, 255) or the grey ramp (232 + i, level 8 + 10i), whichever is closer in RGB distance.

## default (ANSI names)

| Token | Value | Note |
| --- | --- | --- |
| background | terminal default | the user keeps the terminal background |
| text | terminal default | foreground |
| text_muted | DarkGray (ANSI 8) | dim for dark and light terminals alike |
| accent | Cyan (ANSI 6) |  |
| border | DarkGray (ANSI 8) |  |
| selection_bg / selection_fg | reverse video | no colour is set, so the selection works in every terminal |
| success / session_active | Green (ANSI 2) |  |
| warning | Yellow (ANSI 3) |  |
| error | Red (ANSI 1) | bold is added |
| path | terminal default | paths are plain text |
| missing / session_none / connector | DarkGray (ANSI 8) | the text cue carries the meaning |

Contrast of the ANSI names depends on the user's palette, so it is not computed here.

## ink (dark)

| Token | Hex | 256-colour | Contrast vs background | 4.5:1 for text |
| --- | --- | --- | --- | --- |
| text | #C5C9C7 | 251 | 10.73 | yes |
| text_muted | #909398 | 246 | 5.82 | yes |
| accent | #7FB4CA | 110 | 7.93 | yes |
| border | #4b4e57 | 239 | 2.16 | no |
| selection_bg (text on it: selection_fg) | #393B44 | 237 | 6.67 | yes |
| success | #98BB6C | 107 | 8.26 | yes |
| warning | #DCA561 | 179 | 8.20 | yes |
| error | #C34043 | 131 | 3.53 | no, 3:1 minimum for large or bold only |
| session_active | #98BB6C | 107 | 8.26 | yes |
| session_none | #909398 | 246 | 5.82 | yes |
| path | #8ea4a2 | 247 | 6.82 | yes |
| missing | #909398 | 246 | 5.82 | yes |
| connector | #909398 | 246 | 5.82 | yes |

Token source: background=`inkBg0`, text=`fg`, text_muted=`gray3`, accent=`blue`, border=`inkBg4`, selection_bg=`inkBg3`, selection_fg=`fg`, success=`green`, warning=`yellow`, error=`red`, session_active=`green`, session_none=`gray3`, path=`aqua`, missing=`gray3`, connector=`gray3`.

On the dark themes `error` (red) is below 4.5:1 against the background (see the table). `docs/design.md` section 8, question 4.

## mist (dark)

| Token | Hex | 256-colour | Contrast vs background | 4.5:1 for text |
| --- | --- | --- | --- | --- |
| text | #C5C9C7 | 251 | 9.08 | yes |
| text_muted | #909398 | 246 | 4.93 | yes |
| accent | #7FB4CA | 110 | 6.71 | yes |
| border | #5C6066 | 241 | 2.40 | no |
| selection_bg (text on it: selection_fg) | #393B44 | 237 | 6.67 | yes |
| success | #98BB6C | 107 | 6.98 | yes |
| warning | #DCA561 | 179 | 6.94 | yes |
| error | #C34043 | 131 | 2.99 | no |
| session_active | #98BB6C | 107 | 6.98 | yes |
| session_none | #909398 | 246 | 4.93 | yes |
| path | #8ea4a2 | 247 | 5.77 | yes |
| missing | #909398 | 246 | 4.93 | yes |
| connector | #909398 | 246 | 4.93 | yes |

Token source: background=`mistBg0`, text=`fg`, text_muted=`gray3`, accent=`blue`, border=`mistBg3`, selection_bg=`mistBg2`, selection_fg=`fg`, success=`green`, warning=`yellow`, error=`red`, session_active=`green`, session_none=`gray3`, path=`aqua`, missing=`gray3`, connector=`gray3`.

On the dark themes `error` (red) is below 4.5:1 against the background (see the table). `docs/design.md` section 8, question 4.

## zen (dark)

| Token | Hex | 256-colour | Contrast vs background | 4.5:1 for text |
| --- | --- | --- | --- | --- |
| text | #C5C9C7 | 251 | 11.58 | yes |
| text_muted | #909398 | 246 | 6.29 | yes |
| accent | #7FB4CA | 110 | 8.57 | yes |
| border | #393B44 | 237 | 1.74 | no |
| selection_bg (text on it: selection_fg) | #393B44 | 237 | 6.67 | yes |
| success | #98BB6C | 107 | 8.91 | yes |
| warning | #DCA561 | 179 | 8.86 | yes |
| error | #C34043 | 131 | 3.82 | no, 3:1 minimum for large or bold only |
| session_active | #98BB6C | 107 | 8.91 | yes |
| session_none | #909398 | 246 | 6.29 | yes |
| path | #8ea4a2 | 247 | 7.36 | yes |
| missing | #909398 | 246 | 6.29 | yes |
| connector | #909398 | 246 | 6.29 | yes |

Token source: background=`zenBg0`, text=`fg`, text_muted=`gray3`, accent=`blue`, border=`zenBg3`, selection_bg=`inkBg3`, selection_fg=`fg`, success=`green`, warning=`yellow`, error=`red`, session_active=`green`, session_none=`gray3`, path=`aqua`, missing=`gray3`, connector=`gray3`.

On the dark themes `error` (red) is below 4.5:1 against the background (see the table). `docs/design.md` section 8, question 4.

## pearl (light)

| Token | Hex | 256-colour | Contrast vs background | 4.5:1 for text |
| --- | --- | --- | --- | --- |
| text | #22262D | 235 | 13.45 | yes |
| text_muted | #6D6D69 | 242 | 4.60 | yes |
| accent | #4d699b | 60 | 4.88 | yes |
| border | #545464 | 240 | 6.58 | yes |
| selection_bg (text on it: selection_fg) | #dddddb | 253 | 11.16 | yes |
| success | #6f894e | 65 | 3.47 | no, 3:1 minimum for large or bold only |
| warning | #77713f | 95 | 4.41 | no, 3:1 minimum for large or bold only |
| error | #c84053 | 167 | 4.31 | no, 3:1 minimum for large or bold only |
| session_active | #6f894e | 65 | 3.47 | no, 3:1 minimum for large or bold only |
| session_none | #6D6D69 | 242 | 4.60 | yes |
| path | #4e8ca2 | 67 | 3.32 | no, 3:1 minimum for large or bold only |
| missing | #6D6D69 | 242 | 4.60 | yes |
| connector | #6D6D69 | 242 | 4.60 | yes |

Token source: background=`pearlWhite0`, text=`pearlBlack0`, text_muted=`pearlGray3`, accent=`pearlBlue4`, border=`pearlBlack1`, selection_bg=`pearlWhite2`, selection_fg=`pearlBlack0`, success=`pearlGreen`, warning=`pearlYellow`, error=`pearlRed`, session_active=`pearlGreen`, session_none=`pearlGray3`, path=`pearlTeal1`, missing=`pearlGray3`, connector=`pearlGray3`.

Pearl text states below 4.5:1: `success` (3.47), `path` (3.32), `error` (4.31), `warning` (4.41). See `docs/design.md` section 8, question 4.

## Selection and cursor

Kanso `ui.bg_p2` (the cursor line) is not used by ledge; the selected row uses `selection_bg` and `selection_fg` only. Text on the selection: ink 6.67:1, mist 6.67:1, zen 6.67:1 (`fg` on `inkBg3`), pearl 11.16:1 (`pearlBlack0` on `pearlWhite2`).

## Override example

With `"theme": "ink"` and `"theme_colors": {"accent": "#7fb4ca"}`, only `accent` changes; the rest of `ink` stays. Values in `theme_colors` are truecolor and are mapped to the nearest 256-colour index on 256-colour terminals (`docs/design.md` section 6.3).

//! Themes (design section 6, S-37): the semantic tokens of section 6.1, the built-in themes of section 6.2 with the
//! values of `docs/design/themes.md`, the `theme` and `theme_colors` overrides of P-60, and the mapping to the colour
//! level of the terminal (section 7). Pure: no I/O and no environment; the caller passes the config values and the
//! `COLORTERM` and `TERM` values.

use crate::messages;
use ratatui::style::Color;

/// Design 6.1: the semantic tokens. Code asks for a token, never for a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Background,
    Text,
    TextMuted,
    Accent,
    Border,
    SelectionBg,
    SelectionFg,
    Success,
    Warning,
    Error,
    SessionActive,
    SessionNone,
    Path,
    Missing,
    Connector,
}

impl Token {
    /// Every token, in the order of the palette arrays of the built-in themes.
    pub const ALL: [Token; 15] = [
        Token::Background,
        Token::Text,
        Token::TextMuted,
        Token::Accent,
        Token::Border,
        Token::SelectionBg,
        Token::SelectionFg,
        Token::Success,
        Token::Warning,
        Token::Error,
        Token::SessionActive,
        Token::SessionNone,
        Token::Path,
        Token::Missing,
        Token::Connector,
    ];

    /// The name the config uses in `theme_colors` (design 6.1).
    pub fn name(self) -> &'static str {
        match self {
            Token::Background => "background",
            Token::Text => "text",
            Token::TextMuted => "text_muted",
            Token::Accent => "accent",
            Token::Border => "border",
            Token::SelectionBg => "selection_bg",
            Token::SelectionFg => "selection_fg",
            Token::Success => "success",
            Token::Warning => "warning",
            Token::Error => "error",
            Token::SessionActive => "session_active",
            Token::SessionNone => "session_none",
            Token::Path => "path",
            Token::Missing => "missing",
            Token::Connector => "connector",
        }
    }

    /// The token named `name`, or `None` for a name that is not a token.
    pub fn from_name(name: &str) -> Option<Token> {
        Token::ALL.into_iter().find(|token| token.name() == name)
    }

    fn index(self) -> usize {
        Token::ALL
            .iter()
            .position(|token| *token == self)
            .unwrap_or(0)
    }
}

/// The colour a terminal can show (design 7): truecolour, 256 indexes, or the 16 ANSI names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourLevel {
    TrueColour,
    Indexed,
    Ansi,
}

impl ColourLevel {
    /// Design 7: `COLORTERM` `truecolor` or `24bit` gives truecolour; a `TERM` with `256color` gives the 256 indexes;
    /// anything else gives the ANSI names of the `default` theme.
    pub fn detect(colorterm: Option<&str>, term: Option<&str>) -> Self {
        if matches!(colorterm, Some("truecolor" | "24bit")) {
            ColourLevel::TrueColour
        } else if term.is_some_and(|term| term.contains("256color")) {
            ColourLevel::Indexed
        } else {
            ColourLevel::Ansi
        }
    }
}

/// The colour of each token, and the emphasis the theme adds to some tokens (design 8, item 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    colors: [Color; 15],
    /// The pearl theme draws its low-contrast text states bold (design section 8, item 4).
    low_contrast_bold: bool,
}

/// Names of the built-in themes (design 6.2).
pub const BUILTIN_NAMES: [&str; 5] = ["default", "ink", "mist", "zen", "pearl"];

/// `Color::Reset` is the terminal's own colour: no colour is set for the token.
const RESET: Color = Color::Reset;

const fn hex(value: u32) -> Color {
    Color::Rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            colors: [
                RESET,
                RESET,
                Color::DarkGray,
                Color::Cyan,
                Color::DarkGray,
                RESET,
                RESET,
                Color::Green,
                Color::Yellow,
                Color::Red,
                Color::Green,
                Color::DarkGray,
                RESET,
                Color::DarkGray,
                Color::DarkGray,
            ],
            low_contrast_bold: false,
        }
    }
}

impl Theme {
    /// Design 6.2: the built-in theme `name` with its truecolour values, or `None` for a name that is not built in.
    pub fn builtin(name: &str) -> Option<Theme> {
        let colors = match name {
            "default" => return Some(Theme::default()),
            // docs/design/themes.md, dark: Kanso ink, mist and zen.
            "ink" => dark(0x14171d, 0x393b44, 0x4b4e57),
            "mist" => dark(0x22262d, 0x393b44, 0x5c6066),
            "zen" => dark(0x090e13, 0x393b44, 0x393b44),
            // docs/design/themes.md, light: Kanso pearl.
            "pearl" => {
                return Some(Theme {
                    colors: [
                        hex(0xf2f1ef), // background: pearlWhite0
                        hex(0x22262d), // text: pearlBlack0
                        hex(0x6d6d69), // text_muted: pearlGray3
                        hex(0x4d699b), // accent: pearlBlue4
                        hex(0x545464), // border: pearlBlack1
                        hex(0xdddddb), // selection_bg: pearlWhite2
                        hex(0x22262d), // selection_fg: pearlBlack0
                        hex(0x6f894e), // success: pearlGreen
                        hex(0x77713f), // warning: pearlYellow
                        hex(0xc84053), // error: pearlRed
                        hex(0x6f894e), // session_active: pearlGreen
                        hex(0x6d6d69), // session_none: pearlGray3
                        hex(0x4e8ca2), // path: pearlTeal1
                        hex(0x6d6d69), // missing: pearlGray3
                        hex(0x6d6d69), // connector: pearlGray3
                    ],
                    low_contrast_bold: true,
                });
            }
            _ => return None,
        };
        Some(Theme {
            colors,
            low_contrast_bold: false,
        })
    }

    /// The colour of `token`. `Color::Reset` is the terminal's own colour.
    pub fn color(&self, token: Token) -> Color {
        self.colors[token.index()]
    }

    fn set(&mut self, token: Token, color: Color) {
        self.colors[token.index()] = color;
    }

    /// Design 6.1: the selected row is reverse video when the theme sets no selection colour (`default`).
    pub fn selection_reversed(&self) -> bool {
        self.color(Token::SelectionBg) == RESET
    }

    /// Design 4.5, 8 item 4: errors are bold on every theme; the pearl theme also draws its low-contrast states bold.
    pub fn is_bold(&self, token: Token) -> bool {
        match token {
            Token::Error => true,
            Token::Success | Token::Warning | Token::Path | Token::SessionActive => {
                self.low_contrast_bold
            }
            _ => false,
        }
    }

    /// Design 7: the colours of this theme for a terminal at `level`. Truecolour values become the nearest 256-colour
    /// index on an indexed terminal. The ANSI level has only the names, so it uses the `default` theme.
    pub fn at_level(&self, level: ColourLevel) -> Theme {
        match level {
            ColourLevel::TrueColour => self.clone(),
            ColourLevel::Indexed => Theme {
                colors: self.colors.map(|color| match color {
                    Color::Rgb(r, g, b) => Color::Indexed(nearest_index(r, g, b)),
                    other => other,
                }),
                low_contrast_bold: self.low_contrast_bold,
            },
            ColourLevel::Ansi => Theme::default(),
        }
    }
}

/// A dark built-in: the background, the selection background and the border; the text colours are the same on
/// every dark theme (docs/design/themes.md).
fn dark(background: u32, selection: u32, border: u32) -> [Color; 15] {
    [
        hex(background),
        hex(0xc5c9c7), // text: fg
        hex(0x909398), // text_muted: gray3
        hex(0x7fb4ca), // accent: blue
        hex(border),
        hex(selection),
        hex(0xc5c9c7), // selection_fg: fg
        hex(0x98bb6c), // success: green
        hex(0xdca561), // warning: yellow
        hex(0xc34043), // error: red
        hex(0x98bb6c), // session_active: green
        hex(0x909398), // session_none: gray3
        hex(0x8ea4a2), // path: aqua
        hex(0x909398), // missing: gray3
        hex(0x909398), // connector: gray3
    ]
}

/// Design 7, docs/design/themes.md: the nearest xterm 256-colour index of an RGB value, from the colour cube
/// (16 + 36r + 6g + b, levels 0, 95, 135, 175, 215, 255) or the grey ramp (232 + i, level 8 + 10i), whichever is
/// nearer in RGB distance.
pub fn nearest_index(r: u8, g: u8, b: u8) -> u8 {
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let nearest_level = |v: u8| -> usize {
        let v = i32::from(v);
        (0..LEVELS.len())
            .min_by_key(|&i| (v - LEVELS[i]).abs())
            .unwrap_or(0)
    };
    let (ri, gi, bi) = (nearest_level(r), nearest_level(g), nearest_level(b));
    let cube_rgb = [LEVELS[ri], LEVELS[gi], LEVELS[bi]];
    let cube_index = 16 + 36 * ri + 6 * gi + bi;

    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let average = (r + g + b) / 3;
    let grey_step = ((average - 8 + 5).div_euclid(10)).clamp(0, 23);
    let grey = 8 + 10 * grey_step;

    let distance = |a: [i32; 3], b: [i32; 3]| -> i32 { (0..3).map(|k| (a[k] - b[k]).pow(2)).sum() };
    let rgb = [r, g, b];
    let cube_distance = distance(rgb, cube_rgb);
    let grey_distance = distance(rgb, [grey, grey, grey]);
    if cube_distance <= grey_distance {
        u8::try_from(cube_index).unwrap_or(0)
    } else {
        u8::try_from(232 + grey_step).unwrap_or(0)
    }
}

/// Parses `#rrggbb` (either case). Anything else is `None`.
pub fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(hex(value))
}

/// The theme of a run and the warnings for the config values it could not use (P-60).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub theme: Theme,
    /// Full warning lines for stderr, in config order.
    pub warnings: Vec<String>,
}

/// P-60, FR-CONF-09: the theme for `name` (`None` means `default`) with the `colors` overrides applied. An unknown
/// name warns and uses `default`. A `theme_colors` entry whose name is not a token, or whose value is not `#rrggbb`,
/// warns and is ignored, so the theme's own colour stays. On the ANSI level only the `default` names are used, as
/// design 6.3 says, so the overrides cannot be shown and are dropped.
pub fn resolve(name: Option<&str>, colors: &[(String, String)], level: ColourLevel) -> Resolved {
    let mut warnings = Vec::new();
    let mut theme = match name {
        None => Theme::default(),
        Some(name) => match Theme::builtin(name) {
            Some(theme) => theme,
            None => {
                warnings.push(messages::warning(&format!(
                    "theme '{name}' is unknown; using default."
                )));
                Theme::default()
            }
        },
    };
    for (key, value) in colors {
        let Some(token) = Token::from_name(key) else {
            warnings.push(messages::warning(&format!(
                "theme_colors '{key}' is not a colour token; ignored."
            )));
            continue;
        };
        let Some(color) = parse_hex(value) else {
            warnings.push(messages::warning(&format!(
                "theme_colors '{value}' is not a #rrggbb colour; ignored."
            )));
            continue;
        };
        if level != ColourLevel::Ansi {
            theme.set(token, color);
        }
    }
    Resolved {
        theme: theme.at_level(level),
        warnings,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn rgb(text: &str) -> Color {
        parse_hex(text).unwrap()
    }

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // design 6.2: the five built-in names exist and any other name does not
    #[test]
    fn every_builtin_name_is_built_in() {
        for name in BUILTIN_NAMES {
            assert!(Theme::builtin(name).is_some(), "{name}");
        }
        assert!(Theme::builtin("solarized").is_none());
        assert!(Theme::builtin("Ink").is_none());
    }

    // design 6.1: every token has a name that round-trips
    #[test]
    fn token_names_round_trip() {
        for token in Token::ALL {
            assert_eq!(Token::from_name(token.name()), Some(token));
        }
        assert_eq!(Token::from_name("accent_bg"), None);
    }

    // design 6.2, themes.md: the default theme uses the ANSI names and the terminal's own text and background
    #[test]
    fn default_theme_uses_ansi_names() {
        let theme = Theme::default();
        assert_eq!(theme.color(Token::Background), Color::Reset);
        assert_eq!(theme.color(Token::Text), Color::Reset);
        assert_eq!(theme.color(Token::Accent), Color::Cyan);
        assert_eq!(theme.color(Token::TextMuted), Color::DarkGray);
        assert_eq!(theme.color(Token::Success), Color::Green);
        assert_eq!(theme.color(Token::Warning), Color::Yellow);
        assert_eq!(theme.color(Token::Error), Color::Red);
        assert_eq!(theme.color(Token::Missing), Color::DarkGray);
        assert!(theme.selection_reversed());
        assert!(theme.is_bold(Token::Error));
        assert!(!theme.is_bold(Token::Success));
    }

    // themes.md, ink: the token values of the table
    #[test]
    fn ink_has_the_kanso_values() {
        let theme = Theme::builtin("ink").unwrap();
        assert_eq!(theme.color(Token::Background), rgb("#14171d"));
        assert_eq!(theme.color(Token::Text), rgb("#C5C9C7"));
        assert_eq!(theme.color(Token::TextMuted), rgb("#909398"));
        assert_eq!(theme.color(Token::Accent), rgb("#7FB4CA"));
        assert_eq!(theme.color(Token::Border), rgb("#4b4e57"));
        assert_eq!(theme.color(Token::SelectionBg), rgb("#393B44"));
        assert_eq!(theme.color(Token::Error), rgb("#C34043"));
        assert_eq!(theme.color(Token::Path), rgb("#8ea4a2"));
        assert!(!theme.selection_reversed());
        assert!(!theme.is_bold(Token::Success));
    }

    // themes.md, mist: background, border and selection differ from ink
    #[test]
    fn mist_has_the_kanso_values() {
        let theme = Theme::builtin("mist").unwrap();
        assert_eq!(theme.color(Token::Background), rgb("#22262D"));
        assert_eq!(theme.color(Token::Border), rgb("#5C6066"));
        assert_eq!(theme.color(Token::SelectionBg), rgb("#393B44"));
        assert_eq!(theme.color(Token::Warning), rgb("#DCA561"));
    }

    // themes.md, zen: the border is zen's own shade
    #[test]
    fn zen_has_the_kanso_values() {
        let theme = Theme::builtin("zen").unwrap();
        assert_eq!(theme.color(Token::Background), rgb("#090E13"));
        assert_eq!(theme.color(Token::Border), rgb("#393B44"));
        assert_eq!(theme.color(Token::SessionNone), rgb("#909398"));
    }

    // themes.md, pearl: light values, and the low-contrast text states are bold
    #[test]
    fn pearl_has_the_kanso_values_and_bold_low_contrast_states() {
        let theme = Theme::builtin("pearl").unwrap();
        assert_eq!(theme.color(Token::Background), rgb("#f2f1ef"));
        assert_eq!(theme.color(Token::Text), rgb("#22262D"));
        assert_eq!(theme.color(Token::Accent), rgb("#4d699b"));
        assert_eq!(theme.color(Token::Border), rgb("#545464"));
        assert_eq!(theme.color(Token::SelectionBg), rgb("#dddddb"));
        assert_eq!(theme.color(Token::Warning), rgb("#77713f"));
        assert_eq!(theme.color(Token::Path), rgb("#4e8ca2"));
        for token in [
            Token::Success,
            Token::Warning,
            Token::Path,
            Token::SessionActive,
            Token::Error,
        ] {
            assert!(theme.is_bold(token), "{token:?}");
        }
        assert!(!theme.is_bold(Token::Accent));
        assert!(!theme.is_bold(Token::Text));
    }

    // FR-CONF-02, P-60: an override replaces that one token and keeps the rest of the theme
    #[test]
    fn override_changes_only_its_token() {
        let base = Theme::builtin("ink").unwrap();
        let resolved = resolve(
            Some("ink"),
            &pairs(&[("accent", "#112233")]),
            ColourLevel::TrueColour,
        );
        assert!(resolved.warnings.is_empty());
        assert_eq!(resolved.theme.color(Token::Accent), rgb("#112233"));
        for token in Token::ALL {
            if token != Token::Accent {
                assert_eq!(resolved.theme.color(token), base.color(token), "{token:?}");
            }
        }
    }

    // P-60, FR-CONF-09: an unknown theme name warns with the P-60 text and uses default
    #[test]
    fn unknown_theme_warns_and_uses_default() {
        let resolved = resolve(Some("neon"), &[], ColourLevel::TrueColour);
        assert_eq!(resolved.theme, Theme::default());
        assert_eq!(
            resolved.warnings,
            vec!["Warning: theme 'neon' is unknown; using default.".to_string()]
        );
    }

    // P-60: no name is the default theme, with no warning
    #[test]
    fn missing_theme_is_default_without_warning() {
        let resolved = resolve(None, &[], ColourLevel::TrueColour);
        assert_eq!(resolved.theme, Theme::default());
        assert!(resolved.warnings.is_empty());
    }

    // P-60, FR-CONF-09: a name that is not a token, and a value that is not #rrggbb, warn and change nothing
    #[test]
    fn bad_override_warns_and_keeps_the_theme_colour() {
        let resolved = resolve(
            Some("ink"),
            &pairs(&[
                ("acent", "#112233"),
                ("accent", "blue"),
                ("border", "#12345"),
            ]),
            ColourLevel::TrueColour,
        );
        assert_eq!(
            resolved.warnings,
            vec![
                "Warning: theme_colors 'acent' is not a colour token; ignored.".to_string(),
                "Warning: theme_colors 'blue' is not a #rrggbb colour; ignored.".to_string(),
                "Warning: theme_colors '#12345' is not a #rrggbb colour; ignored.".to_string(),
            ]
        );
        assert_eq!(resolved.theme, Theme::builtin("ink").unwrap());
    }

    // P-60: the hex form is #rrggbb in either case, and only that
    #[test]
    fn hex_values_are_checked_strictly() {
        assert_eq!(parse_hex("#7fb4ca"), Some(Color::Rgb(0x7f, 0xb4, 0xca)));
        assert_eq!(parse_hex("#7FB4CA"), Some(Color::Rgb(0x7f, 0xb4, 0xca)));
        assert_eq!(parse_hex("7fb4ca"), None);
        assert_eq!(parse_hex("#7fb4c"), None);
        assert_eq!(parse_hex("#7fb4cg"), None);
        assert_eq!(parse_hex("#7fb4ca "), None);
    }

    // design 7: COLORTERM truecolor or 24bit gives truecolour; TERM with 256color gives indexes; else ANSI
    #[test]
    fn colour_level_follows_colorterm_and_term() {
        assert_eq!(
            ColourLevel::detect(Some("truecolor"), Some("xterm")),
            ColourLevel::TrueColour
        );
        assert_eq!(
            ColourLevel::detect(Some("24bit"), None),
            ColourLevel::TrueColour
        );
        assert_eq!(
            ColourLevel::detect(None, Some("xterm-256color")),
            ColourLevel::Indexed
        );
        assert_eq!(ColourLevel::detect(None, Some("xterm")), ColourLevel::Ansi);
        assert_eq!(ColourLevel::detect(None, None), ColourLevel::Ansi);
    }

    // design 7, themes.md: the 256-colour indexes of the table
    #[test]
    fn nearest_index_matches_the_table() {
        let index = |text: &str| {
            let Color::Rgb(r, g, b) = rgb(text) else {
                panic!()
            };
            nearest_index(r, g, b)
        };
        assert_eq!(index("#C5C9C7"), 251);
        assert_eq!(index("#909398"), 246);
        assert_eq!(index("#7FB4CA"), 110);
        assert_eq!(index("#4b4e57"), 239);
        assert_eq!(index("#C34043"), 131);
        assert_eq!(index("#8ea4a2"), 247);
        assert_eq!(index("#22262D"), 235);
        assert_eq!(index("#4d699b"), 60);
        assert_eq!(index("#dddddb"), 253);
    }

    // design 7: on an indexed terminal the theme holds indexes; on the ANSI level the default names
    #[test]
    fn levels_map_the_palette() {
        let ink = Theme::builtin("ink").unwrap();
        let indexed = ink.at_level(ColourLevel::Indexed);
        assert_eq!(indexed.color(Token::Text), Color::Indexed(251));
        assert_eq!(indexed.color(Token::Background), Color::Indexed(234));
        assert_eq!(ink.at_level(ColourLevel::TrueColour), ink);
        assert_eq!(ink.at_level(ColourLevel::Ansi), Theme::default());
    }

    // design 6.3: on a 16-colour terminal the token falls back to the default ANSI name, so an override is not shown
    #[test]
    fn ansi_level_drops_overrides() {
        let resolved = resolve(
            Some("pearl"),
            &pairs(&[("accent", "#112233")]),
            ColourLevel::Ansi,
        );
        assert_eq!(resolved.theme, Theme::default());
        assert!(resolved.warnings.is_empty());
    }
}

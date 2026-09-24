//! Color tokens. The default `terminal` theme wears the terminal's own palette;
//! the rest are ports of popular IDE themes. Every theme states a handful of
//! colors and derives the rest.

#[cfg(unix)]
use std::fmt::Write as _;

use ratatui::style::Color;

/// The theme to paint with: the terminal's own palette, or one of ten published
/// schemes. `name()` is what `config.json` stores, so these strings are fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    Terminal,
    Dracula,
    Nord,
    TokyoNight,
    Catppuccin,
    OneDark,
    Gruvbox,
    Monokai,
    NightOwl,
    SolarizedDark,
    GithubLight,
}

pub const THEME_CHOICES: [ThemeName; 11] = [
    ThemeName::Terminal,
    ThemeName::Dracula,
    ThemeName::Nord,
    ThemeName::TokyoNight,
    ThemeName::Catppuccin,
    ThemeName::OneDark,
    ThemeName::Gruvbox,
    ThemeName::Monokai,
    ThemeName::NightOwl,
    ThemeName::SolarizedDark,
    ThemeName::GithubLight,
];

/// Stands in for the `terminal` theme until the terminal answers, or if it never does.
pub const FALLBACK: ThemeName = ThemeName::Nord;

#[derive(Clone, Copy, Debug)]
pub struct ColorScheme {
    pub bg: &'static str,
    pub fg: &'static str,
    pub red: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub blue: &'static str,
    pub magenta: &'static str,
    pub cyan: &'static str,
    pub orange: &'static str,
}
// The ten schemes, as published: nine colours each, named at the point of use.

/// The dracula scheme.
const DRACULA: ColorScheme = ColorScheme {
    bg: "#282a36",
    fg: "#f8f8f2",
    red: "#ff5555",
    green: "#50fa7b",
    yellow: "#f1fa8c",
    blue: "#6272a4",
    magenta: "#ff79c6",
    cyan: "#8be9fd",
    orange: "#ffb86c",
};

/// The nord scheme.
const NORD: ColorScheme = ColorScheme {
    bg: "#2e3440",
    fg: "#d8dee9",
    red: "#bf616a",
    green: "#a3be8c",
    yellow: "#ebcb8b",
    blue: "#81a1c1",
    magenta: "#b48ead",
    cyan: "#88c0d0",
    orange: "#d08770",
};

/// The tokyo-night scheme.
const TOKYO_NIGHT: ColorScheme = ColorScheme {
    bg: "#1a1b26",
    fg: "#c0caf5",
    red: "#f7768e",
    green: "#9ece6a",
    yellow: "#e0af68",
    blue: "#7aa2f7",
    magenta: "#bb9af7",
    cyan: "#7dcfff",
    orange: "#ff9e64",
};

/// The catppuccin scheme.
const CATPPUCCIN: ColorScheme = ColorScheme {
    bg: "#1e1e2e",
    fg: "#cdd6f4",
    red: "#f38ba8",
    green: "#a6e3a1",
    yellow: "#f9e2af",
    blue: "#89b4fa",
    magenta: "#cba6f7",
    cyan: "#94e2d5",
    orange: "#fab387",
};

/// The one-dark scheme.
const ONE_DARK: ColorScheme = ColorScheme {
    bg: "#282c34",
    fg: "#abb2bf",
    red: "#e06c75",
    green: "#98c379",
    yellow: "#e5c07b",
    blue: "#61afef",
    magenta: "#c678dd",
    cyan: "#56b6c2",
    orange: "#d19a66",
};

/// The gruvbox scheme.
const GRUVBOX: ColorScheme = ColorScheme {
    bg: "#282828",
    fg: "#ebdbb2",
    red: "#fb4934",
    green: "#b8bb26",
    yellow: "#fabd2f",
    blue: "#83a598",
    magenta: "#d3869b",
    cyan: "#8ec07c",
    orange: "#fe8019",
};

/// The monokai scheme.
const MONOKAI: ColorScheme = ColorScheme {
    bg: "#272822",
    fg: "#f8f8f2",
    red: "#f92672",
    green: "#a6e22e",
    yellow: "#e6db74",
    blue: "#66d9ef",
    magenta: "#ae81ff",
    cyan: "#66d9ef",
    orange: "#fd971f",
};

/// The night-owl scheme.
const NIGHT_OWL: ColorScheme = ColorScheme {
    bg: "#011627",
    fg: "#d6deeb",
    red: "#ef5350",
    green: "#22da6e",
    yellow: "#c5e478",
    blue: "#82aaff",
    magenta: "#c792ea",
    cyan: "#21c7a8",
    orange: "#f78c6c",
};

/// The solarized-dark scheme.
const SOLARIZED_DARK: ColorScheme = ColorScheme {
    bg: "#002b36",
    fg: "#93a1a1",
    red: "#dc322f",
    green: "#859900",
    yellow: "#b58900",
    blue: "#268bd2",
    magenta: "#d33682",
    cyan: "#2aa198",
    orange: "#cb4b16",
};

/// The github-light scheme.
const GITHUB_LIGHT: ColorScheme = ColorScheme {
    bg: "#ffffff",
    fg: "#24292f",
    red: "#cf222e",
    green: "#1a7f37",
    yellow: "#9a6700",
    blue: "#0969da",
    magenta: "#8250df",
    cyan: "#1b7c83",
    orange: "#bc4c00",
};

impl ThemeName {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo-night",
            Self::Catppuccin => "catppuccin",
            Self::OneDark => "one-dark",
            Self::Gruvbox => "gruvbox",
            Self::Monokai => "monokai",
            Self::NightOwl => "night-owl",
            Self::SolarizedDark => "solarized-dark",
            Self::GithubLight => "github-light",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        THEME_CHOICES.into_iter().find(|t| t.name() == value)
    }

    /// The scheme's own colours. `terminal` has none, so it stands in with
    /// [`FALLBACK`] and is overridden later from what the terminal reported.
    #[must_use]
    pub const fn scheme(self) -> ColorScheme {
        match self {
            Self::Terminal => FALLBACK.scheme(),
            Self::Dracula => DRACULA,
            Self::Nord => NORD,
            Self::TokyoNight => TOKYO_NIGHT,
            Self::Catppuccin => CATPPUCCIN,
            Self::OneDark => ONE_DARK,
            Self::Gruvbox => GRUVBOX,
            Self::Monokai => MONOKAI,
            Self::NightOwl => NIGHT_OWL,
            Self::SolarizedDark => SOLARIZED_DARK,
            Self::GithubLight => GITHUB_LIGHT,
        }
    }
}

/// A palette reply is a few hundred bytes; past this it is not one, and parsing it
/// again on every poll would cost more than the whole startup wait.
const OSC_REPLY_LIMIT: usize = 64 * 1024;

/// How long startup waits for the terminal's answer before painting in the stand-in scheme.
pub const PALETTE_WAIT_MS: u64 = 300;

/// The terminal's own colors, read with OSC 10/11/4. `ansi` is the 16-color palette.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerminalColors {
    pub fg: String,
    pub bg: String,
    /// Entries the terminal did not report are `None`.
    pub ansi: Vec<Option<String>>,
}

impl TerminalColors {
    /// A palette is used only when the answer is complete: a foreground, a
    /// background and all 16 ANSI colours.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.fg.is_empty()
            && !self.bg.is_empty()
            && self.ansi.len() == 16
            && self.ansi.iter().all(Option::is_some)
    }
}

/// Every color a theme paints with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub bg_alt: Color,
    pub grid: Color,
    pub fg: Color,
    pub highlight: Color,
    pub selection_bg: Color,
    pub tb_bg: Color,
    pub tb_border: Color,
    pub tb_label: Color,
    pub tb_hover: Color,
    pub tb_hover_bg: Color,
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub menu_active_bg: Color,
    pub menu_active_fg: Color,
    /// The status bar wears the scheme's blue; its label colour is picked for contrast.
    pub status_bg: Color,
    pub status_fg: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub purple: Color,
    pub orange: Color,
    pub cyan: Color,
    pub undo: Color,
    pub redo: Color,
    pub disabled: Color,
}

fn parse_hex(h: &str) -> (u8, u8, u8) {
    let v = h.trim_start_matches('#');
    let expanded = if v.len() == 3 {
        v.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        v.to_string()
    };
    let at = |i: usize| -> u8 {
        expanded
            .get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0)
    };
    (at(0), at(2), at(4))
}

/// How far a derived grey moves from the background toward the text, out of
/// [`MIX_FULL`]. Each is a fraction of the way across: 0.04, 0.07, 0.14, 0.20,
/// 0.26, 0.32, 0.35, 0.55.
const MIX_FULL: u32 = 256;
const MIX_CHECKER: u32 = 10;
const MIX_GRID: u32 = 18;
const MIX_HIGHLIGHT: u32 = 36;
const MIX_HOVER_BG: u32 = 51;
const MIX_SELECTION: u32 = 67;
const MIX_BORDER: u32 = 82;
const MIX_DISABLED: u32 = 90;
const MIX_MUTED: u32 = 141;

/// Blends two hex colors; `weight` is how far to move from `a` toward `b`, out of
/// [`MIX_FULL`]. Integer arithmetic, so a derived grey is exactly reproducible.
fn mix(a: &str, b: &str, weight: u32) -> Color {
    let (ar, ag, ab) = parse_hex(a);
    let (br, bg, bb) = parse_hex(b);
    let chan = |x: u8, y: u8| -> u8 {
        // Rounded, not truncated: the nearest 1/256th, like the float blend did.
        let v =
            (u32::from(x) * (MIX_FULL - weight) + u32::from(y) * weight + MIX_FULL / 2) / MIX_FULL;
        u8::try_from(v).unwrap_or(u8::MAX)
    };
    Color::Rgb(chan(ar, br), chan(ag, bg), chan(ab, bb))
}

fn rgb(hex: &str) -> Color {
    let (r, g, b) = parse_hex(hex);
    Color::Rgb(r, g, b)
}

/// Not a colour: the chrome fields `palette()` derives once it knows the surfaces.
const UNSET: Color = Color::Reset;

/// Black or white, whichever reads better on `bg`. The menu and status bars put
/// text on a colour that is not the scheme's text/background pair, so the label
/// cannot just inherit `fg` — schemes such as one-dark or catppuccin would give
/// it well under 2:1. Falls back to `fallback` when the colour is not concrete.
fn readable_on(bg: Color, fallback: Color) -> Color {
    let Color::Rgb(r, g, b) = bg else {
        return fallback;
    };
    let channel = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126f64.mul_add(
        channel(r),
        0.7152f64.mul_add(channel(g), 0.0722 * channel(b)),
    );
    // White beats black as a background once the luminance passes 0.179.
    if luminance < 0.179 {
        Color::White
    } else {
        Color::Black
    }
}

/// The eight colors a scheme is built from, resolved to owned hex strings so the
/// static schemes and the terminal's reported palette share one shape.
struct Resolved {
    bg: String,
    fg: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    magenta: String,
    cyan: String,
    orange: String,
}

impl From<ColorScheme> for Resolved {
    fn from(c: ColorScheme) -> Self {
        Self {
            bg: c.bg.to_string(),
            fg: c.fg.to_string(),
            red: c.red.to_string(),
            green: c.green.to_string(),
            yellow: c.yellow.to_string(),
            blue: c.blue.to_string(),
            magenta: c.magenta.to_string(),
            cyan: c.cyan.to_string(),
            orange: c.orange.to_string(),
        }
    }
}

/// Greys are blends toward the text color, so one rule fits dark and light schemes alike.
fn tokens(c: &Resolved, inherit: bool) -> Palette {
    let dim = |weight: u32| mix(&c.bg, &c.fg, weight);
    // Only the inherited theme leaves the background to the terminal; a scheme paints its own.
    let owned = |hex: &str| if inherit { Color::Reset } else { rgb(hex) };
    Palette {
        bg: owned(&c.bg),
        bg_alt: dim(MIX_CHECKER),
        grid: dim(MIX_GRID),
        fg: rgb(&c.fg),
        highlight: dim(MIX_HIGHLIGHT),
        selection_bg: dim(MIX_SELECTION),
        tb_bg: owned(&c.bg),
        tb_border: dim(MIX_BORDER),
        tb_label: dim(MIX_MUTED),
        tb_hover: rgb(&c.fg),
        tb_hover_bg: dim(MIX_HOVER_BG),
        // `palette()` derives these five once the terminal's own colours are in,
        // so they are unset here: nothing in this literal can be mistaken for the
        // menu bar's colour.
        menu_bg: UNSET,
        menu_fg: UNSET,
        menu_active_bg: UNSET,
        menu_active_fg: UNSET,
        status_bg: rgb(&c.blue),
        status_fg: UNSET,
        text: rgb(&c.fg),
        muted: dim(MIX_MUTED),
        accent: rgb(&c.cyan),
        success: rgb(&c.green),
        warning: rgb(&c.yellow),
        danger: rgb(&c.red),
        purple: rgb(&c.magenta),
        orange: rgb(&c.orange),
        cyan: rgb(&c.cyan),
        undo: rgb(&c.green),
        redo: rgb(&c.red),
        disabled: dim(MIX_DISABLED),
    }
}

/// The terminal's palette as a scheme. Colors it did not report fall back to the stand-in theme.
fn terminal_scheme(term: &TerminalColors) -> Resolved {
    let f = FALLBACK.scheme();
    let at = |i: usize, fallback: &str| -> String {
        term.ansi
            .get(i)
            .and_then(Clone::clone)
            .unwrap_or_else(|| fallback.to_string())
    };
    Resolved {
        bg: term.bg.clone(),
        fg: term.fg.clone(),
        red: at(1, f.red),
        green: at(2, f.green),
        yellow: at(3, f.yellow),
        blue: at(4, f.blue),
        magenta: at(5, f.magenta),
        cyan: at(6, f.cyan),
        orange: at(9, f.orange),
    }
}

/// `term` is the terminal's detected palette, needed only by the `terminal` theme.
/// Without it that theme falls back to a scheme, since there is nothing to inherit.
#[must_use]
pub fn palette(name: ThemeName, term: Option<&TerminalColors>) -> Palette {
    // Only `terminal`, and only from a terminal that reported a foreground and a
    // background, has anything to inherit.
    let reported =
        term.filter(|t| name == ThemeName::Terminal && !t.fg.is_empty() && !t.bg.is_empty());
    let scheme = reported.map_or_else(|| Resolved::from(name.scheme()), terminal_scheme);
    let mut palette = tokens(&scheme, reported.is_some());
    if let Some(term) = reported {
        // The terminal reported a palette, so the chrome wears it: MS Edit's grey
        // menu/toolbar strip with black labels. Without a report the chrome stays
        // on the stand-in scheme, so it matches the canvas. `gutter` needs no
        // override — it is derived from the reported background either way.
        let at = |i: usize, fallback: &str| {
            term.ansi
                .get(i)
                .and_then(Option::as_deref)
                .map_or_else(|| rgb(fallback), rgb)
        };
        palette.status_bg = at(4, "#0000aa");
        palette.tb_bg = at(7, "#c0c0c0");
        palette.tb_border = at(8, "#555555");
        palette.tb_label = at(0, "#000000");
        palette.tb_hover = at(15, "#ffffff");
        palette.tb_hover_bg = at(4, "#0000aa");
        palette.text = at(0, "#000000");
        palette.muted = at(8, "#555555");
        palette.disabled = at(8, "#555555");
        palette.selection_bg = at(4, "#0000aa");
    }
    // Idiomatic menu-bar highlight: the selected entry inverts the strip, so the
    // label is legible on any scheme without a second colour to keep in sync.
    palette.menu_bg = palette.tb_bg;
    palette.menu_fg = readable_on(palette.menu_bg, palette.fg);
    palette.menu_active_bg = palette.menu_fg;
    palette.menu_active_fg = palette.menu_bg;
    palette.status_fg = readable_on(palette.status_bg, palette.fg);
    palette
}

// ------------------------------------------------------------------ detection

/// Reads the terminal's palette with OSC 10/11/4, best effort: a terminal that does
/// not answer gets `None` and keeps the stand-in scheme.
#[must_use]
#[cfg(unix)]
pub fn detect_terminal_colors(timeout_ms: u64) -> Option<TerminalColors> {
    unix_osc_query(timeout_ms)
}

#[cfg(not(unix))]
pub const fn detect_terminal_colors(_timeout_ms: u64) -> Option<TerminalColors> {
    None
}

#[cfg(unix)]
fn unix_osc_query(timeout_ms: u64) -> Option<TerminalColors> {
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    let query = concat!(
        "\x1b]4;0;?;1;?;2;?;3;?;4;?;5;?;6;?;7;?\x07",
        "\x1b]4;8;?;9;?;10;?;11;?;12;?;13;?;14;?;15;?\x07",
        "\x1b]10;?\x07",
        "\x1b]11;?\x07",
    );
    let mut out = std::io::stdout();
    out.write_all(query.as_bytes()).ok()?;
    out.flush().ok()?;

    let fd = std::io::stdin().as_raw_fd();
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        if let Some(colors) = parse_osc_colors(&buf) {
            if colors.ansi.iter().all(Option::is_some) {
                // All 16 ANSI colours are in; OSC 10/11 would have arrived in the
                // same reply, so either the palette is usable or it never will be.
                return colors.is_complete().then_some(colors);
            }
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let wait = i32::try_from(left.as_millis().clamp(1, 50)).unwrap_or(50);
        let mut fds = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(std::ptr::from_mut(&mut fds), 1, wait) };
        if ready <= 0 {
            continue;
        }
        let mut chunk = [0u8; 2048];
        let read = unsafe { libc::read(fd, chunk.as_mut_ptr().cast(), chunk.len()) };
        let Ok(n) = usize::try_from(read) else {
            break;
        };
        if buf.len() >= OSC_REPLY_LIMIT {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    parse_osc_colors(&buf).filter(TerminalColors::is_complete)
}

/// Extracts `OSC 10;`, `OSC 11;` and `OSC 4;<index>;` replies from raw terminal bytes.
///
/// Only the unix query reads a terminal directly; elsewhere the palette is whatever
/// the caller passed in.
#[cfg(unix)]
fn parse_osc_colors(raw: &[u8]) -> Option<TerminalColors> {
    // tmux passthrough doubles the ESC of the wrapped sequence.
    let text: String = String::from_utf8_lossy(raw).replace("\u{1b}\u{1b}", "\u{1b}");
    let mut fg = None;
    let mut bg = None;
    let mut ansi: Vec<Option<String>> = vec![None; 16];
    for part in text.split('\u{1b}').skip(1) {
        let Some(body) = part.strip_prefix(']') else {
            continue;
        };
        let body = body.split(['\u{7}', '\u{1b}']).next().unwrap_or("");
        let mut fields = body.split(';');
        let Some(code) = fields.next() else { continue };
        let (slot, value) = match code {
            "10" => (&mut fg, fields.next()),
            "11" => (&mut bg, fields.next()),
            "4" => {
                let index = fields.next().and_then(|i| i.parse::<usize>().ok());
                let value = fields.next();
                match index {
                    Some(i) if i < 16 => (&mut ansi[i], value),
                    _ => continue,
                }
            }
            _ => continue,
        };
        if let Some(hex) = value.and_then(rgb_spec_to_hex) {
            *slot = Some(hex);
        }
    }
    match (fg, bg) {
        (Some(fg), Some(bg)) => Some(TerminalColors { fg, bg, ansi }),
        _ => None,
    }
}

/// `rgb:RRRR/GGGG/BBBB` (1-4 hex digits each) or `#rrggbb` -> `#rrggbb`.
#[cfg(unix)]
fn rgb_spec_to_hex(spec: &str) -> Option<String> {
    if let Some(rest) = spec.strip_prefix('#') {
        let clean: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
        if clean.len() == 6 {
            return Some(format!("#{}", clean.to_lowercase()));
        }
        return None;
    }
    let rest = spec.strip_prefix("rgb:")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let mut out = String::from("#");
    for part in parts {
        let value = u32::from_str_radix(part, 16).ok()?;
        let scaled = match part.len() {
            1 => value * 17,
            2 => value,
            3 => value >> 4,
            _ => value >> 8,
        };
        write!(out, "{:02x}", scaled.min(255)).expect("writing to a String cannot fail");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(c: Color) -> String {
        match c {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            other => format!("{other:?}"),
        }
    }

    /// WCAG relative contrast between two concrete colours.
    fn contrast(one: Color, other: Color) -> f64 {
        let lum = |c: Color| {
            // The bars label themselves with the terminal's own black/white.
            let c = match c {
                Color::Black => Color::Rgb(0, 0, 0),
                Color::White => Color::Rgb(255, 255, 255),
                c => c,
            };
            let Color::Rgb(r, g, b) = c else {
                panic!("not rgb: {c:?}")
            };
            let channel = |c: u8| {
                let c = f64::from(c) / 255.0;
                if c <= 0.03928 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126f64.mul_add(
                channel(r),
                0.7152f64.mul_add(channel(g), 0.0722 * channel(b)),
            )
        };
        let (first, second) = (lum(one), lum(other));
        let (low, high) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        (high + 0.05) / (low + 0.05)
    }

    fn term() -> TerminalColors {
        TerminalColors {
            fg: "#ffffff".to_string(),
            bg: "#000000".to_string(),
            ansi: vec![
                Some("#000000".into()),
                Some("#ff0000".into()),
                Some("#00ff00".into()),
                Some("#ffff00".into()),
                Some("#0000ff".into()),
                Some("#ff00ff".into()),
                Some("#00ffff".into()),
                Some("#c0c0c0".into()),
            ],
        }
    }

    #[test]
    fn every_theme_choice_resolves_and_the_schemes_are_distinct() {
        assert_eq!(THEME_CHOICES.len(), 11);
        assert_eq!(THEME_CHOICES[0], ThemeName::Terminal);
        let backgrounds: std::collections::HashSet<String> = THEME_CHOICES
            .iter()
            .map(|t| hex(palette(*t, None).bg))
            .collect();
        // Ten published schemes, plus `terminal` standing in with its fallback.
        assert_eq!(backgrounds.len(), 10, "{backgrounds:?}");
    }

    #[test]
    fn the_grid_is_a_faint_blend_of_background_toward_text() {
        let p = palette(ThemeName::Gruvbox, None);
        assert_eq!(hex(p.grid), "#363532");
        let (Color::Rgb(gr, _, _), Color::Rgb(fr, _, _), Color::Rgb(bgr, _, _)) =
            (p.grid, p.fg, p.bg_alt)
        else {
            panic!("rgb")
        };
        assert!(gr < fr && gr > bgr);
    }

    #[test]
    fn a_scheme_paints_its_own_background_the_terminal_theme_does_not() {
        let scheme = palette(ThemeName::Dracula, None);
        assert_eq!(hex(scheme.bg), "#282a36");
        let inherited = palette(ThemeName::Terminal, Some(&term()));
        assert_eq!(inherited.bg, Color::Reset);
        assert_eq!(inherited.tb_bg, Color::Rgb(192, 192, 192));
        assert_eq!(hex(inherited.fg), "#ffffff");
    }

    #[test]
    fn the_terminal_theme_takes_text_and_accents_from_the_terminal() {
        let p = palette(ThemeName::Terminal, Some(&term()));
        assert_eq!(hex(p.fg), "#ffffff");
        assert_eq!(hex(p.danger), "#ff0000");
        assert_eq!(hex(p.success), "#00ff00");
        assert_eq!(hex(p.accent), "#00ffff");
        assert_eq!(hex(p.grid), "#121212");
        // Only 8 of 16 reported, so `orange` (bright red, index 9) falls back.
        assert_eq!(hex(p.orange), FALLBACK.scheme().orange);
    }

    #[test]
    fn with_no_terminal_colors_the_terminal_theme_stands_in_with_a_scheme() {
        let p = palette(ThemeName::Terminal, None);
        assert_eq!(hex(p.bg), FALLBACK.scheme().bg);
    }

    #[test]
    fn terminal_chrome_uses_the_reported_ansi_palette() {
        let p = palette(ThemeName::Terminal, Some(&term()));
        assert_eq!(p.menu_bg, Color::Rgb(192, 192, 192));
        assert_eq!(p.menu_fg, Color::Black);
        assert_eq!(p.menu_active_bg, Color::Black);
        assert_eq!(p.menu_active_fg, Color::Rgb(192, 192, 192));
        assert_eq!(p.status_bg, Color::Rgb(0, 0, 255));
        assert_eq!(p.status_fg, Color::White);
        assert_eq!(p.selection_bg, Color::Rgb(0, 0, 255));
    }

    /// Without a report the chrome must stay on the stand-in scheme. Freezing the
    /// MS Edit greys here would leave a Nord canvas wearing grey chrome.
    #[test]
    fn an_unreported_terminal_keeps_one_scheme_for_canvas_and_chrome() {
        let nord = FALLBACK.scheme();
        let p = palette(ThemeName::Terminal, None);
        assert_eq!(p.bg, Color::Rgb(0x2e, 0x34, 0x40));
        assert_eq!(p.tb_bg, p.bg);
        assert_eq!(hex(p.menu_bg), nord.bg);
        assert_ne!(p.menu_bg, Color::Rgb(192, 192, 192));
    }

    /// Every theme must clear 4.5:1 on both bars. `blue` was a dead scheme field
    /// before this, so its hexes were never chosen to hold text.
    #[test]
    fn both_bars_stay_legible_on_every_theme() {
        let themes = THEME_CHOICES
            .iter()
            .map(|t| (*t, palette(*t, Some(&term()))));
        for (theme, p) in themes {
            for (what, fg, bg) in [
                ("menu", p.menu_fg, p.menu_bg),
                ("status", p.status_fg, p.status_bg),
                ("menu active", p.menu_active_fg, p.menu_active_bg),
            ] {
                let ratio = contrast(fg, bg);
                assert!(ratio >= 4.5, "{theme:?} {what}: only {ratio:.2}:1");
            }
        }
    }

    #[test]
    fn partial_terminal_palette_keeps_the_fallback() {
        let mut colors = term();
        assert!(!colors.is_complete());
        colors.ansi.resize(16, Some("#000000".into()));
        assert!(colors.is_complete());
        colors.ansi[15] = None;
        assert!(!colors.is_complete());
    }

    #[cfg(unix)]
    #[test]
    fn osc_replies_are_parsed_in_both_terminator_styles() {
        let raw = b"\x1b]10;rgb:ffff/ffff/ffff\x07\x1b]11;rgb:0000/0000/0000\x1b\\\x1b]4;1;rgb:ff/00/00\x07";
        let c = parse_osc_colors(raw).expect("fg and bg");
        assert_eq!(c.fg, "#ffffff");
        assert_eq!(c.bg, "#000000");
        assert_eq!(c.ansi[1].as_deref(), Some("#ff0000"));
    }
}

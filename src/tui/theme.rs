//! Color tokens. The default `terminal` theme wears the terminal's own palette;
//! the rest are ports of popular IDE themes. Every theme states a handful of
//! colors and derives the rest.

use ratatui::style::Color;

/// Ten IDE themes, each sampled from its published palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemeName {
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

pub const SCHEME_NAMES: [SchemeName; 10] = [
    SchemeName::Dracula,
    SchemeName::Nord,
    SchemeName::TokyoNight,
    SchemeName::Catppuccin,
    SchemeName::OneDark,
    SchemeName::Gruvbox,
    SchemeName::Monokai,
    SchemeName::NightOwl,
    SchemeName::SolarizedDark,
    SchemeName::GithubLight,
];

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

impl SchemeName {
    pub fn name(self) -> &'static str {
        match self {
            SchemeName::Dracula => "dracula",
            SchemeName::Nord => "nord",
            SchemeName::TokyoNight => "tokyo-night",
            SchemeName::Catppuccin => "catppuccin",
            SchemeName::OneDark => "one-dark",
            SchemeName::Gruvbox => "gruvbox",
            SchemeName::Monokai => "monokai",
            SchemeName::NightOwl => "night-owl",
            SchemeName::SolarizedDark => "solarized-dark",
            SchemeName::GithubLight => "github-light",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        SCHEME_NAMES.into_iter().find(|s| s.name() == value)
    }

    pub fn scheme(self) -> ColorScheme {
        match self {
            SchemeName::Dracula => ColorScheme {
                bg: "#282a36",
                fg: "#f8f8f2",
                red: "#ff5555",
                green: "#50fa7b",
                yellow: "#f1fa8c",
                blue: "#6272a4",
                magenta: "#ff79c6",
                cyan: "#8be9fd",
                orange: "#ffb86c",
            },
            SchemeName::Nord => ColorScheme {
                bg: "#2e3440",
                fg: "#d8dee9",
                red: "#bf616a",
                green: "#a3be8c",
                yellow: "#ebcb8b",
                blue: "#81a1c1",
                magenta: "#b48ead",
                cyan: "#88c0d0",
                orange: "#d08770",
            },
            SchemeName::TokyoNight => ColorScheme {
                bg: "#1a1b26",
                fg: "#c0caf5",
                red: "#f7768e",
                green: "#9ece6a",
                yellow: "#e0af68",
                blue: "#7aa2f7",
                magenta: "#bb9af7",
                cyan: "#7dcfff",
                orange: "#ff9e64",
            },
            SchemeName::Catppuccin => ColorScheme {
                bg: "#1e1e2e",
                fg: "#cdd6f4",
                red: "#f38ba8",
                green: "#a6e3a1",
                yellow: "#f9e2af",
                blue: "#89b4fa",
                magenta: "#cba6f7",
                cyan: "#94e2d5",
                orange: "#fab387",
            },
            SchemeName::OneDark => ColorScheme {
                bg: "#282c34",
                fg: "#abb2bf",
                red: "#e06c75",
                green: "#98c379",
                yellow: "#e5c07b",
                blue: "#61afef",
                magenta: "#c678dd",
                cyan: "#56b6c2",
                orange: "#d19a66",
            },
            SchemeName::Gruvbox => ColorScheme {
                bg: "#282828",
                fg: "#ebdbb2",
                red: "#fb4934",
                green: "#b8bb26",
                yellow: "#fabd2f",
                blue: "#83a598",
                magenta: "#d3869b",
                cyan: "#8ec07c",
                orange: "#fe8019",
            },
            SchemeName::Monokai => ColorScheme {
                bg: "#272822",
                fg: "#f8f8f2",
                red: "#f92672",
                green: "#a6e22e",
                yellow: "#e6db74",
                blue: "#66d9ef",
                magenta: "#ae81ff",
                cyan: "#66d9ef",
                orange: "#fd971f",
            },
            SchemeName::NightOwl => ColorScheme {
                bg: "#011627",
                fg: "#d6deeb",
                red: "#ef5350",
                green: "#22da6e",
                yellow: "#c5e478",
                blue: "#82aaff",
                magenta: "#c792ea",
                cyan: "#21c7a8",
                orange: "#f78c6c",
            },
            SchemeName::SolarizedDark => ColorScheme {
                bg: "#002b36",
                fg: "#93a1a1",
                red: "#dc322f",
                green: "#859900",
                yellow: "#b58900",
                blue: "#268bd2",
                magenta: "#d33682",
                cyan: "#2aa198",
                orange: "#cb4b16",
            },
            SchemeName::GithubLight => ColorScheme {
                bg: "#ffffff",
                fg: "#24292f",
                red: "#cf222e",
                green: "#1a7f37",
                yellow: "#9a6700",
                blue: "#0969da",
                magenta: "#8250df",
                cyan: "#1b7c83",
                orange: "#bc4c00",
            },
        }
    }
}

/// `terminal` wears the terminal's own palette; the rest are the schemes above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    Terminal,
    Scheme(SchemeName),
}

pub const THEME_CHOICES: [ThemeName; 11] = [
    ThemeName::Terminal,
    ThemeName::Scheme(SchemeName::Dracula),
    ThemeName::Scheme(SchemeName::Nord),
    ThemeName::Scheme(SchemeName::TokyoNight),
    ThemeName::Scheme(SchemeName::Catppuccin),
    ThemeName::Scheme(SchemeName::OneDark),
    ThemeName::Scheme(SchemeName::Gruvbox),
    ThemeName::Scheme(SchemeName::Monokai),
    ThemeName::Scheme(SchemeName::NightOwl),
    ThemeName::Scheme(SchemeName::SolarizedDark),
    ThemeName::Scheme(SchemeName::GithubLight),
];

impl ThemeName {
    pub fn name(self) -> &'static str {
        match self {
            ThemeName::Terminal => "terminal",
            ThemeName::Scheme(s) => s.name(),
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        THEME_CHOICES.into_iter().find(|t| t.name() == value)
    }
}

/// Stands in for the `terminal` theme until the terminal answers, or if it never does.
pub const FALLBACK_SCHEME: SchemeName = SchemeName::Nord;

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

/// Every color a theme paints with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub name: ThemeName,
    pub bg: Color,
    pub bg_alt: Color,
    pub grid: Color,
    pub fg: Color,
    pub scratch: Color,
    pub highlight: Color,
    pub selection_bg: Color,
    pub tb_bg: Color,
    pub tb_border: Color,
    pub tb_label: Color,
    pub tb_hover: Color,
    pub tb_hover_bg: Color,
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

/// How far the lattice sits from the background, toward the text color.
const GRID_MIX: f64 = 0.07;
/// The checker style's alternate cell, likewise derived.
const CHECKER_MIX: f64 = 0.04;

fn parse_hex(h: &str) -> (f64, f64, f64) {
    let v = h.trim_start_matches('#');
    let expanded = if v.len() == 3 {
        v.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        v.to_string()
    };
    let at = |i: usize| -> f64 {
        expanded
            .get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0) as f64
    };
    (at(0), at(2), at(4))
}

/// Blends two hex colors; `t` is how far to move from `a` toward `b`.
fn mix(a: &str, b: &str, t: f64) -> Color {
    let (ar, ag, ab) = parse_hex(a);
    let (br, bg, bb) = parse_hex(b);
    let chan = |x: f64, y: f64| (x + (y - x) * t).round().clamp(0.0, 255.0) as u8;
    Color::Rgb(chan(ar, br), chan(ag, bg), chan(ab, bb))
}

fn rgb(hex: &str) -> Color {
    let (r, g, b) = parse_hex(hex);
    Color::Rgb(r as u8, g as u8, b as u8)
}

/// The eight colors a scheme is built from, resolved to owned hex strings so the
/// static schemes and the terminal's reported palette share one shape.
struct Resolved {
    bg: String,
    fg: String,
    red: String,
    green: String,
    yellow: String,
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
            magenta: c.magenta.to_string(),
            cyan: c.cyan.to_string(),
            orange: c.orange.to_string(),
        }
    }
}

/// Greys are blends toward the text color, so one rule fits dark and light schemes alike.
fn tokens(name: ThemeName, c: &Resolved, inherit: bool) -> Palette {
    let dim = |t: f64| mix(&c.bg, &c.fg, t);
    // Only the inherited theme leaves the background to the terminal; a scheme paints its own.
    let owned = |hex: &str| if inherit { Color::Reset } else { rgb(hex) };
    Palette {
        name,
        bg: owned(&c.bg),
        bg_alt: dim(CHECKER_MIX),
        grid: dim(GRID_MIX),
        fg: rgb(&c.fg),
        scratch: rgb(&c.fg),
        highlight: dim(0.14),
        selection_bg: dim(0.26),
        tb_bg: owned(&c.bg),
        tb_border: dim(0.32),
        tb_label: dim(0.55),
        tb_hover: rgb(&c.fg),
        tb_hover_bg: dim(0.2),
        text: rgb(&c.fg),
        muted: dim(0.55),
        accent: rgb(&c.cyan),
        success: rgb(&c.green),
        warning: rgb(&c.yellow),
        danger: rgb(&c.red),
        purple: rgb(&c.magenta),
        orange: rgb(&c.orange),
        cyan: rgb(&c.cyan),
        undo: rgb(&c.green),
        redo: rgb(&c.red),
        disabled: dim(0.35),
    }
}

/// The terminal's palette as a scheme. Colors it did not report fall back to the stand-in theme.
fn terminal_scheme(term: &TerminalColors) -> Resolved {
    let f = FALLBACK_SCHEME.scheme();
    let at = |i: usize, fallback: &str| -> String {
        term.ansi
            .get(i)
            .and_then(|c| c.clone())
            .unwrap_or_else(|| fallback.to_string())
    };
    Resolved {
        bg: term.bg.clone(),
        fg: term.fg.clone(),
        red: at(1, f.red),
        green: at(2, f.green),
        yellow: at(3, f.yellow),
        magenta: at(5, f.magenta),
        cyan: at(6, f.cyan),
        orange: at(9, f.orange),
    }
}

/// `term` is the terminal's detected palette, needed only by the `terminal` theme.
/// Without it that theme falls back to a scheme, since there is nothing to inherit.
pub fn palette(name: ThemeName, term: Option<&TerminalColors>) -> Palette {
    let inherit =
        name == ThemeName::Terminal && term.is_some_and(|t| !t.fg.is_empty() && !t.bg.is_empty());
    let scheme = if inherit {
        terminal_scheme(term.expect("checked above"))
    } else {
        Resolved::from(match name {
            ThemeName::Terminal => FALLBACK_SCHEME.scheme(),
            ThemeName::Scheme(s) => s.scheme(),
        })
    };
    tokens(name, &scheme, inherit)
}

// ------------------------------------------------------------------ detection

/// Reads the terminal's palette (OSC 10/11/4) so the `terminal` theme can inherit
/// it. Best effort: terminals that do not answer get `None` and keep the stand-in
/// scheme, so nothing here is worth failing over.
pub fn detect_terminal_colors(timeout_ms: u64) -> Option<TerminalColors> {
    #[cfg(unix)]
    {
        unix_osc_query(timeout_ms)
    }
    #[cfg(not(unix))]
    {
        let _ = timeout_ms;
        None
    }
}

#[cfg(unix)]
fn unix_osc_query(timeout_ms: u64) -> Option<TerminalColors> {
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    let mut query = String::from("\x1b]10;?\x07\x1b]11;?\x07\x1b]4;0;?");
    for i in 1..16 {
        query.push_str(&format!(";{i};?"));
    }
    query.push('\x07');
    let mut out = std::io::stdout();
    out.write_all(query.as_bytes()).ok()?;
    out.flush().ok()?;

    let fd = std::io::stdin().as_raw_fd();
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        if let Some(colors) = parse_osc_colors(&buf) {
            if !colors.ansi.iter().any(|c| c.is_none()) {
                return Some(colors);
            }
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let wait = (left.as_millis() as i32).clamp(1, 50);
        let mut fds = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fds, 1, wait) };
        if ready <= 0 {
            continue;
        }
        let mut chunk = [0u8; 2048];
        let n = unsafe { libc::read(fd, chunk.as_mut_ptr() as *mut libc::c_void, chunk.len()) };
        if n <= 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }
    parse_osc_colors(&buf)
}

/// Extracts `OSC 10;`, `OSC 11;` and `OSC 4;<index>;` replies from raw terminal bytes.
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
fn rgb_spec_to_hex(spec: &str) -> Option<String> {
    if let Some(rest) = spec.strip_prefix('#') {
        let clean: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
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
        out.push_str(&format!("{:02x}", scaled.min(255)));
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
    fn every_theme_choice_resolves_and_the_ten_schemes_are_listed() {
        assert_eq!(SCHEME_NAMES.len(), 10);
        assert_eq!(THEME_CHOICES.len(), 11);
        assert_eq!(THEME_CHOICES[0], ThemeName::Terminal);
        for t in THEME_CHOICES {
            assert_eq!(palette(t, None).name, t);
        }
    }

    #[test]
    fn the_grid_is_a_faint_blend_of_background_toward_text() {
        let p = palette(ThemeName::Scheme(SchemeName::Gruvbox), None);
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
        let scheme = palette(ThemeName::Scheme(SchemeName::Dracula), None);
        assert_eq!(hex(scheme.bg), "#282a36");
        let inherited = palette(ThemeName::Terminal, Some(&term()));
        assert_eq!(inherited.bg, Color::Reset);
        assert_eq!(inherited.tb_bg, Color::Reset);
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
        assert_eq!(hex(p.orange), FALLBACK_SCHEME.scheme().orange);
    }

    #[test]
    fn with_no_terminal_colors_the_terminal_theme_stands_in_with_a_scheme() {
        let p = palette(ThemeName::Terminal, None);
        assert_eq!(hex(p.bg), FALLBACK_SCHEME.scheme().bg);
    }

    #[test]
    fn osc_replies_are_parsed_in_both_terminator_styles() {
        let raw = b"\x1b]10;rgb:ffff/ffff/ffff\x07\x1b]11;rgb:0000/0000/0000\x1b\\\x1b]4;1;rgb:ff/00/00\x07";
        let c = parse_osc_colors(raw).expect("fg and bg");
        assert_eq!(c.fg, "#ffffff");
        assert_eq!(c.bg, "#000000");
        assert_eq!(c.ansi[1].as_deref(), Some("#ff0000"));
    }
}

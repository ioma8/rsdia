// Color tokens. The default `terminal` theme wears the terminal's own palette; the rest are
// ports of popular IDE themes. Every theme states a handful of colors and derives the rest.
import { type CliRenderer, RGBA } from "@opentui/core";

export interface ThemeTokens {
  /** In the `terminal` theme this is the terminal's own background; the hex is the blending fallback. */
  bg: string;
  /** Checker grid style only. */
  bgAlt: string;
  grid: string;
  /** Committed glyphs. */
  fg: string;
  /** In-progress glyphs (ASCIIFlow draws them in the text color). */
  scratch: string;
  /** Background behind in-progress glyphs. */
  highlight: string;
  selectionBg: string;
  /** Like `bg`. */
  tbBg: string;
  tbBorder: string;
  tbLabel: string;
  tbHover: string;
  tbHoverBg: string;
  text: string;
  muted: string;
  accent: string;
  success: string;
  warning: string;
  danger: string;
  purple: string;
  orange: string;
  cyan: string;
  undo: string;
  redo: string;
  disabled: string;
}

/** What a theme actually states. Greys, the grid and the selection are derived from `bg` and `fg`. */
export interface ColorScheme {
  bg: string;
  fg: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  orange: string;
}

/** Ten IDE themes, each sampled from its published palette. */
export const SCHEMES = {
  dracula: {
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
  nord: {
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
  "tokyo-night": {
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
  catppuccin: {
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
  "one-dark": {
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
  gruvbox: {
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
  monokai: {
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
  "night-owl": {
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
  "solarized-dark": {
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
  "github-light": {
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
} satisfies Record<string, ColorScheme>;

export type SchemeName = keyof typeof SCHEMES;
export type ThemeName = "terminal" | SchemeName;

export const SCHEME_NAMES = Object.keys(SCHEMES) as SchemeName[];

/** Stands in for the `terminal` theme until the terminal answers, or if it never does. */
export const FALLBACK_SCHEME: SchemeName = "nord";

/** How long startup waits for the terminal's answer before painting in the stand-in scheme. */
export const PALETTE_WAIT_MS = 300;

export type Palette = { readonly [K in keyof ThemeTokens]: RGBA } & { readonly name: ThemeName };

/** The terminal's own colors, read with OSC 10/11/4. `ansi` is the 16-color palette. */
export interface TerminalColors {
  fg: string;
  bg: string;
  /** Entries the terminal did not report are null. */
  ansi: (string | null)[];
}

/**
 * How far the lattice sits from the background, toward the text color. ASCIIFlow's own gridlines
 * are barely there, and a derived grid also survives whatever background the theme has.
 */
const GRID_MIX = 0.07;
/** The checker style's alternate cell, likewise derived. */
const CHECKER_MIX = 0.04;

const hex = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, "0");

/** Blends two hex colors; `t` is how far to move from `a` toward `b`. */
function mix(a: string, b: string, t: number): string {
  const [ar, ag, ab] = rgb(a);
  const [br, bg, bb] = rgb(b);
  return `#${hex(ar + (br - ar) * t)}${hex(ag + (bg - ag) * t)}${hex(ab + (bb - ab) * t)}`;
}

function rgb(h: string): [number, number, number] {
  const v = h.replace("#", "");
  const n = v.length === 3 ? [...v].map((c) => c + c).join("") : v;
  return [Number.parseInt(n.slice(0, 2), 16), Number.parseInt(n.slice(2, 4), 16), Number.parseInt(n.slice(4, 6), 16)];
}

/** Greys are blends toward the text color, so one rule fits dark and light schemes alike. */
function tokens(c: ColorScheme): ThemeTokens {
  const dim = (t: number) => mix(c.bg, c.fg, t);
  return {
    bg: c.bg,
    bgAlt: dim(CHECKER_MIX),
    grid: dim(GRID_MIX),
    fg: c.fg,
    scratch: c.fg,
    highlight: dim(0.14),
    selectionBg: dim(0.26),
    tbBg: c.bg,
    tbBorder: dim(0.32),
    tbLabel: dim(0.55),
    tbHover: c.fg,
    tbHoverBg: dim(0.2),
    text: c.fg,
    muted: dim(0.55),
    accent: c.cyan,
    success: c.green,
    warning: c.yellow,
    danger: c.red,
    purple: c.magenta,
    orange: c.orange,
    cyan: c.cyan,
    undo: c.green,
    redo: c.red,
    disabled: dim(0.35),
  };
}

/** The terminal's palette as a scheme. Colors it did not report fall back to the stand-in theme. */
function terminalScheme(c: TerminalColors): ColorScheme {
  const f = SCHEMES[FALLBACK_SCHEME];
  const at = (i: number, fallback: string) => c.ansi[i] ?? fallback;
  return {
    bg: c.bg,
    fg: c.fg,
    red: at(1, f.red),
    green: at(2, f.green),
    yellow: at(3, f.yellow),
    blue: at(4, f.blue),
    magenta: at(5, f.magenta),
    cyan: at(6, f.cyan),
    orange: at(9, f.orange),
  };
}

const cache = new Map<string, Palette>();

/** Tokens drawn with the terminal's default background (SGR 49), so the `terminal` theme is see-through. */
const BG_TOKENS = new Set(["bg", "tbBg"]);

/**
 * `term` is the terminal's detected palette, needed only by the `terminal` theme. Without it that
 * theme falls back to a scheme, since there is nothing to inherit.
 */
export function palette(name: ThemeName, term: TerminalColors | null = null): Palette {
  const inherit = name === "terminal" && term !== null;
  const key = inherit ? `terminal|${term.bg}${term.fg}${term.ansi.join("")}` : name;
  let p = cache.get(key);
  if (!p) {
    const scheme = inherit ? terminalScheme(term) : SCHEMES[name === "terminal" ? FALLBACK_SCHEME : name];
    p = Object.freeze({
      name,
      ...Object.fromEntries(
        Object.entries(tokens(scheme)).map(([k, v]) => [
          k,
          // Only the inherited theme leaves the background to the terminal; a scheme paints its own.
          inherit && BG_TOKENS.has(k) ? RGBA.defaultBackground(v) : RGBA.fromHex(v),
        ]),
      ),
    }) as Palette;
    cache.set(key, p);
  }
  return p;
}

/**
 * Reads the terminal's palette (OSC 10/11/4) so the `terminal` theme can inherit it. Best effort:
 * terminals that do not answer get null and keep the stand-in scheme. The renderer caches the
 * answer and folds concurrent calls into one query, so asking twice is free.
 */
export async function detectTerminalColors(renderer: CliRenderer): Promise<TerminalColors | null> {
  try {
    const c = await renderer.getPalette({ size: 16 });
    if (!c.defaultForeground || !c.defaultBackground) return null;
    return { fg: c.defaultForeground, bg: c.defaultBackground, ansi: c.palette.slice(0, 16) };
  } catch {
    return null;
  }
}

/** Resolves to the terminal's colors, or null if it has not answered within `ms`. */
export async function detectTerminalColorsSoon(renderer: CliRenderer, ms: number): Promise<TerminalColors | null> {
  let timer: ReturnType<typeof setTimeout>;
  const deadline = new Promise<null>((resolve) => {
    timer = setTimeout(() => resolve(null), ms);
  });
  return Promise.race([detectTerminalColors(renderer), deadline]).finally(() => clearTimeout(timer));
}

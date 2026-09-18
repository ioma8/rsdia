// Palette derivation: the faint grid, the IDE schemes, and the terminal-inherited theme.
import { describe, expect, test } from "bun:test";
import { THEME_CHOICES } from "../../src/storage/config";
import { FALLBACK_SCHEME, palette, SCHEME_NAMES, SCHEMES, type TerminalColors } from "../../src/tui/theme";

const hex = (c: { r: number; g: number; b: number }) =>
  `#${[c.r, c.g, c.b].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("")}`;

/** A white-on-black terminal reporting only the first 8 ANSI colors. */
const term: TerminalColors = {
  fg: "#ffffff",
  bg: "#000000",
  ansi: ["#000000", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#c0c0c0"],
};

describe("palette", () => {
  test("every theme choice resolves, and the ten schemes are all listed", () => {
    expect(SCHEME_NAMES).toHaveLength(10);
    expect([...THEME_CHOICES]).toEqual(["terminal", ...SCHEME_NAMES]);
    for (const name of THEME_CHOICES) expect(palette(name).name).toBe(name);
  });

  test("the grid is a faint blend of the background toward the text", () => {
    const p = palette("gruvbox");
    // 7% of the way from #282828 toward #ebdbb2.
    expect(hex(p.grid)).toBe("#363532");
    // Fainter than the text, and nearer the background than the checker cell is far from it.
    expect(p.grid.r).toBeLessThan(p.fg.r);
    expect(p.grid.r).toBeGreaterThan(p.bgAlt.r);
  });

  test("a scheme paints its own background; the terminal theme leaves it to the terminal", () => {
    const scheme = palette("dracula");
    expect(hex(scheme.bg)).toBe("#282a36");
    expect(scheme.bg.intent).toBe("rgb");

    const inherited = palette("terminal", term);
    expect(inherited.bg.intent).toBe("default");
    expect(inherited.tbBg.intent).toBe("default");
    expect(inherited.fg.intent).toBe("rgb");
  });

  test("the terminal theme takes its text and accents from the terminal", () => {
    const p = palette("terminal", term);
    expect(hex(p.fg)).toBe("#ffffff");
    expect(hex(p.danger)).toBe("#ff0000");
    expect(hex(p.success)).toBe("#00ff00");
    expect(hex(p.accent)).toBe("#00ffff");
    // Derived from the terminal's real background, not a theme's.
    expect(hex(p.grid)).toBe("#121212");
  });

  test("colors the terminal did not report fall back to the stand-in scheme", () => {
    // Only 8 of 16 reported, so `orange` (bright red, index 9) is missing.
    expect(hex(palette("terminal", term).orange)).toBe(SCHEMES[FALLBACK_SCHEME].orange);
  });

  test("with no terminal colors the terminal theme stands in with a scheme", () => {
    const p = palette("terminal");
    expect(hex(p.bg)).toBe(SCHEMES[FALLBACK_SCHEME].bg);
    // Nothing was inherited, so it paints that scheme's background rather than the terminal's.
    expect(p.bg.intent).toBe("rgb");
  });
});

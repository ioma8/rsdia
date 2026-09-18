// User config: ${XDG_CONFIG_HOME:-~/.config}/lazydraw/config.json
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { DEFAULT_EXPORT, type ExportConfig, isWrapper } from "../core/export";
import type { ThemeName } from "../tui/theme";

export type GridStyle = "lattice" | "checker" | "dots" | "off";
/** `terminal` wears the terminal's own palette; the rest are the IDE themes in `tui/theme.ts`. */
export type ThemeChoice = (typeof THEME_CHOICES)[number];

export const GRID_STYLES: readonly GridStyle[] = ["lattice", "checker", "dots", "off"];

export const THEME_CHOICES = [
  "terminal",
  "dracula",
  "nord",
  "tokyo-night",
  "catppuccin",
  "one-dark",
  "gruvbox",
  "monokai",
  "night-owl",
  "solarized-dark",
  "github-light",
] as const satisfies readonly ThemeName[];

// Fails to compile if a theme in tui/theme.ts is missing from the list above.
type ThemesListed = Exclude<ThemeName, ThemeChoice> extends never ? true : never;
const _themesListed: ThemesListed = true;

export interface Config {
  theme: ThemeChoice;
  grid: GridStyle;
  /** Whether finishing a selection puts it on the clipboard by itself (§settings). */
  copyOnSelect: boolean;
  lastDrawing: string | null;
  export: ExportConfig;
}

export const DEFAULT_CONFIG: Config = {
  theme: "terminal",
  grid: "lattice",
  copyOnSelect: false,
  lastDrawing: null,
  export: DEFAULT_EXPORT,
};

export function configPath(): string {
  return join(process.env.XDG_CONFIG_HOME || join(homedir(), ".config"), "lazydraw", "config.json");
}

export function loadConfig(path = configPath()): Config {
  let raw: Partial<Config> = {};
  try {
    raw = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return structuredClone(DEFAULT_CONFIG);
  }
  const exp: Partial<ExportConfig> = raw.export ?? {};
  return {
    theme: THEME_CHOICES.includes(raw.theme!) ? raw.theme! : DEFAULT_CONFIG.theme,
    grid: GRID_STYLES.includes(raw.grid!) ? raw.grid! : DEFAULT_CONFIG.grid,
    copyOnSelect: raw.copyOnSelect === true,
    lastDrawing: typeof raw.lastDrawing === "string" ? raw.lastDrawing : null,
    export: {
      characters: exp.characters === "basic" ? "basic" : "extended",
      wrapper: exp.wrapper && isWrapper(exp.wrapper) ? exp.wrapper : "none",
      fenced: exp.fenced === true,
    },
  };
}

export function saveConfig(config: Config, path = configPath()): void {
  try {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, `${JSON.stringify(config, null, 2)}\n`);
  } catch {
    // Config is a convenience; never crash the editor over it.
  }
}

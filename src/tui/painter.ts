// Drawing helpers over an OpenTUI buffer, plus click hotspots. OpenTUI
// synthesizes no click event, so a click is a down and an up on the same hotspot.
import { type OptimizedBuffer, RGBA, TextAttributes } from "@opentui/core";
import type { Palette } from "./theme";

export interface Hotspot {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  action: () => void;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const inRect = (r: Rect, x: number, y: number) => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;

export const ATTR = TextAttributes;

export class Painter {
  hotspots: Hotspot[] = [];
  /** Screen areas covered by UI chrome; canvas drags may not start there. */
  chrome: Rect[] = [];

  constructor(
    readonly buf: OptimizedBuffer,
    readonly pal: Palette,
    readonly width: number,
    readonly height: number,
    /** Pointer position, for hover styling. */
    readonly hover: { x: number; y: number } | null,
  ) {}

  cell(x: number, y: number, ch: string, fg: RGBA, bg: RGBA, attr = 0): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return;
    this.buf.setCell(x, y, ch, fg, bg, attr);
  }

  /** Writes single-width text, clipped to the screen. Returns the x after the text. */
  text(x: number, y: number, s: string, fg: RGBA, bg: RGBA = this.pal.tbBg, attr = 0, maxX = this.width): number {
    for (const ch of s) {
      if (x >= maxX) break;
      this.cell(x, y, ch, fg, bg, attr);
      x++;
    }
    return x;
  }

  fill(r: Rect, bg: RGBA, ch = " ", fg: RGBA = bg): void {
    for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) this.cell(x, y, ch, fg, bg);
  }

  /** Bordered panel; registers the area as chrome. */
  panel(r: Rect, border: RGBA = this.pal.tbBorder, bg: RGBA = this.pal.tbBg): void {
    this.fill(r, bg);
    const x1 = r.x + r.w - 1;
    const y1 = r.y + r.h - 1;
    for (let x = r.x + 1; x < x1; x++) {
      this.cell(x, r.y, "─", border, bg);
      this.cell(x, y1, "─", border, bg);
    }
    for (let y = r.y + 1; y < y1; y++) {
      this.cell(r.x, y, "│", border, bg);
      this.cell(x1, y, "│", border, bg);
    }
    this.cell(r.x, r.y, "┌", border, bg);
    this.cell(x1, r.y, "┐", border, bg);
    this.cell(r.x, y1, "└", border, bg);
    this.cell(x1, y1, "┘", border, bg);
    this.chrome.push(r);
  }

  hot(id: string, x: number, y: number, w: number, action: () => void, h = 1): void {
    this.hotspots.push({ id, x, y, w, h, action });
  }

  isHover(x: number, y: number, w: number, h = 1): boolean {
    return this.hover != null && inRect({ x, y, w, h }, this.hover.x, this.hover.y);
  }

  /**
   * A clickable label. Hover brightens it; `active` bolds it in `activeColor`.
   * Returns the x after the label.
   */
  button(
    id: string,
    x: number,
    y: number,
    label: string,
    action: () => void,
    opts: { fg?: RGBA; active?: boolean; activeColor?: RGBA; disabled?: boolean; bg?: RGBA } = {},
  ): number {
    const w = [...label].length;
    const bg = opts.bg ?? this.pal.tbBg;
    let fg = opts.fg ?? this.pal.tbLabel;
    let attr = 0;
    const hovered = !opts.disabled && this.isHover(x, y, w);
    if (opts.disabled) fg = this.pal.disabled;
    else if (opts.active) {
      fg = opts.activeColor ?? this.pal.text;
      attr = ATTR.BOLD;
    }
    if (hovered) fg = opts.fg && !opts.active ? opts.fg : this.pal.tbHover;
    const end = this.text(x, y, label, fg, hovered ? this.pal.tbHoverBg : bg, attr);
    if (!opts.disabled) this.hot(id, x, y, w, action);
    return end;
  }
}

export function hotspotAt(hotspots: readonly Hotspot[], x: number, y: number): Hotspot | undefined {
  // Later hotspots are drawn on top.
  for (let i = hotspots.length - 1; i >= 0; i--) if (inRect(hotspots[i]!, x, y)) return hotspots[i];
  return undefined;
}

export { RGBA };

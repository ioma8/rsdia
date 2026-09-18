import type { Canvas } from "../canvas";
import type { Vector } from "../vector";

export interface Mods {
  ctrl: boolean;
  alt: boolean;
  /** Reverse elbow orientation: Ctrl, or the `f` toggle (§6). */
  flip: boolean;
}

export const NO_MODS: Mods = { ctrl: false, alt: false, flip: false };

export function mods(partial: Partial<Mods> = {}): Mods {
  const m = { ...NO_MODS, ...partial };
  return { ...m, flip: partial.flip ?? m.ctrl };
}

/** Non-printable keys, as ASCIIFlow names them. */
export const KEY = {
  ENTER: "<enter>",
  BACKSPACE: "<backspace>",
  DELETE: "<delete>",
  UP: "<up>",
  DOWN: "<down>",
  LEFT: "<left>",
  RIGHT: "<right>",
} as const;

/** Terminal stand-in for the CSS cursor. */
export type HoverHint = "default" | "crosshair" | "move" | "resize-h" | "resize-v" | "text";

export interface ToolContext {
  readonly canvas: Canvas;
}

export interface Tool {
  start(p: Vector, m: Mods): void;
  move(p: Vector, m: Mods): void;
  end(): void;
  /** Tool switched away. */
  cleanup(): void;
  /** Returns true when the key was consumed. */
  handleKey(key: string, m: Mods): boolean;
  hoverHint(p: Vector, m: Mods): HoverHint;
}

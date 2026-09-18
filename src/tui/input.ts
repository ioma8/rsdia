// Terminal key events -> tool key names (§6).
import type { KeyEvent } from "@opentui/core";
import { KEY } from "../core/tools/tool";
import { isPlaceable } from "../core/text";

const NAMED: Record<string, string> = {
  return: KEY.ENTER,
  enter: KEY.ENTER,
  linefeed: KEY.ENTER,
  backspace: KEY.BACKSPACE,
  delete: KEY.DELETE,
  up: KEY.UP,
  down: KEY.DOWN,
  left: KEY.LEFT,
  right: KEY.RIGHT,
};

/** The printable character a key produces, if any. */
export function printable(k: KeyEvent): string | null {
  if (k.ctrl || k.meta || k.super) return null;
  if (k.name === "space") return " ";
  if (k.sequence && isPlaceable(k.sequence)) return k.sequence;
  // Kitty-protocol events may carry an escape sequence; fall back to the name.
  if ([...k.name].length === 1) {
    const ch = k.shift ? k.name.toUpperCase() : k.name;
    if (isPlaceable(ch)) return ch;
  }
  return null;
}

/** Tool key for a terminal key: a printable character or an ASCIIFlow key name. */
export function toolKey(k: KeyEvent): string | null {
  return NAMED[k.name] ?? printable(k);
}

/** Alt+1..6 arrives as ESC+digit (meta) or as option on macOS with kitty keys. */
export function altDigit(k: KeyEvent): number | null {
  if (!(k.meta || k.option) || k.ctrl) return null;
  const n = Number(k.name);
  return Number.isInteger(n) && n >= 1 && n <= 6 ? n : null;
}

export const isCtrl = (k: KeyEvent, name: string) => k.ctrl && !k.meta && k.name === name;

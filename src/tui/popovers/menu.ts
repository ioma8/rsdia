// `≡` menu for terminals too narrow for the full toolbar.
import type { Host } from "../host";
import type { Painter } from "../painter";
import type { ItemId } from "../toolbar";
import { placePopover } from "./common";

const ENTRIES: [ItemId, string][] = [
  ["files", "files"],
  ["export", "export"],
  ["undo", "undo"],
  ["redo", "redo"],
  ["settings", "settings"],
  ["help", "help"],
  ["quit", "quit"],
];

export function renderMenu(p: Painter, host: Host, anchorX: number, top: number): void {
  const r = placePopover(p, anchorX, top, 18, ENTRIES.length + 2);
  p.panel(r);
  ENTRIES.forEach(([id, label], i) => {
    const fg = id === "undo" ? p.pal.undo : id === "redo" ? p.pal.redo : id === "quit" ? p.pal.danger : undefined;
    p.button(`menu:${id}`, r.x + 1, r.y + 1 + i, ` ${label.padEnd(r.w - 4)} `, () => host.activate(id, r.x + 1), {
      fg,
    });
  });
}

import type { Painter, Rect } from "../painter";

/** Places a popover below its toolbar label, clamped to the screen. */
export function placePopover(p: Painter, anchorX: number, top: number, w: number, h: number): Rect {
  w = Math.min(w, p.width);
  h = Math.min(h, Math.max(3, p.height - top - 1));
  const x = Math.max(0, Math.min(anchorX - 2, p.width - w));
  return { x, y: top, w, h };
}

/** Horizontal rule inside a panel. */
export function rule(p: Painter, r: Rect, y: number): void {
  for (let x = r.x + 1; x < r.x + r.w - 1; x++) p.cell(x, y, "─", p.pal.tbBorder, p.pal.tbBg);
  p.cell(r.x, y, "├", p.pal.tbBorder, p.pal.tbBg);
  p.cell(r.x + r.w - 1, y, "┤", p.pal.tbBorder, p.pal.tbBg);
}

/** Lays out buttons left to right, wrapping inside `r`. Returns the next free row. */
export function flow(
  p: Painter,
  r: Rect,
  y: number,
  buttons: { id: string; label: string; action: () => void; active?: boolean; fg?: Painter["pal"]["text"] }[],
  lead = "",
): number {
  const left = r.x + 2;
  const right = r.x + r.w - 2;
  let x = left;
  if (lead) x = p.text(x, y, lead, p.pal.muted);
  const indent = x;
  for (const b of buttons) {
    if (x + b.label.length > right && x > indent) {
      y++;
      x = indent;
    }
    x = p.button(b.id, x, y, b.label, b.action, { active: b.active, activeColor: p.pal.accent, fg: b.fg }) + 2;
  }
  return y + 1;
}

/** Height needed by `flow` for the same inputs. */
export function flowHeight(width: number, labels: string[], lead = ""): number {
  const inner = width - 4 - lead.length;
  let rows = 1;
  let x = 0;
  for (const l of labels) {
    if (x + l.length > inner && x > 0) {
      rows++;
      x = 0;
    }
    x += l.length + 2;
  }
  return rows;
}

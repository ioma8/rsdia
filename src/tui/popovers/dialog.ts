// Modal input and confirm dialogs, centered on screen.
import type { Dialog } from "../host";
import { ATTR, type Painter } from "../painter";

export function renderDialog(p: Painter, d: Dialog, submit: () => void, cancel: () => void): void {
  const { pal } = p;
  const w = Math.min(p.width, 56);
  const h = d.kind === "input" ? 7 : 6;
  const r = { x: Math.floor((p.width - w) / 2), y: Math.max(0, Math.floor((p.height - h) / 2)), w, h };
  p.panel(r, pal.accent);
  p.text(r.x + 2, r.y, ` ${d.title} `, pal.accent, pal.tbBg, ATTR.BOLD);
  if (d.kind === "input") {
    const y = r.y + 2;
    const labelEnd = p.text(r.x + 2, y, `${d.label}: `, pal.muted);
    const fieldW = r.x + r.w - 2 - labelEnd;
    // Scroll the field so the cursor stays visible.
    const start = Math.max(0, d.cursor - fieldW + 1);
    const chars = [...d.value];
    for (let i = 0; i < fieldW; i++) {
      const ch = chars[start + i] ?? " ";
      p.cell(labelEnd + i, y, ch, pal.text, pal.selectionBg, start + i === d.cursor ? ATTR.INVERSE : 0);
    }
    if (d.error) p.text(r.x + 2, y + 1, d.error, pal.danger, pal.tbBg, 0, r.x + r.w - 2);
  } else {
    p.text(r.x + 2, r.y + 2, d.message, pal.text, pal.tbBg, 0, r.x + r.w - 2);
  }
  const y = r.y + r.h - 2;
  const ok = d.kind === "input" ? "[ok]" : `[${d.yes}]`;
  let x = r.x + r.w - 4 - ok.length - "[cancel]".length;
  x = p.button("dialog:ok", x, y, ok, submit, { fg: d.kind === "confirm" ? pal.danger : pal.success }) + 2;
  p.button("dialog:cancel", x, y, "[cancel]", cancel);
}

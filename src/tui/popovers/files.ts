// `files` popover: drawing list plus new / rename / fork / clear / delete / import.
import type { Host } from "../host";
import type { Painter } from "../painter";
import { flow, flowHeight, placePopover, rule } from "./common";

const WIDTH = 50;

export function renderFiles(p: Painter, host: Host, anchorX: number, top: number): void {
  const { pal } = p;
  const drawings = host.drawings;
  const actions = [
    { id: "files:new", label: "[new]", action: () => host.newDrawing(), fg: pal.accent },
    { id: "files:rename", label: "[rename]", action: () => host.renameDrawing() },
    { id: "files:fork", label: "[fork]", action: () => host.forkDrawing() },
    { id: "files:import", label: "[import]", action: () => host.importText() },
    { id: "files:clear", label: "[clear]", action: () => host.clearDrawing(), fg: pal.warning },
    { id: "files:delete", label: "[delete]", action: () => host.deleteDrawing(), fg: pal.danger },
  ];
  const maxRows = Math.max(1, p.height - top - 6 - flowHeight(WIDTH, actions.map((a) => a.label)));
  const rows = drawings.slice(0, maxRows);
  const h = 2 + Math.max(1, rows.length) + 1 + flowHeight(WIDTH, actions.map((a) => a.label));
  const r = placePopover(p, anchorX, top, WIDTH, h);
  p.panel(r);

  let y = r.y + 1;
  if (rows.length === 0) {
    p.text(r.x + 2, y++, "no drawings yet", pal.muted);
  }
  for (const [i, d] of rows.entries()) {
    const current = d.path === host.currentPath;
    const size = `${d.size} cells`;
    const nameWidth = r.w - 6 - size.length;
    const name = d.name.length > nameWidth ? `${d.name.slice(0, nameWidth - 1)}…` : d.name;
    const label = `${current ? ">" : " "} ${name.padEnd(nameWidth)} ${size}`;
    p.button(`files:row:${i}`, r.x + 1, y, ` ${label} `.slice(0, r.w - 2), () => host.openDrawing(d.path), {
      active: current,
      activeColor: pal.text,
    });
    y++;
  }
  if (drawings.length > rows.length) {
    p.text(r.x + r.w - 12, y - 1, ` +${drawings.length - rows.length} more `, pal.muted);
  }
  rule(p, r, y++);
  flow(p, r, y, actions);
}

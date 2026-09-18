// `export` dialog (§5.2): charset, comment wrapper, fence, preview, copy / save.
import { WRAPPERS } from "../../core/export";
import type { Host } from "../host";
import type { Painter } from "../painter";
import { flow, flowHeight, placePopover, rule } from "./common";

export function renderExport(p: Painter, host: Host, anchorX: number, top: number): void {
  const { pal } = p;
  const cfg = host.config.export;
  const preview = host.exportPreview().split("\n");
  const previewWidth = Math.max(...preview.map((l) => [...l].length));
  const w = Math.min(p.width, Math.max(64, previewWidth + 4));
  const wrapLabels = WRAPPERS.map((x) => x.label);
  const wrapRows = flowHeight(w, wrapLabels, "wrap:       ");
  const chrome = 2 + 1 + wrapRows + 1 + 1 + 1 + 1;
  const maxPreview = Math.max(1, p.height - top - 1 - chrome);
  const shown = preview.slice(0, maxPreview);
  const h = chrome + shown.length;
  const r = placePopover(p, anchorX, top, w, h);
  p.panel(r);

  let y = r.y + 1;
  y = flow(
    p,
    r,
    y,
    [
      {
        id: "exp:ext",
        label: "extended",
        active: cfg.characters === "extended",
        action: () => host.setExport({ characters: "extended" }),
      },
      {
        id: "exp:basic",
        label: "basic",
        active: cfg.characters === "basic",
        action: () => host.setExport({ characters: "basic" }),
      },
      {
        id: "exp:fence",
        label: cfg.fenced ? "[x] markdown fence" : "[ ] markdown fence",
        active: cfg.fenced,
        action: () => host.setExport({ fenced: !cfg.fenced }),
      },
    ],
    "characters: ",
  );
  y = flow(
    p,
    r,
    y,
    WRAPPERS.map((wr) => ({
      id: `exp:wrap:${wr.id}`,
      label: wr.label,
      active: cfg.wrapper === wr.id,
      action: () => host.setExport({ wrapper: wr.id }),
    })),
    "wrap:       ",
  );
  rule(p, r, y++);
  for (const line of shown) {
    p.text(r.x + 2, y++, line, pal.fg, pal.tbBg, 0, r.x + r.w - 2);
  }
  if (preview.length > shown.length) {
    p.text(r.x + r.w - 16, y - 1, ` +${preview.length - shown.length} lines `, pal.muted);
  }
  rule(p, r, y++);
  let x = r.x + 2;
  x = p.button("exp:copy", x, y, "[copy to clipboard]", () => host.copyExport(), { fg: pal.success }) + 2;
  p.button("exp:save", x, y, "[save…]", () => host.saveExport(), { fg: pal.accent });
}

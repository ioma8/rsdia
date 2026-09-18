// `settings` popover: grid style, theme, copy-on-select, recenter.
import { GRID_STYLES, THEME_CHOICES } from "../../storage/config";
import type { Host } from "../host";
import type { Painter } from "../painter";
import { flow, flowHeight, placePopover } from "./common";

const WIDTH = 58;

/** Grid row, theme rows (they wrap), the copy-on-select row, recenter, and the two borders. */
export function settingsHeight(): number {
  return 2 + 1 + flowHeight(WIDTH, [...THEME_CHOICES], "theme: ") + 1 + 1;
}

export function renderSettings(p: Painter, host: Host, anchorX: number, top: number): void {
  const r = placePopover(p, anchorX, top, WIDTH, settingsHeight());
  p.panel(r);
  let y = r.y + 1;
  y = flow(
    p,
    r,
    y,
    GRID_STYLES.map((g) => ({
      id: `settings:grid:${g}`,
      label: g,
      active: host.config.grid === g,
      action: () => host.setGrid(g),
    })),
    "grid:  ",
  );
  y = flow(
    p,
    r,
    y,
    THEME_CHOICES.map((t) => ({
      id: `settings:theme:${t}`,
      label: t,
      active: host.config.theme === t,
      action: () => host.setTheme(t),
    })),
    "theme: ",
  );
  y = flow(
    p,
    r,
    y,
    [
      {
        id: "settings:copyOnSelect",
        label: host.config.copyOnSelect ? "on" : "off",
        active: host.config.copyOnSelect,
        action: () => host.setCopyOnSelect(!host.config.copyOnSelect),
      },
    ],
    "copy on select: ",
  );
  p.button("settings:recenter", r.x + 2, y, "[recenter]", () => host.recenter(), { fg: p.pal.orange });
}

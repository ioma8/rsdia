// `help` popover: active-tool help and the shortcut table.
import type { ToolId } from "../../core/editor";
import type { Host } from "../host";
import type { Painter } from "../painter";
import { toolColor } from "../toolbar";
import { placePopover, rule } from "./common";

export const TOOL_HELP: Record<ToolId, string> = {
  box: "drag corner to corner",
  select: "drag to move boxes, words and selections; drag a line to resize, a line end to reshape",
  arrow: "drag start to end. press f to flip the elbow",
  line: "drag start to end. press f to flip the elbow",
  text: "click and type. enter: new line, esc: finish, arrows move",
  eraser: "drag to erase",
};

const SHORTCUTS: [string, string][] = [
  ["1-6 / alt+1-6", "switch tool (box select arrow line text eraser)"],
  ["r v a l t e", "switch tool by letter"],
  ["ctrl+z  ctrl+y", "undo / redo (ctrl+shift+z with kitty keys)"],
  ["ctrl+c ctrl+x ctrl+v", "copy / cut / paste selection"],
  ["del  arrows", "erase / nudge selection"],
  ["f", "flip elbow while dragging a line or arrow"],
  ["scroll", "pan vertically, or horizontally with a trackpad"],
  ["space+drag  middle-drag", "pan freely"],
  ["ctrl+o  ctrl+e  ctrl+s", "files / export / save now"],
  ["?  esc", "help / close popover, cancel, deselect"],
  ["ctrl+q twice", "quit (second press within 3s)"],
];

export function renderHelp(p: Painter, host: Host, anchorX: number, top: number): void {
  const { pal } = p;
  const w = Math.min(p.width, 76);
  const h = 2 + 1 + 1 + SHORTCUTS.length + 1;
  const r = placePopover(p, anchorX, top, w, h);
  p.panel(r);
  const tool = host.editor.tool;
  let y = r.y + 1;
  const x = p.text(r.x + 2, y, `${tool}: `, toolColor(pal, tool), pal.tbBg, 1);
  p.text(x, y++, TOOL_HELP[tool], pal.text, pal.tbBg, 0, r.x + r.w - 2);
  rule(p, r, y++);
  const keyW = Math.max(...SHORTCUTS.map(([k]) => k.length)) + 2;
  for (const [k, d] of SHORTCUTS) {
    p.text(r.x + 2, y, k, pal.accent);
    p.text(r.x + 2 + keyW, y++, d, pal.text, pal.tbBg, 0, r.x + r.w - 2);
  }
}

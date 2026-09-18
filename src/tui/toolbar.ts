// Floating toolbar (§5.1): `≡ │ tools │ export │ ⟲ ⟳ │ ⚙ │ help`.
import { TOOL_IDS, type ToolId } from "../core/editor";
import type { Painter } from "./painter";
import type { Palette } from "./theme";

export type PanelId = "files" | "export" | "settings" | "help" | "menu";

/** `quit` has no place on the bar; it is a menu-only entry. */
export type ItemId =
  | "menu"
  | "files"
  | "export"
  | "undo"
  | "redo"
  | "settings"
  | "help"
  | "quit"
  | ToolId;

interface Item {
  id: ItemId;
  label: string;
}

interface Form {
  name: "full" | "compact" | "tight" | "narrow" | "tiny";
  pad: number;
  groups: Item[][];
}

const tools = (labels: readonly string[]): Item[] => TOOL_IDS.map((id, i) => ({ id, label: labels[i]! }));

const FULL_TOOLS = ["box", "select", "arrow", "line", "text", "eraser"];
const SHORT_TOOLS = ["box", "sel", "arrow", "line", "text", "erase"];
const NARROW_TOOLS = ["box", "sel", "arw", "lin", "txt", "ers"];

const wide = (toolLabels: string[], exp: string, help: string): Item[][] => [
  [{ id: "files", label: "≡" }],
  tools(toolLabels),
  [{ id: "export", label: exp }],
  [
    { id: "undo", label: "⟲" },
    { id: "redo", label: "⟳" },
  ],
  [{ id: "settings", label: "⚙" }],
  [{ id: "help", label: help }],
];

const FORMS: Form[] = [
  { name: "full", pad: 2, groups: wide(FULL_TOOLS, "export", "help") },
  { name: "compact", pad: 1, groups: wide(SHORT_TOOLS, "exp", "?") },
  { name: "tight", pad: 0, groups: wide(SHORT_TOOLS, "exp", "?") },
  { name: "narrow", pad: 1, groups: [[{ id: "menu", label: "≡" }], tools(NARROW_TOOLS)] },
  { name: "tiny", pad: 0, groups: [[{ id: "menu", label: "≡" }], tools(["b", "s", "a", "l", "t", "e"])] },
];

function formWidth(f: Form): number {
  let w = 2 + (f.groups.length - 1);
  for (const g of f.groups) w += 2 * f.pad + g.map((i) => i.label.length).reduce((a, b) => a + b, 0) + (g.length - 1);
  return w;
}

export interface ItemSpan {
  id: ItemId;
  x: number;
  w: number;
}

export interface ToolbarLayout {
  form: Form["name"];
  x: number;
  y: number;
  w: number;
  /** Row below the bar, where popovers open. */
  bottom: number;
  items: ItemSpan[];
}

export const BAR_Y = 1;

/** Widest form first; smaller terminals get shorter labels. */
export function layoutToolbar(screenWidth: number): ToolbarLayout {
  const form = FORMS.find((f) => formWidth(f) <= screenWidth) ?? FORMS[FORMS.length - 1]!;
  const w = formWidth(form);
  const x = Math.max(0, Math.floor((screenWidth - w) / 2));
  const items: ItemSpan[] = [];
  let cx = x + 1;
  form.groups.forEach((g, gi) => {
    cx += form.pad;
    g.forEach((item, ii) => {
      items.push({ id: item.id, x: cx, w: item.label.length });
      cx += item.label.length + (ii < g.length - 1 ? 1 : 0);
    });
    cx += form.pad;
    if (gi < form.groups.length - 1) cx += 1;
  });
  return { form: form.name, x, y: BAR_Y, w, bottom: BAR_Y + 3, items };
}

export interface ToolbarHost {
  readonly tool: ToolId;
  readonly panel: PanelId | null;
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly showChips: boolean;
  activate(id: ItemId, anchorX: number): void;
}

export function toolColor(pal: Palette, id: ToolId) {
  return {
    box: pal.cyan,
    select: pal.success,
    arrow: pal.purple,
    line: pal.accent,
    text: pal.warning,
    eraser: pal.orange,
  }[id];
}

const PANEL_FOR: Partial<Record<ItemId, PanelId>> = {
  files: "files",
  export: "export",
  settings: "settings",
  help: "help",
  menu: "menu",
};

export function renderToolbar(p: Painter, host: ToolbarHost, layout: ToolbarLayout): void {
  const { pal } = p;
  const form = FORMS.find((f) => f.name === layout.form)!;
  p.panel({ x: layout.x, y: layout.y, w: layout.w, h: 3 });
  const row = layout.y + 1;

  // Group separators.
  let cx = layout.x + 1;
  form.groups.forEach((g, gi) => {
    cx += 2 * form.pad + g.map((i) => i.label.length).reduce((a, b) => a + b, 0) + (g.length - 1);
    if (gi < form.groups.length - 1) {
      p.cell(cx, row, "│", pal.tbBorder, pal.tbBg);
      cx += 1;
    }
  });

  for (const span of layout.items) {
    const label = form.groups.flat().find((i) => i.id === span.id)?.label ?? "";
    const act = () => host.activate(span.id, span.x);
    const id = `tb:${span.id}`;
    if ((TOOL_IDS as readonly string[]).includes(span.id)) {
      const tool = span.id as ToolId;
      p.button(id, span.x, row, label, act, { active: host.tool === tool, activeColor: toolColor(pal, tool) });
    } else if (span.id === "undo") {
      p.button(id, span.x, row, label, act, { fg: pal.undo, disabled: !host.canUndo });
    } else if (span.id === "redo") {
      p.button(id, span.x, row, label, act, { fg: pal.redo, disabled: !host.canRedo });
    } else {
      const panel = PANEL_FOR[span.id];
      p.button(id, span.x, row, label, act, { active: panel != null && host.panel === panel });
    }
  }

  // Shortcut chips on the bottom border, under each tool (ASCIIFlow shows them while Alt is held).
  if (host.showChips) {
    for (const span of layout.items) {
      const i = TOOL_IDS.indexOf(span.id as ToolId);
      if (i >= 0) p.cell(span.x, layout.y + 2, String(i + 1), pal.bg, pal.warning, 1);
    }
  }
}

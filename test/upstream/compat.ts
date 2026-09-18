// Minimal stand-ins for the ASCIIFlow store and chai, so ASCIIFlow's own spec
// files (ported verbatim into this directory) run against lazydraw's core.
import { expect as bunExpect } from "bun:test";
import { Canvas } from "../../src/core/canvas";
import { Editor, type ToolId } from "../../src/core/editor";
import { mods, type Mods, type Tool } from "../../src/core/tools/tool";
import type { Vector } from "../../src/core/vector";

export enum ToolMode {
  BOX = 1,
  SELECT = 2,
  ARROWS = 6,
  LINES = 4,
  TEXT = 7,
}

const TOOL_BY_MODE: Record<ToolMode, ToolId> = {
  [ToolMode.BOX]: "box",
  [ToolMode.SELECT]: "select",
  [ToolMode.ARROWS]: "arrow",
  [ToolMode.LINES]: "line",
  [ToolMode.TEXT]: "text",
};

const editor = new Editor();

// The specs reset browser storage between tests; lazydraw has none.
(globalThis as any).localStorage ??= { clear() {} };

/** Upstream tools accept `{}` or no modifiers at all. */
function wrap(tool: Tool) {
  const m = (x?: Partial<Mods>) => mods(x ?? {});
  return {
    tool,
    start: (p: Vector, x?: Partial<Mods>) => tool.start(p, m(x)),
    move: (p: Vector, x?: Partial<Mods>) => tool.move(p, m(x)),
    end: () => tool.end(),
    handleKey: (k: string, x?: Partial<Mods>) => tool.handleKey(k, m(x)),
    cleanup: () => tool.cleanup(),
  };
}

// The select tool is a singleton whose private fields the specs reset directly.
const selectTool = new Proxy(wrap(editor.select), {
  get: (target, prop) => (prop in target ? (target as any)[prop] : (editor.select as any)[prop]),
  set: (_target, prop, value) => (((editor.select as any)[prop] = value), true),
});

export const store = {
  get currentCanvas(): Canvas {
    return editor.canvas;
  },
  get currentTool() {
    return editor.tool === "select" ? selectTool : wrap(editor.current);
  },
  selectTool,
};

export const useAppStore = {
  setState(state: { selectedToolMode: ToolMode }) {
    editor.setTool(TOOL_BY_MODE[state.selectedToolMode]);
    editor.canvas = new Canvas();
  },
};

export const DrawingId = {
  local: (id: string | null) => ({ type: "local", localId: id }),
};

function chainable(actual: unknown) {
  const equals = (expected: unknown) => bunExpect(actual).toEqual(expected);
  return { equals, equal: equals, deep: { equals, equal: equals }, to: { equal: equals, deep: { equal: equals } } };
}

export function expect(actual: unknown, _message?: string) {
  return chainable(actual);
}

export const assert = {
  equal: (actual: unknown, expected: unknown, _message?: string) => bunExpect(actual).toEqual(expected),
  isNull: (actual: unknown, _message?: string) => bunExpect(actual).toBeNull(),
  exists: (actual: unknown, _message?: string) => bunExpect(actual ?? undefined).toBeDefined(),
  notExists: (actual: unknown, _message?: string) => bunExpect(actual ?? undefined).toBeUndefined(),
};

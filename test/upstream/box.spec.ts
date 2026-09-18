// @ts-nocheck -- verbatim upstream code, loosely typed.
// Ported from ASCIIFlow (client/draw/box.spec.ts), MIT © Lewis Hemens.
// Only the imports are changed; the assertions are upstream's.
import { beforeEach, describe, it } from "bun:test";

import {
  DrawingId,
  store,
  ToolMode,
  useAppStore,
} from "./compat";
import { textToLayer } from "../../src/core/text";
import { Vector } from "../../src/core/vector";
import { assert } from "./compat";

function reset() {
  localStorage.clear();
  useAppStore.setState(
    {
      route: DrawingId.local(null),
      selectedToolMode: ToolMode.BOX,
      freeformCharacter: "x",
      altPressed: false,
      currentCursor: "default",
      modifierKeys: {},
      unicode: true,
      controlsOpen: true,
      fileControlsOpen: true,
      editControlsOpen: true,
      helpControlsOpen: true,
      exportConfig: {},
      localDrawingIds: [],
      darkMode: false,
      canvasVersion: 0,
    },
    true
  );
}

describe("DrawBox snapping", () => {
  beforeEach(reset);

  it("draws a clean box when nothing is adjacent", () => {
    const tool = store.currentTool; // boxTool
    tool.start(new Vector(0, 0), {});
    tool.move(new Vector(2, 2), {});
    tool.end();
    const c = store.currentCanvas.committed;
    assert.equal(c.get(new Vector(0, 0)), "┌");
    assert.equal(c.get(new Vector(2, 0)), "┐");
    assert.equal(c.get(new Vector(0, 2)), "└");
    assert.equal(c.get(new Vector(2, 2)), "┘");
  });

  it("connects to a line that terminates on an edge", () => {
    store.currentCanvas.committed = textToLayer("\n──"); // ─ at (0,1),(1,1)
    const tool = store.currentTool;
    tool.start(new Vector(2, 0), {});
    tool.move(new Vector(4, 2), {}); // box (2,0)-(4,2); left edge at x=2
    tool.end();
    const c = store.currentCanvas.committed;
    assert.equal(c.get(new Vector(2, 1)), "┤"); // edge became a junction
    assert.equal(c.get(new Vector(1, 1)), "─"); // line still there, connected
  });
});

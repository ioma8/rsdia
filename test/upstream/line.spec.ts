// @ts-nocheck -- verbatim upstream code, loosely typed.
// Ported from ASCIIFlow (client/draw/line.spec.ts), MIT © Lewis Hemens.
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
      selectedToolMode: ToolMode.LINES,
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

describe("DrawLine onto an existing line", () => {
  beforeEach(reset);

  it("forms a T when a vertical line ends on a horizontal line", () => {
    store.currentCanvas.committed = textToLayer("───"); // (0..2, 0)
    const tool = store.currentTool; // lineTool
    tool.start(new Vector(1, -2), {});
    tool.move(new Vector(1, 0), {}); // draw down onto the middle
    tool.end();
    assert.equal(store.currentCanvas.committed.get(new Vector(1, 0)), "┴");
  });

  it("forms a T when a horizontal line ends on a vertical line", () => {
    store.currentCanvas.committed = textToLayer(["│", "│", "│"].join("\n")); // (0, 0..2)
    const tool = store.currentTool;
    tool.start(new Vector(-2, 1), {});
    tool.move(new Vector(0, 1), {}); // draw right onto the middle
    tool.end();
    assert.equal(store.currentCanvas.committed.get(new Vector(0, 1)), "┤");
  });
});

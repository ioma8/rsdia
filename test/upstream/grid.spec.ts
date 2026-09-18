// @ts-nocheck -- verbatim upstream code, loosely typed.
// Ported from ASCIIFlow (client/draw/grid.spec.ts), MIT © Lewis Hemens.
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

const tool = store.selectTool;

function reset() {
  (tool as any).selectedCells = [];
  (tool as any).selectBox = undefined;
  (tool as any).activeBox = null;
  (tool as any).attachments = [];
  (tool as any).lineReshape = null;
  (tool as any).dragStart = null;
  (tool as any).selecting = false;
  localStorage.clear();
  useAppStore.setState(
    {
      route: DrawingId.local(null),
      selectedToolMode: ToolMode.SELECT,
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

describe("2x2 grid move", () => {
  beforeEach(reset);

  it("reflows the shared walls so a moved quadrant stays connected", () => {
    store.currentCanvas.committed = textToLayer(
      [
        "┌──┬──┐",
        "│  │  │",
        "├──┼──┤",
        "│  │  │",
        "└──┴──┘",
      ].join("\n")
    );
    tool.start(new Vector(1, 3), {}); // bottom-left quadrant interior
    tool.move(new Vector(1, 6), {}); // move the divider down 3
    tool.end();
    const c = store.currentCanvas.committed;
    // The shared walls followed, and every junction is clean after snapping:
    // the divider now sits at row 5 with proper ├ ┼ ┤ junctions...
    assert.equal(c.get(new Vector(0, 5)), "├");
    assert.equal(c.get(new Vector(3, 5)), "┼");
    assert.equal(c.get(new Vector(6, 5)), "┤");
    // ...the top-middle junction is untouched...
    assert.equal(c.get(new Vector(3, 0)), "┬");
    // ...and the old divider-right simplified to a plain wall (its arm moved).
    assert.equal(c.get(new Vector(6, 2)), "│");
  });
});

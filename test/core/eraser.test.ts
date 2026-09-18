import { describe, expect, test } from "bun:test";
import { layerToText } from "../../src/core/text";
import { Vector } from "../../src/core/vector";
import { runScenario } from "../scenario";

const v = (x: number, y: number) => new Vector(x, y);

describe("eraser tool", () => {
  test("drags erase committed cells and detach from lines they touch", () => {
    const editor = runScenario([
      { tool: "box" },
      { drag: [[0, 0], [4, 2]] },
      { tool: "eraser" },
      { drag: [[0, 1], [1, 1]] },
    ]);
    const { committed } = editor.canvas;
    // The left edge's middle row is gone; the corners no longer connect down to it.
    expect(committed.get(v(0, 1))).toBeNull();
    expect(committed.get(v(0, 0))).toBe("┌");
  });

  test("a zero-size drag erases just the one cell, as an undoable step", () => {
    const editor = runScenario([{ tool: "box" }, { drag: [[0, 0], [3, 0]] }, { tool: "eraser" }, { drag: [[1, 0]] }]);
    expect(editor.canvas.committed.get(v(1, 0))).toBeNull();
    expect(editor.canvas.canUndo).toBe(true);
    editor.undo();
    expect(layerToText(editor.canvas.committed)).toContain("─");
  });
});

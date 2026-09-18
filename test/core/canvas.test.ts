import { describe, expect, test } from "bun:test";
import { Box } from "../../src/core/box";
import { Canvas } from "../../src/core/canvas";
import { connect, connectionGlyph, disconnect } from "../../src/core/glyphs";
import { Layer } from "../../src/core/layer";
import { isPlaceable, layerToText, textToLayer } from "../../src/core/text";
import { Direction, Vector } from "../../src/core/vector";

const v = (x: number, y: number) => new Vector(x, y);

describe("canvas", () => {
  test("commit, undo, redo", () => {
    const c = new Canvas();
    c.setScratch(Layer.from([[v(0, 0), "a"]]));
    expect(c.rendered().get(v(0, 0))).toBe("a");
    expect(c.committed.size()).toBe(0);
    c.commitScratch();
    c.commit(Layer.from([[v(0, 0), "b"], [v(1, 0), "c"]]));
    expect(layerToText(c.committed)).toBe("bc");
    c.undo();
    expect(layerToText(c.committed)).toBe("a");
    c.redo();
    expect(layerToText(c.committed)).toBe("bc");
    c.undo();
    c.undo();
    expect(c.committed.size()).toBe(0);
    expect(c.canUndo).toBe(false);
    c.commit(Layer.from([[v(5, 5), "z"]]));
    expect(c.canRedo).toBe(false);
  });

  test("no-op commits don't create undo steps", () => {
    const c = new Canvas(textToLayer("x"));
    expect(c.commit(Layer.from([[v(0, 0), "x"]]))).toBe(false);
    expect(c.canUndo).toBe(false);
  });

  test("erase markers delete and undo restores", () => {
    const c = new Canvas(textToLayer("ab"));
    c.commit(Layer.from([[v(0, 0), ""]]));
    expect(layerToText(c.committed)).toBe("b");
    c.undo();
    expect(layerToText(c.committed)).toBe("ab");
  });

  test("undo restores the selection from before the gesture", () => {
    const c = new Canvas();
    const box = new Box(v(0, 0), v(2, 2));
    c.setSelection(box);
    c.setScratch(Layer.from([[v(0, 0), "a"]]));
    c.setSelection(null);
    c.commitScratch();
    c.undo();
    expect(c.selection).toBe(box);
  });

  test("clear is one undo step", () => {
    const c = new Canvas(textToLayer("abc\ndef"));
    c.clear();
    expect(c.committed.size()).toBe(0);
    c.undo();
    expect(layerToText(c.committed)).toBe("abc\ndef");
  });
});

describe("glyphs", () => {
  test("connect forms tees and crosses", () => {
    expect(connect("─", Direction.DOWN)).toBe("┬");
    expect(connect("│", Direction.RIGHT)).toBe("├");
    expect(connect("┬", Direction.UP)).toBe("┼");
    expect(connect("┌", [Direction.UP, Direction.LEFT])).toBe("┼");
    expect(() => connect("►", Direction.UP)).toThrow();
  });

  test("disconnect keeps a glyph when no clean one remains", () => {
    expect(disconnect("┼", Direction.UP)).toBe("┬");
    expect(disconnect("├", Direction.RIGHT)).toBe("│");
    expect(disconnect("└", Direction.UP)).toBe("└");
  });

  test("connectionGlyph", () => {
    expect(connectionGlyph([Direction.DOWN, Direction.RIGHT])).toBe("┌");
    expect(connectionGlyph([Direction.DOWN])).toBeNull();
  });
});

describe("text", () => {
  test("wide characters are rejected or replaced", () => {
    expect(isPlaceable("a")).toBe(true);
    expect(isPlaceable("─")).toBe(true);
    expect(isPlaceable("漢")).toBe(false);
    expect(isPlaceable("😀")).toBe(false);
    expect(layerToText(textToLayer("a漢b"))).toBe("a?b");
  });

  test("layerToText with a box keeps its full extent", () => {
    expect(layerToText(textToLayer("a"), new Box(v(0, 0), v(2, 1)))).toBe("a  \n   ");
  });
});

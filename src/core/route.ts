// Straight and single-elbow line routing, plus endpoint connection and
// orientation inference for the line/arrow tools.
// Ported from ASCIIFlow (client/draw/utils.ts, client/draw/line.ts), MIT © Lewis Hemens.
import { connect, connectable, connects, disconnect, isSpecial, UNICODE } from "./glyphs";
import { Layer, StackedLayers, type LayerView } from "./layer";
import { Direction, Vector } from "./vector";

export function line(start: Vector, end: Vector, horizontalFirst: boolean): Layer {
  if (start.x === end.x || start.y === end.y) return straightLine(start, end);
  return cornerLine(start, end, horizontalFirst);
}

export function cornerLine(start: Vector, end: Vector, horizontalFirst: boolean): Layer {
  const corner = horizontalFirst ? new Vector(end.x, start.y) : new Vector(start.x, end.y);
  const layer = straightLine(start, corner);
  layer.setFrom(straightLine(corner, end));
  const right = start.x < end.x;
  const down = start.y < end.y;
  let glyph: string;
  if (horizontalFirst) {
    glyph = right
      ? down
        ? UNICODE.cornerTopRight
        : UNICODE.cornerBottomRight
      : down
        ? UNICODE.cornerTopLeft
        : UNICODE.cornerBottomLeft;
  } else {
    glyph = down
      ? right
        ? UNICODE.cornerBottomLeft
        : UNICODE.cornerBottomRight
      : right
        ? UNICODE.cornerTopLeft
        : UNICODE.cornerTopRight;
  }
  layer.set(corner, glyph);
  return layer;
}

export function straightLine(start: Vector, end: Vector): Layer {
  const layer = new Layer();
  if (start.x !== end.x && start.y !== end.y) {
    throw new Error(`Can't draw a straight line between ${start} and ${end}`);
  }
  if (start.x === end.x) {
    const top = Math.min(start.y, end.y);
    const bottom = Math.max(start.y, end.y);
    for (let y = top; y <= bottom; y++) layer.set(new Vector(start.x, y), UNICODE.lineVertical);
  }
  if (start.y === end.y) {
    const left = Math.min(start.x, end.x);
    const right = Math.max(start.x, end.x);
    for (let x = left; x <= right; x++) layer.set(new Vector(x, start.y), UNICODE.lineHorizontal);
  }
  return layer;
}

/** Arrow head glyph at `end` for a route from `start`. */
export function arrowFor(start: Vector, end: Vector, horizontalFirst: boolean): string {
  if (end.x === start.x) return end.y < start.y ? UNICODE.arrowUp : UNICODE.arrowDown;
  if (end.y === start.y) return end.x < start.x ? UNICODE.arrowLeft : UNICODE.arrowRight;
  if (horizontalFirst) return end.y < start.y ? UNICODE.arrowUp : UNICODE.arrowDown;
  return end.x > start.x ? UNICODE.arrowRight : UNICODE.arrowLeft;
}

interface CellContext {
  left: boolean;
  right: boolean;
  up: boolean;
  down: boolean;
  leftup: boolean;
  leftdown: boolean;
  rightup: boolean;
  rightdown: boolean;
}

export function cellContext(p: Vector, layer: LayerView): CellContext {
  const s = (v: Vector) => isSpecial(layer.get(v));
  return {
    left: s(p.left()),
    right: s(p.right()),
    up: s(p.up()),
    down: s(p.down()),
    leftup: s(p.left().up()),
    leftdown: s(p.left().down()),
    rightup: s(p.right().up()),
    rightdown: s(p.right().down()),
  };
}

/**
 * Elbow orientation for a drag from `start` to `end`, inferred from the
 * structure around both endpoints. `flip` reverses the inference.
 */
export function inferHorizontalFirst(start: Vector, end: Vector, committed: LayerView, flip: boolean): boolean {
  const s = cellContext(start, committed);
  const e = cellContext(end, committed);
  const horizontalStart = (s.up && s.down) || (s.leftup && s.leftdown) || (s.rightup && s.rightdown);
  const verticalEnd = (e.left && e.right) || (e.leftup && e.rightup) || (e.leftdown && e.rightdown);
  return (horizontalStart || verticalEnd) !== flip;
}

/**
 * Connects the given endpoint cells of `layer` to any structure pointing at
 * them, then trims connections that lead nowhere.
 */
export function connectEndpoints(layer: Layer, base: Layer, ends: readonly Vector[]): void {
  const combined = new StackedLayers([base, layer]);
  for (const p of ends) {
    const incoming = Direction.ALL.filter(
      (d) => connects(combined.get(p.add(d)), d.opposite()) && connectable(layer.get(p), d),
    );
    layer.set(p, connect(layer.get(p), incoming)!);
    layer.set(p, disconnect(layer.get(p), Direction.ALL.filter((d) => !incoming.includes(d)))!);
  }
}

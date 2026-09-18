// Entity detection (words, line tips, boxes) and moves that keep attached
// lines connected. Ported from ASCIIFlow (client/draw/entity.ts), MIT © Lewis Hemens.
import { Box, boundingBox } from "./box";
import { ARROW_FOR, connections, connects, isArrow, isBoxDrawing, UNICODE } from "./glyphs";
import { isErase, Layer } from "./layer";
import { snap } from "./snap";
import { Direction, Vector } from "./vector";

const H = UNICODE.lineHorizontal;
const V = UNICODE.lineVertical;

export function isContent(value: string | null | undefined): value is string {
  return value != null && !isErase(value);
}

export function isText(value: string | null | undefined): value is string {
  return isContent(value) && !isBoxDrawing(value);
}

/** Maximal horizontal run of text characters under `p`. */
export function detectWord(layer: Layer, p: Vector): Vector[] | null {
  if (!isText(layer.get(p))) return null;
  let left = p;
  let right = p;
  while (isText(layer.get(left.left()))) left = left.left();
  while (isText(layer.get(right.right()))) right = right.right();
  const cells: Vector[] = [];
  for (let x = left.x; x <= right.x; x++) cells.push(new Vector(x, p.y));
  return cells;
}

export interface LineTip {
  tip: Vector;
  axis: "horizontal" | "vertical";
  /** Direction from the tip toward the rest of the line. */
  bodyDir: Direction;
  arrow: string | null;
}

/** The free end of a line or arrow under `p`, if any. */
export function detectLineTip(layer: Layer, p: Vector): LineTip | null {
  const value = layer.get(p);
  let axis: LineTip["axis"];
  if (value === H) axis = "horizontal";
  else if (value === V) axis = "vertical";
  else if (isArrow(value)) {
    axis = value === UNICODE.arrowLeft || value === UNICODE.arrowRight ? "horizontal" : "vertical";
  } else return null;

  const dirs = axis === "horizontal" ? [Direction.LEFT, Direction.RIGHT] : [Direction.UP, Direction.DOWN];
  const connected = dirs.filter((d) => connects(layer.get(p.add(d)), d.opposite()));
  if (connected.length !== 1) return null;
  return { tip: p, axis, bodyDir: connected[0]!, arrow: isArrow(value) ? value : null };
}

const BENDS = new Set([
  UNICODE.cornerTopLeft,
  UNICODE.cornerTopRight,
  UNICODE.cornerBottomRight,
  UNICODE.cornerBottomLeft,
]);

const straightCharFor = (d: Direction) => (d.isHorizontal() ? H : V);

/**
 * Walks from a tip toward the body and stops at the first bend (the pivot).
 * Returns the cells from the tip up to and including that corner.
 */
export function traceLineFromTip(
  layer: Layer,
  tip: Vector,
  bodyDir: Direction,
): { cells: Vector[]; anchor: Vector } {
  const cells: Vector[] = [tip];
  let current = tip.add(bodyDir);
  for (let steps = 0; steps < 1000; steps++) {
    const value = layer.get(current);
    if (value === straightCharFor(bodyDir)) {
      cells.push(current);
      current = current.add(bodyDir);
      continue;
    }
    if (value != null && BENDS.has(value) && connects(value, bodyDir.opposite())) {
      cells.push(current);
      return { cells, anchor: current };
    }
    break;
  }
  return { cells, anchor: cells[cells.length - 1]! };
}

const isVerticalBorder = (v: string | null) =>
  isBoxDrawing(v) && (connects(v, Direction.UP) || connects(v, Direction.DOWN));
const isHorizontalBorder = (v: string | null) =>
  isBoxDrawing(v) && (connects(v, Direction.LEFT) || connects(v, Direction.RIGHT));

const RAY_LIMIT = 400;

function ray(layer: Layer, from: Vector, d: Direction, pred: (v: string | null) => boolean): Vector | null {
  let p = from;
  for (let i = 0; i < RAY_LIMIT; i++) {
    p = p.add(d);
    if (pred(layer.get(p))) return p;
  }
  return null;
}

function verifyPerimeter(layer: Layer, box: Box): boolean {
  for (let x = box.left(); x <= box.right(); x++) {
    if (!isBoxDrawing(layer.get(new Vector(x, box.top())))) return false;
    if (!isBoxDrawing(layer.get(new Vector(x, box.bottom())))) return false;
  }
  for (let y = box.top(); y <= box.bottom(); y++) {
    if (!isBoxDrawing(layer.get(new Vector(box.left(), y)))) return false;
    if (!isBoxDrawing(layer.get(new Vector(box.right(), y)))) return false;
  }
  return true;
}

function boxFromSeed(layer: Layer, seed: Vector): Box | null {
  const left = ray(layer, seed, Direction.LEFT, isVerticalBorder);
  const right = ray(layer, seed, Direction.RIGHT, isVerticalBorder);
  const up = ray(layer, seed, Direction.UP, isHorizontalBorder);
  const down = ray(layer, seed, Direction.DOWN, isHorizontalBorder);
  if (!left || !right || !up || !down) return null;
  const box = new Box(new Vector(left.x, up.y), new Vector(right.x, down.y));
  if (box.right() - box.left() < 1 || box.bottom() - box.top() < 1) return null;
  return verifyPerimeter(layer, box) ? box : null;
}

const area = (b: Box) => (b.right() - b.left()) * (b.bottom() - b.top());

const DIAGONALS = [new Vector(-1, -1), new Vector(1, -1), new Vector(-1, 1), new Vector(1, 1)];
const COMPONENT_LIMIT = 4000;

function borderComponent(layer: Layer, start: Vector): Vector[] | null {
  const seen = new Set([start.key()]);
  const stack = [start];
  const cells: Vector[] = [];
  while (stack.length > 0) {
    const current = stack.pop()!;
    cells.push(current);
    if (cells.length > COMPONENT_LIMIT) return null;
    for (const d of Direction.ALL) {
      const next = current.add(d);
      if (!seen.has(next.key()) && isBoxDrawing(layer.get(next))) {
        seen.add(next.key());
        stack.push(next);
      }
    }
  }
  return cells;
}

/** Smallest closed box enclosing `p` (on its border or inside), or null. */
export function findBox(layer: Layer, p: Vector): Box | null {
  const candidates: Box[] = [];
  const seeds: Vector[] = [];
  if (!isBoxDrawing(layer.get(p))) {
    seeds.push(p);
  } else {
    for (const d of [...Direction.ALL, ...DIAGONALS]) {
      const n = p.add(d);
      if (!isBoxDrawing(layer.get(n))) seeds.push(n);
    }
  }
  for (const seed of seeds) {
    const box = boxFromSeed(layer, seed);
    if (box && box.contains(p)) candidates.push(box);
  }
  if (isBoxDrawing(layer.get(p))) {
    const component = borderComponent(layer, p);
    const bounds = component && boundingBox(component);
    if (
      bounds &&
      bounds.right() - bounds.left() >= 1 &&
      bounds.bottom() - bounds.top() >= 1 &&
      bounds.contains(p) &&
      verifyPerimeter(layer, bounds)
    ) {
      candidates.push(bounds);
    }
  }
  let best: Box | null = null;
  for (const b of candidates) if (!best || area(b) < area(best)) best = b;
  return best;
}

export function cellsInBox(layer: Layer, box: Box): Vector[] {
  const out: Vector[] = [];
  for (const [k, v] of layer.map) {
    if (isErase(v)) continue;
    const p = Vector.fromKey(k);
    if (box.contains(p)) out.push(p);
  }
  return out;
}

/** Translates `cells` by `delta`, snapping the surrounding structure. */
export function moveCells(committed: Layer, cells: readonly Vector[], delta: Vector): Layer {
  const layer = new Layer();
  for (const c of cells) layer.set(c, "");
  const protect = new Set<string>();
  for (const c of cells) {
    const v = committed.get(c);
    if (isContent(v)) {
      const t = c.add(delta);
      layer.set(t, v);
      protect.add(t.key());
    }
  }
  layer.setFrom(snap(layer, committed, protect));
  return layer;
}

export interface BoxAttachment {
  /** Border cell of the box the line attaches to. */
  anchor: Vector;
  /** Outward direction, away from the box. */
  out: Direction;
  /** Fixed endpoint the connector must still reach. */
  far: Vector;
  /** Connector cells between the box and `far`, corners included. */
  runCells: Vector[];
  /** The line ends in an arrow head pointing into the box. */
  arrowIntoBox: boolean;
}

function turnFrom(value: string, dir: Direction): Direction | null {
  for (const d of connections(value)) if (d !== dir.opposite()) return d;
  return null;
}

function perimeter(box: Box): [Vector, Direction][] {
  const out: [Vector, Direction][] = [];
  for (let x = box.left(); x <= box.right(); x++) {
    out.push([new Vector(x, box.top()), Direction.UP]);
    out.push([new Vector(x, box.bottom()), Direction.DOWN]);
  }
  for (let y = box.top(); y <= box.bottom(); y++) {
    out.push([new Vector(box.left(), y), Direction.LEFT]);
    out.push([new Vector(box.right(), y), Direction.RIGHT]);
  }
  return out;
}

/** Lines leaving `box`, each followed through corners to its terminal. */
export function traceBoxAttachments(layer: Layer, box: Box): BoxAttachment[] {
  const attachments: BoxAttachment[] = [];
  for (const [anchor, out] of perimeter(box)) {
    const first = anchor.add(out);
    const firstValue = layer.get(first);
    const straightOut = firstValue === straightCharFor(out) && connects(firstValue, out.opposite());
    const behind = first.add(out);
    const arrowIntoBox =
      firstValue === ARROW_FOR.get(out.opposite()) &&
      layer.get(behind) === straightCharFor(out) &&
      connects(layer.get(behind), out.opposite());
    if (!straightOut && !arrowIntoBox) continue;

    const runCells: Vector[] = [];
    let dir = out;
    let current = first;
    if (arrowIntoBox) {
      runCells.push(first);
      current = behind;
    }
    for (let steps = 0; steps < 1000; steps++) {
      const value = layer.get(current);
      if (value === straightCharFor(dir)) {
        runCells.push(current);
        current = current.add(dir);
        continue;
      }
      if (value != null && BENDS.has(value) && connects(value, dir.opposite())) {
        const next = turnFrom(value, dir);
        if (!next) break;
        runCells.push(current);
        dir = next;
        current = current.add(dir);
        continue;
      }
      break;
    }
    attachments.push({ anchor, out, far: current, runCells, arrowIntoBox });
  }
  return attachments;
}

function cornerFor(a: Direction, b: Direction): string {
  const h = a.isHorizontal() ? a : b;
  const v = a.isHorizontal() ? b : a;
  if (h === Direction.LEFT) return v === Direction.UP ? UNICODE.cornerBottomRight : UNICODE.cornerTopRight;
  return v === Direction.UP ? UNICODE.cornerBottomLeft : UNICODE.cornerTopLeft;
}

function drawStraight(layer: Layer, from: Vector, to: Vector, excludeEnd: boolean) {
  const step = new Vector(Math.sign(to.x - from.x), Math.sign(to.y - from.y));
  const ch = step.x !== 0 ? H : V;
  let p = from;
  while (true) {
    if (!(excludeEnd && p.equals(to))) layer.set(p, ch);
    if (p.equals(to)) break;
    p = p.add(step);
  }
}

function drawConnector(layer: Layer, from: Vector, to: Vector, out: Direction, farValue: string | null) {
  if (from.equals(to)) return;
  const horizontalFirst = out.isHorizontal();
  const bend = horizontalFirst ? new Vector(to.x, from.y) : new Vector(from.x, to.y);
  drawStraight(layer, from, bend, bend.equals(to));
  let approach = new Vector(out.x, out.y);
  if (!bend.equals(to)) {
    drawStraight(layer, bend, to, true);
    approach = new Vector(Math.sign(to.x - bend.x), Math.sign(to.y - bend.y));
  }
  if (!bend.equals(from) && !bend.equals(to)) {
    const next = horizontalFirst
      ? to.y > bend.y
        ? Direction.DOWN
        : Direction.UP
      : to.x > bend.x
        ? Direction.RIGHT
        : Direction.LEFT;
    layer.set(bend, cornerFor(out.opposite(), next));
  }
  if (isArrow(farValue)) {
    for (const [d, ch] of ARROW_FOR) {
      if (d.x === approach.x && d.y === approach.y) layer.set(to, ch);
    }
  }
}

/** Moves a box and its contents by `delta`, reflowing attached lines. */
export function moveBoxWithAttachments(
  committed: Layer,
  box: Box,
  attachments: readonly BoxAttachment[],
  delta: Vector,
): Layer {
  const layer = new Layer();
  const content = cellsInBox(committed, box);
  for (const c of content) layer.set(c, "");
  for (const a of attachments) for (const c of a.runCells) layer.set(c, "");
  for (const c of content) {
    const v = committed.get(c);
    if (isContent(v)) layer.set(c.add(delta), v);
  }
  for (const a of attachments) {
    const newAnchor = a.anchor.add(a.out).add(delta);
    drawConnector(layer, newAnchor, a.far, a.out, committed.get(a.far));
    if (a.arrowIntoBox) layer.set(newAnchor, ARROW_FOR.get(a.out.opposite())!);
  }
  const protect = new Set(content.map((c) => c.add(delta).key()));
  layer.setFrom(snap(layer, committed, protect));
  return layer;
}

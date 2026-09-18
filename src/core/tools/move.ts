// Drags a straight line segment sideways, stretching attached lines.
// Ported from ASCIIFlow (client/draw/move.ts), MIT © Lewis Hemens.
import { connects, isArrow, isSpecial, UNICODE } from "../glyphs";
import { Layer } from "../layer";
import { Direction, Vector } from "../vector";
import type { HoverHint, Tool, ToolContext } from "./tool";

interface AttachmentTrace {
  source: Vector;
  end: Vector;
  direction: Direction;
}

interface LineTrace {
  orientation: "horizontal" | "vertical";
  positions: Vector[];
  attachments: AttachmentTrace[];
}

const isStraight = (v: string | null) => v === UNICODE.lineHorizontal || v === UNICODE.lineVertical;

export class MoveTool implements Tool {
  private trace: LineTrace | null = null;

  constructor(private readonly ctx: ToolContext) {}

  start(p: Vector): void {
    const committed = this.ctx.canvas.committed;
    if (!isStraight(committed.get(p))) return;
    this.trace = traceLine(committed, p);
    this.move(p);
  }

  move(p: Vector): void {
    const trace = this.trace;
    if (!trace) return;
    const committed = this.ctx.canvas.committed;
    const layer = new Layer();
    const ends = (d: Direction) => trace.attachments.filter((a) => a.direction === d).map((a) => a.end);
    const minX = Math.max(...ends(Direction.LEFT).map((e) => e.x));
    const maxX = Math.min(...ends(Direction.RIGHT).map((e) => e.x));
    const minY = Math.max(...ends(Direction.UP).map((e) => e.y));
    const maxY = Math.min(...ends(Direction.DOWN).map((e) => e.y));
    const effective = new Vector(Math.min(Math.max(p.x, minX), maxX), Math.min(Math.max(p.y, minY), maxY));
    const origin = trace.positions[0]!;
    const moveDirection =
      trace.orientation === "vertical"
        ? effective.x < origin.x
          ? Direction.LEFT
          : Direction.RIGHT
        : effective.y < origin.y
          ? Direction.UP
          : Direction.DOWN;
    const units = Math.abs(moveDirection.isHorizontal() ? effective.x - origin.x : effective.y - origin.y);

    for (const a of trace.attachments) {
      if (a.direction === moveDirection) {
        for (let i = 0; i < units; i++) layer.set(a.source.add(a.direction.scale(i)), "");
      }
    }
    for (const pos of trace.positions) layer.set(pos, "");
    for (const pos of trace.positions) {
      layer.set(pos.add(moveDirection.scale(units)), committed.get(pos) ?? "");
    }
    for (const a of trace.attachments) {
      if (a.direction === moveDirection.opposite()) {
        for (let i = 1; i <= units; i++) {
          layer.set(
            a.source.add(a.direction.scale(-i)),
            a.direction.isHorizontal() ? UNICODE.lineHorizontal : UNICODE.lineVertical,
          );
        }
      }
    }
    this.ctx.canvas.setScratch(layer);
  }

  end(): void {
    this.trace = null;
    this.ctx.canvas.commitScratch();
  }

  cleanup(): void {
    this.trace = null;
  }

  handleKey(): boolean {
    return false;
  }

  hoverHint(p: Vector): HoverHint {
    const v = this.ctx.canvas.committed.get(p);
    if (v === UNICODE.lineHorizontal) return "resize-v";
    if (v === UNICODE.lineVertical) return "resize-h";
    return isSpecial(v) ? "move" : "default";
  }
}

function traceLine(layer: Layer, position: Vector): LineTrace {
  const horizontal = layer.get(position) === UNICODE.lineHorizontal;
  const directions = horizontal ? [Direction.LEFT, Direction.RIGHT] : [Direction.UP, Direction.DOWN];
  const attachmentDirections = horizontal ? [Direction.UP, Direction.DOWN] : [Direction.LEFT, Direction.RIGHT];

  const positions: Vector[] = [position];
  const attachments: AttachmentTrace[] = [];
  for (const d of directions) {
    let current = position;
    while (true) {
      const next = current.add(d);
      if (!connects(layer.get(current), d) || !connects(layer.get(next), d.opposite())) break;
      current = next;
      positions.push(current);
    }
  }

  // `positions` may grow while iterating (arrow heads), matching ASCIIFlow.
  for (const current of positions) {
    for (const ad of attachmentDirections) {
      if (connects(layer.get(current), ad) && connects(layer.get(current.add(ad)), ad.opposite())) {
        attachments.push(traceAttachment(layer, current.add(ad), ad));
      }
      if (isArrow(layer.get(current.add(ad))) && connects(layer.get(current.add(ad.scale(2))), ad.opposite())) {
        positions.push(current.add(ad));
        attachments.push(traceAttachment(layer, current.add(ad.scale(2)), ad));
      }
    }
  }

  return { orientation: horizontal ? "horizontal" : "vertical", positions, attachments };
}

function traceAttachment(layer: Layer, position: Vector, direction: Direction): AttachmentTrace {
  const traceValue = direction.isHorizontal() ? UNICODE.lineHorizontal : UNICODE.lineVertical;
  let p = position;
  while (layer.get(p.add(direction)) === traceValue) p = p.add(direction);
  return { source: position, end: p, direction };
}

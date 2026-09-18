// Ported from ASCIIFlow (client/draw/box.ts), MIT © Lewis Hemens.
import { Box } from "../box";
import { UNICODE } from "../glyphs";
import { Layer } from "../layer";
import { snap } from "../snap";
import { Vector } from "../vector";
import type { HoverHint, Mods, Tool, ToolContext } from "./tool";

export function drawBox(box: Box): Layer {
  const layer = new Layer();
  if (box.right() !== box.left()) {
    for (let x = box.left(); x <= box.right(); x++) {
      layer.set(new Vector(x, box.top()), UNICODE.lineHorizontal);
      layer.set(new Vector(x, box.bottom()), UNICODE.lineHorizontal);
    }
  }
  if (box.top() !== box.bottom()) {
    for (let y = box.top(); y <= box.bottom(); y++) {
      layer.set(new Vector(box.left(), y), UNICODE.lineVertical);
      layer.set(new Vector(box.right(), y), UNICODE.lineVertical);
    }
  }
  if (box.left() !== box.right() && box.top() !== box.bottom()) {
    layer.set(box.topLeft(), UNICODE.cornerTopLeft);
    layer.set(box.topRight(), UNICODE.cornerTopRight);
    layer.set(box.bottomRight(), UNICODE.cornerBottomRight);
    layer.set(box.bottomLeft(), UNICODE.cornerBottomLeft);
  }
  return layer;
}

export class BoxTool implements Tool {
  private startPosition: Vector | null = null;

  constructor(private readonly ctx: ToolContext) {}

  start(p: Vector): void {
    this.startPosition = p;
  }

  move(p: Vector): void {
    if (!this.startPosition) return;
    const canvas = this.ctx.canvas;
    const layer = drawBox(new Box(this.startPosition, p));
    layer.setFrom(snap(layer, canvas.committed));
    canvas.setScratch(layer);
  }

  end(): void {
    this.startPosition = null;
    this.ctx.canvas.commitScratch();
  }

  cleanup(): void {
    this.startPosition = null;
  }

  handleKey(): boolean {
    return false;
  }

  hoverHint(_p: Vector, _m: Mods): HoverHint {
    return "crosshair";
  }
}

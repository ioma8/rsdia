// Line and arrow tools. Ported from ASCIIFlow (client/draw/line.ts), MIT © Lewis Hemens.
import { Layer } from "../layer";
import { arrowFor, connectEndpoints, inferHorizontalFirst, line } from "../route";
import { snap } from "../snap";
import type { Vector } from "../vector";
import type { HoverHint, Mods, Tool, ToolContext } from "./tool";

/** Builds the scratch layer for a line/arrow drag. */
export function drawLine(committed: Layer, start: Vector, end: Vector, isArrow: boolean, flip: boolean): Layer {
  const horizontalFirst = inferHorizontalFirst(start, end, committed, flip);
  const layer = line(start, end, horizontalFirst);
  if (isArrow) layer.set(end, arrowFor(start, end, horizontalFirst));
  // Endpoints join whatever structure points at them. An arrow's head stays a head.
  connectEndpoints(layer, committed, isArrow ? [start] : [start, end]);
  layer.setFrom(snap(layer, committed));
  return layer;
}

export class LineTool implements Tool {
  private startPosition: Vector | null = null;
  private endPosition: Vector | null = null;
  private lastMods: Mods | null = null;

  constructor(
    private readonly ctx: ToolContext,
    readonly isArrow: boolean,
  ) {}

  get active(): boolean {
    return this.startPosition != null;
  }

  start(p: Vector, m: Mods): void {
    this.startPosition = p;
    this.endPosition = p;
    this.draw(m);
  }

  move(p: Vector, m: Mods): void {
    this.endPosition = p;
    this.draw(m);
  }

  private draw(m: Mods): void {
    const { startPosition: s, endPosition: e } = this;
    if (!s || !e) return;
    this.lastMods = m;
    const canvas = this.ctx.canvas;
    // A zero-length drag draws nothing (lazydraw §4.2).
    if (s.equals(e)) {
      canvas.setScratch(new Layer());
      return;
    }
    canvas.setScratch(drawLine(canvas.committed, s, e, this.isArrow, m.flip));
  }

  end(): void {
    this.startPosition = null;
    this.endPosition = null;
    this.ctx.canvas.commitScratch();
  }

  cleanup(): void {
    this.startPosition = null;
    this.endPosition = null;
  }

  /** Re-renders with new modifiers, so flipping mid-drag updates the preview. */
  handleKey(_key: string, m: Mods): boolean {
    if (!this.active) return false;
    if (this.lastMods?.flip !== m.flip) this.draw(m);
    return false;
  }

  hoverHint(): HoverHint {
    return "crosshair";
  }
}

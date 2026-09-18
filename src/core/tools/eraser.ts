// Freeform eraser: drags a path of erase markers, snapped so adjacent lines detach cleanly.
import { Layer } from "../layer";
import { snap } from "../snap";
import type { Vector } from "../vector";
import type { HoverHint, Tool, ToolContext } from "./tool";

export class EraserTool implements Tool {
  private layer: Layer | null = null;

  constructor(private readonly ctx: ToolContext) {}

  get drawing(): boolean {
    return this.layer != null;
  }

  private draw(p: Vector): void {
    if (!this.layer) return;
    this.layer.set(p, "");
    const canvas = this.ctx.canvas;
    const scratch = this.layer.clone();
    scratch.setFrom(snap(scratch, canvas.committed));
    canvas.setScratch(scratch);
  }

  start(p: Vector): void {
    this.layer = new Layer();
    this.draw(p);
  }

  move(p: Vector): void {
    this.draw(p);
  }

  end(): void {
    this.layer = null;
    this.ctx.canvas.commitScratch();
  }

  cleanup(): void {
    this.layer = null;
  }

  handleKey(): boolean {
    return false;
  }

  hoverHint(): HoverHint {
    return "crosshair";
  }
}

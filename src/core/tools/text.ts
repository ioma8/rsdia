// Text tool. Based on ASCIIFlow (client/draw/text.ts), MIT © Lewis Hemens.
// lazydraw changes (§5.2): Enter starts a new line under the start column,
// Escape commits, and each session has its own keystroke undo stack.
import { Layer } from "../layer";
import { isPlaceable } from "../text";
import { Vector } from "../vector";
import { KEY, type HoverHint, type Tool, type ToolContext } from "./tool";

interface Snapshot {
  layer: Layer;
  cursor: Vector;
  typed: string;
}

export class TextTool implements Tool {
  cursor: Vector | null = null;
  private lineStart: Vector | null = null;
  private layer: Layer | null = null;
  private history: Snapshot[] = [];

  constructor(private readonly ctx: ToolContext) {}

  /** A cursor is placed, so printable keys are text, not shortcuts. */
  get editing(): boolean {
    return this.cursor != null;
  }

  start(p: Vector): void {
    this.cursor = p;
    this.lineStart = p;
    if (!this.layer) this.layer = new Layer();
    this.ctx.canvas.setScratch(this.layer);
  }

  move(): void {}

  end(): void {}

  private snapshot(typed: string) {
    this.history.push({ layer: this.layer!.clone(), cursor: this.cursor!, typed });
  }

  private write(p: Vector, value: string) {
    this.layer = this.layer!.clone();
    this.layer.set(p, value);
    this.ctx.canvas.setScratch(this.layer);
  }

  handleKey(key: string): boolean {
    if (!this.cursor || !this.layer) return false;
    const c = this.cursor;
    switch (key) {
      case KEY.ENTER:
        this.cursor = new Vector(this.lineStart!.x, c.y + 1);
        return true;
      case KEY.BACKSPACE:
        this.snapshot("\b");
        this.cursor = c.left();
        // A space, not a delete, so it overwrites committed text underneath.
        this.write(this.cursor, " ");
        return true;
      case KEY.DELETE:
        this.snapshot("\x7f");
        this.write(c, " ");
        return true;
      case KEY.LEFT:
        this.cursor = c.left();
        return true;
      case KEY.RIGHT:
        this.cursor = c.right();
        return true;
      case KEY.UP:
        this.cursor = c.up();
        return true;
      case KEY.DOWN:
        this.cursor = c.down();
        return true;
    }
    if (!isPlaceable(key)) return false;
    this.snapshot(key);
    this.write(c, key);
    this.cursor = c.right();
    return true;
  }

  /** Undoes the last keystroke of this session. False when there is none. */
  undoKeystroke(): boolean {
    const snap = this.history.pop();
    if (!snap) return false;
    this.layer = snap.layer;
    this.cursor = snap.cursor;
    this.ctx.canvas.setScratch(this.layer);
    return true;
  }

  /** The most recent keystroke, if it was a typed space (used by space-to-pan). */
  get lastTypedSpace(): boolean {
    return this.history.at(-1)?.typed === " ";
  }

  /** Commits the session as one undo step and removes the cursor. */
  commit(): void {
    if (this.layer) this.ctx.canvas.commitScratch();
    this.layer = null;
    this.history = [];
    this.cursor = null;
    this.lineStart = null;
  }

  cleanup(): void {
    this.commit();
  }

  hoverHint(): HoverHint {
    return "text";
  }
}

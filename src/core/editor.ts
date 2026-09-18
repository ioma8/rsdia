// Routes input to the active tool. The terminal-free counterpart of
// ASCIIFlow's store + controller: tests drive it directly.
import { Canvas } from "./canvas";
import { BoxTool } from "./tools/box";
import { EraserTool } from "./tools/eraser";
import { LineTool } from "./tools/line";
import { SelectTool } from "./tools/select";
import { TextTool } from "./tools/text";
import type { HoverHint, Mods, Tool, ToolContext } from "./tools/tool";
import type { Vector } from "./vector";

export const TOOL_IDS = ["box", "select", "arrow", "line", "text", "eraser"] as const;
export type ToolId = (typeof TOOL_IDS)[number];

export class Editor implements ToolContext {
  canvas: Canvas;
  private toolId: ToolId = "box";
  private readonly tools: Record<ToolId, Tool>;
  /** A pointer gesture is in progress on the canvas. */
  drawing = false;

  constructor(canvas = new Canvas()) {
    this.canvas = canvas;
    this.tools = {
      box: new BoxTool(this),
      select: new SelectTool(this),
      arrow: new LineTool(this, true),
      line: new LineTool(this, false),
      text: new TextTool(this),
      eraser: new EraserTool(this),
    };
  }

  get tool(): ToolId {
    return this.toolId;
  }

  get current(): Tool {
    return this.tools[this.toolId];
  }

  get select(): SelectTool {
    return this.tools.select as SelectTool;
  }

  get text(): TextTool {
    return this.tools.text as TextTool;
  }

  /** Printable keys belong to the text tool rather than shortcuts. */
  get textEntry(): boolean {
    return this.toolId === "text" && this.text.editing;
  }

  setTool(id: ToolId): void {
    if (id === this.toolId) return;
    this.cancelGesture();
    this.current.cleanup();
    this.toolId = id;
  }

  /** Swaps in another drawing's canvas. */
  setCanvas(canvas: Canvas): void {
    this.cancelGesture();
    this.current.cleanup();
    this.canvas = canvas;
  }

  down(p: Vector, m: Mods): void {
    this.drawing = true;
    this.current.start(p, m);
  }

  move(p: Vector, m: Mods): void {
    if (this.drawing) this.current.move(p, m);
  }

  up(): void {
    if (!this.drawing) return;
    this.drawing = false;
    this.current.end();
  }

  /** Aborts an in-progress drag without committing. */
  cancelGesture(): void {
    if (!this.drawing) return;
    this.drawing = false;
    this.canvas.clearScratch();
    this.current.cleanup();
  }

  key(key: string, m: Mods): boolean {
    return this.current.handleKey(key, m);
  }

  hoverHint(p: Vector, m: Mods): HoverHint {
    return this.current.hoverHint(p, m);
  }

  /** Undo: a live text session loses its last keystroke first (§5.2). */
  undo(): boolean {
    if (this.drawing) return false;
    if (this.textEntry && this.text.undoKeystroke()) return true;
    if (this.textEntry) this.text.commit();
    return this.canvas.undo();
  }

  redo(): boolean {
    if (this.drawing) return false;
    if (this.textEntry) this.text.commit();
    return this.canvas.redo();
  }

  /** Commits pending tool state (a text session) before save/switch. */
  flush(): void {
    if (this.toolId === "text") this.text.commit();
  }
}

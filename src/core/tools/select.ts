// Entity-aware select & move tool.
// Ported from ASCIIFlow (client/draw/select.ts), MIT © Lewis Hemens, plus
// lazydraw's keyboard nudge, clipboard and paste (§5.2).
import { Box, boundingBox } from "../box";
import { isBoxDrawing } from "../glyphs";
import {
  type BoxAttachment,
  cellsInBox,
  detectLineTip,
  detectWord,
  findBox,
  moveBoxWithAttachments,
  moveCells,
  traceBoxAttachments,
  traceLineFromTip,
} from "../entity";
import { Layer, StackedLayers } from "../layer";
import { arrowFor, connectEndpoints, line } from "../route";
import { snap } from "../snap";
import { layerToText, textToLayer } from "../text";
import { Vector } from "../vector";
import { MoveTool } from "./move";
import { KEY, type HoverHint, type Mods, type Tool, type ToolContext } from "./tool";

interface LineReshape {
  cells: Vector[];
  anchor: Vector;
  isArrow: boolean;
  horizontalSegment: boolean;
  moved: boolean;
}

const NUDGE: Record<string, Vector> = {
  [KEY.UP]: new Vector(0, -1),
  [KEY.DOWN]: new Vector(0, 1),
  [KEY.LEFT]: new Vector(-1, 0),
  [KEY.RIGHT]: new Vector(1, 0),
};

function dedupe(cells: Vector[]): Vector[] {
  const seen = new Set<string>();
  return cells.filter((c) => !seen.has(c.key()) && (seen.add(c.key()), true));
}

export class SelectTool implements Tool {
  selectBox: Box | null = null;
  private selectedCells: Vector[] = [];
  private selecting = false;
  private selectAnchor: Vector | null = null;
  private dragStart: Vector | null = null;
  private dragEnd: Vector | null = null;
  private moveTool: MoveTool | null = null;
  private lineReshape: LineReshape | null = null;
  private activeBox: Box | null = null;
  private attachments: BoxAttachment[] = [];

  constructor(private readonly ctx: ToolContext) {}

  private get canvas() {
    return this.ctx.canvas;
  }

  get hasSelection(): boolean {
    return this.selectedCells.length > 0 && this.canvas.selection != null;
  }

  start(p: Vector): void {
    const committed = this.canvas.committed;
    const value = committed.get(p);

    if (this.selectedCells.length > 0 && this.inSelection(p)) {
      this.beginDrag(p);
      return;
    }

    const tip = detectLineTip(committed, p);
    if (tip) {
      const { cells, anchor } = traceLineFromTip(committed, tip.tip, tip.bodyDir);
      this.lineReshape = {
        cells,
        anchor,
        isArrow: tip.arrow != null,
        horizontalSegment: tip.axis === "horizontal",
        moved: false,
      };
      return;
    }

    const word = detectWord(committed, p);
    if (word) {
      this.setSelection(word);
      this.beginDrag(p);
      return;
    }

    if (isBoxDrawing(value)) {
      this.moveTool = new MoveTool(this.ctx);
      this.moveTool.start(p);
      return;
    }

    const box = findBox(committed, p);
    if (box) {
      this.setSelection(cellsInBox(committed, box));
      this.activeBox = box;
      this.beginDrag(p);
      return;
    }

    this.startSelect(p);
  }

  move(p: Vector, m: Mods): void {
    if (this.lineReshape) this.reshapeTip(p, m);
    else if (this.dragStart) this.moveDrag(p);
    else if (this.moveTool) this.moveTool.move(p);
    else if (this.selecting) this.moveSelect(p);
  }

  end(): void {
    if (this.lineReshape) {
      if (this.lineReshape.moved) this.canvas.commitScratch();
      else this.canvas.clearScratch();
      this.lineReshape = null;
    } else if (this.dragStart && this.dragEnd) {
      const delta = this.dragEnd.subtract(this.dragStart);
      const box = this.activeBox;
      this.canvas.commitScratch();
      if (box) {
        const moved = box.translate(delta);
        this.selectedCells = cellsInBox(this.canvas.committed, moved);
        this.selectBox = moved;
        this.activeBox = moved;
        this.canvas.setSelection(moved);
      } else {
        this.setSelection(
          this.selectedCells.map((c) => c.add(delta)),
          true,
        );
      }
    } else if (this.moveTool) {
      this.moveTool.end();
      this.moveTool = null;
    } else if (this.selecting) {
      this.finishSelect();
    }
    this.dragStart = null;
    this.dragEnd = null;
    this.selecting = false;
  }

  private setSelection(cells: Vector[], keepScratch = false): void {
    this.selectedCells = dedupe(cells);
    this.selectBox = boundingBox(this.selectedCells);
    this.activeBox = null;
    if (!keepScratch) this.canvas.clearScratch();
    this.canvas.setSelection(this.selectBox);
  }

  private inSelection(p: Vector): boolean {
    return this.selectBox != null && this.canvas.selection != null && this.selectBox.contains(p);
  }

  private startSelect(p: Vector): void {
    this.selecting = true;
    this.selectAnchor = p;
    this.selectBox = new Box(p, p);
    this.selectedCells = [];
    this.activeBox = null;
    this.canvas.setSelection(this.selectBox);
  }

  private moveSelect(p: Vector): void {
    this.selectBox = new Box(this.selectAnchor!, p);
    this.canvas.setSelection(this.selectBox);
  }

  private finishSelect(): void {
    this.selectedCells = cellsInBox(this.canvas.committed, this.selectBox!);
    // A rubber-band selection moves like a box: lines crossing its edge reflow.
    this.activeBox = this.selectBox;
  }

  private beginDrag(p: Vector): void {
    this.dragStart = p;
    this.dragEnd = p;
    this.attachments = this.activeBox ? traceBoxAttachments(this.canvas.committed, this.activeBox) : [];
  }

  private moveDrag(p: Vector): void {
    this.dragEnd = p;
    const delta = p.subtract(this.dragStart!);
    const committed = this.canvas.committed;
    if (this.activeBox) {
      this.canvas.setScratch(moveBoxWithAttachments(committed, this.activeBox, this.attachments, delta));
      this.canvas.setSelection(this.activeBox.translate(delta));
    } else {
      this.canvas.setScratch(moveCells(committed, this.selectedCells, delta));
      this.canvas.setSelection(boundingBox(this.selectedCells.map((c) => c.add(delta))));
    }
  }

  private reshapeTip(target: Vector, m: Mods): void {
    const r = this.lineReshape!;
    r.moved = true;
    const base = this.canvas.committed.clone();
    for (const c of r.cells) base.delete(c);

    const layer = new Layer();
    for (const c of r.cells) layer.set(c, "");

    if (!r.anchor.equals(target)) {
      // Leave the pivot along the line's axis, then bend toward the cursor.
      const horizontalFirst = r.horizontalSegment !== m.flip;
      layer.setFrom(line(r.anchor, target, horizontalFirst));
      if (r.isArrow) layer.set(target, arrowFor(r.anchor, target, horizontalFirst));
      connectEndpoints(layer, base, r.isArrow ? [r.anchor] : [r.anchor, target]);
    }

    layer.setFrom(snap(layer, base));
    this.canvas.setScratch(layer);
    this.canvas.clearSelection();
  }

  hoverHint(p: Vector): HoverHint {
    const committed = this.canvas.committed;
    if (this.selectedCells.length > 0 && this.inSelection(p)) return "move";
    if (detectLineTip(committed, p)) return "crosshair";
    if (detectWord(committed, p)) return "move";
    if (isBoxDrawing(committed.get(p))) return new MoveTool(this.ctx).hoverHint(p);
    if (findBox(committed, p)) return "move";
    return "default";
  }

  /** Text of the current selection, or null. */
  copySelection(): string | null {
    if (!this.selectBox || !this.canvas.selection) return null;
    const cells = new Layer();
    for (const c of this.selectedCells) {
      const v = this.canvas.committed.get(c);
      if (v) cells.set(c, v);
    }
    return layerToText(new StackedLayers([cells]), this.selectBox);
  }

  /** Erases the selected content as one undo step. */
  eraseSelection(): boolean {
    if (this.selectedCells.length === 0) return false;
    const layer = new Layer();
    for (const c of this.selectedCells) layer.set(c, "");
    layer.setFrom(snap(layer, this.canvas.committed));
    const changed = this.canvas.commit(layer);
    this.selectedCells = [];
    this.selectBox = null;
    this.activeBox = null;
    this.canvas.clearSelection();
    return changed;
  }

  /** Pastes text with its top-left at `at`, committed, and selects it. */
  paste(text: string, at: Vector): void {
    const pasted = textToLayer(text, at);
    if (pasted.size() === 0) return;
    const layer = pasted.clone();
    layer.setFrom(snap(pasted, this.canvas.committed));
    this.canvas.commit(layer);
    this.setSelection(pasted.keys());
  }

  handleKey(key: string): boolean {
    if (key === KEY.BACKSPACE || key === KEY.DELETE) {
      return this.eraseSelection();
    }
    const delta = NUDGE[key];
    if (delta && this.hasSelection && !this.dragStart && !this.selecting) {
      const from = this.selectBox!.topLeft();
      this.beginDrag(from);
      this.moveDrag(from.add(delta));
      this.end();
      return true;
    }
    return false;
  }

  /** Switching away drops the selection and any in-progress state. */
  cleanup(): void {
    this.selectedCells = [];
    this.selectBox = null;
    this.activeBox = null;
    this.attachments = [];
    this.lineReshape = null;
    this.moveTool = null;
    this.dragStart = null;
    this.dragEnd = null;
    this.selecting = false;
    this.canvas.clearSelection();
    this.canvas.clearScratch();
  }
}

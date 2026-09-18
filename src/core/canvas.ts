// Committed + scratch layers with undo/redo. Mirrors ASCIIFlow's CanvasStore
// (client/store/canvas.ts), minus persistence, which lives in storage/.
import type { Box } from "./box";
import { Layer, StackedLayers } from "./layer";

const MAX_UNDO = 500;

export class Canvas {
  committed = new Layer();
  scratch = new Layer();
  selection: Box | null = null;
  private undoLayers: Layer[] = [];
  private redoLayers: Layer[] = [];
  private undoSelections: (Box | null)[] = [];
  private redoSelections: (Box | null)[] = [];
  private pendingSelection: Box | null = null;
  private listeners = new Set<(committed: boolean) => void>();

  constructor(committed?: Layer) {
    if (committed) this.committed = committed;
  }

  /** `committed` is true when the committed layer changed. */
  onChange(fn: (committed: boolean) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private notify(committed = false) {
    for (const fn of this.listeners) fn(committed);
  }

  get canUndo(): boolean {
    return this.undoLayers.length > 0;
  }

  get canRedo(): boolean {
    return this.redoLayers.length > 0;
  }

  /** Committed overlaid with scratch. */
  rendered(): StackedLayers {
    return new StackedLayers([this.committed, this.scratch]);
  }

  setSelection(box: Box | null): void {
    this.selection = box;
    this.notify();
  }

  clearSelection(): void {
    this.setSelection(null);
  }

  setScratch(layer: Layer): void {
    // Capture the selection as a gesture begins, so undo can restore it.
    if (this.scratch.size() === 0 && layer.size() > 0) this.pendingSelection = this.selection;
    this.scratch = layer;
    this.notify();
  }

  clearScratch(): void {
    this.scratch = new Layer();
    this.notify();
  }

  private pushUndo(layer: Layer, selection: Box | null) {
    this.undoLayers.push(layer);
    this.undoSelections.push(selection);
    if (this.undoLayers.length > MAX_UNDO) {
      this.undoLayers.shift();
      this.undoSelections.shift();
    }
  }

  /** Applies scratch to committed as a single undo step. Returns true if anything changed. */
  commitScratch(): boolean {
    const [next, undo] = this.committed.apply(this.scratch);
    this.scratch = new Layer();
    if (undo.size() === 0) {
      this.notify();
      return false;
    }
    this.committed = next;
    this.pushUndo(undo, this.pendingSelection);
    this.redoLayers = [];
    this.redoSelections = [];
    this.notify(true);
    return true;
  }

  /** Commits a diff directly, bypassing scratch. */
  commit(diff: Layer): boolean {
    this.setScratch(diff);
    return this.commitScratch();
  }

  /** Erases everything as one undo step. */
  clear(): void {
    const diff = new Layer();
    for (const k of this.committed.map.keys()) diff.map.set(k, "");
    this.selection = null;
    this.commit(diff);
  }

  undo(): boolean {
    const diff = this.undoLayers.pop();
    if (!diff) return false;
    const [next, redo] = this.committed.apply(diff);
    this.committed = next;
    this.redoLayers.push(redo);
    this.redoSelections.push(this.selection);
    this.selection = this.undoSelections.pop() ?? null;
    this.scratch = new Layer();
    this.notify(true);
    return true;
  }

  redo(): boolean {
    const diff = this.redoLayers.pop();
    if (!diff) return false;
    const [next, undo] = this.committed.apply(diff);
    this.committed = next;
    this.undoLayers.push(undo);
    this.undoSelections.push(this.selection);
    this.selection = this.redoSelections.pop() ?? null;
    this.scratch = new Layer();
    this.notify(true);
    return true;
  }
}

import { Vector } from "./vector";

/** Read-only view of cell glyphs. `null` means empty. */
export interface LayerView {
  get(p: Vector): string | null;
  keys(): Vector[];
}

/** "" and " " are erase markers when a layer is applied onto another. */
export const isErase = (value: string | null | undefined): boolean => value === "" || value === " ";

/**
 * Sparse map of cell -> glyph. Port of ASCIIFlow's `Layer`.
 * A layer used as a diff may hold erase markers ("" or " ").
 */
export class Layer implements LayerView {
  map = new Map<string, string>();

  static from(entries: Iterable<[Vector, string]>): Layer {
    const l = new Layer();
    for (const [p, v] of entries) l.set(p, v);
    return l;
  }

  clone(): Layer {
    const l = new Layer();
    l.map = new Map(this.map);
    return l;
  }

  get(p: Vector): string | null {
    return this.map.get(p.key()) ?? null;
  }

  has(p: Vector): boolean {
    return this.map.has(p.key());
  }

  set(p: Vector, value: string): void {
    this.map.set(p.key(), value);
  }

  delete(p: Vector): void {
    this.map.delete(p.key());
  }

  clear(): void {
    this.map.clear();
  }

  size(): number {
    return this.map.size;
  }

  keys(): Vector[] {
    return [...this.map.keys()].map(Vector.fromKey);
  }

  entries(): [Vector, string][] {
    return [...this.map.entries()].map(([k, v]) => [Vector.fromKey(k), v]);
  }

  /** Copies every entry (erase markers included) from `other` into this layer. */
  setFrom(other: Layer): void {
    for (const [k, v] of other.map) this.map.set(k, v);
  }

  /**
   * Applies a diff layer. Returns the resulting layer and the inverse diff
   * that undoes the operation. Does not mutate `this`.
   */
  apply(diff: Layer): [Layer, Layer] {
    const next = this.clone();
    const undo = new Layer();
    for (const [k, v] of diff.map) {
      const old = this.map.get(k);
      if (isErase(v)) next.map.delete(k);
      else next.map.set(k, v);
      if (old !== v) undo.map.set(k, old ?? "");
    }
    return [next, undo];
  }
}

/** Stack of layers, topmost last. Erase markers in upper layers hide lower cells. */
export class StackedLayers implements LayerView {
  constructor(private readonly layers: Layer[]) {}

  get(p: Vector): string | null {
    const k = p.key();
    for (let i = this.layers.length - 1; i >= 0; i--) {
      const layer = this.layers[i]!;
      if (layer.map.has(k)) {
        const v = layer.map.get(k)!;
        return isErase(v) ? null : v;
      }
    }
    return null;
  }

  keys(): Vector[] {
    const keys = new Set<string>();
    for (const l of this.layers) for (const k of l.map.keys()) keys.add(k);
    return [...keys].map(Vector.fromKey);
  }
}

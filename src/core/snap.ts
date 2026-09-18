// Merges freshly drawn structure with committed structure: forms tees and
// crosses, detaches from deleted cells, and normalises leftover junctions.
// Ported from ASCIIFlow (client/snap.ts), MIT © Lewis Hemens.
import {
  connect,
  connectable,
  connectionGlyph,
  connects,
  disconnect,
  isArrow,
  isBoxDrawing,
} from "./glyphs";
import { isErase, Layer, StackedLayers } from "./layer";
import { Direction, Vector } from "./vector";

/**
 * Returns a layer of extra edits to apply on top of `scratch`.
 * `protect` holds cell keys that must not be normalised (content being moved).
 */
export function snap(scratch: Layer, committed: Layer, protect: ReadonlySet<string> = new Set()): Layer {
  const layer = new Layer();
  const modifiedScratch = new StackedLayers([scratch, layer]);

  for (const position of scratch.keys()) {
    const value = scratch.get(position);
    if (!isBoxDrawing(value)) continue;
    for (const direction of Direction.ALL) {
      const adjacent = position.add(direction);
      // Don't snap to other scratch cells.
      if (scratch.has(adjacent)) continue;
      const adjacentValue = committed.get(adjacent);
      if (!isBoxDrawing(adjacentValue)) continue;
      // Connect this cell to the adjacent committed glyph.
      if (
        connects(adjacentValue, direction.opposite()) &&
        !connects(value, direction) &&
        connectable(value, direction)
      ) {
        layer.set(position, connect(modifiedScratch.get(position), direction)!);
      }
      // Connect the adjacent committed glyph to this cell, accumulating so a
      // cell gaining several connections in one pass keeps all of them.
      const currentAdjacent = layer.has(adjacent) ? layer.get(adjacent) : adjacentValue;
      if (
        connects(value, direction) &&
        !connects(currentAdjacent, direction.opposite()) &&
        connectable(currentAdjacent, direction.opposite())
      ) {
        layer.set(adjacent, connect(currentAdjacent, direction.opposite())!);
      }
    }
  }

  // Unsnap from deleted cells.
  for (const position of scratch.keys()) {
    if (!isErase(scratch.get(position))) continue;
    for (const direction of Direction.ALL) {
      const adjacent = position.add(direction);
      if (scratch.has(adjacent)) continue;
      const adjacentValue = committed.get(adjacent);
      if (!isBoxDrawing(adjacentValue)) continue;
      const currentAdjacent = layer.has(adjacent) ? layer.get(adjacent) : adjacentValue;
      if (connects(currentAdjacent, direction.opposite())) {
        layer.set(adjacent, disconnect(currentAdjacent, direction.opposite())!);
      }
    }
  }

  // Normalise each touched line glyph to the neighbours it actually connects to.
  const stateAt = (p: Vector): string | null => {
    const k = p.key();
    if (layer.map.has(k)) {
      const v = layer.map.get(k)!;
      return isErase(v) ? null : v;
    }
    if (scratch.map.has(k)) {
      const v = scratch.map.get(k)!;
      return isErase(v) ? null : v;
    }
    return committed.get(p);
  };

  const candidates = new Set<string>();
  const consider = (p: Vector) => {
    const k = p.key();
    if (!protect.has(k)) candidates.add(k);
  };
  for (const position of scratch.keys()) {
    consider(position);
    for (const d of Direction.ALL) consider(position.add(d));
  }

  for (let pass = 0; pass < 2; pass++) {
    for (const k of candidates) {
      const position = Vector.fromKey(k);
      const value = stateAt(position);
      if (!isBoxDrawing(value) || isArrow(value)) continue;
      const dirs = Direction.ALL.filter((d) => {
        const n = stateAt(position.add(d));
        return isBoxDrawing(n) && connects(n, d.opposite());
      });
      const glyph = connectionGlyph(dirs);
      if (glyph && glyph !== value) layer.set(position, glyph);
    }
  }

  return layer;
}

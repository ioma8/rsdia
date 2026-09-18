import { Vector } from "./vector";

/** Rectangle spanned by two corner cells (inclusive), in any order. */
export class Box {
  constructor(
    public readonly start: Vector,
    public readonly end: Vector,
  ) {}

  left(): number {
    return Math.min(this.start.x, this.end.x);
  }
  right(): number {
    return Math.max(this.start.x, this.end.x);
  }
  top(): number {
    return Math.min(this.start.y, this.end.y);
  }
  bottom(): number {
    return Math.max(this.start.y, this.end.y);
  }
  width(): number {
    return this.right() - this.left() + 1;
  }
  height(): number {
    return this.bottom() - this.top() + 1;
  }
  topLeft(): Vector {
    return new Vector(this.left(), this.top());
  }
  topRight(): Vector {
    return new Vector(this.right(), this.top());
  }
  bottomLeft(): Vector {
    return new Vector(this.left(), this.bottom());
  }
  bottomRight(): Vector {
    return new Vector(this.right(), this.bottom());
  }

  contains(p: Vector): boolean {
    return p.x >= this.left() && p.x <= this.right() && p.y >= this.top() && p.y <= this.bottom();
  }

  translate(delta: Vector): Box {
    return new Box(this.topLeft().add(delta), this.bottomRight().add(delta));
  }
}

/** Bounding box of an arbitrary set of cells, or null when empty. */
export function boundingBox(cells: Iterable<Vector>): Box | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const c of cells) {
    if (c.x < minX) minX = c.x;
    if (c.y < minY) minY = c.y;
    if (c.x > maxX) maxX = c.x;
    if (c.y > maxY) maxY = c.y;
  }
  if (minX === Infinity) return null;
  return new Box(new Vector(minX, minY), new Vector(maxX, maxY));
}

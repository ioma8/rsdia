/** Immutable 2D integer vector. Canvas cells are addressed by Vector. */
export class Vector {
  constructor(
    public readonly x: number,
    public readonly y: number,
  ) {}

  static fromKey(key: string): Vector {
    const i = key.indexOf(",");
    return new Vector(Number(key.slice(0, i)), Number(key.slice(i + 1)));
  }

  key(): string {
    return `${this.x},${this.y}`;
  }

  /** Display form, matching ASCIIFlow's `x:y`. */
  toString(): string {
    return `${this.x}:${this.y}`;
  }

  equals(other: Vector | null | undefined): boolean {
    return other != null && this.x === other.x && this.y === other.y;
  }

  add(other: Vector): Vector {
    return new Vector(this.x + other.x, this.y + other.y);
  }

  subtract(other: Vector): Vector {
    return new Vector(this.x - other.x, this.y - other.y);
  }

  scale(factor: number): Vector {
    return new Vector(this.x * factor, this.y * factor);
  }

  up(n = 1): Vector {
    return new Vector(this.x, this.y - n);
  }

  down(n = 1): Vector {
    return new Vector(this.x, this.y + n);
  }

  left(n = 1): Vector {
    return new Vector(this.x - n, this.y);
  }

  right(n = 1): Vector {
    return new Vector(this.x + n, this.y);
  }
}

/** The four unit directions. Instances are singletons, so compare with ===. */
export class Direction extends Vector {
  static readonly UP = new Direction(0, -1, "up");
  static readonly DOWN = new Direction(0, 1, "down");
  static readonly LEFT = new Direction(-1, 0, "left");
  static readonly RIGHT = new Direction(1, 0, "right");
  static readonly ALL: readonly Direction[] = [
    Direction.UP,
    Direction.DOWN,
    Direction.LEFT,
    Direction.RIGHT,
  ];

  private constructor(
    x: number,
    y: number,
    public readonly name: string,
  ) {
    super(x, y);
  }

  static of(x: number, y: number): Direction {
    const d = Direction.ALL.find((d) => d.x === x && d.y === y);
    if (!d) throw new Error(`Not a unit direction: ${x},${y}`);
    return d;
  }

  opposite(): Direction {
    switch (this) {
      case Direction.UP:
        return Direction.DOWN;
      case Direction.DOWN:
        return Direction.UP;
      case Direction.LEFT:
        return Direction.RIGHT;
      default:
        return Direction.LEFT;
    }
  }

  isHorizontal(): boolean {
    return this === Direction.LEFT || this === Direction.RIGHT;
  }
}

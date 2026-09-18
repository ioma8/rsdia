// Box-drawing glyph tables and connection rules.
// Ported from ASCIIFlow (client/constants.ts, client/characters.ts), MIT © Lewis Hemens.
import { Direction } from "./vector";

export interface CharacterSet {
  cornerTopLeft: string;
  cornerTopRight: string;
  cornerBottomRight: string;
  cornerBottomLeft: string;
  arrowLeft: string;
  arrowRight: string;
  arrowUp: string;
  arrowDown: string;
  lineVertical: string;
  lineHorizontal: string;
  junctionDown: string;
  junctionUp: string;
  junctionLeft: string;
  junctionRight: string;
  junctionAll: string;
}

export const UNICODE: CharacterSet = {
  cornerTopLeft: "┌",
  cornerTopRight: "┐",
  cornerBottomRight: "┘",
  cornerBottomLeft: "└",
  arrowLeft: "◄",
  arrowRight: "►",
  arrowUp: "▲",
  arrowDown: "▼",
  lineVertical: "│",
  lineHorizontal: "─",
  junctionDown: "┬",
  junctionUp: "┴",
  junctionLeft: "┤",
  junctionRight: "├",
  junctionAll: "┼",
};

export const ASCII: CharacterSet = {
  cornerTopLeft: "+",
  cornerTopRight: "+",
  cornerBottomRight: "+",
  cornerBottomLeft: "+",
  arrowLeft: "<",
  arrowRight: ">",
  arrowUp: "^",
  arrowDown: "v",
  lineVertical: "|",
  lineHorizontal: "-",
  junctionDown: "+",
  junctionUp: "+",
  junctionLeft: "+",
  junctionRight: "+",
  junctionAll: "+",
};

/** Unicode glyph -> "ASCII Basic" glyph, used by export. */
export const BASIC: ReadonlyMap<string, string> = new Map(
  (Object.keys(UNICODE) as (keyof CharacterSet)[]).map((k) => [UNICODE[k], ASCII[k]]),
);

const U = 1;
const R = 2;
const D = 4;
const L = 8;

const BIT = new Map<Direction, number>([
  [Direction.UP, U],
  [Direction.RIGHT, R],
  [Direction.DOWN, D],
  [Direction.LEFT, L],
]);

/** Connection mask of every line/junction glyph. */
const LINE_MASK = new Map<string, number>([
  ["┌", D | R],
  ["┐", D | L],
  ["┘", U | L],
  ["└", U | R],
  ["─", L | R],
  ["│", U | D],
  ["┬", D | L | R],
  ["┴", U | L | R],
  ["┤", U | D | L],
  ["├", U | D | R],
  ["┼", U | D | L | R],
]);

/** Arrow heads connect only on the side their shaft enters from. */
const ARROW_MASK = new Map<string, number>([
  ["◄", R],
  ["►", L],
  ["▲", D],
  ["▼", U],
]);

const MASK_TO_LINE = new Map<number, string>([...LINE_MASK].map(([g, m]) => [m, g]));

const popcount = (m: number) => ((m & U) && 1) + ((m & R) && 1) + ((m & D) && 1) + ((m & L) && 1);

/** Lines, junctions and arrow heads — ASCIIFlow's BOX_DRAWING_VALUES. */
export function isBoxDrawing(value: string | null | undefined): value is string {
  return value != null && (LINE_MASK.has(value) || ARROW_MASK.has(value));
}

/** Lines and junctions, without arrows. */
export function isLine(value: string | null | undefined): value is string {
  return value != null && LINE_MASK.has(value);
}

export function isArrow(value: string | null | undefined): value is string {
  return value != null && ARROW_MASK.has(value);
}

/** ASCIIFlow's `isSpecial`: any glyph the select tool treats as structure. */
export const isSpecial = isBoxDrawing;

function maskOf(value: string | null | undefined): number {
  if (value == null) return 0;
  return LINE_MASK.get(value) ?? ARROW_MASK.get(value) ?? 0;
}

export function connects(value: string | null | undefined, dir: Direction): boolean {
  return (maskOf(value) & BIT.get(dir)!) !== 0;
}

export function connectable(value: string | null | undefined, dir: Direction): boolean {
  if (value == null) return false;
  if (LINE_MASK.has(value)) return true;
  return connects(value, dir);
}

export function connections(value: string | null | undefined): Direction[] {
  return Direction.ALL.filter((d) => connects(value, d));
}

/** The line glyph connecting exactly `dirs`, or null (fewer than two directions). */
export function connectionGlyph(dirs: readonly Direction[]): string | null {
  let m = 0;
  for (const d of dirs) m |= BIT.get(d)!;
  return MASK_TO_LINE.get(m) ?? null;
}

/** Adds connection(s) to a glyph. Throws for glyphs that cannot take the connection. */
export function connect<T extends string | null>(value: T, dir: Direction | readonly Direction[]): T | string {
  if (Array.isArray(dir)) {
    return (dir as Direction[]).reduce<T | string>((v, d) => connect(v as T, d), value);
  }
  const d = dir as Direction;
  if (connects(value, d)) return value;
  const m = value == null ? undefined : LINE_MASK.get(value);
  if (m === undefined) throw new Error(`Can't connect ${value} in direction ${d.name}`);
  return MASK_TO_LINE.get(m | BIT.get(d)!)!;
}

/** Removes connection(s) from a glyph when a clean glyph remains; otherwise keeps it. */
export function disconnect<T extends string | null>(value: T, dir: Direction | readonly Direction[]): T | string {
  if (Array.isArray(dir)) {
    return (dir as Direction[]).reduce<T | string>((v, d) => disconnect(v as T, d), value);
  }
  const d = dir as Direction;
  if (!connects(value, d)) return value;
  const m = value == null ? undefined : LINE_MASK.get(value);
  if (m === undefined) return value;
  const next = m & ~BIT.get(d)!;
  return popcount(next) >= 2 ? MASK_TO_LINE.get(next)! : value;
}

export const ARROW_FOR = new Map<Direction, string>([
  [Direction.LEFT, UNICODE.arrowLeft],
  [Direction.RIGHT, UNICODE.arrowRight],
  [Direction.UP, UNICODE.arrowUp],
  [Direction.DOWN, UNICODE.arrowDown],
]);

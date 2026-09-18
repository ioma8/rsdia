// Canvas compositing (§3.4): grid, committed, scratch, selection, text cursor, hover.
import { boundingBox } from "../core/box";
import type { Editor } from "../core/editor";
import { isErase } from "../core/layer";
import type { HoverHint } from "../core/tools/tool";
import { Vector } from "../core/vector";
import type { GridStyle } from "../storage/config";
import { ATTR, type Painter } from "./painter";

/** Canvas cell shown at screen (0, 0). */
export class Viewport {
  origin = new Vector(0, 0);

  toCanvas(sx: number, sy: number): Vector {
    return new Vector(sx + this.origin.x, sy + this.origin.y);
  }

  pan(dx: number, dy: number): void {
    this.origin = new Vector(this.origin.x + dx, this.origin.y + dy);
  }

  /** Centers the drawing; large drawings align to the top-left below the toolbar. */
  recenter(editor: Editor, width: number, height: number, top: number): void {
    const box = boundingBox(editor.canvas.committed.keys());
    const availH = height - top - 1;
    if (!box) {
      this.origin = new Vector(-2, -top);
      return;
    }
    const ox = box.width() <= width - 4 ? box.left() - Math.floor((width - box.width()) / 2) : box.left() - 2;
    const oy =
      box.height() <= availH ? box.top() - top - Math.floor((availH - box.height()) / 2) : box.top() - top;
    this.origin = new Vector(ox, oy);
  }
}

export interface CanvasViewState {
  editor: Editor;
  viewport: Viewport;
  grid: GridStyle;
  hoverCell: Vector | null;
  hoverHint: HoverHint;
  /** Blink phase for the text cursor. */
  cursorOn: boolean;
}

export function renderCanvas(p: Painter, s: CanvasViewState): void {
  const { pal } = p;
  const { canvas } = s.editor;
  const committed = canvas.committed.map;
  const scratch = canvas.scratch.map;
  const sel = canvas.selection;
  const { x: ox, y: oy } = s.viewport.origin;
  const highlightHover =
    s.hoverCell != null && s.editor.tool === "select" && !s.editor.drawing && s.hoverHint !== "default";

  for (let sy = 0; sy < p.height; sy++) {
    const cy = sy + oy;
    for (let sx = 0; sx < p.width; sx++) {
      const cx = sx + ox;
      const key = `${cx},${cy}`;
      let bg = pal.bg;
      let fg = pal.fg;
      let attr = 0;
      let ch: string | undefined;

      const sv = scratch.get(key);
      if (sv !== undefined) {
        bg = pal.highlight;
        fg = pal.scratch;
        ch = isErase(sv) ? undefined : sv;
      } else {
        ch = committed.get(key);
      }

      if (sel && cx >= sel.left() && cx <= sel.right() && cy >= sel.top() && cy <= sel.bottom()) {
        bg = pal.selectionBg;
      }

      if (ch === undefined) {
        // Grid decoration, anchored to canvas coordinates so it doesn't shimmer when panning.
        if (bg === pal.bg) {
          switch (s.grid) {
            case "lattice":
              // U+1FB7C: left and bottom hairlines in one glyph. Terminals draw it flush with the cell edges, unlike an underline.
              p.buf.setCell(sx, sy, "\u{1FB7C}", pal.grid, bg, 0);
              continue;
            case "checker":
              if (((cx + cy) & 1) === 1) bg = pal.bgAlt;
              break;
            case "dots":
              p.buf.setCell(sx, sy, "·", pal.grid, bg, 0);
              continue;
          }
        }
        ch = " ";
      }

      if (highlightHover && cx === s.hoverCell!.x && cy === s.hoverCell!.y && ch !== " ") {
        attr |= ATTR.INVERSE;
      }
      p.buf.setCell(sx, sy, ch, fg, bg, attr);
    }
  }

  // Dimensions label while dragging structure (ASCIIFlow shows W×H).
  const tool = s.editor.tool;
  if (s.editor.drawing && canvas.scratch.size() > 0 && tool !== "text" && tool !== "eraser") {
    const box = boundingBox(canvas.scratch.keys());
    if (box) {
      const label = `${box.width()}×${box.height()}`;
      p.text(box.right() + 1 - ox, box.bottom() + 1 - oy, label, pal.bg, pal.selectionBg);
    }
  }

  // Text cursor: reverse video.
  const cursor = s.editor.tool === "text" ? s.editor.text.cursor : null;
  if (cursor && s.cursorOn) {
    const sx = cursor.x - ox;
    const sy = cursor.y - oy;
    if (sx >= 0 && sy >= 0 && sx < p.width && sy < p.height) {
      const v = canvas.rendered().get(cursor) ?? " ";
      p.buf.setCell(sx, sy, v, pal.fg, pal.bg, ATTR.INVERSE);
    }
  }
}

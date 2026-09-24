//! Canvas compositing: grid, committed, scratch, selection, text cursor, hover.

use ratatui::style::Modifier;

use crate::core::editor::Editor;
use crate::core::grid::bounding_box;
use crate::core::layer::is_erase;
use crate::core::vector::Pos;
use crate::storage::config::GridStyle;
use crate::tui::painter::Painter;

/// Canvas cell shown at screen (0, 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub origin: Pos,
}

impl Viewport {
    pub fn to_canvas(self, sx: i32, sy: i32) -> Pos {
        Pos::new(sx + self.origin.x, sy + self.origin.y)
    }

    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.origin = Pos::new(self.origin.x + dx, self.origin.y + dy);
    }

    /// Centers the drawing; large drawings align to the top-left below the toolbar.
    pub fn recenter(&mut self, editor: &Editor, width: i32, height: i32, top: i32) {
        let keys: Vec<Pos> = editor.canvas.committed.positions().collect();
        let avail_h = height - top - 1;
        let Some(b) = bounding_box(keys) else {
            self.origin = Pos::new(-2, -top);
            return;
        };
        let ox = if b.width() <= width - 4 {
            b.left() - (width - b.width()) / 2
        } else {
            b.left() - 2
        };
        let oy = if b.height() <= avail_h {
            b.top() - top - (avail_h - b.height()) / 2
        } else {
            b.top() - top
        };
        self.origin = Pos::new(ox, oy);
    }
}

pub struct CanvasViewState<'a> {
    pub editor: &'a Editor,
    pub viewport: Viewport,
    pub grid: GridStyle,
    pub hover_cell: Option<Pos>,
    /// The cell under the pointer is something the select tool would grab.
    pub hover_is_target: bool,
    /// Blink phase for the text cursor.
    pub cursor_on: bool,
}

pub fn render_canvas(p: &mut Painter, s: &CanvasViewState) {
    let pal = p.pal;
    let canvas = &s.editor.canvas;
    let sel = canvas.selection;
    let (ox, oy) = (s.viewport.origin.x, s.viewport.origin.y);
    let highlight_hover = s.hover_cell.is_some()
        && s.editor.tool() == crate::core::editor::ToolId::Select
        && !s.editor.drawing
        && s.hover_is_target;

    for sy in 0..p.height {
        let cy = sy + oy;
        for sx in 0..p.width {
            let cx = sx + ox;
            let pos = Pos::new(cx, cy);
            let mut bg = pal.bg;
            let mut fg = pal.fg;
            let mut modifier = Modifier::empty();
            let mut ch: Option<char> = None;

            // A scratch cell always takes the highlight, even when it only erases.
            if let Some(sv) = canvas.scratch.get(pos) {
                bg = pal.highlight;
                fg = pal.scratch;
                if !is_erase(sv) {
                    ch = Some(sv);
                }
            } else if let Some(v) = canvas.committed.glyph(pos) {
                ch = Some(v);
            }

            if let Some(b) = sel {
                if b.contains(pos) {
                    bg = pal.selection_bg;
                }
            }

            let Some(ch) = ch else {
                // Grid decoration, anchored to canvas coordinates so it doesn't shimmer when panning.
                if bg == pal.bg {
                    match s.grid {
                        GridStyle::Lattice => {
                            // U+1FB7C: left and bottom hairlines in one glyph, flush with the cell edges.
                            p.cell(sx, sy, "\u{1FB7C}", pal.grid, bg, Modifier::empty());
                            continue;
                        }
                        GridStyle::Checker => {
                            if (cx + cy) % 2 != 0 {
                                bg = pal.bg_alt;
                            }
                        }
                        GridStyle::Dots => {
                            p.cell(sx, sy, "·", pal.grid, bg, Modifier::empty());
                            continue;
                        }
                        GridStyle::Off => {}
                    }
                }
                p.cell(sx, sy, " ", fg, bg, modifier);
                continue;
            };

            if highlight_hover && s.hover_cell == Some(pos) && ch != ' ' {
                modifier |= Modifier::REVERSED;
            }
            p.cell(sx, sy, &ch.to_string(), fg, bg, modifier);
        }
    }

    // Dimensions label while dragging structure (ASCIIFlow shows W×H).
    let tool = s.editor.tool();
    if s.editor.drawing
        && !canvas.scratch.is_empty()
        && tool != crate::core::editor::ToolId::Text
        && tool != crate::core::editor::ToolId::Eraser
    {
        if let Some(b) = bounding_box(canvas.scratch.positions().collect::<Vec<_>>()) {
            let label = format!("{}×{}", b.width(), b.height());
            let y = b.bottom() + 1 - oy;
            let x = b.right() + 1 - ox;
            p.text(x, y, &label, pal.bg, pal.selection_bg);
        }
    }

    // Text cursor: reverse video.
    let cursor = if tool == crate::core::editor::ToolId::Text {
        s.editor.text().cursor
    } else {
        None
    };
    if let (Some(cursor), true) = (cursor, s.cursor_on) {
        let sx = cursor.x - ox;
        let sy = cursor.y - oy;
        if sx >= 0 && sy >= 0 && sx < p.width && sy < p.height {
            let v = canvas
                .glyph_at(cursor)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            p.cell(sx, sy, &v, pal.fg, pal.bg, Modifier::REVERSED);
        }
    }
}

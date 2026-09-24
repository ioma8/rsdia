//! Canvas compositing: grid, committed, scratch, selection, text cursor, hover.

use ratatui::style::Modifier;

use crate::core::editor::Editor;
use crate::core::grid::bounding_box;
use crate::core::layer::is_erase;
use crate::core::vector::Pos;
use crate::storage::config::GridStyle;
use crate::tui::painter::{Painter, Rect};

/// Width of the line-number gutter, including its separator column.
const GUTTER_WIDTH: i32 = 5;

/// Canvas coordinates map to screen coordinates after the line-number gutter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub origin: Pos,
}

impl Viewport {
    pub fn to_canvas(self, sx: i32, sy: i32) -> Pos {
        Pos::new(sx - GUTTER_WIDTH + self.origin.x, sy + self.origin.y)
    }

    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.origin = Pos::new(self.origin.x + dx, self.origin.y + dy);
    }

    /// Centers the drawing inside `area`, the canvas strip between the menu bar and
    /// the tool picker; a drawing too large for it aligns to the strip's top-left.
    pub fn recenter(&mut self, editor: &Editor, area: Rect) {
        let keys: Vec<Pos> = editor.canvas.committed.positions().collect();
        let (top, avail_w, avail_h) = (area.y, area.w - GUTTER_WIDTH, area.h);
        let Some(b) = bounding_box(keys) else {
            self.origin = Pos::new(-2, -top);
            return;
        };
        let ox = if b.width() <= avail_w - 4 {
            b.left() - (avail_w - b.width()) / 2
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
    pub top: i32,
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

    for sy in s.top..p.height - 1 {
        let cy = sy + oy;
        // Document rows, like an editor gutter: panning scrolls the numbers. The
        // muted colour is the app's secondary-text token, which terminal themes
        // honour; `DIM` would look right where supported and full-brightness where
        // it is not, and chrome that competes with the drawing is the worse failure.
        // The clip keeps a wide row number from eating the separator column.
        p.text_clipped(
            0,
            sy,
            &format!("{:>3}", cy + 1),
            pal.muted,
            pal.bg,
            Modifier::empty(),
            GUTTER_WIDTH - 1,
        );
        p.cell(
            GUTTER_WIDTH - 1,
            sy,
            "│",
            pal.muted,
            pal.bg,
            Modifier::empty(),
        );
        for sx in GUTTER_WIDTH..p.width {
            let cx = sx - GUTTER_WIDTH + ox;
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
            let x = GUTTER_WIDTH + b.right() + 1 - ox;
            p.text(x, y, &label, pal.bg, pal.selection_bg);
        }
    }

    p.chrome.push(Rect {
        x: 0,
        y: s.top,
        w: GUTTER_WIDTH,
        h: (p.height - s.top - 1).max(0),
    });

    // Text cursor: reverse video.
    let cursor = if tool == crate::core::editor::ToolId::Text {
        s.editor.text().cursor
    } else {
        None
    };
    if let (Some(cursor), true) = (cursor, s.cursor_on) {
        let sx = GUTTER_WIDTH + cursor.x - ox;
        let sy = cursor.y - oy;
        if sx >= GUTTER_WIDTH && sy >= s.top && sx < p.width && sy < p.height - 1 {
            let v = canvas
                .glyph_at(cursor)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            p.cell(sx, sy, &v, pal.fg, pal.bg, Modifier::REVERSED);
        }
    }
}

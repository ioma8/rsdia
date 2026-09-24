//! Canvas compositing: grid, committed, scratch, selection, text cursor, hover.

use ratatui::style::{Color, Modifier, Style};

use crate::core::canvas::Canvas;
use crate::core::editor::{Editor, ToolId};
use crate::core::grid::bounding_box;
use crate::core::layer::is_erase;
use crate::core::vector::Pos;
use crate::storage::config::GridStyle;
use crate::tui::painter::{Painter, Rect};
use crate::tui::theme::Palette;

/// Width of the line-number gutter, including its separator column.
const GUTTER_WIDTH: i32 = 5;

/// Canvas coordinates map to screen coordinates after the line-number gutter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub origin: Pos,
}

impl Viewport {
    #[must_use]
    pub const fn to_canvas(self, sx: i32, sy: i32) -> Pos {
        Pos::new(sx - GUTTER_WIDTH + self.origin.x, sy + self.origin.y)
    }

    pub const fn pan(&mut self, dx: i32, dy: i32) {
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

/// A canvas cell: the glyph written on it, the background under it, and whether
/// the pointer highlight covers it.
fn cell_of(
    s: &CanvasViewState,
    canvas: &Canvas,
    pos: Pos,
    pal: &Palette,
    highlight_hover: bool,
) -> (Option<char>, Color, bool) {
    let mut bg = pal.bg;
    let mut glyph = None;
    // A scratch cell always takes the highlight, even when it only erases.
    if let Some(v) = canvas.scratch.get(pos) {
        bg = pal.highlight;
        if !is_erase(v) {
            glyph = Some(v);
        }
    } else if let Some(v) = canvas.committed.glyph(pos) {
        glyph = Some(v);
    }
    if canvas.selection.is_some_and(|b| b.contains(pos)) {
        bg = pal.selection_bg;
    }
    let reversed = highlight_hover && s.hover_cell == Some(pos) && glyph != Some(' ');
    (glyph, bg, reversed)
}

/// What an empty canvas cell shows: the grid style's decoration, if it asks for
/// one. Anchored to canvas coordinates so it does not shimmer while panning, and
/// skipped when a selection or the scratch layer owns the background.
fn backdrop(
    style: GridStyle,
    cx: i32,
    cy: i32,
    pal: &Palette,
    bg: Color,
) -> (&'static str, Color, Color) {
    if bg != pal.bg {
        return (" ", pal.fg, bg);
    }
    match style {
        // U+1FB7C: left and bottom hairlines in one glyph, flush with the edges.
        GridStyle::Lattice => ("\u{1FB7C}", pal.grid, pal.bg),
        GridStyle::Dots => ("·", pal.grid, pal.bg),
        GridStyle::Checker if (cx + cy) % 2 != 0 => (" ", pal.fg, pal.bg_alt),
        GridStyle::Checker | GridStyle::Off => (" ", pal.fg, pal.bg),
    }
}

pub fn render_canvas(p: &mut Painter, s: &CanvasViewState) {
    let pal = p.pal;
    let canvas = &s.editor.canvas;
    let (ox, oy) = (s.viewport.origin.x, s.viewport.origin.y);
    let highlight_hover = s.hover_cell.is_some()
        && s.editor.tool() == ToolId::Select
        && !s.editor.drawing
        && s.hover_is_target;

    for sy in s.top..p.height - 1 {
        let cy = sy + oy;
        // Document rows, like an editor gutter: panning scrolls the numbers. The
        // muted colour is the app's secondary-text token, which terminal themes
        // honour; `DIM` would look right where supported and full-brightness where
        // it is not, and chrome that competes with the drawing is the worse failure.
        // The clip keeps a wide row number from eating the separator column.
        let style = Style::new().fg(pal.muted).bg(pal.bg);
        p.text_clipped(0, sy, &format!("{:>3}", cy + 1), style, GUTTER_WIDTH - 1);
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
            let (glyph, bg, reversed) = cell_of(s, canvas, Pos::new(cx, cy), &pal, highlight_hover);
            if let Some(glyph) = glyph {
                let modifier = if reversed {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                };
                p.cell(sx, sy, &glyph.to_string(), pal.fg, bg, modifier);
            } else {
                let (sym, fg, bg) = backdrop(s.grid, cx, cy, &pal, bg);
                p.cell(sx, sy, sym, fg, bg, Modifier::empty());
            }
        }
    }

    // Dimensions label while dragging structure (ASCIIFlow shows W×H).
    let tool = s.editor.tool();
    if s.editor.drawing
        && !canvas.scratch.is_empty()
        && tool != ToolId::Text
        && tool != ToolId::Eraser
    {
        if let Some(b) = bounding_box(canvas.scratch.positions()) {
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
    let cursor = if tool == ToolId::Text {
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
                .map_or_else(|| " ".to_string(), String::from);
            p.cell(sx, sy, &v, pal.fg, pal.bg, Modifier::REVERSED);
        }
    }
}

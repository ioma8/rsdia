//! Box tool.
//!
//! Ported from ASCIIFlow (`client/draw/box.ts`), MIT © Lewis Hemens.

use crate::core::canvas::Canvas;
use crate::core::glyphs::UNICODE;
use crate::core::grid::Bounds;
use crate::core::layer::Layer;
use crate::core::snap::snap;
use crate::core::vector::Pos;
use std::collections::HashSet;

use super::tool::{Mods, Tool};

pub fn draw_box(b: Bounds) -> Layer {
    let mut layer = Layer::new();
    if b.right() != b.left() {
        for x in b.left()..=b.right() {
            layer.set(Pos::new(x, b.top()), UNICODE.line_horizontal);
            layer.set(Pos::new(x, b.bottom()), UNICODE.line_horizontal);
        }
    }
    if b.top() != b.bottom() {
        for y in b.top()..=b.bottom() {
            layer.set(Pos::new(b.left(), y), UNICODE.line_vertical);
            layer.set(Pos::new(b.right(), y), UNICODE.line_vertical);
        }
    }
    if b.left() != b.right() && b.top() != b.bottom() {
        layer.set(b.top_left(), UNICODE.corner_top_left);
        layer.set(b.top_right(), UNICODE.corner_top_right);
        layer.set(b.bottom_right(), UNICODE.corner_bottom_right);
        layer.set(b.bottom_left(), UNICODE.corner_bottom_left);
    }
    layer
}

#[derive(Default)]
pub struct BoxTool {
    start_position: Option<Pos>,
}

impl Tool for BoxTool {
    fn start(&mut self, _canvas: &mut Canvas, p: Pos, _m: Mods) {
        self.start_position = Some(p);
    }

    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        let Some(start) = self.start_position else {
            return;
        };
        let mut layer = draw_box(Bounds::new(start, p));
        layer.set_from(&snap(&layer, &canvas.committed, &HashSet::new()));
        canvas.set_scratch(layer);
    }

    fn end(&mut self, canvas: &mut Canvas) {
        self.start_position = None;
        canvas.commit_scratch();
    }

    fn cleanup(&mut self, _canvas: &mut Canvas) {
        self.start_position = None;
    }

    fn handle_key(&mut self, _canvas: &mut Canvas, _key: super::tool::Key, _m: Mods) -> bool {
        false
    }

    fn hover_is_target(&self, _canvas: &Canvas, _p: Pos, _m: Mods) -> bool {
        true
    }
}

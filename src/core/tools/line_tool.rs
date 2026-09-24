//! Line and arrow tools.
//!
//! Ported from ASCIIFlow (`client/draw/line.ts`), MIT © Lewis Hemens.

use std::collections::HashSet;

use crate::core::canvas::Canvas;
use crate::core::layer::Layer;
use crate::core::route::{arrow_head, connect_endpoints, infer_horizontal_first, line};
use crate::core::snap::snap;
use crate::core::vector::Pos;

use super::tool::{HoverHint, Key, Mods, Tool};

/// Builds the scratch layer for a line/arrow drag.
pub fn draw_line(committed: &Layer, start: Pos, end: Pos, is_arrow: bool, flip: bool) -> Layer {
    let horizontal_first = infer_horizontal_first(start, end, committed, flip);
    let mut layer = line(start, end, horizontal_first);
    if is_arrow {
        layer.set(end, arrow_head(start, end, horizontal_first));
    }
    // Endpoints join whatever structure points at them. An arrow's head stays a head.
    let ends = if is_arrow {
        vec![start]
    } else {
        vec![start, end]
    };
    connect_endpoints(&mut layer, committed, &ends);
    layer.set_from(&snap(&layer, committed, &HashSet::new()));
    layer
}

pub struct LineTool {
    pub is_arrow: bool,
    start_position: Option<Pos>,
    end_position: Option<Pos>,
    last_mods: Option<Mods>,
}

impl LineTool {
    pub fn new(is_arrow: bool) -> Self {
        Self {
            is_arrow,
            start_position: None,
            end_position: None,
            last_mods: None,
        }
    }

    pub fn active(&self) -> bool {
        self.start_position.is_some()
    }

    fn draw(&mut self, canvas: &mut Canvas, m: Mods) {
        let (Some(s), Some(e)) = (self.start_position, self.end_position) else {
            return;
        };
        self.last_mods = Some(m);
        // A zero-length drag draws nothing.
        if s == e {
            canvas.set_scratch(Layer::new());
            return;
        }
        canvas.set_scratch(draw_line(&canvas.committed, s, e, self.is_arrow, m.flip));
    }
}

impl Tool for LineTool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, m: Mods) {
        self.start_position = Some(p);
        self.end_position = Some(p);
        self.draw(canvas, m);
    }

    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, m: Mods) {
        self.end_position = Some(p);
        self.draw(canvas, m);
    }

    fn end(&mut self, canvas: &mut Canvas) {
        self.start_position = None;
        self.end_position = None;
        canvas.commit_scratch();
    }

    fn cleanup(&mut self, _canvas: &mut Canvas) {
        self.start_position = None;
        self.end_position = None;
    }

    /// Re-renders with new modifiers, so flipping mid-drag updates the preview.
    fn handle_key(&mut self, canvas: &mut Canvas, _key: Key, m: Mods) -> bool {
        if !self.active() {
            return false;
        }
        if self.last_mods.map(|l| l.flip) != Some(m.flip) {
            self.draw(canvas, m);
        }
        false
    }

    fn hover_hint(&self, _canvas: &Canvas, _p: Pos, _m: Mods) -> HoverHint {
        HoverHint::Crosshair
    }
}

//! Freeform eraser: drags a path of erase markers, snapped so adjacent lines
//! detach cleanly.

use std::collections::HashSet;

use crate::core::canvas::Canvas;
use crate::core::layer::{Layer, ERASE};
use crate::core::snap::snap;
use crate::core::vector::Pos;

use super::tool::{Key, Mods, Tool};

#[derive(Default)]
pub struct EraserTool {
    layer: Option<Layer>,
}

impl EraserTool {
    fn draw(&mut self, canvas: &mut Canvas, p: Pos) {
        let Some(layer) = self.layer.as_mut() else {
            return;
        };
        layer.set(p, ERASE);
        let mut scratch = layer.clone();
        scratch.set_from(&snap(&scratch, &canvas.committed, &HashSet::new()));
        canvas.set_scratch(scratch);
    }
}

impl Tool for EraserTool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        self.layer = Some(Layer::new());
        self.draw(canvas, p);
    }

    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        self.draw(canvas, p);
    }

    fn end(&mut self, canvas: &mut Canvas) {
        self.layer = None;
        canvas.commit_scratch();
    }

    fn cleanup(&mut self, _canvas: &mut Canvas) {
        self.layer = None;
    }

    fn handle_key(&mut self, _canvas: &mut Canvas, _key: Key, _m: Mods) -> bool {
        false
    }

    fn hover_is_target(&self, _canvas: &Canvas, _p: Pos, _m: Mods) -> bool {
        true
    }
}

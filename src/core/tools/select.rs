//! Entity-aware select & move tool.
//!
//! Ported from ASCIIFlow (`client/draw/select.ts`), MIT © Lewis Hemens, plus
//! rsdia's keyboard nudge, clipboard and paste.

use indexmap::IndexMap;

use crate::core::canvas::Canvas;
use crate::core::entity::{
    cells_in_box, detect_line_tip, detect_word, find_box, move_box_with_attachments, move_cells,
    trace_box_attachments, trace_line_from_tip, BoxAttachment,
};
use crate::core::glyphs::is_box_drawing;
use crate::core::grid::{bounding_box, Bounds};
use crate::core::layer::{Layer, LayerView, StackedLayers, ERASE};
use crate::core::route::{arrow_head, connect_endpoints, line};
use crate::core::snap::snap;
use crate::core::text::{layer_to_text, text_to_layer};
use crate::core::vector::Pos;
use std::collections::HashSet;

use super::moveline::MoveTool;
use super::tool::{HoverHint, Key, Mods, Tool};

struct LineReshape {
    cells: Vec<Pos>,
    anchor: Pos,
    is_arrow: bool,
    horizontal_segment: bool,
    moved: bool,
}

fn nudge(key: Key) -> Option<Pos> {
    match key {
        Key::Up => Some(Pos::new(0, -1)),
        Key::Down => Some(Pos::new(0, 1)),
        Key::Left => Some(Pos::new(-1, 0)),
        Key::Right => Some(Pos::new(1, 0)),
        _ => None,
    }
}

fn dedupe(cells: Vec<Pos>) -> Vec<Pos> {
    let mut seen: IndexMap<Pos, ()> = IndexMap::new();
    for c in cells {
        seen.insert(c, ());
    }
    seen.into_keys().collect()
}

#[derive(Default)]
pub struct SelectTool {
    pub select_box: Option<Bounds>,
    selected_cells: Vec<Pos>,
    selecting: bool,
    select_anchor: Option<Pos>,
    drag_start: Option<Pos>,
    drag_end: Option<Pos>,
    move_tool: Option<MoveTool>,
    line_reshape: Option<LineReshape>,
    active_box: Option<Bounds>,
    attachments: Vec<BoxAttachment>,
}

impl SelectTool {
    pub fn has_selection(&self, canvas: &Canvas) -> bool {
        !self.selected_cells.is_empty() && canvas.selection.is_some()
    }

    fn in_selection(&self, canvas: &Canvas, p: Pos) -> bool {
        self.select_box.is_some_and(|b| b.contains(p)) && canvas.selection.is_some()
    }

    fn set_selection(&mut self, canvas: &mut Canvas, cells: Vec<Pos>, keep_scratch: bool) {
        self.selected_cells = dedupe(cells);
        self.select_box = bounding_box(self.selected_cells.iter().copied());
        self.active_box = None;
        if !keep_scratch {
            canvas.clear_scratch();
        }
        canvas.set_selection(self.select_box);
    }

    fn start_select(&mut self, canvas: &mut Canvas, p: Pos) {
        self.selecting = true;
        self.select_anchor = Some(p);
        self.select_box = Some(Bounds::new(p, p));
        self.selected_cells = Vec::new();
        self.active_box = None;
        canvas.set_selection(self.select_box);
    }

    fn move_select(&mut self, canvas: &mut Canvas, p: Pos) {
        let anchor = self.select_anchor.expect("a selection drag has an anchor");
        self.select_box = Some(Bounds::new(anchor, p));
        canvas.set_selection(self.select_box);
    }

    fn finish_select(&mut self, canvas: &mut Canvas) {
        let Some(b) = self.select_box else { return };
        self.selected_cells = cells_in_box(&canvas.committed, b);
        // A rubber-band selection moves like a box: lines crossing its edge reflow.
        self.active_box = self.select_box;
    }

    fn begin_drag(&mut self, canvas: &mut Canvas, p: Pos) {
        self.drag_start = Some(p);
        self.drag_end = Some(p);
        self.attachments = match self.active_box {
            Some(b) => trace_box_attachments(&canvas.committed, b),
            None => Vec::new(),
        };
    }

    fn move_drag(&mut self, canvas: &mut Canvas, p: Pos) {
        self.drag_end = Some(p);
        let Some(start) = self.drag_start else { return };
        let delta = p.subtract(start);
        if let Some(b) = self.active_box {
            canvas.set_scratch(move_box_with_attachments(
                &canvas.committed,
                b,
                &self.attachments,
                delta,
            ));
            canvas.set_selection(Some(b.translate(delta)));
        } else {
            canvas.set_scratch(move_cells(&canvas.committed, &self.selected_cells, delta));
            canvas.set_selection(bounding_box(
                self.selected_cells.iter().map(|c| c.add(delta)),
            ));
        }
    }

    fn reshape_tip(&mut self, canvas: &mut Canvas, target: Pos, m: Mods) {
        let Some(r) = self.line_reshape.as_mut() else {
            return;
        };
        r.moved = true;
        let cells = r.cells.clone();
        let anchor = r.anchor;
        let is_arrow = r.is_arrow;
        let horizontal_segment = r.horizontal_segment;

        let mut base = canvas.committed.clone();
        for c in &cells {
            base.delete(*c);
        }

        let mut layer = Layer::new();
        for c in &cells {
            layer.set(*c, ERASE);
        }

        if anchor != target {
            // Leave the pivot along the line's axis, then bend toward the cursor.
            let horizontal_first = horizontal_segment != m.flip;
            layer.set_from(&line(anchor, target, horizontal_first));
            if is_arrow {
                layer.set(target, arrow_head(anchor, target, horizontal_first));
            }
            let ends = if is_arrow {
                vec![anchor]
            } else {
                vec![anchor, target]
            };
            connect_endpoints(&mut layer, &base, &ends);
        }

        layer.set_from(&snap(&layer, &base, &HashSet::new()));
        canvas.set_scratch(layer);
        canvas.clear_selection();
    }

    /// Text of the current selection, or `None`.
    pub fn copy_selection(&self, canvas: &Canvas) -> Option<String> {
        let b = self.select_box?;
        canvas.selection.as_ref()?;
        let mut cells = Layer::new();
        for c in &self.selected_cells {
            if let Some(v) = canvas.committed.get(*c) {
                cells.set(*c, v);
            }
        }
        Some(layer_to_text(
            &StackedLayers::new(vec![&cells]),
            Some(b),
            false,
        ))
    }

    /// Erases the selected content as one undo step.
    pub fn erase_selection(&mut self, canvas: &mut Canvas) -> bool {
        if self.selected_cells.is_empty() {
            return false;
        }
        let mut layer = Layer::new();
        for c in &self.selected_cells {
            layer.set(*c, ERASE);
        }
        layer.set_from(&snap(&layer, &canvas.committed, &HashSet::new()));
        let changed = canvas.commit(layer);
        self.selected_cells = Vec::new();
        self.select_box = None;
        self.active_box = None;
        canvas.clear_selection();
        changed
    }

    /// Pastes text with its top-left at `at`, committed, and selects it.
    pub fn paste(&mut self, canvas: &mut Canvas, text: &str, at: Pos) {
        let pasted = text_to_layer(text, at);
        if pasted.is_empty() {
            return;
        }
        let mut layer = pasted.clone();
        layer.set_from(&snap(&pasted, &canvas.committed, &HashSet::new()));
        canvas.commit(layer);
        let cells: Vec<Pos> = pasted.keys();
        self.set_selection(canvas, cells, false);
    }
}

impl Tool for SelectTool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        let value = canvas.committed.get(p);

        if !self.selected_cells.is_empty() && self.in_selection(canvas, p) {
            self.begin_drag(canvas, p);
            return;
        }

        if let Some(tip) = detect_line_tip(&canvas.committed, p) {
            let trace = trace_line_from_tip(&canvas.committed, tip.tip, tip.body_dir);
            self.line_reshape = Some(LineReshape {
                cells: trace.cells,
                anchor: trace.anchor,
                is_arrow: tip.arrow.is_some(),
                horizontal_segment: tip.axis == crate::core::entity::Axis::Horizontal,
                moved: false,
            });
            return;
        }

        if let Some(word) = detect_word(&canvas.committed, p) {
            self.set_selection(canvas, word, false);
            self.begin_drag(canvas, p);
            return;
        }

        if value.is_some_and(is_box_drawing) {
            let mut move_tool = MoveTool::default();
            move_tool.start(canvas, p, Mods::NONE);
            self.move_tool = Some(move_tool);
            return;
        }

        if let Some(b) = find_box(&canvas.committed, p) {
            let cells = cells_in_box(&canvas.committed, b);
            self.set_selection(canvas, cells, false);
            self.active_box = Some(b);
            self.begin_drag(canvas, p);
            return;
        }

        self.start_select(canvas, p);
    }

    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, m: Mods) {
        if self.line_reshape.is_some() {
            self.reshape_tip(canvas, p, m);
        } else if self.drag_start.is_some() {
            self.move_drag(canvas, p);
        } else if let Some(tool) = self.move_tool.as_mut() {
            tool.move_to(canvas, p, m);
        } else if self.selecting {
            self.move_select(canvas, p);
        }
    }

    fn end(&mut self, canvas: &mut Canvas) {
        if let Some(r) = self.line_reshape.take() {
            if r.moved {
                canvas.commit_scratch();
            } else {
                canvas.clear_scratch();
            }
        } else if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
            let delta = end.subtract(start);
            let box_at_drag = self.active_box;
            canvas.commit_scratch();
            if let Some(b) = box_at_drag {
                let moved = b.translate(delta);
                self.selected_cells = cells_in_box(&canvas.committed, moved);
                self.select_box = Some(moved);
                self.active_box = Some(moved);
                canvas.set_selection(Some(moved));
            } else {
                let cells = self.selected_cells.iter().map(|c| c.add(delta)).collect();
                self.set_selection(canvas, cells, true);
            }
        } else if let Some(mut tool) = self.move_tool.take() {
            tool.end(canvas);
        } else if self.selecting {
            self.finish_select(canvas);
        }
        self.drag_start = None;
        self.drag_end = None;
        self.selecting = false;
    }

    fn cleanup(&mut self, canvas: &mut Canvas) {
        self.selected_cells = Vec::new();
        self.select_box = None;
        self.active_box = None;
        self.attachments = Vec::new();
        self.line_reshape = None;
        self.move_tool = None;
        self.drag_start = None;
        self.drag_end = None;
        self.selecting = false;
        canvas.clear_selection();
        canvas.clear_scratch();
    }

    fn handle_key(&mut self, canvas: &mut Canvas, key: Key, _m: Mods) -> bool {
        if key == Key::Backspace || key == Key::Delete {
            return self.erase_selection(canvas);
        }
        let Some(delta) = nudge(key) else {
            return false;
        };
        if self.has_selection(canvas) && self.drag_start.is_none() && !self.selecting {
            let from = self.select_box.expect("a selection has a box").top_left();
            self.begin_drag(canvas, from);
            self.move_drag(canvas, from.add(delta));
            self.end(canvas);
            return true;
        }
        false
    }

    fn hover_hint(&self, canvas: &Canvas, p: Pos, _m: Mods) -> HoverHint {
        let committed = &canvas.committed;
        if !self.selected_cells.is_empty() && self.in_selection(canvas, p) {
            return HoverHint::Move;
        }
        if detect_line_tip(committed, p).is_some() {
            return HoverHint::Crosshair;
        }
        if detect_word(committed, p).is_some() {
            return HoverHint::Move;
        }
        if committed.get(p).is_some_and(is_box_drawing) {
            return MoveTool::default().hover_hint(canvas, p, Mods::NONE);
        }
        if find_box(committed, p).is_some() {
            return HoverHint::Move;
        }
        HoverHint::Default
    }
}

//! Committed + scratch layers with undo/redo.
//!
//! Mirrors ASCIIFlow's `CanvasStore` (`client/store/canvas.ts`), minus persistence,
//! which lives in `storage`.

use super::grid::Bounds;
use super::layer::{Layer, StackedLayers, ERASE};

const MAX_UNDO: usize = 500;

/// Revision counters replace the JS `onChange` listener set: the app is the only
/// observer, and it polls these after every event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revision {
    pub any: u64,
    /// Bumped only when the committed layer changed (autosave trigger).
    pub committed: u64,
}

#[derive(Default)]
pub struct Canvas {
    pub committed: Layer,
    pub scratch: Layer,
    pub selection: Option<Bounds>,
    undo_layers: Vec<Layer>,
    redo_layers: Vec<Layer>,
    undo_selections: Vec<Option<Bounds>>,
    redo_selections: Vec<Option<Bounds>>,
    pending_selection: Option<Bounds>,
    revision: Revision,
}

impl Canvas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_committed(committed: Layer) -> Self {
        Self {
            committed,
            ..Self::default()
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_layers.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_layers.is_empty()
    }

    /// Committed overlaid with scratch.
    pub fn rendered(&self) -> StackedLayers<'_> {
        StackedLayers::new(vec![&self.committed, &self.scratch])
    }

    fn notify(&mut self, committed: bool) {
        self.revision.any += 1;
        if committed {
            self.revision.committed += 1;
        }
    }

    pub fn set_selection(&mut self, bounds: Option<Bounds>) {
        self.selection = bounds;
        self.notify(false);
    }

    pub fn clear_selection(&mut self) {
        self.set_selection(None);
    }

    pub fn set_scratch(&mut self, layer: Layer) {
        // Capture the selection as a gesture begins, so undo can restore it.
        if self.scratch.is_empty() && !layer.is_empty() {
            self.pending_selection = self.selection;
        }
        self.scratch = layer;
        self.notify(false);
    }

    pub fn clear_scratch(&mut self) {
        self.scratch = Layer::new();
        self.notify(false);
    }

    fn push_undo(&mut self, layer: Layer, selection: Option<Bounds>) {
        self.undo_layers.push(layer);
        self.undo_selections.push(selection);
        if self.undo_layers.len() > MAX_UNDO {
            self.undo_layers.remove(0);
            self.undo_selections.remove(0);
        }
    }

    /// Applies scratch to committed as a single undo step. Returns true if anything changed.
    pub fn commit_scratch(&mut self) -> bool {
        let (next, undo) = self.committed.apply(&self.scratch);
        self.scratch = Layer::new();
        if undo.is_empty() {
            self.notify(false);
            return false;
        }
        self.committed = next;
        let pending = self.pending_selection.take();
        self.push_undo(undo, pending);
        self.redo_layers.clear();
        self.redo_selections.clear();
        self.notify(true);
        true
    }

    /// Commits a diff directly, bypassing scratch.
    pub fn commit(&mut self, diff: Layer) -> bool {
        self.set_scratch(diff);
        self.commit_scratch()
    }

    /// Erases everything as one undo step.
    pub fn clear(&mut self) {
        let mut diff = Layer::new();
        for p in self.committed.positions() {
            diff.set(p, ERASE);
        }
        self.selection = None;
        self.commit(diff);
    }

    pub fn undo(&mut self) -> bool {
        let Some(diff) = self.undo_layers.pop() else {
            return false;
        };
        let (next, redo) = self.committed.apply(&diff);
        self.committed = next;
        self.redo_layers.push(redo);
        self.redo_selections.push(self.selection);
        self.selection = self.undo_selections.pop().flatten();
        self.scratch = Layer::new();
        self.notify(true);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(diff) = self.redo_layers.pop() else {
            return false;
        };
        let (next, undo) = self.committed.apply(&diff);
        self.committed = next;
        self.undo_layers.push(undo);
        self.undo_selections.push(self.selection);
        self.selection = self.redo_selections.pop().flatten();
        self.scratch = Layer::new();
        self.notify(true);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::layer::LayerView;
    use super::super::text::{layer_to_text, text_to_layer};
    use super::super::vector::Pos;
    use super::*;

    fn v(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn commit_undo_redo() {
        let mut c = Canvas::new();
        c.set_scratch(Layer::from_entries([(v(0, 0), 'a')]));
        assert_eq!(c.rendered().get(v(0, 0)), Some('a'));
        assert_eq!(c.committed.len(), 0);
        c.commit_scratch();
        c.commit(Layer::from_entries([(v(0, 0), 'b'), (v(1, 0), 'c')]));
        assert_eq!(layer_to_text(&c.committed, None, false), "bc");
        c.undo();
        assert_eq!(layer_to_text(&c.committed, None, false), "a");
        c.redo();
        assert_eq!(layer_to_text(&c.committed, None, false), "bc");
        c.undo();
        c.undo();
        assert_eq!(c.committed.len(), 0);
        assert!(!c.can_undo());
        c.commit(Layer::from_entries([(v(5, 5), 'z')]));
        assert!(!c.can_redo());
    }

    #[test]
    fn no_op_commits_dont_create_undo_steps() {
        let mut c = Canvas::with_committed(text_to_layer("x", Pos::default()));
        assert!(!c.commit(Layer::from_entries([(v(0, 0), 'x')])));
        assert!(!c.can_undo());
    }

    #[test]
    fn erase_markers_delete_and_undo_restores() {
        let mut c = Canvas::with_committed(text_to_layer("ab", Pos::default()));
        c.commit(Layer::from_entries([(v(0, 0), ERASE)]));
        assert_eq!(layer_to_text(&c.committed, None, false), "b");
        c.undo();
        assert_eq!(layer_to_text(&c.committed, None, false), "ab");
    }

    #[test]
    fn undo_restores_the_selection_from_before_the_gesture() {
        let mut c = Canvas::new();
        let b = Bounds::new(v(0, 0), v(2, 2));
        c.set_selection(Some(b));
        c.set_scratch(Layer::from_entries([(v(0, 0), 'a')]));
        c.set_selection(None);
        c.commit_scratch();
        c.undo();
        assert_eq!(c.selection, Some(b));
    }

    #[test]
    fn clear_is_one_undo_step() {
        let mut c = Canvas::with_committed(text_to_layer("abc\ndef", Pos::default()));
        c.clear();
        assert_eq!(c.committed.len(), 0);
        c.undo();
        assert_eq!(layer_to_text(&c.committed, None, false), "abc\ndef");
    }
}

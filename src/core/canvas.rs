//! Committed + scratch layers with undo/redo.
//!
//! Mirrors `ASCIIFlow`'s `CanvasStore` (`client/store/canvas.ts`), minus persistence,
//! which lives in `storage`.

use super::grid::Bounds;
use super::layer::{Layer, ERASE};
use super::vector::Pos;

const MAX_UNDO: usize = 500;

/// Revision counters replace the JS `onChange` listener set: the app is the only
/// observer, and it polls these after every event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revision {
    pub any: u64,
    /// Bumped only when the committed layer changed (autosave trigger).
    pub committed: u64,
}

/// One undo step: the diff that reverses the change, and the selection to restore
/// with it.
struct Step {
    diff: Layer,
    selection: Option<Bounds>,
}

#[derive(Default)]
pub struct Canvas {
    pub committed: Layer,
    pub scratch: Layer,
    pub selection: Option<Bounds>,
    undo: Vec<Step>,
    redo: Vec<Step>,
    pending_selection: Option<Bounds>,
    revision: Revision,
}

impl Canvas {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_committed(committed: Layer) -> Self {
        Self {
            committed,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub const fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub const fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The glyph shown at `p`: scratch over committed, erase markers hidden.
    #[must_use]
    pub fn glyph_at(&self, p: Pos) -> Option<char> {
        self.scratch.glyph(p).or_else(|| self.committed.glyph(p))
    }

    const fn notify(&mut self, committed: bool) {
        self.revision.any += 1;
        if committed {
            self.revision.committed += 1;
        }
    }

    pub const fn set_selection(&mut self, bounds: Option<Bounds>) {
        self.selection = bounds;
        self.notify(false);
    }

    pub const fn clear_selection(&mut self) {
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

    fn push_undo(&mut self, diff: Layer, selection: Option<Bounds>) {
        self.undo.push(Step { diff, selection });
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
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
        self.redo.clear();
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

    /// Moves one step between the two stacks: applies its diff and records the
    /// reversal, with the selection as it stood, so undo and redo mirror exactly.
    fn step(&mut self, backwards: bool) -> bool {
        let (from, to) = if backwards {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some(entry) = from.pop() else {
            return false;
        };
        let (next, reversal) = self.committed.apply(&entry.diff);
        self.committed = next;
        to.push(Step {
            diff: reversal,
            selection: self.selection,
        });
        self.selection = entry.selection;
        self.scratch = Layer::new();
        self.notify(true);
        true
    }

    pub fn undo(&mut self) -> bool {
        self.step(true)
    }

    pub fn redo(&mut self) -> bool {
        self.step(false)
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(c.glyph_at(v(0, 0)), Some('a'));
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

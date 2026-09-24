//! Text tool.
//!
//! Based on ASCIIFlow (`client/draw/text.ts`), MIT © Lewis Hemens.
//! rsdia changes: Enter starts a new line under the start column, Escape
//! commits, and each session has its own keystroke undo stack.

use crate::core::canvas::Canvas;
use crate::core::layer::Layer;
use crate::core::text::is_placeable;
use crate::core::vector::Pos;

use super::tool::{Key, Mods, Tool};

struct Snapshot {
    layer: Layer,
    cursor: Pos,
    typed: Key,
}

#[derive(Default)]
pub struct TextTool {
    pub cursor: Option<Pos>,
    line_start: Option<Pos>,
    layer: Option<Layer>,
    history: Vec<Snapshot>,
}

impl TextTool {
    /// A cursor is placed, so printable keys are text, not shortcuts.
    pub fn editing(&self) -> bool {
        self.cursor.is_some()
    }

    fn snapshot(&mut self, typed: Key) {
        if let (Some(layer), Some(cursor)) = (self.layer.as_ref(), self.cursor) {
            self.history.push(Snapshot {
                layer: layer.clone(),
                cursor,
                typed,
            });
        }
    }

    fn write(&mut self, canvas: &mut Canvas, p: Pos, value: char) {
        let Some(layer) = self.layer.as_mut() else {
            return;
        };
        layer.set(p, value);
        canvas.set_scratch(layer.clone());
    }

    /// Undoes the last keystroke of this session. False when there is none.
    pub fn undo_keystroke(&mut self, canvas: &mut Canvas) -> bool {
        let Some(snap) = self.history.pop() else {
            return false;
        };
        self.layer = Some(snap.layer);
        self.cursor = Some(snap.cursor);
        if let Some(layer) = self.layer.as_ref() {
            canvas.set_scratch(layer.clone());
        }
        true
    }

    /// The most recent keystroke, if it was a typed space (used by space-to-pan).
    pub fn last_typed_space(&self) -> bool {
        matches!(self.history.last().map(|s| s.typed), Some(Key::Char(' ')))
    }

    /// Commits the session as one undo step and removes the cursor.
    pub fn commit(&mut self, canvas: &mut Canvas) {
        if self.layer.is_some() {
            canvas.commit_scratch();
        }
        self.layer = None;
        self.history.clear();
        self.cursor = None;
        self.line_start = None;
    }
}

impl Tool for TextTool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        self.cursor = Some(p);
        self.line_start = Some(p);
        if self.layer.is_none() {
            self.layer = Some(Layer::new());
        }
        if let Some(layer) = self.layer.as_ref() {
            canvas.set_scratch(layer.clone());
        }
    }

    fn move_to(&mut self, _canvas: &mut Canvas, _p: Pos, _m: Mods) {}

    fn end(&mut self, _canvas: &mut Canvas) {}

    fn cleanup(&mut self, canvas: &mut Canvas) {
        self.commit(canvas);
    }

    fn handle_key(&mut self, canvas: &mut Canvas, key: Key, _m: Mods) -> bool {
        let (Some(c), Some(_)) = (self.cursor, self.layer.as_ref()) else {
            return false;
        };
        match key {
            Key::Enter => {
                let line_start = self.line_start.unwrap_or(c);
                self.cursor = Some(Pos::new(line_start.x, c.y + 1));
                true
            }
            Key::Backspace => {
                self.snapshot(Key::Backspace);
                self.cursor = Some(c.left());
                // A space, not a delete, so it overwrites committed text underneath.
                self.write(canvas, c.left(), ' ');
                true
            }
            Key::Delete => {
                self.snapshot(Key::Delete);
                self.write(canvas, c, ' ');
                true
            }
            Key::Left => {
                self.cursor = Some(c.left());
                true
            }
            Key::Right => {
                self.cursor = Some(c.right());
                true
            }
            Key::Up => {
                self.cursor = Some(c.up());
                true
            }
            Key::Down => {
                self.cursor = Some(c.down());
                true
            }
            Key::Char(ch) if !is_placeable(ch) => false,
            Key::Char(ch) => {
                self.snapshot(Key::Char(ch));
                self.write(canvas, c, ch);
                self.cursor = Some(c.right());
                true
            }
        }
    }

    fn hover_is_target(&self, _canvas: &Canvas, _p: Pos, _m: Mods) -> bool {
        true
    }
}

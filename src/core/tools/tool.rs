//! Shared tool vocabulary: modifiers, keys, hover hints.
//!
//! Ported from `ASCIIFlow`'s tool contract; rsdia's tools take the canvas as an
//! explicit parameter instead of holding a reference back to the editor.

use crate::core::canvas::Canvas;
use crate::core::vector::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    /// Reverse elbow orientation: Ctrl, or the `f` toggle.
    pub flip: bool,
}

impl Mods {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        flip: false,
    };

    /// Ctrl also flips, matching `ASCIIFlow`'s shift-drag.
    #[must_use]
    pub const fn new(ctrl: bool, alt: bool) -> Self {
        Self {
            ctrl,
            alt,
            flip: ctrl,
        }
    }

    #[must_use]
    pub const fn flipped(mut self, flip: bool) -> Self {
        self.flip = flip;
        self
    }
}

/// A tool-facing key: a printable character or one of the named keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
}

/// Tools receive the canvas for each call; they never store a reference to it.
pub trait Tool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, m: Mods);
    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, m: Mods);
    fn end(&mut self, canvas: &mut Canvas);
    /// Tool switched away.
    fn cleanup(&mut self, canvas: &mut Canvas);
    /// Returns true when the key was consumed.
    fn handle_key(&mut self, canvas: &mut Canvas, key: Key, m: Mods) -> bool;
    /// Whether the cell under the pointer is something this tool would grab.
    fn hover_is_target(&self, canvas: &Canvas, p: Pos, m: Mods) -> bool;
}

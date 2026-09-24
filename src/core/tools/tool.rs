//! Shared tool vocabulary: modifiers, keys, hover hints.
//!
//! Ported from ASCIIFlow's tool contract; rsdia's tools take the canvas as an
//! explicit parameter instead of holding a reference back to the editor.

use super::super::vector::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    /// Reverse elbow orientation: Ctrl, or the `f` toggle.
    pub flip: bool,
}

impl Mods {
    pub const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        flip: false,
    };

    /// Ctrl also flips, matching ASCIIFlow's shift-drag.
    pub fn new(ctrl: bool, alt: bool) -> Self {
        Self {
            ctrl,
            alt,
            flip: ctrl,
        }
    }

    pub fn flipped(mut self, flip: bool) -> Self {
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

/// Terminal stand-in for the CSS cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverHint {
    Default,
    Crosshair,
    Move,
    ResizeH,
    ResizeV,
    Text,
}

/// Tools receive the canvas for each call; they never store a reference to it.
pub trait Tool {
    fn start(&mut self, canvas: &mut super::super::canvas::Canvas, p: Pos, m: Mods);
    fn move_to(&mut self, canvas: &mut super::super::canvas::Canvas, p: Pos, m: Mods);
    fn end(&mut self, canvas: &mut super::super::canvas::Canvas);
    /// Tool switched away.
    fn cleanup(&mut self, canvas: &mut super::super::canvas::Canvas);
    /// Returns true when the key was consumed.
    fn handle_key(&mut self, canvas: &mut super::super::canvas::Canvas, key: Key, m: Mods) -> bool;
    fn hover_hint(&self, canvas: &super::super::canvas::Canvas, p: Pos, m: Mods) -> HoverHint;
}

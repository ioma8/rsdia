//! Terminal key events -> tool keys.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::text::is_placeable;
use crate::core::tools::tool::Key;

fn blocked(k: &KeyEvent) -> bool {
    k.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
}

/// The printable character a key produces, if any.
#[must_use]
pub fn printable(k: &KeyEvent) -> Option<char> {
    if blocked(k) {
        return None;
    }
    match k.code {
        KeyCode::Char(' ') => Some(' '),
        // Kitty-protocol events carry the shifted character already; legacy ones
        // report the unshifted one with SHIFT set.
        KeyCode::Char(c) if c.is_ascii_lowercase() && k.modifiers.contains(KeyModifiers::SHIFT) => {
            Some(c.to_ascii_uppercase())
        }
        KeyCode::Char(c) if is_placeable(c) => Some(c),
        _ => None,
    }
}

/// Alt on its own, so a ctrl+alt chord is never read as a mnemonic.
#[must_use]
pub const fn is_alt(k: &KeyEvent) -> bool {
    k.modifiers.contains(KeyModifiers::ALT) && !k.modifiers.contains(KeyModifiers::CONTROL)
}

/// Tool key for a terminal key: a printable character or a named key.
pub fn tool_key(k: &KeyEvent) -> Option<Key> {
    match k.code {
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Delete => Some(Key::Delete),
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        _ => printable(k).map(Key::Char),
    }
}

/// The `ctrl+<char>` binding a key carries, if any. Ctrl+Shift+Z reports as `z` too.
#[must_use]
pub const fn ctrl_char(k: &KeyEvent) -> Option<char> {
    if !k.modifiers.contains(KeyModifiers::CONTROL) || k.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }
    match k.code {
        KeyCode::Char(c) if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

#[must_use]
pub fn is_ctrl(k: &KeyEvent, c: char) -> bool {
    ctrl_char(k) == Some(c)
}

/// Alt+1..6 arrives as ESC+digit (meta) or as option on macOS with kitty keys.
#[must_use]
pub fn alt_digit(k: &KeyEvent) -> Option<usize> {
    if !is_alt(k) {
        return None;
    }
    match k.code {
        KeyCode::Char(c @ '1'..='6') => c.to_digit(10).map(|d| d as usize),
        _ => None,
    }
}

#[must_use]
pub const fn is_shift(k: &KeyEvent) -> bool {
    k.modifiers.contains(KeyModifiers::SHIFT)
        || matches!(k.code, KeyCode::Char(c) if c.is_ascii_uppercase())
}

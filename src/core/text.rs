//! Layer <-> plain text.
//!
//! Ported from ASCIIFlow (`client/text_utils.ts`), MIT © Lewis Hemens.

use unicode_width::UnicodeWidthChar;

use super::grid::{bounding_box, Bounds};
use super::layer::Layer;
use super::vector::Pos;

fn is_control(c: char) -> bool {
    (c as u32) < 32 || c as u32 == 127
}

/// Terminal display width of a single code point: 0, 1 or 2.
pub fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

/// Text tool / import only accept single-width printable characters.
pub fn is_placeable(c: char) -> bool {
    !is_control(c) && char_width(c) == 1
}

/// Renders the layer as text. Without `bounds`, uses the bounding box of all
/// non-empty cells. Trailing spaces are kept when a box is given (ASCIIFlow
/// behaviour); `trim_right` strips them per row.
pub fn layer_to_text(layer: &Layer, bounds: Option<Bounds>, trim_right: bool) -> String {
    let cells: Vec<Pos> = layer
        .positions()
        .filter(|p| layer.get(*p).is_some())
        .collect();
    let bounds = match bounds.or_else(|| bounding_box(cells.iter().copied())) {
        Some(b) => b,
        None => return String::new(),
    };
    let (w, h) = (
        bounds.width().max(0) as usize,
        bounds.height().max(0) as usize,
    );
    let mut rows: Vec<Vec<char>> = vec![vec![' '; w]; h];
    for p in cells {
        if !bounds.contains(p) {
            continue;
        }
        let Some(mut v) = layer.get(p) else { continue };
        if is_control(v) {
            v = ' ';
        }
        let (x, y) = (
            (p.x - bounds.left()) as usize,
            (p.y - bounds.top()) as usize,
        );
        if let Some(row) = rows.get_mut(y) {
            if let Some(cell) = row.get_mut(x) {
                *cell = v;
            }
        }
    }
    let lines: Vec<String> = rows
        .into_iter()
        .map(|row| {
            let line: String = row.into_iter().collect();
            if trim_right {
                line.trim_end_matches(' ').to_string()
            } else {
                line
            }
        })
        .collect();
    lines.join("\n")
}

/// Loads text at `offset`. Spaces and control characters are skipped; wide
/// characters are replaced with `?` so the grid stays aligned.
pub fn text_to_layer(value: &str, offset: Pos) -> Layer {
    let mut layer = Layer::new();
    let normalized = value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");
    for (y, line) in normalized.split('\n').enumerate() {
        for (x, ch) in line.chars().enumerate() {
            if ch != ' ' && !is_control(ch) {
                layer.set(
                    Pos::new(x as i32, y as i32).add(offset),
                    if char_width(ch) == 1 { ch } else { '?' },
                );
            }
        }
    }
    layer
}

/// Size of a block of text in cells.
pub fn text_size(value: &str) -> Pos {
    // `\r\n` and a lone `\r` both become `\n`, matching ASCIIFlow's regex.
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let mut width = 0;
    let mut lines = 0;
    for line in normalized.split('\n') {
        width = width.max(line.chars().count());
        lines += 1;
    }
    Pos::new(width as i32, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn wide_characters_are_rejected_or_replaced() {
        assert!(is_placeable('a') && is_placeable('─'));
        assert!(!is_placeable('漢') && !is_placeable('😀'));
        assert_eq!(
            layer_to_text(&text_to_layer("a漢b", v(0, 0)), None, false),
            "a?b"
        );
    }

    #[test]
    fn a_box_keeps_the_full_extent() {
        let l = text_to_layer("a", v(0, 0));
        let text = layer_to_text(&l, Some(Bounds::new(v(0, 0), v(2, 1))), false);
        assert_eq!(text, "a  \n   ");
    }

    #[test]
    fn trailing_spaces_trim_per_row_and_crlf_is_normalised() {
        let l = text_to_layer("a  \r\n b", v(0, 0));
        assert_eq!(layer_to_text(&l, None, true), "a\n b");
        assert_eq!(text_size("ab\r\nc"), v(2, 2));
        assert_eq!(char_width('─'), 1);
    }
}

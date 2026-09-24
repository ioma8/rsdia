//! Straight and single-elbow line routing, plus endpoint connection and
//! orientation inference for the line/arrow tools.
//!
//! Ported from `ASCIIFlow` (`client/draw/utils.ts`, `client/draw/line.ts`), MIT © Lewis Hemens.

use super::glyphs::{connect_all, connectable, connects, disconnect_all, is_special, UNICODE};
use super::layer::Layer;
use super::vector::{Direction, Pos};

#[must_use]
pub fn line(start: Pos, end: Pos, horizontal_first: bool) -> Layer {
    if start.x == end.x || start.y == end.y {
        return straight_line(start, end);
    }
    corner_line(start, end, horizontal_first)
}

#[must_use]
pub fn corner_line(start: Pos, end: Pos, horizontal_first: bool) -> Layer {
    let corner = if horizontal_first {
        Pos::new(end.x, start.y)
    } else {
        Pos::new(start.x, end.y)
    };
    let mut layer = straight_line(start, corner);
    layer.set_from(&straight_line(corner, end));
    let right = start.x < end.x;
    let down = start.y < end.y;
    let glyph = if horizontal_first {
        match (right, down) {
            (true, true) => UNICODE.corner_top_right,
            (true, false) => UNICODE.corner_bottom_right,
            (false, true) => UNICODE.corner_top_left,
            (false, false) => UNICODE.corner_bottom_left,
        }
    } else {
        match (down, right) {
            (true, true) => UNICODE.corner_bottom_left,
            (true, false) => UNICODE.corner_bottom_right,
            (false, true) => UNICODE.corner_top_left,
            (false, false) => UNICODE.corner_top_right,
        }
    };
    layer.set(corner, glyph);
    layer
}

/// A one-cell-wide run between two points on the same row or column.
///
/// # Panics
///
/// If the points share neither a row nor a column.
#[must_use]
pub fn straight_line(start: Pos, end: Pos) -> Layer {
    let mut layer = Layer::new();
    assert!(
        start.x == end.x || start.y == end.y,
        "can't draw a straight line between {start} and {end}"
    );
    if start.x == end.x {
        let (top, bottom) = (start.y.min(end.y), start.y.max(end.y));
        for y in top..=bottom {
            layer.set(Pos::new(start.x, y), UNICODE.line_vertical);
        }
    }
    if start.y == end.y {
        let (left, right) = (start.x.min(end.x), start.x.max(end.x));
        for x in left..=right {
            layer.set(Pos::new(x, start.y), UNICODE.line_horizontal);
        }
    }
    layer
}

/// Arrow head glyph at `end` for a route from `start`.
#[must_use]
pub const fn arrow_head(start: Pos, end: Pos, horizontal_first: bool) -> char {
    if end.x == start.x {
        return if end.y < start.y {
            UNICODE.arrow_up
        } else {
            UNICODE.arrow_down
        };
    }
    if end.y == start.y {
        return if end.x < start.x {
            UNICODE.arrow_left
        } else {
            UNICODE.arrow_right
        };
    }
    if horizontal_first {
        if end.y < start.y {
            UNICODE.arrow_up
        } else {
            UNICODE.arrow_down
        }
    } else if end.x > start.x {
        UNICODE.arrow_right
    } else {
        UNICODE.arrow_left
    }
}

/// Whether structure runs vertically through `c`: a neighbour above and below it,
/// or the two arms of an elbow on the same side of it.
fn runs_vertically(c: Pos, layer: &Layer) -> bool {
    let s = |v: Pos| layer.get(v).is_some_and(is_special);
    (s(c.up()) && s(c.down()))
        || (s(c.left().up()) && s(c.left().down()))
        || (s(c.right().up()) && s(c.right().down()))
}

/// The same, horizontally.
fn runs_horizontally(c: Pos, layer: &Layer) -> bool {
    let s = |v: Pos| layer.get(v).is_some_and(is_special);
    (s(c.left()) && s(c.right()))
        || (s(c.left().up()) && s(c.right().up()))
        || (s(c.left().down()) && s(c.right().down()))
}

/// Elbow orientation for a drag from `start` to `end`, inferred from the
/// structure around both endpoints. `flip` reverses the inference.
#[must_use]
pub fn infer_horizontal_first(start: Pos, end: Pos, committed: &Layer, flip: bool) -> bool {
    // A vertical run where the drag starts, or a horizontal one where it ends,
    // means the elbow should turn left/right first.
    (runs_vertically(start, committed) || runs_horizontally(end, committed)) != flip
}

fn combined_get(layer: &Layer, base: &Layer, p: Pos) -> Option<char> {
    layer.get(p).or_else(|| base.get(p))
}

/// Connects the given endpoint cells of `layer` to any structure pointing at
/// them, then trims connections that lead nowhere.
pub fn connect_endpoints(layer: &mut Layer, base: &Layer, ends: &[Pos]) {
    for &p in ends {
        let incoming: Vec<Direction> = Direction::ALL
            .into_iter()
            .filter(|d| {
                let neighbour = combined_get(layer, base, p.add(d.delta()));
                let points_back = neighbour.is_some_and(|n| connects(n, d.opposite()));
                points_back && layer.get(p).is_some_and(|v| connectable(v, *d))
            })
            .collect();
        if let Some(v) = layer.get(p) {
            let connected = connect_all(v, &incoming);
            let rest: Vec<Direction> = Direction::ALL
                .into_iter()
                .filter(|d| !incoming.contains(d))
                .collect();
            layer.set(p, disconnect_all(connected, &rest));
        }
    }
}

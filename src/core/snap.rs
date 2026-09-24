//! Merges freshly drawn structure with committed structure: forms tees and
//! crosses, detaches from deleted cells, and normalises leftover junctions.
//!
//! Ported from `ASCIIFlow` (`client/snap.ts`), MIT © Lewis Hemens.

use super::glyphs::{
    connect, connectable, connection_glyph, connects, disconnect, is_arrow, is_box_drawing,
};
use std::collections::{BTreeSet, HashSet};
use std::hash::BuildHasher;

use super::layer::{is_erase, Layer};
use super::vector::{Direction, Pos};

/// Walks every scratch cell whose own glyph `keep` accepts, and for each of its
/// four neighbours that is committed (never scratch) and holds a box glyph, calls
/// `f` with the cell, the neighbour, the direction, and both glyphs. The snap-in and
/// unsnap passes differ only in `keep` and what `f` does with an edge.
fn over_edges(
    scratch: &Layer,
    committed: &Layer,
    keep: impl Fn(char) -> bool,
    mut f: impl FnMut(Pos, Pos, Direction, char, char),
) {
    for position in scratch.positions() {
        let Some(value) = scratch.get(position) else {
            continue;
        };
        if !keep(value) {
            continue;
        }
        for direction in Direction::ALL {
            let adjacent = position.add(direction.delta());
            // Don't snap to other scratch cells.
            if scratch.has(adjacent) {
                continue;
            }
            let Some(adjacent_value) = committed.get(adjacent) else {
                continue;
            };
            if !is_box_drawing(adjacent_value) {
                continue;
            }
            f(position, adjacent, direction, value, adjacent_value);
        }
    }
}

/// Returns a layer of extra edits to apply on top of `scratch`.
/// `protect` holds cells that must not be normalised (content being moved).
#[must_use]
pub fn snap<H: BuildHasher>(
    scratch: &Layer,
    committed: &Layer,
    protect: &HashSet<Pos, H>,
) -> Layer {
    let mut layer = Layer::new();

    // Cells already written this pass, or held by scratch: both beat committed.
    let state_at = |layer: &Layer, p: Pos| -> Option<char> {
        if let Some(v) = layer.get(p) {
            return if is_erase(v) { None } else { Some(v) };
        }
        if let Some(v) = scratch.get(p) {
            return if is_erase(v) { None } else { Some(v) };
        }
        committed.get(p)
    };

    // Connect each scratch glyph to the committed glyphs it lands beside.
    over_edges(
        scratch,
        committed,
        is_box_drawing,
        |position, adjacent, direction, value, adjacent_value| {
            // Connect this cell to the adjacent committed glyph.
            if connects(adjacent_value, direction.opposite())
                && !connects(value, direction)
                && connectable(value, direction)
            {
                let current = state_at(&layer, position).unwrap_or(value);
                layer.set(position, connect(current, direction));
            }
            // Connect the adjacent committed glyph to this cell, accumulating so a
            // cell gaining several connections in one pass keeps all of them.
            let current_adjacent = layer.get(adjacent).unwrap_or(adjacent_value);
            if connects(value, direction)
                && !connects(current_adjacent, direction.opposite())
                && connectable(current_adjacent, direction.opposite())
            {
                layer.set(adjacent, connect(current_adjacent, direction.opposite()));
            }
        },
    );

    // Unsnap from deleted cells.
    over_edges(
        scratch,
        committed,
        is_erase,
        |_position, adjacent, direction, _value, adjacent_value| {
            let current_adjacent = layer.get(adjacent).unwrap_or(adjacent_value);
            if connects(current_adjacent, direction.opposite()) {
                layer.set(adjacent, disconnect(current_adjacent, direction.opposite()));
            }
        },
    );

    // Normalise each touched line glyph to the neighbours it actually connects to.
    let mut candidates: BTreeSet<Pos> = BTreeSet::new();
    for position in scratch.positions() {
        for p in std::iter::once(position).chain(Direction::ALL.map(|d| position.add(d.delta()))) {
            if !protect.contains(&p) {
                candidates.insert(p);
            }
        }
    }

    for _pass in 0..2 {
        for position in candidates.iter().copied() {
            let Some(value) = state_at(&layer, position) else {
                continue;
            };
            if !is_box_drawing(value) || is_arrow(value) {
                continue;
            }
            let dirs: Vec<Direction> = Direction::ALL
                .into_iter()
                .filter(|d| {
                    state_at(&layer, position.add(d.delta()))
                        .is_some_and(|n| is_box_drawing(n) && connects(n, d.opposite()))
                })
                .collect();
            if let Some(glyph) = connection_glyph(&dirs) {
                if glyph != value {
                    layer.set(position, glyph);
                }
            }
        }
    }

    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn v(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn rebuilds_a_junction_that_gains_connections_from_two_sides() {
        let committed = Layer::from_entries([(v(0, 1), '─'), (v(1, 1), '─'), (v(2, 1), '─')]);
        let scratch = Layer::from_entries([(v(1, 0), '│'), (v(1, 2), '│')]);
        let result = snap(&scratch, &committed, &HashSet::new());
        assert_eq!(result.get(v(1, 1)), Some('┼'));
    }

    #[test]
    fn connects_a_scratch_cell_to_committed_neighbours_on_both_sides() {
        let committed = Layer::from_entries([(v(0, 1), '│'), (v(2, 1), '│')]);
        let scratch = Layer::from_entries([(v(1, 1), '─')]);
        let result = snap(&scratch, &committed, &HashSet::new());
        assert_eq!(result.get(v(0, 1)), Some('├'));
        assert_eq!(result.get(v(2, 1)), Some('┤'));
    }
}

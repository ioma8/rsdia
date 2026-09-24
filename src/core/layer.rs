//! Sparse maps of cell -> glyph, and stacks of them.

use std::collections::BTreeMap;

use super::vector::Pos;

/// A layer with no cell at a position. Glyphs are single code points, so a cell
/// is either absent or holds a `char`.
///
/// erase markers: `ASCIIFlow` uses `""` and `" "` to mean "delete the cell below".
/// Both collapse to [`ERASE`] here, which is why `Layer::apply` treats a space as
/// a deletion rather than as content.
pub const ERASE: char = ' ';

#[must_use]
pub const fn is_erase(c: char) -> bool {
    c == ERASE
}

/// Sparse map of cell -> glyph. A layer used as a diff may hold erase markers.
///
/// Cells read out row-major (`BTreeMap`), so `snap`'s normalisation pass sees the
/// same order no matter which order a tool happened to build the layer in.
#[derive(Clone, Debug, Default)]
pub struct Layer {
    map: BTreeMap<Pos, char>,
}

impl Layer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: impl IntoIterator<Item = (Pos, char)>) -> Self {
        let mut layer = Self::new();
        for (p, v) in entries {
            layer.set(p, v);
        }
        layer
    }

    /// Raw lookup: erase markers come back as `Some(ERASE)`, absent cells as `None`.
    #[must_use]
    pub fn get(&self, p: Pos) -> Option<char> {
        self.map.get(&p).copied()
    }

    /// The glyph a reader wants: `None` for an absent cell or an erase marker.
    #[must_use]
    pub fn glyph(&self, p: Pos) -> Option<char> {
        self.get(p).filter(|v| !is_erase(*v))
    }

    #[must_use]
    pub fn has(&self, p: Pos) -> bool {
        self.map.contains_key(&p)
    }

    pub fn set(&mut self, p: Pos, value: char) {
        self.map.insert(p, value);
    }

    pub fn delete(&mut self, p: Pos) {
        self.map.remove(&p);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Row-major cells, erase markers included.
    pub fn positions(&self) -> impl Iterator<Item = Pos> + '_ {
        self.map.keys().copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = (Pos, char)> + '_ {
        self.map.iter().map(|(p, v)| (*p, *v))
    }

    /// Copies every entry (erase markers included) from `other` into this layer.
    pub fn set_from(&mut self, other: &Self) {
        for (p, v) in other.entries() {
            self.map.insert(p, v);
        }
    }

    /// Applies a diff layer. Returns the resulting layer and the inverse diff that
    /// undoes the operation. Does not mutate `self`.
    #[must_use]
    pub fn apply(&self, diff: &Self) -> (Self, Self) {
        let mut next = self.clone();
        let mut undo = Self::new();
        for (k, v) in diff.entries() {
            let old = self.map.get(&k).copied();
            if is_erase(v) {
                next.map.remove(&k);
            } else {
                next.map.insert(k, v);
            }
            if old != Some(v) {
                undo.map.insert(k, old.unwrap_or(ERASE));
            }
        }
        (next, undo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn apply_returns_the_inverse_diff() {
        let base = Layer::from_entries([(v(0, 0), 'a'), (v(1, 0), 'b')]);
        let diff = Layer::from_entries([(v(1, 0), 'c'), (v(2, 0), 'd'), (v(0, 0), ERASE)]);
        let (next, undo) = base.apply(&diff);
        assert_eq!(next.get(v(0, 0)), None);
        assert_eq!(next.get(v(1, 0)), Some('c'));
        assert_eq!(next.get(v(2, 0)), Some('d'));
        let (back, _) = next.apply(&undo);
        assert_eq!(back.get(v(0, 0)), Some('a'));
        assert_eq!(back.get(v(1, 0)), Some('b'));
        assert_eq!(back.get(v(2, 0)), None);
    }

    #[test]
    fn glyph_hides_erase_markers() {
        let layer = Layer::from_entries([(v(0, 0), 'a'), (v(1, 0), ERASE)]);
        assert_eq!(layer.get(v(0, 0)), Some('a'));
        assert_eq!(layer.glyph(v(0, 0)), Some('a'));
        assert_eq!(layer.get(v(1, 0)), Some(ERASE));
        assert_eq!(layer.glyph(v(1, 0)), None);
        assert_eq!(layer.glyph(v(9, 9)), None);
    }

    #[test]
    fn cells_are_iterated_row_major() {
        let mut layer = Layer::new();
        for p in [v(5, 5), v(0, 0), v(2, 2)] {
            layer.set(p, 'x');
        }
        assert_eq!(
            layer.positions().collect::<Vec<_>>(),
            vec![v(0, 0), v(2, 2), v(5, 5)]
        );
    }
}

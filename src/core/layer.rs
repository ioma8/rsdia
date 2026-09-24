//! Sparse maps of cell -> glyph, and stacks of them.

use indexmap::IndexMap;

use super::vector::Pos;

/// A layer with no cell at a position. Glyphs are single code points, so a cell
/// is either absent or holds a `char`.
///
/// erase markers: ASCIIFlow uses `""` and `" "` to mean "delete the cell below".
/// Both collapse to [`ERASE`] here, which is why `Layer::apply` treats a space as
/// a deletion rather than as content.
pub const ERASE: char = ' ';

pub fn is_erase(c: char) -> bool {
    c == ERASE
}

/// Read-only view of cell glyphs. `None` means empty.
pub trait LayerView {
    fn get(&self, p: Pos) -> Option<char>;
    fn keys(&self) -> Vec<Pos>;
}

/// Sparse map of cell -> glyph. A layer used as a diff may hold erase markers.
///
/// Insertion order is preserved (`IndexMap`), because `snap` iterates the cells it
/// was handed in the order the tool built them and its result depends on it.
#[derive(Clone, Debug, Default)]
pub struct Layer {
    map: IndexMap<Pos, char>,
}

impl Layer {
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
    pub fn get(&self, p: Pos) -> Option<char> {
        self.map.get(&p).copied()
    }

    pub fn has(&self, p: Pos) -> bool {
        self.map.contains_key(&p)
    }

    pub fn set(&mut self, p: Pos, value: char) {
        self.map.insert(p, value);
    }

    pub fn delete(&mut self, p: Pos) {
        self.map.shift_remove(&p);
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Insertion-ordered cells, erase markers included.
    pub fn positions(&self) -> impl Iterator<Item = Pos> + '_ {
        self.map.keys().copied()
    }

    pub fn entries(&self) -> impl Iterator<Item = (Pos, char)> + '_ {
        self.map.iter().map(|(p, v)| (*p, *v))
    }

    /// Copies every entry (erase markers included) from `other` into this layer.
    pub fn set_from(&mut self, other: &Layer) {
        for (p, v) in other.entries() {
            self.map.insert(p, v);
        }
    }

    /// Applies a diff layer. Returns the resulting layer and the inverse diff that
    /// undoes the operation. Does not mutate `self`.
    pub fn apply(&self, diff: &Layer) -> (Layer, Layer) {
        let mut next = self.clone();
        let mut undo = Layer::new();
        for (k, v) in diff.entries() {
            let old = self.map.get(&k).copied();
            if is_erase(v) {
                next.map.shift_remove(&k);
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

impl LayerView for Layer {
    fn get(&self, p: Pos) -> Option<char> {
        Layer::get(self, p)
    }

    fn keys(&self) -> Vec<Pos> {
        self.map.keys().copied().collect()
    }
}

/// Stack of layers, topmost last. Erase markers in upper layers hide lower cells.
pub struct StackedLayers<'a> {
    layers: Vec<&'a Layer>,
}

impl<'a> StackedLayers<'a> {
    pub fn new(layers: Vec<&'a Layer>) -> Self {
        Self { layers }
    }
}

impl LayerView for StackedLayers<'_> {
    fn get(&self, p: Pos) -> Option<char> {
        for layer in self.layers.iter().rev() {
            if let Some(v) = layer.get(p) {
                return if is_erase(v) { None } else { Some(v) };
            }
        }
        None
    }

    fn keys(&self) -> Vec<Pos> {
        let mut keys: IndexMap<Pos, ()> = IndexMap::new();
        for layer in &self.layers {
            for p in layer.positions() {
                keys.insert(p, ());
            }
        }
        keys.into_keys().collect()
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
    fn stacking_hides_lower_cells() {
        let lower = Layer::from_entries([(v(0, 0), 'a')]);
        let upper = Layer::from_entries([(v(0, 0), ERASE), (v(1, 0), 'b')]);
        let stacked = StackedLayers::new(vec![&lower, &upper]);
        assert_eq!(stacked.get(v(0, 0)), None);
        assert_eq!(stacked.get(v(1, 0)), Some('b'));
        assert_eq!(stacked.keys().len(), 2);
    }

    #[test]
    fn insertion_order_is_kept() {
        let mut layer = Layer::new();
        for p in [v(5, 5), v(0, 0), v(2, 2)] {
            layer.set(p, 'x');
        }
        assert_eq!(LayerView::keys(&layer), vec![v(5, 5), v(0, 0), v(2, 2)]);
    }
}

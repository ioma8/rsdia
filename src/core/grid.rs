//! Inclusive rectangles over the cell grid.

use super::vector::Pos;

/// Rectangle spanned by two corner cells (inclusive), in any order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub start: Pos,
    pub end: Pos,
}

impl Bounds {
    pub const fn new(start: Pos, end: Pos) -> Self {
        Self { start, end }
    }

    pub fn left(&self) -> i32 {
        self.start.x.min(self.end.x)
    }

    pub fn right(&self) -> i32 {
        self.start.x.max(self.end.x)
    }

    pub fn top(&self) -> i32 {
        self.start.y.min(self.end.y)
    }

    pub fn bottom(&self) -> i32 {
        self.start.y.max(self.end.y)
    }

    pub fn width(&self) -> i32 {
        self.right() - self.left() + 1
    }

    pub fn height(&self) -> i32 {
        self.bottom() - self.top() + 1
    }

    pub fn top_left(&self) -> Pos {
        Pos::new(self.left(), self.top())
    }

    pub fn top_right(&self) -> Pos {
        Pos::new(self.right(), self.top())
    }

    pub fn bottom_left(&self) -> Pos {
        Pos::new(self.left(), self.bottom())
    }

    pub fn bottom_right(&self) -> Pos {
        Pos::new(self.right(), self.bottom())
    }

    pub fn contains(&self, p: Pos) -> bool {
        p.x >= self.left() && p.x <= self.right() && p.y >= self.top() && p.y <= self.bottom()
    }

    pub fn translate(&self, delta: Pos) -> Bounds {
        Bounds::new(self.top_left().add(delta), self.bottom_right().add(delta))
    }
}

/// Bounding box of an arbitrary set of cells, or `None` when empty.
pub fn bounding_box(cells: impl IntoIterator<Item = Pos>) -> Option<Bounds> {
    let mut min: Option<Pos> = None;
    let mut max = Pos::new(0, 0);
    for c in cells {
        match min {
            None => {
                min = Some(c);
                max = c;
            }
            Some(m) => {
                min = Some(Pos::new(m.x.min(c.x), m.y.min(c.y)));
                max = Pos::new(max.x.max(c.x), max.y.max(c.y));
            }
        }
    }
    min.map(|m| Bounds::new(m, max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_span_both_corners_orders() {
        let b = Bounds::new(Pos::new(4, 6), Pos::new(0, 1));
        assert_eq!((b.left(), b.top(), b.right(), b.bottom()), (0, 1, 4, 6));
        assert_eq!((b.width(), b.height()), (5, 6));
        assert!(b.contains(Pos::new(2, 3)));
        assert!(!b.contains(Pos::new(2, 7)));
        assert_eq!(b.translate(Pos::new(1, 1)).top_left(), Pos::new(1, 2));
        // The bounding box normalises its corners, unlike the box it was built from.
        assert_eq!(
            bounding_box([Pos::new(2, 2), Pos::new(0, 9)]),
            Some(Bounds::new(Pos::new(0, 2), Pos::new(2, 9)))
        );
        assert_eq!(bounding_box([]), None);
    }
}

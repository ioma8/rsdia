//! Grid geometry: cell positions and the four unit directions.

use std::fmt;

/// A canvas cell coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub const fn add(self, other: Pos) -> Self {
        self.offset(other.x, other.y)
    }

    pub const fn subtract(self, other: Pos) -> Self {
        self.offset(-other.x, -other.y)
    }

    pub const fn scale(self, n: i32) -> Self {
        Self {
            x: self.x * n,
            y: self.y * n,
        }
    }

    pub const fn up(self) -> Self {
        self.offset(0, -1)
    }

    pub const fn down(self) -> Self {
        self.offset(0, 1)
    }

    pub const fn left(self) -> Self {
        self.offset(-1, 0)
    }

    pub const fn right(self) -> Self {
        self.offset(1, 0)
    }
}

/// Display form, matching ASCIIFlow's `x:y`.
impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.x, self.y)
    }
}

/// The four unit directions. Values, so they compare and hash by content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];

    pub const fn delta(self) -> Pos {
        match self {
            Direction::Up => Pos::new(0, -1),
            Direction::Down => Pos::new(0, 1),
            Direction::Left => Pos::new(-1, 0),
            Direction::Right => Pos::new(1, 0),
        }
    }

    /// Connection bit for this direction's axis end.
    pub const fn bit(self) -> u8 {
        match self {
            Direction::Up => 1,
            Direction::Right => 2,
            Direction::Down => 4,
            Direction::Left => 8,
        }
    }

    pub const fn opposite(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    pub const fn is_horizontal(self) -> bool {
        matches!(self, Direction::Left | Direction::Right)
    }

    /// `n` steps along this direction.
    pub const fn scale(self, n: i32) -> Pos {
        self.delta().scale(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_opposites() {
        let p = Pos::new(3, 4);
        assert_eq!(p.up().down(), p);
        assert_eq!(p.left().right(), p);
        assert_eq!(p.add(Direction::Right.scale(3)), Pos::new(6, 4));
        assert_eq!(p.to_string(), "3:4");
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::Up.bit(), 1);
        assert!(Direction::Left.is_horizontal());
    }
}

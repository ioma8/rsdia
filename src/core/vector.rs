//! Grid geometry: cell positions and the four unit directions.

use std::fmt;

/// A canvas cell coordinate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    #[must_use]
    pub const fn add(self, other: Self) -> Self {
        self.offset(other.x, other.y)
    }

    #[must_use]
    pub const fn subtract(self, other: Self) -> Self {
        self.offset(-other.x, -other.y)
    }

    #[must_use]
    pub const fn scale(self, n: i32) -> Self {
        Self {
            x: self.x * n,
            y: self.y * n,
        }
    }

    #[must_use]
    pub const fn up(self) -> Self {
        self.offset(0, -1)
    }

    #[must_use]
    pub const fn down(self) -> Self {
        self.offset(0, 1)
    }

    #[must_use]
    pub const fn left(self) -> Self {
        self.offset(-1, 0)
    }

    #[must_use]
    pub const fn right(self) -> Self {
        self.offset(1, 0)
    }
}

/// Display form, matching `ASCIIFlow`'s `x:y`.
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
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];

    #[must_use]
    pub const fn delta(self) -> Pos {
        match self {
            Self::Up => Pos::new(0, -1),
            Self::Down => Pos::new(0, 1),
            Self::Left => Pos::new(-1, 0),
            Self::Right => Pos::new(1, 0),
        }
    }

    /// Connection bit for this direction's axis end.
    #[must_use]
    pub const fn bit(self) -> u8 {
        match self {
            Self::Up => 1,
            Self::Right => 2,
            Self::Down => 4,
            Self::Left => 8,
        }
    }

    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// `n` steps along this direction.
    #[must_use]
    pub const fn scale(self, n: i32) -> Pos {
        self.delta().scale(n)
    }
}

// ---------------------------------------------------------------- screen space

// Screen geometry is `i32`: a pointer or a drag may sit outside the terminal, and
// an off-screen cell must be ignored rather than painted at the far edge, which
// `u16` arithmetic would do. These four are the only places the two spaces meet.
// (Each saturates rather than wrapping.)

/// A coordinate as a buffer column or row; anything off the top edge is 0.
#[must_use]
pub fn px(v: i32) -> u16 {
    if v < 0 {
        return 0;
    }
    u16::try_from(v).unwrap_or(u16::MAX)
}

/// A count (a length, a glyph count) as the `i32` layout arithmetic uses.
#[must_use]
pub fn units(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// A coordinate as a slice index; anything off the top edge is 0.
#[must_use]
pub fn index(v: i32) -> usize {
    usize::try_from(v).unwrap_or(0)
}

/// A count clamped into the `u16` width a widget takes.
#[must_use]
pub fn wide(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
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

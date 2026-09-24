//! Box-drawing glyph tables and connection rules.
//!
//! Ported from ASCIIFlow (`client/constants.ts`, `client/characters.ts`), MIT © Lewis Hemens.

use super::vector::Direction;

/// The fifteen glyphs a drawing is built from, in one of two charsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterSet {
    pub corner_top_left: char,
    pub corner_top_right: char,
    pub corner_bottom_right: char,
    pub corner_bottom_left: char,
    pub arrow_left: char,
    pub arrow_right: char,
    pub arrow_up: char,
    pub arrow_down: char,
    pub line_vertical: char,
    pub line_horizontal: char,
    pub junction_down: char,
    pub junction_up: char,
    pub junction_left: char,
    pub junction_right: char,
    pub junction_all: char,
}

pub const UNICODE: CharacterSet = CharacterSet {
    corner_top_left: '┌',
    corner_top_right: '┐',
    corner_bottom_right: '┘',
    corner_bottom_left: '└',
    arrow_left: '◄',
    arrow_right: '►',
    arrow_up: '▲',
    arrow_down: '▼',
    line_vertical: '│',
    line_horizontal: '─',
    junction_down: '┬',
    junction_up: '┴',
    junction_left: '┤',
    junction_right: '├',
    junction_all: '┼',
};

pub const ASCII: CharacterSet = CharacterSet {
    corner_top_left: '+',
    corner_top_right: '+',
    corner_bottom_right: '+',
    corner_bottom_left: '+',
    arrow_left: '<',
    arrow_right: '>',
    arrow_up: '^',
    arrow_down: 'v',
    line_vertical: '|',
    line_horizontal: '-',
    junction_down: '+',
    junction_up: '+',
    junction_left: '+',
    junction_right: '+',
    junction_all: '+',
};

/// Unicode glyph -> "ASCII Basic" glyph, used by export.
pub fn to_basic(c: char) -> char {
    match c {
        '┌' => '+',
        '┐' => '+',
        '┘' => '+',
        '└' => '+',
        '◄' => '<',
        '►' => '>',
        '▲' => '^',
        '▼' => 'v',
        '│' => '|',
        '─' => '-',
        '┬' | '┴' | '┤' | '├' | '┼' => '+',
        other => other,
    }
}

/// Connection mask of every line/junction glyph.
fn line_mask(c: char) -> Option<u8> {
    Some(match c {
        '┌' => D | R,
        '┐' => D | L,
        '┘' => U | L,
        '└' => U | R,
        '─' => L | R,
        '│' => U | D,
        '┬' => D | L | R,
        '┴' => U | L | R,
        '┤' => U | D | L,
        '├' => U | D | R,
        '┼' => U | D | L | R,
        _ => return None,
    })
}

/// Arrow heads connect only on the side their shaft enters from.
fn arrow_mask(c: char) -> Option<u8> {
    Some(match c {
        '◄' => R,
        '►' => L,
        '▲' => D,
        '▼' => U,
        _ => return None,
    })
}

fn mask_to_line(m: u8) -> Option<char> {
    Some(match m {
        m if m == D | R => '┌',
        m if m == D | L => '┐',
        m if m == U | L => '┘',
        m if m == U | R => '└',
        m if m == L | R => '─',
        m if m == U | D => '│',
        m if m == D | L | R => '┬',
        m if m == U | L | R => '┴',
        m if m == U | D | L => '┤',
        m if m == U | D | R => '├',
        m if m == U | D | L | R => '┼',
        _ => return None,
    })
}

const U: u8 = 1;
const R: u8 = 2;
const D: u8 = 4;
const L: u8 = 8;

const fn popcount(m: u8) -> u32 {
    (m & U).count_ones() + (m & R).count_ones() + (m & D).count_ones() + (m & L).count_ones()
}

fn mask_of(c: char) -> u8 {
    line_mask(c).or_else(|| arrow_mask(c)).unwrap_or(0)
}

/// Lines, junctions and arrow heads — ASCIIFlow's `BOX_DRAWING_VALUES`.
pub fn is_box_drawing(c: char) -> bool {
    line_mask(c).is_some() || arrow_mask(c).is_some()
}

pub fn is_arrow(c: char) -> bool {
    arrow_mask(c).is_some()
}

/// ASCIIFlow's `isSpecial`: any glyph the select tool treats as structure.
pub fn is_special(c: char) -> bool {
    is_box_drawing(c)
}

pub fn connects(c: char, dir: Direction) -> bool {
    mask_of(c) & dir.bit() != 0
}

pub fn connectable(c: char, dir: Direction) -> bool {
    line_mask(c).is_some() || connects(c, dir)
}

pub fn connections(c: char) -> Vec<Direction> {
    Direction::ALL
        .into_iter()
        .filter(|d| connects(c, *d))
        .collect()
}

/// The line glyph connecting exactly `dirs`, or `None` (fewer than two directions).
pub fn connection_glyph(dirs: &[Direction]) -> Option<char> {
    let mut m = 0u8;
    for d in dirs {
        m |= d.bit();
    }
    mask_to_line(m)
}

/// Adds a connection to a glyph. Panics for glyphs that cannot take the connection,
/// matching ASCIIFlow's throw; `connectable` is the guard.
pub fn connect(c: char, dir: Direction) -> char {
    if connects(c, dir) {
        return c;
    }
    match line_mask(c) {
        Some(m) => mask_to_line(m | dir.bit())
            .unwrap_or_else(|| unreachable!("mask {m} plus {} is a line", dir.bit())),
        None => panic!("can't connect {c} in direction {dir:?}"),
    }
}

pub fn connect_all(c: char, dirs: &[Direction]) -> char {
    dirs.iter().fold(c, |v, d| connect(v, *d))
}

/// Removes a connection from a glyph when a clean glyph remains; otherwise keeps it.
pub fn disconnect(c: char, dir: Direction) -> char {
    if !connects(c, dir) {
        return c;
    }
    match line_mask(c) {
        Some(m) => {
            let next = m & !dir.bit();
            if popcount(next) >= 2 {
                mask_to_line(next).unwrap_or(c)
            } else {
                c
            }
        }
        None => c,
    }
}

pub fn disconnect_all(c: char, dirs: &[Direction]) -> char {
    dirs.iter().fold(c, |v, d| disconnect(v, *d))
}

pub fn arrow_for(d: Direction) -> char {
    match d {
        Direction::Left => UNICODE.arrow_left,
        Direction::Right => UNICODE.arrow_right,
        Direction::Up => UNICODE.arrow_up,
        Direction::Down => UNICODE.arrow_down,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_forms_tees_and_crosses() {
        assert_eq!(connect('─', Direction::Down), '┬');
        assert_eq!(connect('│', Direction::Right), '├');
        assert_eq!(connect('┬', Direction::Up), '┼');
        assert_eq!(connect_all('┌', &[Direction::Up, Direction::Left]), '┼');
        assert_eq!(connect_all('─', &[Direction::Down, Direction::Up]), '┼');
    }

    #[test]
    #[should_panic(expected = "can't connect")]
    fn connecting_an_arrow_panics() {
        connect('►', Direction::Up);
    }

    #[test]
    fn disconnect_keeps_a_glyph_when_no_clean_one_remains() {
        assert_eq!(disconnect('┼', Direction::Up), '┬');
        assert_eq!(disconnect('├', Direction::Right), '│');
        assert_eq!(disconnect('└', Direction::Up), '└');
    }

    #[test]
    fn connection_glyph_needs_two_directions() {
        assert_eq!(
            connection_glyph(&[Direction::Down, Direction::Right]),
            Some('┌')
        );
        assert_eq!(connection_glyph(&[Direction::Down]), None);
    }

    #[test]
    fn masks_round_trip_through_the_charset() {
        assert!(is_box_drawing(UNICODE.junction_all) && !is_box_drawing('a'));
        assert!(is_arrow(UNICODE.arrow_left) && !is_box_drawing('a'));
        assert_eq!(to_basic(UNICODE.corner_top_left), ASCII.corner_top_left);
        assert_eq!(to_basic('x'), 'x');
    }
}

//! Entity detection (words, line tips, boxes) and moves that keep attached
//! lines connected.
//!
//! Ported from ASCIIFlow (`client/draw/entity.ts`), MIT © Lewis Hemens.

use std::collections::HashSet;

use super::glyphs::{arrows, connections, connects, is_arrow, is_box_drawing, UNICODE};
use super::grid::{bounding_box, Bounds};
use super::layer::{is_erase, Layer, ERASE};
use super::snap::snap;
use super::vector::{Direction, Pos};

const H: char = UNICODE.line_horizontal;
const V: char = UNICODE.line_vertical;

pub fn is_content(c: char) -> bool {
    !is_erase(c)
}

pub fn is_text(c: char) -> bool {
    is_content(c) && !is_box_drawing(c)
}

/// Maximal horizontal run of text characters under `p`.
pub fn detect_word(layer: &Layer, p: Pos) -> Option<Vec<Pos>> {
    if !layer.get(p).is_some_and(is_text) {
        return None;
    }
    let mut left = p;
    let mut right = p;
    while layer.get(left.left()).is_some_and(is_text) {
        left = left.left();
    }
    while layer.get(right.right()).is_some_and(is_text) {
        right = right.right();
    }
    Some((left.x..=right.x).map(|x| Pos::new(x, p.y)).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineTip {
    pub tip: Pos,
    pub axis: Axis,
    /// Direction from the tip toward the rest of the line.
    pub body_dir: Direction,
    pub arrow: Option<char>,
}

/// The free end of a line or arrow under `p`, if any.
pub fn detect_line_tip(layer: &Layer, p: Pos) -> Option<LineTip> {
    let value = layer.get(p)?;
    let axis = match value {
        H => Axis::Horizontal,
        V => Axis::Vertical,
        c if is_arrow(c) => {
            if c == UNICODE.arrow_left || c == UNICODE.arrow_right {
                Axis::Horizontal
            } else {
                Axis::Vertical
            }
        }
        _ => return None,
    };
    let dirs = match axis {
        Axis::Horizontal => [Direction::Left, Direction::Right],
        Axis::Vertical => [Direction::Up, Direction::Down],
    };
    let connected: Vec<Direction> = dirs
        .into_iter()
        .filter(|d| {
            layer
                .get(p.add(d.delta()))
                .is_some_and(|n| connects(n, d.opposite()))
        })
        .collect();
    if connected.len() != 1 {
        return None;
    }
    Some(LineTip {
        tip: p,
        axis,
        body_dir: connected[0],
        arrow: is_arrow(value).then_some(value),
    })
}

fn is_bend(c: char) -> bool {
    c == UNICODE.corner_top_left
        || c == UNICODE.corner_top_right
        || c == UNICODE.corner_bottom_right
        || c == UNICODE.corner_bottom_left
}

fn straight_char_for(d: Direction) -> char {
    if d.is_horizontal() {
        H
    } else {
        V
    }
}

pub struct TipTrace {
    pub cells: Vec<Pos>,
    pub anchor: Pos,
}

/// Walks from a tip toward the body and stops at the first bend (the pivot).
/// Returns the cells from the tip up to and including that corner.
pub fn trace_line_from_tip(layer: &Layer, tip: Pos, body_dir: Direction) -> TipTrace {
    let mut cells = vec![tip];
    let mut current = tip.add(body_dir.delta());
    for _ in 0..1000 {
        let value = layer.get(current);
        if value == Some(straight_char_for(body_dir)) {
            cells.push(current);
            current = current.add(body_dir.delta());
            continue;
        }
        if let Some(v) = value {
            if is_bend(v) && connects(v, body_dir.opposite()) {
                cells.push(current);
                return TipTrace {
                    cells,
                    anchor: current,
                };
            }
        }
        break;
    }
    let anchor = *cells.last().expect("the tip is always present");
    TipTrace { cells, anchor }
}

fn is_vertical_border(c: Option<char>) -> bool {
    c.is_some_and(|v| {
        is_box_drawing(v) && (connects(v, Direction::Up) || connects(v, Direction::Down))
    })
}

fn is_horizontal_border(c: Option<char>) -> bool {
    c.is_some_and(|v| {
        is_box_drawing(v) && (connects(v, Direction::Left) || connects(v, Direction::Right))
    })
}

const RAY_LIMIT: usize = 400;

fn ray(layer: &Layer, from: Pos, d: Direction, pred: impl Fn(Option<char>) -> bool) -> Option<Pos> {
    let mut p = from;
    for _ in 0..RAY_LIMIT {
        p = p.add(d.delta());
        if pred(layer.get(p)) {
            return Some(p);
        }
    }
    None
}

fn verify_perimeter(layer: &Layer, b: Bounds) -> bool {
    for x in b.left()..=b.right() {
        if !layer.get(Pos::new(x, b.top())).is_some_and(is_box_drawing) {
            return false;
        }
        if !layer
            .get(Pos::new(x, b.bottom()))
            .is_some_and(is_box_drawing)
        {
            return false;
        }
    }
    for y in b.top()..=b.bottom() {
        if !layer.get(Pos::new(b.left(), y)).is_some_and(is_box_drawing) {
            return false;
        }
        if !layer
            .get(Pos::new(b.right(), y))
            .is_some_and(is_box_drawing)
        {
            return false;
        }
    }
    true
}

fn box_from_seed(layer: &Layer, seed: Pos) -> Option<Bounds> {
    let left = ray(layer, seed, Direction::Left, is_vertical_border)?;
    let right = ray(layer, seed, Direction::Right, is_vertical_border)?;
    let up = ray(layer, seed, Direction::Up, is_horizontal_border)?;
    let down = ray(layer, seed, Direction::Down, is_horizontal_border)?;
    let b = Bounds::new(Pos::new(left.x, up.y), Pos::new(right.x, down.y));
    if b.right() - b.left() < 1 || b.bottom() - b.top() < 1 {
        return None;
    }
    verify_perimeter(layer, b).then_some(b)
}

fn area(b: &Bounds) -> i32 {
    (b.right() - b.left()) * (b.bottom() - b.top())
}

const DIAGONALS: [Pos; 4] = [
    Pos::new(-1, -1),
    Pos::new(1, -1),
    Pos::new(-1, 1),
    Pos::new(1, 1),
];

const COMPONENT_LIMIT: usize = 4000;

fn border_component(layer: &Layer, start: Pos) -> Option<Vec<Pos>> {
    let mut seen: HashSet<Pos> = HashSet::from([start]);
    let mut stack = vec![start];
    let mut cells = Vec::new();
    while let Some(current) = stack.pop() {
        cells.push(current);
        if cells.len() > COMPONENT_LIMIT {
            return None;
        }
        for d in Direction::ALL {
            let next = current.add(d.delta());
            if !seen.contains(&next) && layer.get(next).is_some_and(is_box_drawing) {
                seen.insert(next);
                stack.push(next);
            }
        }
    }
    Some(cells)
}

/// Smallest closed box enclosing `p` (on its border or inside), or `None`.
pub fn find_box(layer: &Layer, p: Pos) -> Option<Bounds> {
    let mut candidates: Vec<Bounds> = Vec::new();
    let mut seeds: Vec<Pos> = Vec::new();
    if !layer.get(p).is_some_and(is_box_drawing) {
        seeds.push(p);
    } else {
        for d in Direction::ALL
            .into_iter()
            .map(Direction::delta)
            .chain(DIAGONALS)
        {
            let n = p.add(d);
            if !layer.get(n).is_some_and(is_box_drawing) {
                seeds.push(n);
            }
        }
    }
    for seed in seeds {
        if let Some(b) = box_from_seed(layer, seed) {
            if b.contains(p) {
                candidates.push(b);
            }
        }
    }
    if layer.get(p).is_some_and(is_box_drawing) {
        let bounds = border_component(layer, p).and_then(bounding_box);
        if let Some(b) = bounds {
            if b.right() - b.left() >= 1
                && b.bottom() - b.top() >= 1
                && b.contains(p)
                && verify_perimeter(layer, b)
            {
                candidates.push(b);
            }
        }
    }
    candidates
        .into_iter()
        .fold(None::<Bounds>, |best, b| match best {
            Some(prev) if area(&prev) <= area(&b) => Some(prev),
            _ => Some(b),
        })
}

pub fn cells_in_box(layer: &Layer, b: Bounds) -> Vec<Pos> {
    let mut out = Vec::new();
    for (p, v) in layer.entries() {
        if is_erase(v) {
            continue;
        }
        if b.contains(p) {
            out.push(p);
        }
    }
    out
}

/// Translates `cells` by `delta`, snapping the surrounding structure.
pub fn move_cells(committed: &Layer, cells: &[Pos], delta: Pos) -> Layer {
    let mut layer = Layer::new();
    for &c in cells {
        layer.set(c, ERASE);
    }
    let mut protect: HashSet<Pos> = HashSet::new();
    for &c in cells {
        if let Some(v) = committed.get(c) {
            if is_content(v) {
                let t = c.add(delta);
                layer.set(t, v);
                protect.insert(t);
            }
        }
    }
    layer.set_from(&snap(&layer, committed, &protect));
    layer
}

/// A line leaving a box: where it attaches, where it must still reach, and the
/// cells in between.
pub struct BoxAttachment {
    /// Border cell of the box the line attaches to.
    pub anchor: Pos,
    /// Outward direction, away from the box.
    pub out: Direction,
    /// Fixed endpoint the connector must still reach.
    pub far: Pos,
    /// Connector cells between the box and `far`, corners included.
    pub run_cells: Vec<Pos>,
    /// The line ends in an arrow head pointing into the box.
    pub arrow_into_box: bool,
}

fn turn_from(value: char, dir: Direction) -> Option<Direction> {
    connections(value)
        .into_iter()
        .find(|d| *d != dir.opposite())
}

fn perimeter(b: Bounds) -> Vec<(Pos, Direction)> {
    let mut out = Vec::new();
    for x in b.left()..=b.right() {
        out.push((Pos::new(x, b.top()), Direction::Up));
        out.push((Pos::new(x, b.bottom()), Direction::Down));
    }
    for y in b.top()..=b.bottom() {
        out.push((Pos::new(b.left(), y), Direction::Left));
        out.push((Pos::new(b.right(), y), Direction::Right));
    }
    out
}

/// Lines leaving `box`, each followed through corners to its terminal.
pub fn trace_box_attachments(layer: &Layer, b: Bounds) -> Vec<BoxAttachment> {
    let mut attachments = Vec::new();
    for (anchor, out) in perimeter(b) {
        let first = anchor.add(out.delta());
        let first_value = layer.get(first);
        let straight_out = first_value == Some(straight_char_for(out))
            && first_value.is_some_and(|v| connects(v, out.opposite()));
        let behind = first.add(out.delta());
        let arrow_into_box = first_value == Some(super::glyphs::arrow_for(out.opposite()))
            && layer.get(behind) == Some(straight_char_for(out))
            && layer
                .get(behind)
                .is_some_and(|v| connects(v, out.opposite()));
        if !straight_out && !arrow_into_box {
            continue;
        }

        let mut run_cells = Vec::new();
        let mut dir = out;
        let mut current = first;
        if arrow_into_box {
            run_cells.push(first);
            current = behind;
        }
        for _ in 0..1000 {
            let value = layer.get(current);
            if value == Some(straight_char_for(dir)) {
                run_cells.push(current);
                current = current.add(dir.delta());
                continue;
            }
            if let Some(v) = value {
                if is_bend(v) && connects(v, dir.opposite()) {
                    let Some(next) = turn_from(v, dir) else { break };
                    run_cells.push(current);
                    dir = next;
                    current = current.add(dir.delta());
                    continue;
                }
            }
            break;
        }
        attachments.push(BoxAttachment {
            anchor,
            out,
            far: current,
            run_cells,
            arrow_into_box,
        });
    }
    attachments
}

fn corner_for(a: Direction, b: Direction) -> char {
    let h = if a.is_horizontal() { a } else { b };
    let v = if a.is_horizontal() { b } else { a };
    if h == Direction::Left {
        if v == Direction::Up {
            UNICODE.corner_bottom_right
        } else {
            UNICODE.corner_top_right
        }
    } else if v == Direction::Up {
        UNICODE.corner_bottom_left
    } else {
        UNICODE.corner_top_left
    }
}

fn draw_straight(layer: &mut Layer, from: Pos, to: Pos, exclude_end: bool) {
    let step = Pos::new((to.x - from.x).signum(), (to.y - from.y).signum());
    let ch = if step.x != 0 { H } else { V };
    let mut p = from;
    loop {
        if !(exclude_end && p == to) {
            layer.set(p, ch);
        }
        if p == to {
            break;
        }
        p = p.add(step);
    }
}

fn draw_connector(layer: &mut Layer, from: Pos, to: Pos, out: Direction, far_value: Option<char>) {
    if from == to {
        return;
    }
    let horizontal_first = out.is_horizontal();
    let bend = if horizontal_first {
        Pos::new(to.x, from.y)
    } else {
        Pos::new(from.x, to.y)
    };
    draw_straight(layer, from, bend, bend == to);
    let mut approach = out.delta();
    if bend != to {
        draw_straight(layer, bend, to, true);
        approach = Pos::new((to.x - bend.x).signum(), (to.y - bend.y).signum());
    }
    if bend != from && bend != to {
        let next = if horizontal_first {
            if to.y > bend.y {
                Direction::Down
            } else {
                Direction::Up
            }
        } else if to.x > bend.x {
            Direction::Right
        } else {
            Direction::Left
        };
        layer.set(bend, corner_for(out.opposite(), next));
    }
    if far_value.is_some_and(is_arrow) {
        for (d, ch) in arrows() {
            if d.delta() == approach {
                layer.set(to, ch);
            }
        }
    }
}

/// Moves a box and its contents by `delta`, reflowing attached lines.
pub fn move_box_with_attachments(
    committed: &Layer,
    b: Bounds,
    attachments: &[BoxAttachment],
    delta: Pos,
) -> Layer {
    let mut layer = Layer::new();
    let content = cells_in_box(committed, b);
    for &c in &content {
        layer.set(c, ERASE);
    }
    for a in attachments {
        for &c in &a.run_cells {
            layer.set(c, ERASE);
        }
    }
    for &c in &content {
        if let Some(v) = committed.get(c) {
            if is_content(v) {
                layer.set(c.add(delta), v);
            }
        }
    }
    for a in attachments {
        let new_anchor = a.anchor.add(a.out.delta()).add(delta);
        draw_connector(&mut layer, new_anchor, a.far, a.out, committed.get(a.far));
        if a.arrow_into_box {
            layer.set(new_anchor, super::glyphs::arrow_for(a.out.opposite()));
        }
    }
    let protect: HashSet<Pos> = content.iter().map(|c| c.add(delta)).collect();
    layer.set_from(&snap(&layer, committed, &protect));
    layer
}

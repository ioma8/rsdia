//! Drags a straight line segment sideways, stretching attached lines.
//!
//! Ported from `ASCIIFlow` (`client/draw/move.ts`), MIT © Lewis Hemens.

use crate::core::canvas::Canvas;
use crate::core::glyphs::{connects, is_arrow, is_special, UNICODE};
use crate::core::layer::Layer;
use crate::core::vector::{Direction, Pos};

use super::tool::{Key, Mods, Tool};

struct AttachmentTrace {
    source: Pos,
    end: Pos,
    direction: Direction,
}

struct LineTrace {
    horizontal: bool,
    positions: Vec<Pos>,
    attachments: Vec<AttachmentTrace>,
}

const fn is_straight(v: Option<char>) -> bool {
    matches!(v, Some(c) if c == UNICODE.line_horizontal || c == UNICODE.line_vertical)
}

#[derive(Default)]
pub struct MoveTool {
    trace: Option<LineTrace>,
}

impl Tool for MoveTool {
    fn start(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        if !is_straight(canvas.committed.get(p)) {
            return;
        }
        self.trace = Some(trace_line(&canvas.committed, p));
        self.move_to(canvas, p, Mods::NONE);
    }

    fn move_to(&mut self, canvas: &mut Canvas, p: Pos, _m: Mods) {
        let Some(trace) = self.trace.as_ref() else {
            return;
        };
        let committed = &canvas.committed;
        let mut layer = Layer::new();
        let ends = |d: Direction| -> Vec<Pos> {
            trace
                .attachments
                .iter()
                .filter(|a| a.direction == d)
                .map(|a| a.end)
                .collect()
        };
        let min_x = ends(Direction::Left)
            .iter()
            .map(|e| e.x)
            .max()
            .unwrap_or(i32::MIN);
        let max_x = ends(Direction::Right)
            .iter()
            .map(|e| e.x)
            .min()
            .unwrap_or(i32::MAX);
        let min_y = ends(Direction::Up)
            .iter()
            .map(|e| e.y)
            .max()
            .unwrap_or(i32::MIN);
        let max_y = ends(Direction::Down)
            .iter()
            .map(|e| e.y)
            .min()
            .unwrap_or(i32::MAX);
        let effective = Pos::new(p.x.max(min_x).min(max_x), p.y.max(min_y).min(max_y));
        let origin = trace.positions[0];
        let move_direction = if !trace.horizontal {
            if effective.x < origin.x {
                Direction::Left
            } else {
                Direction::Right
            }
        } else if effective.y < origin.y {
            Direction::Up
        } else {
            Direction::Down
        };
        let units = if move_direction.is_horizontal() {
            effective.x - origin.x
        } else {
            effective.y - origin.y
        }
        .abs();

        for a in &trace.attachments {
            if a.direction == move_direction {
                for i in 0..units {
                    layer.set(
                        a.source.add(a.direction.scale(i)),
                        crate::core::layer::ERASE,
                    );
                }
            }
        }
        for pos in &trace.positions {
            layer.set(*pos, crate::core::layer::ERASE);
        }
        for pos in &trace.positions {
            layer.set(
                pos.add(move_direction.scale(units)),
                committed.get(*pos).unwrap_or(crate::core::layer::ERASE),
            );
        }
        for a in &trace.attachments {
            if a.direction == move_direction.opposite() {
                for i in 1..=units {
                    layer.set(
                        a.source.add(a.direction.scale(-i)),
                        if a.direction.is_horizontal() {
                            UNICODE.line_horizontal
                        } else {
                            UNICODE.line_vertical
                        },
                    );
                }
            }
        }
        canvas.set_scratch(layer);
    }

    fn end(&mut self, canvas: &mut Canvas) {
        self.trace = None;
        canvas.commit_scratch();
    }

    fn cleanup(&mut self, _canvas: &mut Canvas) {
        self.trace = None;
    }

    fn handle_key(&mut self, _canvas: &mut Canvas, _key: Key, _m: Mods) -> bool {
        false
    }

    fn hover_is_target(&self, canvas: &Canvas, p: Pos, _m: Mods) -> bool {
        canvas.committed.get(p).is_some_and(|v| {
            v == UNICODE.line_horizontal || v == UNICODE.line_vertical || is_special(v)
        })
    }
}

fn trace_line(layer: &Layer, position: Pos) -> LineTrace {
    let horizontal = layer.get(position) == Some(UNICODE.line_horizontal);
    let directions = if horizontal {
        [Direction::Left, Direction::Right]
    } else {
        [Direction::Up, Direction::Down]
    };
    let attachment_directions = if horizontal {
        [Direction::Up, Direction::Down]
    } else {
        [Direction::Left, Direction::Right]
    };

    let mut positions = vec![position];
    let mut attachments = Vec::new();
    for d in directions {
        let mut current = position;
        loop {
            let next = current.add(d.delta());
            if !layer.get(current).is_some_and(|v| connects(v, d))
                || !layer.get(next).is_some_and(|v| connects(v, d.opposite()))
            {
                break;
            }
            current = next;
            positions.push(current);
        }
    }

    // `positions` may grow while iterating (arrow heads), matching ASCIIFlow.
    let mut i = 0;
    while i < positions.len() {
        let current = positions[i];
        for ad in attachment_directions {
            if layer.get(current).is_some_and(|v| connects(v, ad))
                && layer
                    .get(current.add(ad.delta()))
                    .is_some_and(|v| connects(v, ad.opposite()))
            {
                attachments.push(trace_attachment(layer, current.add(ad.delta()), ad));
            }
            let head = current.add(ad.delta());
            let beyond = current.add(ad.scale(2));
            if layer.get(head).is_some_and(is_arrow)
                && layer
                    .get(beyond)
                    .is_some_and(|v| connects(v, ad.opposite()))
            {
                positions.push(head);
                attachments.push(trace_attachment(layer, beyond, ad));
            }
        }
        i += 1;
    }

    LineTrace {
        horizontal,
        positions,
        attachments,
    }
}

fn trace_attachment(layer: &Layer, position: Pos, direction: Direction) -> AttachmentTrace {
    let trace_value = if direction.is_horizontal() {
        UNICODE.line_horizontal
    } else {
        UNICODE.line_vertical
    };
    let mut p = position;
    while layer.get(p.add(direction.delta())) == Some(trace_value) {
        p = p.add(direction.delta());
    }
    AttachmentTrace {
        source: position,
        end: p,
        direction,
    }
}

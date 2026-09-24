//! `ASCIIFlow`'s own spec files, ported verbatim into Rust.
//!
//! Ported from `ASCIIFlow` (`client/draw/*.spec.ts`, `client/snap.spec.ts`),
//! MIT © Lewis Hemens. Only the harness is rewritten; the assertions are upstream's.
//! The snap spec lives in `src/core/snap.rs` next to the code it covers.

use rsdia::core::editor::{Editor, ToolId};
use rsdia::core::entity::{
    cells_in_box, detect_line_tip, detect_word, find_box, move_cells, trace_line_from_tip,
};
use rsdia::core::layer::Layer;
use rsdia::core::text::{layer_to_text, text_to_layer};
use rsdia::core::tools::tool::Mods;
use rsdia::core::vector::Pos;

const fn v(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn from_text(lines: &[&str]) -> Layer {
    text_to_layer(&lines.join("\n"), Pos::default())
}

fn apply(committed: &Layer, scratch: &Layer) -> Layer {
    committed.apply(scratch).0
}

fn select_editor(committed: Layer) -> Editor {
    let mut editor = Editor::new();
    editor.set_tool(ToolId::Select);
    editor.canvas.committed = committed;
    editor
}

fn drag(editor: &mut Editor, from: Pos, to: Pos) {
    editor.down(from, Mods::NONE);
    editor.move_to(to, Mods::NONE);
    editor.up();
}

// ------------------------------------------------------------------ box.spec

#[test]
fn box_draws_a_clean_box_when_nothing_is_adjacent() {
    let mut editor = Editor::new();
    editor.set_tool(ToolId::Box);
    drag(&mut editor, v(0, 0), v(2, 2));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 0)), Some('┌'));
    assert_eq!(c.get(v(2, 0)), Some('┐'));
    assert_eq!(c.get(v(0, 2)), Some('└'));
    assert_eq!(c.get(v(2, 2)), Some('┘'));
}

#[test]
fn box_connects_to_a_line_that_terminates_on_an_edge() {
    let mut editor = Editor::new();
    editor.set_tool(ToolId::Box);
    editor.canvas.committed = from_text(&["", "──"]);
    drag(&mut editor, v(2, 0), v(4, 2));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(2, 1)), Some('┤'));
    assert_eq!(c.get(v(1, 1)), Some('─'));
}

// ------------------------------------------------------------------ entity.spec

#[test]
fn detect_word_selects_a_contiguous_run_of_text_stopping_at_spaces() {
    let layer = from_text(&["hi there"]);
    let word = detect_word(&layer, v(4, 0)).expect("a word");
    let found: Vec<String> = word.iter().map(std::string::ToString::to_string).collect();
    assert_eq!(found, vec!["3:0", "4:0", "5:0", "6:0", "7:0"]);
}

#[test]
fn detect_word_returns_none_on_empty_and_box_drawing_cells() {
    let layer = from_text(&["┌─┐"]);
    assert_eq!(detect_word(&layer, v(0, 0)), None);
    assert_eq!(detect_word(&layer, v(50, 50)), None);
}

#[test]
fn a_word_moves() {
    let layer = from_text(&["hi there"]);
    let word = detect_word(&layer, v(0, 0)).expect("a word");
    let moved = apply(&layer, &move_cells(&layer, &word, v(0, 3)));
    assert_eq!(moved.get(v(0, 0)), None);
    assert_eq!(moved.get(v(0, 3)), Some('h'));
    assert_eq!(moved.get(v(1, 3)), Some('i'));
    assert_eq!(moved.get(v(3, 0)), Some('t'));
}

#[test]
fn find_box_finds_it_from_the_interior_and_from_its_border() {
    let layer = from_text(&["┌───┐", "│ x │", "└───┘"]);
    for p in [v(3, 1), v(0, 0), v(2, 0), v(4, 2)] {
        let b = find_box(&layer, p).unwrap_or_else(|| panic!("a box from {p}"));
        assert_eq!((b.left(), b.top(), b.right(), b.bottom()), (0, 0, 4, 2));
    }
}

#[test]
fn find_box_returns_none_without_an_enclosing_rectangle() {
    assert_eq!(find_box(&from_text(&["hello"]), v(2, 0)), None);
    assert_eq!(
        find_box(&from_text(&["┌───┐", "│ x │", "└───┘"]), v(40, 40)),
        None
    );
}

#[test]
fn the_whole_box_and_its_contents_move() {
    let layer = from_text(&["┌───┐", "│ x │", "└───┘"]);
    let b = find_box(&layer, v(2, 0)).expect("a box");
    let moved = apply(
        &layer,
        &move_cells(&layer, &cells_in_box(&layer, b), v(10, 5)),
    );
    assert_eq!(
        layer_to_text(&moved, None, false),
        ["┌───┐", "│ x │", "└───┘"].join("\n")
    );
    assert_eq!(moved.get(v(10, 5)), Some('┌'));
    assert_eq!(moved.get(v(12, 6)), Some('x'));
    assert_eq!(moved.get(v(0, 0)), None);
}

#[test]
fn line_tips_are_detected_and_traced_back_to_the_first_bend() {
    let layer = from_text(&["───►"]);
    let tip = detect_line_tip(&layer, v(3, 0)).expect("a tip");
    assert!(tip.horizontal);
    assert_eq!(tip.arrow, Some('►'));
    assert_eq!(detect_line_tip(&layer, v(1, 0)), None);
    let trace = trace_line_from_tip(&layer, tip.tip, tip.body_dir);
    assert_eq!(trace.anchor.to_string(), "0:0");
    assert_eq!(trace.cells.len(), 4);

    // ► tip at (3,0) → left along the top → first corner ┌ at (0,0); the
    // vertical leg below it is left alone.
    let l = from_text(&["┌──►", "│"]);
    let tip = detect_line_tip(&l, v(3, 0)).expect("a tip");
    let trace = trace_line_from_tip(&l, tip.tip, tip.body_dir);
    assert_eq!(trace.anchor.to_string(), "0:0");
    assert_eq!(trace.cells.len(), 4);
}

// ------------------------------------------------------------------ grid.spec

#[test]
fn a_moved_quadrant_reflows_the_shared_walls() {
    let mut editor = select_editor(from_text(&[
        "┌──┬──┐",
        "│  │  │",
        "├──┼──┤",
        "│  │  │",
        "└──┴──┘",
    ]));
    drag(&mut editor, v(1, 3), v(1, 6));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 5)), Some('├'));
    assert_eq!(c.get(v(3, 5)), Some('┼'));
    assert_eq!(c.get(v(6, 5)), Some('┤'));
    assert_eq!(c.get(v(3, 0)), Some('┬'));
    assert_eq!(c.get(v(6, 2)), Some('│'));
}

// ------------------------------------------------------------------ line.spec

#[test]
fn a_vertical_line_ending_on_a_horizontal_line_forms_a_tee() {
    let mut editor = Editor::new();
    editor.set_tool(ToolId::Line);
    editor.canvas.committed = from_text(&["───"]);
    drag(&mut editor, v(1, -2), v(1, 0));
    assert_eq!(editor.canvas.committed.get(v(1, 0)), Some('┴'));
}

#[test]
fn a_horizontal_line_ending_on_a_vertical_line_forms_a_tee() {
    let mut editor = Editor::new();
    editor.set_tool(ToolId::Line);
    editor.canvas.committed = from_text(&["│", "│", "│"]);
    drag(&mut editor, v(-2, 1), v(0, 1));
    assert_eq!(editor.canvas.committed.get(v(0, 1)), Some('┤'));
}

// ------------------------------------------------------------------ select.spec

#[test]
fn select_moves_a_whole_box_and_contents_by_dragging_its_interior() {
    let mut editor = select_editor(from_text(&["┌───┐", "│ x │", "└───┘"]));
    drag(&mut editor, v(3, 1), v(13, 6));
    assert_eq!(
        layer_to_text(&editor.canvas.committed, None, false),
        ["┌───┐", "│ x │", "└───┘"].join("\n")
    );
    assert_eq!(editor.canvas.committed.get(v(10, 5)), Some('┌'));
    assert_eq!(editor.canvas.committed.get(v(12, 6)), Some('x'));
    assert_eq!(editor.canvas.committed.get(v(0, 0)), None);
}

#[test]
fn select_resizes_a_box_edge_instead_of_moving_it() {
    let mut editor = select_editor(from_text(&["┌───┐", "│   │", "└───┘"]));
    drag(&mut editor, v(2, 0), v(2, 1));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 0)), None);
    assert_eq!(c.get(v(0, 1)), Some('┌'));
    assert_eq!(c.get(v(0, 2)), Some('└'));
}

#[test]
fn select_moves_a_word_and_leaves_the_rest_of_the_line() {
    let mut editor = select_editor(from_text(&["hi there"]));
    drag(&mut editor, v(0, 0), v(0, 2));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 2)), Some('h'));
    assert_eq!(c.get(v(1, 2)), Some('i'));
    assert_eq!(c.get(v(3, 0)), Some('t'));
    assert_eq!(c.get(v(0, 0)), None);
}

#[test]
fn select_extends_a_line_tip_outward() {
    let mut editor = select_editor(from_text(&["───►"]));
    drag(&mut editor, v(3, 0), v(6, 0));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(6, 0)), Some('►'));
    assert_eq!(c.get(v(5, 0)), Some('─'));
}

#[test]
fn an_attached_line_shortens_on_a_parallel_move() {
    let mut editor = select_editor(from_text(&["┌─┐", "│ ├──►", "└─┘"]));
    drag(&mut editor, v(1, 1), v(2, 1));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(1, 0)), Some('┌'));
    assert_eq!(c.get(v(3, 1)), Some('├'));
    assert_eq!(c.get(v(4, 1)), Some('─'));
    assert_eq!(c.get(v(5, 1)), Some('►'));
}

#[test]
fn an_attached_line_reflows_with_a_bend_on_a_perpendicular_move() {
    let mut editor = select_editor(from_text(&["┌─┐", "│ ├──►", "└─┘"]));
    drag(&mut editor, v(1, 1), v(1, 2));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 1)), Some('┌'));
    assert_eq!(c.get(v(3, 2)), Some('─'));
    assert_eq!(c.get(v(5, 2)), Some('┘'));
    assert_eq!(c.get(v(5, 1)), Some('▲'));
}

#[test]
fn a_tip_dragged_out_of_the_lines_plane_draws_a_corner() {
    let mut editor = select_editor(from_text(&["───►"]));
    drag(&mut editor, v(3, 0), v(3, 3));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 0)), Some('─'));
    assert_eq!(c.get(v(3, 0)), Some('┐'));
    assert_eq!(c.get(v(3, 1)), Some('│'));
    assert_eq!(c.get(v(3, 3)), Some('▼'));
}

#[test]
fn only_the_last_segment_turns_at_the_first_bend() {
    let mut editor = select_editor(from_text(&["│", "│", "└──►"]));
    drag(&mut editor, v(3, 2), v(3, 4));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 0)), Some('│'));
    assert_eq!(c.get(v(0, 1)), Some('│'));
    assert_eq!(c.get(v(0, 2)), Some('└'));
    assert_eq!(c.get(v(3, 2)), Some('┐'));
    assert_eq!(c.get(v(3, 4)), Some('▼'));
}

#[test]
fn a_connector_is_traced_through_its_corner_and_the_whole_path_reflows() {
    let mut editor = select_editor(from_text(&["┌─┐", "│ ├─┐", "└─┘ │", "    ▼"]));
    drag(&mut editor, v(1, 1), v(1, 2));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 1)), Some('┌'));
    assert_eq!(c.get(v(4, 1)), None);
    assert_eq!(c.get(v(3, 2)), Some('─'));
    assert_eq!(c.get(v(4, 2)), Some('┐'));
    assert_eq!(c.get(v(4, 3)), Some('▼'));
}

#[test]
fn the_selection_moves_through_undo_and_redo_with_its_content() {
    let mut editor = select_editor(from_text(&["┌─┐", "│ │", "└─┘"]));
    drag(&mut editor, v(1, 1), v(1, 4));
    assert_eq!(editor.canvas.selection.map(|b| b.top()), Some(3));
    editor.canvas.undo();
    assert_eq!(editor.canvas.selection.map(|b| b.top()), Some(0));
    editor.canvas.redo();
    assert_eq!(editor.canvas.selection.map(|b| b.top()), Some(3));
}

#[test]
fn switching_off_the_select_tool_purges_the_selection() {
    let mut editor = select_editor(from_text(&["┌─┐", "│ │", "└─┘"]));
    editor.down(v(1, 1), Mods::NONE);
    editor.up();
    assert!(editor.canvas.selection.is_some());
    editor.select_cleanup();
    assert_eq!(editor.canvas.selection, None);
}

#[test]
fn a_line_crossing_the_selection_edge_reflows_on_a_rubber_band_move() {
    let mut editor = select_editor(from_text(&["───►"]));
    // Rubber-band the left cells: start on empty, drag across them.
    drag(&mut editor, v(1, 1), v(0, 0));
    // Drag the selection down by one.
    drag(&mut editor, v(0, 0), v(0, 1));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 1)), Some('─'));
    assert_eq!(c.get(v(0, 0)), None);
    assert_eq!(c.get(v(3, 1)), Some('┘'));
    assert_eq!(c.get(v(3, 0)), Some('▲'));
}

#[test]
fn an_arrow_pointing_into_a_box_stays_attached_when_the_box_moves() {
    let mut editor = select_editor(from_text(&["   ┌─┐", " ─►│ │", "   └─┘"]));
    drag(&mut editor, v(4, 1), v(4, 3));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(3, 2)), Some('┌'));
    assert_eq!(c.get(v(2, 3)), Some('►'));
    assert_eq!(c.get(v(2, 1)), None);
}

#[test]
fn a_line_attached_at_a_box_corner_stays_connected_when_the_box_moves() {
    let mut editor = select_editor(from_text(&["┌─┬──►", "│ │", "└─┘"]));
    drag(&mut editor, v(1, 1), v(1, 4));
    let c = &editor.canvas.committed;
    assert_eq!(c.get(v(0, 3)), Some('┌'));
    assert_eq!(c.get(v(2, 3)), Some('┬'));
    assert_eq!(c.get(v(3, 3)), Some('─'));
    assert_eq!(c.get(v(3, 0)), None);
}

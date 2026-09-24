//! Tool behaviour that only shows up end to end, driven through the scenario runner.

mod common;

use common::{scenario_editor, scenarios_file, Step};
use rsdia::core::editor::ToolId;
use rsdia::core::text::{layer_to_text, text_to_layer};
use rsdia::core::vector::Pos;

fn v(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

#[test]
fn the_eraser_detaches_lines_it_touches() {
    let editor = scenario_editor(&[
        Step::Tool(ToolId::Box),
        Step::Drag {
            points: vec![v(0, 0), v(4, 2)],
            flip: false,
        },
        Step::Tool(ToolId::Eraser),
        Step::Drag {
            points: vec![v(0, 1), v(1, 1)],
            flip: false,
        },
    ]);
    let committed = &editor.canvas.committed;
    assert_eq!(committed.get(v(0, 1)), None);
    assert_eq!(committed.get(v(0, 0)), Some('┌'));
}

#[test]
fn a_zero_size_eraser_drag_erases_one_cell_as_an_undoable_step() {
    let mut editor = scenario_editor(&[
        Step::Tool(ToolId::Box),
        Step::Drag {
            points: vec![v(0, 0), v(3, 0)],
            flip: false,
        },
        Step::Tool(ToolId::Eraser),
        Step::Drag {
            points: vec![v(1, 0)],
            flip: false,
        },
    ]);
    assert_eq!(editor.canvas.committed.get(v(1, 0)), None);
    assert!(editor.canvas.can_undo());
    editor.undo();
    assert!(layer_to_text(&editor.canvas.committed, None, false).contains('─'));
}

#[test]
fn the_scenario_file_is_still_the_shape_we_expect() {
    assert!(scenarios_file().exists());
    // A smoke test for the shared runner: the text tool types where it is told.
    let editor = scenario_editor(&[
        Step::Tool(ToolId::Text),
        Step::Text {
            at: v(2, 3),
            text: "hi".to_string(),
        },
    ]);
    let expected = text_to_layer("hi", v(2, 3));
    assert_eq!(
        layer_to_text(&editor.canvas.committed, None, false),
        layer_to_text(&expected, None, false)
    );
}

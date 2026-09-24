//! The scenario suite: golden fixtures captured from `ASCIIFlow` itself, replayed
//! through the Rust core, plus the end-to-end tool behaviour the fixtures do not
//! cover.
//!
//! The fixture files are unchanged, so replaying them is the proof that the port
//! draws exactly what the implementation they came from drew.

mod common;

use common::loading::load_scenarios;
use common::paths::fixtures_dir;
use common::scenarios::{scenario_editor, Step};
use rsdia::core::editor::ToolId;
use rsdia::core::export::{export_text, Charset, ExportConfig, Wrapper};
use rsdia::core::text::{layer_to_text, text_to_layer};
use rsdia::core::vector::Pos;

const fn config(characters: Charset) -> ExportConfig {
    ExportConfig {
        characters,
        wrapper: Wrapper::None,
        fenced: false,
    }
}

#[test]
fn asciiflow_fixtures_match_byte_for_byte() {
    let dir = fixtures_dir();
    let scenarios = load_scenarios();
    assert_eq!(scenarios.len(), 21, "every scenario is loaded");
    let mut failures = Vec::new();
    for scenario in &scenarios {
        let editor = scenario_editor(&scenario.steps);
        let committed = &editor.canvas.committed;
        for (charset, suffix) in [(Charset::Extended, ""), (Charset::Basic, ".basic")] {
            let got = format!("{}\n", export_text(committed, &config(charset)));
            let path = dir.join(format!("{}{suffix}.txt", scenario.id));
            let want = std::fs::read_to_string(&path).unwrap_or_default();
            if got != want {
                failures.push(format!(
                    "{} {} ({charset:?}):\n--- got ---\n{got}--- want ---\n{want}",
                    scenario.id, scenario.title
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixture mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ------------------------------------------------------------------ tool behaviour

const fn v(x: i32, y: i32) -> Pos {
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
fn the_runner_types_where_a_scenario_tells_it_to() {
    // `tests/fixtures.rs` replays the whole recorded file; this is the same runner
    // on a hand-written step list, so a break in either shows up here too.
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

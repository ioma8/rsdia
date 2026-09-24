#![allow(dead_code)]
//! Shared fixtures and the scenario runner, mirroring `test/scenario.ts` and the
//! golden fixtures captured from ASCIIFlow.

use std::path::PathBuf;

use rsdia::core::editor::{Editor, ToolId};
use rsdia::core::tools::tool::{Key, Mods};
use rsdia::core::vector::Pos;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/asciiflow")
}

pub fn scenarios_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/scenarios/asciiflow.json")
}

#[derive(Clone, Debug)]
pub enum Step {
    Tool(ToolId),
    Drag { points: Vec<Pos>, flip: bool },
    Text { at: Pos, text: String },
    Key(Key),
}

pub struct Scenario {
    pub id: String,
    pub title: String,
    pub steps: Vec<Step>,
}

fn tool_id(name: &str) -> ToolId {
    match name {
        "box" => ToolId::Box,
        "select" => ToolId::Select,
        "arrow" => ToolId::Arrow,
        "line" => ToolId::Line,
        "text" => ToolId::Text,
        "eraser" => ToolId::Eraser,
        other => panic!("unknown tool {other}"),
    }
}

pub fn key_from_name(name: &str) -> Key {
    match name {
        "<enter>" => Key::Enter,
        "<backspace>" => Key::Backspace,
        "<delete>" => Key::Delete,
        "<up>" => Key::Up,
        "<down>" => Key::Down,
        "<left>" => Key::Left,
        "<right>" => Key::Right,
        other => Key::Char(other.chars().next().expect("a single character")),
    }
}

fn point(value: &serde_json::Value) -> Pos {
    let pair = value.as_array().expect("a point is a pair");
    Pos::new(
        pair.first().and_then(|v| v.as_i64()).expect("x") as i32,
        pair.get(1).and_then(|v| v.as_i64()).expect("y") as i32,
    )
}

pub fn load_scenarios() -> Vec<Scenario> {
    let raw = std::fs::read_to_string(scenarios_file()).expect("the scenario file is present");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("scenarios parse");
    value
        .as_array()
        .expect("an array of scenarios")
        .iter()
        .map(|scenario| {
            let steps = scenario["steps"]
                .as_array()
                .expect("steps")
                .iter()
                .map(|step| {
                    if let Some(tool) = step.get("tool").and_then(|v| v.as_str()) {
                        return Step::Tool(tool_id(tool));
                    }
                    if let Some(drag) = step.get("drag").and_then(|v| v.as_array()) {
                        return Step::Drag {
                            points: drag.iter().map(point).collect(),
                            flip: step.get("flip").and_then(|v| v.as_bool()).unwrap_or(false),
                        };
                    }
                    if let Some(at) = step.get("text") {
                        return Step::Text {
                            at: point(at),
                            text: step
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        };
                    }
                    let key = step
                        .get("key")
                        .and_then(|v| v.as_str())
                        .expect("a key step");
                    Step::Key(key_from_name(key))
                })
                .collect();
            Scenario {
                id: scenario["id"].as_str().expect("id").to_string(),
                title: scenario["title"].as_str().expect("title").to_string(),
                steps,
            }
        })
        .collect()
}

pub fn run_scenario(steps: &[Step], editor: &mut Editor) {
    for step in steps {
        match step {
            Step::Tool(tool) => editor.set_tool(*tool),
            Step::Drag { points, flip } => {
                let m = Mods::NONE.flipped(*flip);
                let mut iter = points.iter();
                if let Some(first) = iter.next() {
                    editor.down(*first, m);
                    for p in iter {
                        editor.move_to(*p, m);
                    }
                    editor.up();
                }
            }
            Step::Text { at, text } => {
                editor.down(*at, Mods::NONE);
                editor.up();
                for ch in text.chars() {
                    editor.key(Key::Char(ch), Mods::NONE);
                }
                editor.flush();
            }
            Step::Key(key) => {
                editor.key(*key, Mods::NONE);
            }
        }
    }
}

pub fn scenario_editor(steps: &[Step]) -> Editor {
    let mut editor = Editor::new();
    run_scenario(steps, &mut editor);
    editor
}

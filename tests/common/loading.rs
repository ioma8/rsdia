//! Loads the captured `ASCIIFlow` scenario files: the step format the fixtures were
//! recorded in, and the reader that turns one into `Step`s.

use std::path::PathBuf;

use rsdia::core::editor::ToolId;
use rsdia::core::tools::tool::Key;
use rsdia::core::vector::Pos;

use super::scenarios::Step;

/// The scenario file the loader reads.
fn scenarios_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/scenarios/asciiflow.json")
}

/// One recorded scenario: an id naming its fixture file, a title, and the steps.
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
        i32::try_from(pair.first().and_then(serde_json::Value::as_i64).expect("x"))
            .expect("x fits i32"),
        i32::try_from(pair.get(1).and_then(serde_json::Value::as_i64).expect("y"))
            .expect("y fits i32"),
    )
}

/// Every scenario in the file, in order.
///
/// # Panics
///
/// If the file is missing or is not the shape the fixtures were captured in.
#[must_use]
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
                            flip: step
                                .get("flip")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false),
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

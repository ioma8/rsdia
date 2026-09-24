//! Golden fixtures captured from ASCIIFlow itself, replayed through the Rust core.
//!
//! The fixture files are unchanged, so this suite is the proof that the port draws
//! exactly what the implementation they came from drew.

mod common;

use common::{fixtures_dir, load_scenarios, scenario_editor};
use rsdia::core::export::{export_text, Charset, ExportConfig, Wrapper};

fn config(characters: Charset) -> ExportConfig {
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

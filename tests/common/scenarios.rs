//! The scenario runner: replays a recorded step list through the Rust core.

use rsdia::core::editor::Editor;
use rsdia::core::editor::ToolId;
use rsdia::core::tools::tool::{Key, Mods};
use rsdia::core::vector::Pos;

#[derive(Clone, Debug)]
pub enum Step {
    Tool(ToolId),
    Drag { points: Vec<Pos>, flip: bool },
    Text { at: Pos, text: String },
    Key(Key),
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

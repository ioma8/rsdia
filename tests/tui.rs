//! End to end: the real `App` driven headlessly.
//!
//! The app draws into a plain `ratatui::Buffer` and events are synchronous, so the
//! cases the original suite ran on a test renderer are ordinary assertions here.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use rsdia::core::layer::Layer;
use rsdia::core::vector::Pos;
use rsdia::storage::config::DEFAULT_CONFIG;
use rsdia::storage::drawings::DrawingStore;
use rsdia::tui::app::{App, AppOptions, Clipboard, OpenDrawing};
use rsdia::tui::theme::{palette, TerminalColors, ThemeName};
use rsdia::tui::toolbar::BAR_Y;

struct Harness {
    app: App,
    buf: Buffer,
    dir: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.app.shutdown();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Harness {
    fn new(width: u16, height: u16, layer: Layer, term: Option<TerminalColors>) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "rsdia-tui-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a writable temp dir");
        let store = DrawingStore::new(dir.join("drawings"));
        let path = store.path_for("test");
        store.save(&path, "test", &layer).expect("saved");
        let drawing = OpenDrawing {
            path,
            name: "test".to_string(),
            layer,
        };
        let app = App::new(AppOptions {
            store,
            config: DEFAULT_CONFIG,
            config_path: dir.join("config.json"),
            drawing,
            clipboard: Clipboard::memory(),
            term,
            autosave_ms: Some(20),
        });
        let mut harness = Self {
            app,
            buf: Buffer::empty(Rect::new(0, 0, width, height)),
            dir,
        };
        harness.render();
        harness
    }

    fn render(&mut self) {
        self.app.tick();
        self.app.paint(&mut self.buf);
    }

    fn frame(&mut self) -> Vec<String> {
        self.render();
        let area = self.buf.area();
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| {
                        self.buf
                            .cell((x, y))
                            .map(|c| c.symbol().chars().next().unwrap_or(' '))
                            .unwrap_or(' ')
                    })
                    .collect()
            })
            .collect()
    }

    fn text_frame(&mut self) -> String {
        self.frame().join("\n")
    }

    /// Screen position of a toolbar label.
    fn label(&mut self, text: &str) -> (i32, i32) {
        let lines = self.frame();
        let y = (BAR_Y + 1) as usize;
        (char_find(&lines[y], text).unwrap_or(0) as i32, y as i32)
    }

    fn mouse(&mut self, kind: MouseEventKind, x: i32, y: i32, modifiers: KeyModifiers) {
        self.app.on_mouse(&MouseEvent {
            kind,
            column: x as u16,
            row: y as u16,
            modifiers,
        });
    }

    fn click(&mut self, x: i32, y: i32) {
        self.render();
        self.mouse(
            MouseEventKind::Down(MouseButton::Left),
            x,
            y,
            KeyModifiers::empty(),
        );
        self.mouse(
            MouseEventKind::Up(MouseButton::Left),
            x,
            y,
            KeyModifiers::empty(),
        );
    }

    fn drag(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, button: MouseButton) {
        self.render();
        self.mouse(MouseEventKind::Down(button), x0, y0, KeyModifiers::empty());
        self.mouse(MouseEventKind::Drag(button), x1, y1, KeyModifiers::empty());
        self.mouse(MouseEventKind::Up(button), x1, y1, KeyModifiers::empty());
    }

    fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.app.on_key(&KeyEvent::new(code, modifiers));
    }

    fn press(&mut self, c: char) {
        self.key(KeyCode::Char(c), KeyModifiers::empty());
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.press(c);
        }
    }

    fn escape(&mut self) {
        self.key(KeyCode::Esc, KeyModifiers::empty());
    }

    /// Canvas cell under a screen position.
    fn cell_at(&self, x: i32, y: i32) -> Pos {
        self.app.viewport.to_canvas(x, y)
    }

    fn committed_len(&self) -> usize {
        self.app.editor.canvas.committed.len()
    }
}

/// Character index of `needle`, so byte offsets never leak into screen maths.
fn char_find(hay: &str, needle: &str) -> Option<usize> {
    hay.find(needle).map(|byte| hay[..byte].chars().count())
}

fn cell(line: &str, from: usize, to: usize) -> String {
    line.chars().skip(from).take(to - from).collect()
}

// ------------------------------------------------------------------ toolbar

#[test]
fn the_full_layout_is_79_cells_wide_in_asciiflows_order() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let lines = h.frame();
    assert!(lines[2].contains(
        "│  ≡  │  box select arrow line text eraser  │  export  │  ⟲ ⟳  │  ⚙  │  help  │"
    ));
    let x0 = lines[1].chars().position(|c| c == '┌').expect("a corner");
    let x1 = lines[1].chars().position(|c| c == '┐').expect("a corner");
    assert_eq!(x1 - x0 + 1, 79);
    assert_eq!(x0, ((120 - 79) / 2) as usize);
}

#[test]
fn compact_and_narrow_layouts() {
    let mut h = Harness::new(65, 40, Layer::new(), None);
    assert!(h.frame()[2].contains("│ ≡ │ box sel arrow line text erase │ exp │ ⟲ ⟳ │ ⚙ │ ? │"));
    let mut h = Harness::new(38, 40, Layer::new(), None);
    assert!(h.frame()[2].contains("│ ≡ │ box sel arw lin txt ers │"));
}

#[test]
fn clicking_a_tool_switches_to_it_and_digits_and_alt_digits_do_too() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let (x, y) = h.label("arrow");
    h.click(x, y);
    assert_eq!(h.app.tool(), rsdia::core::editor::ToolId::Arrow);
    h.press('2');
    assert_eq!(h.app.tool(), rsdia::core::editor::ToolId::Select);
    h.key(KeyCode::Char('5'), KeyModifiers::ALT);
    assert_eq!(h.app.tool(), rsdia::core::editor::ToolId::Text);
}

#[test]
fn letter_shortcuts_switch_tools() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    for (key, tool) in [
        ('v', rsdia::core::editor::ToolId::Select),
        ('a', rsdia::core::editor::ToolId::Arrow),
        ('l', rsdia::core::editor::ToolId::Line),
        ('t', rsdia::core::editor::ToolId::Text),
        ('e', rsdia::core::editor::ToolId::Eraser),
        ('r', rsdia::core::editor::ToolId::Box),
    ] {
        h.press(key);
        assert_eq!(h.app.tool(), tool, "key {key}");
    }
}

#[test]
fn there_is_no_title_above_the_bar_and_the_status_bar_names_the_product() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let lines = h.frame();
    assert!(!lines[(BAR_Y - 1) as usize].contains("rsdia"));
    assert!(lines[lines.len() - 1].contains("rsdia"));
}

#[test]
fn popovers_open_under_their_labels_and_close_on_escape_or_an_outside_click() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let (x, y) = h.label("⚙");
    h.click(x, y);
    let frame = h.frame();
    assert!(frame.iter().any(|l| l.contains("lattice")));
    assert!(frame[5].contains("grid:"));
    let rows: Vec<String> = frame[6..9]
        .iter()
        .map(|l| {
            let chars: Vec<char> = l.chars().collect();
            let first = chars.iter().position(|c| *c == '│').unwrap_or(0);
            let last = chars
                .iter()
                .rposition(|c| *c == '│')
                .unwrap_or(chars.len() - 1);
            chars[first + 1..last].iter().collect()
        })
        .collect();
    assert!(rows[0].contains("theme: terminal  dracula  nord  tokyo-night"));
    assert!(rows.join(" ").contains("github-light"));
    for row in &rows {
        assert!(row.chars().count() <= 56, "{row}");
    }
    h.escape();
    assert!(!h.text_frame().contains("lattice"));
    h.click(x, y);
    h.click(5, 30);
    let frame = h.text_frame();
    assert!(!frame.contains("lattice"));
    assert_eq!(h.committed_len(), 0);
}

// ------------------------------------------------------------------ theme

#[test]
fn the_first_frame_already_wears_the_terminals_colors() {
    let term = TerminalColors {
        fg: "#00ff00".to_string(),
        bg: "#110022".to_string(),
        ansi: vec![Some("#00ff00".to_string()); 16],
    };
    let mut h = Harness::new(80, 24, Layer::new(), Some(term.clone()));
    h.render();
    let expected = palette(ThemeName::Terminal, Some(&term));
    let cell = h.buf.cell((10, 10)).expect("a cell").style();
    assert_eq!(cell.bg, Some(expected.bg));
    assert_ne!(cell.bg, Some(palette(ThemeName::Terminal, None).bg));
}

#[test]
fn a_terminal_that_has_not_answered_yet_gets_the_stand_in_scheme() {
    let mut h = Harness::new(80, 24, Layer::new(), None);
    h.render();
    let expected = palette(ThemeName::Terminal, None);
    assert_eq!(
        h.buf.cell((10, 10)).expect("a cell").style().bg,
        Some(expected.bg)
    );
}

// ------------------------------------------------------------------ drawing

#[test]
fn a_box_drag_draws_and_undo_and_redo_buttons_work() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 19, 14, MouseButton::Left);
    let lines = h.frame();
    assert_eq!(cell(&lines[10], 10, 20), "┌────────┐");
    assert_eq!(cell(&lines[14], 10, 20), "└────────┘");
    assert_eq!(h.committed_len(), 26);

    let (x, y) = h.label("⟲");
    h.click(x, y);
    assert_eq!(h.committed_len(), 0);
    let (x, y) = h.label("⟳");
    h.click(x, y);
    assert_eq!(h.committed_len(), 26);
}

#[test]
fn drags_starting_on_the_toolbar_are_ignored() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(25, 2, 30, 20, MouseButton::Left);
    assert_eq!(h.committed_len(), 0);
}

#[test]
fn an_arrow_leaves_a_box_side_and_flips_mid_drag_with_f() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 16, 14, MouseButton::Left);
    h.press('3');
    h.render();
    h.mouse(
        MouseEventKind::Down(MouseButton::Left),
        16,
        12,
        KeyModifiers::empty(),
    );
    h.mouse(
        MouseEventKind::Drag(MouseButton::Left),
        20,
        16,
        KeyModifiers::empty(),
    );
    h.mouse(
        MouseEventKind::Drag(MouseButton::Left),
        24,
        18,
        KeyModifiers::empty(),
    );
    let before = h.app.editor.canvas.scratch.get(h.cell_at(24, 12));
    h.press('f');
    let after = h.app.editor.canvas.scratch.get(h.cell_at(16, 18));
    h.mouse(
        MouseEventKind::Up(MouseButton::Left),
        24,
        18,
        KeyModifiers::empty(),
    );
    assert_eq!(before, Some('┐'));
    assert_eq!(after, Some('└'));
    let lines = h.frame();
    assert_eq!(cell(&lines[18], 16, 25), "└───────►");
}

#[test]
fn the_wheel_pans_the_viewport() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let origin = h.app.viewport.origin;
    h.mouse(MouseEventKind::ScrollDown, 50, 20, KeyModifiers::empty());
    assert_eq!(h.app.viewport.origin.y, origin.y + 1);
    h.mouse(MouseEventKind::ScrollRight, 50, 20, KeyModifiers::empty());
    assert_eq!(h.app.viewport.origin.x, origin.x + 2);
}

#[test]
fn a_middle_drag_pans() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let origin = h.app.viewport.origin;
    h.drag(50, 20, 45, 17, MouseButton::Middle);
    assert_eq!(h.app.viewport.origin.x, origin.x + 5);
    assert_eq!(h.app.viewport.origin.y, origin.y + 3);
    assert_eq!(h.committed_len(), 0);
}

#[test]
fn undo_while_typing_only_removes_keystrokes() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 16, 14, MouseButton::Left);
    h.press('5');
    h.click(30, 20);
    h.type_text("abc");
    h.key(KeyCode::Char('z'), KeyModifiers::CONTROL);
    let rendered = |p| h.app.editor.canvas.glyph_at(p);
    assert_eq!(rendered(h.cell_at(30, 20)), Some('a'));
    assert_eq!(rendered(h.cell_at(31, 20)), Some('b'));
    assert_eq!(rendered(h.cell_at(32, 20)), None);
    assert_eq!(h.committed_len(), 20);
    h.escape();
    assert_eq!(h.committed_len(), 22);
    h.key(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(h.committed_len(), 20);
    h.key(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(h.committed_len(), 0);
}

#[test]
fn enter_returns_the_cursor_to_the_start_column_and_digits_are_text() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('5');
    h.click(30, 20);
    h.type_text("1");
    h.key(KeyCode::Enter, KeyModifiers::empty());
    h.type_text("2");
    h.escape();
    assert_eq!(h.app.tool(), rsdia::core::editor::ToolId::Text);
    let c = &h.app.editor.canvas.committed;
    assert_eq!(c.get(h.cell_at(30, 20)), Some('1'));
    assert_eq!(c.get(h.cell_at(30, 21)), Some('2'));
}

#[test]
fn select_copies_cuts_pastes_and_nudges() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.press('2');
    h.drag(8, 8, 16, 13, MouseButton::Left);
    h.key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(h
        .app
        .clipboard
        .text
        .as_deref()
        .expect("a copy")
        .contains("┌───┐"));
    h.key(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(
        h.app.editor.canvas.committed.get(h.cell_at(11, 10)),
        Some('┌')
    );
    h.key(KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(h.committed_len(), 0);
    // The keyboard paste reads the system clipboard, so seed one.
    h.app.clipboard.set_text("PASTED");
    h.key(KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(h.committed_len(), 6);
}

#[test]
fn a_bracketed_paste_lands_on_the_canvas() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.app.on_paste("+--+\n|  |\n+--+");
    assert_eq!(h.committed_len(), 10);
    assert!(
        h.app.editor.canvas.scratch.is_empty(),
        "the preview layer is gone"
    );
}

// ------------------------------------------------------------------ files, export, storage

#[test]
fn autosave_writes_the_drawing() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.render();
    std::thread::sleep(std::time::Duration::from_millis(25));
    h.app.tick();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(h.app.current_path()).expect("the file"))
            .expect("json");
    assert_eq!(saved["cells"].as_array().expect("cells").len(), 12);
    assert_eq!(saved["name"].as_str(), Some("test"));
}

#[test]
fn a_new_drawing_via_the_dialog_and_a_switch_back_from_the_list() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    let (x, y) = h.label("≡");
    h.click(x, y);
    assert!(h.text_frame().contains("> test"));

    let lines = h.frame();
    let y_new = lines
        .iter()
        .position(|l| l.contains("[new]"))
        .expect("[new]");
    let x_new = char_find(&lines[y_new], "[").expect("[") as i32 + 1;
    h.click(x_new, y_new as i32);
    assert!(h.text_frame().contains("new drawing"));
    h.key(KeyCode::Char('u'), KeyModifiers::CONTROL);
    h.type_text("second");
    h.key(KeyCode::Enter, KeyModifiers::empty());
    h.render();
    assert_eq!(h.app.drawing_name(), "second");
    assert_eq!(h.committed_len(), 0);

    h.click(x, y);
    let lines = h.frame();
    let y_test = lines
        .iter()
        .position(|l| l.contains("  test "))
        .expect("the test row");
    let x_test = char_find(&lines[y_test], "test").expect("t") as i32;
    h.click(x_test, y_test as i32);
    assert_eq!(h.app.drawing_name(), "test");
    assert_eq!(h.committed_len(), 12);
}

#[test]
fn rename_rejects_a_duplicate_name() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.app.store.create("taken").expect("created");
    h.app.rename_drawing();
    h.key(KeyCode::Char('u'), KeyModifiers::CONTROL);
    h.type_text("taken");
    h.key(KeyCode::Enter, KeyModifiers::empty());
    assert!(h.text_frame().contains("a drawing with that name exists"));
    h.escape();
    assert_eq!(h.app.drawing_name(), "test");
}

#[test]
fn the_export_dialog_previews_switches_charset_and_copies() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.key(KeyCode::Char('e'), KeyModifiers::CONTROL);
    let frame = h.frame();
    assert!(frame.iter().any(|l| l.contains("[copy to clipboard]")));
    assert!(frame.iter().any(|l| l.contains("┌───┐")));

    let y = frame
        .iter()
        .position(|l| l.contains("basic"))
        .expect("basic");
    let x = char_find(&frame[y], "basic").expect("basic") as i32;
    h.click(x, y as i32);
    assert!(h.text_frame().contains("+---+"));

    let frame = h.frame();
    let y = frame
        .iter()
        .position(|l| l.contains("# hash"))
        .expect("hash");
    let x = char_find(&frame[y], "#").expect("#") as i32;
    h.click(x, y as i32);

    let frame = h.frame();
    let y = frame
        .iter()
        .position(|l| l.contains("[copy to clipboard]"))
        .expect("copy");
    let x = char_find(&frame[y], "[").expect("[") as i32 + 1;
    h.click(x, y as i32);
    assert_eq!(
        h.app.clipboard.text.as_deref(),
        Some("# +---+\n# |   |\n# +---+")
    );
}

#[test]
fn the_grid_style_toggles_live() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    assert!(h.text_frame().contains('\u{1FB7C}'));
    h.app.set_grid(rsdia::storage::config::GridStyle::Dots);
    let frame = h.text_frame();
    assert!(!frame.contains('\u{1FB7C}'));
    assert!(frame.contains('·'));
}

#[test]
fn help_carries_no_attribution_line() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let (x, y) = h.label("help");
    h.click(x, y);
    let frame = h.text_frame();
    assert!(frame.contains("switch tool"));
    assert!(!frame.to_lowercase().contains("asciiflow"));
    assert!(!frame.contains("github.com"));
}

#[test]
fn notices_sit_where_the_quit_prompt_does_and_the_prompt_outranks_them() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.key(KeyCode::Char('s'), KeyModifiers::CONTROL);
    let lines = h.frame();
    let row = lines
        .iter()
        .find(|l| l.contains("saved"))
        .expect("a notice")
        .clone();
    assert!(row.chars().position(|c| c == 's').expect("saved") < 4);
    assert!(char_find(&row, "test").expect("the name") > row.chars().count() / 2);
    assert!(!row.contains("drag to draw a box"));

    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    let frame = h.text_frame();
    assert!(frame.contains("press ctrl+q again to exit"));
    assert!(!frame.contains("saved"));
}

#[test]
fn bare_q_does_not_quit_and_ctrl_q_twice_does() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('q');
    assert!(!h.app.should_quit);
    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert!(!h.app.should_quit);
    assert!(h.text_frame().contains("press ctrl+q again to exit"));
    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert!(h.app.should_quit);
}

#[test]
fn the_copy_on_select_toggle_shows_and_flips() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let (x, y) = h.label("⚙");
    h.click(x, y);
    let frame = h.text_frame();
    assert!(frame.contains("copy on select:"));
    assert!(frame.contains("copy on select: off"));
    assert!(!h.app.config.copy_on_select);

    let lines = h.frame();
    let y = lines
        .iter()
        .position(|l| l.contains("copy on select:"))
        .expect("the row");
    let x = char_find(&lines[y], "off").expect("off") as i32;
    h.click(x, y as i32);
    assert!(h.text_frame().contains("copy on select: on"));
    assert!(h.app.config.copy_on_select);
}

#[test]
fn copy_on_select_is_off_by_default() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.press('2');
    h.drag(8, 8, 16, 13, MouseButton::Left);
    assert!(h.app.clipboard.text.is_none());
}

#[test]
fn the_settings_toggle_makes_a_finished_selection_copy_itself() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.app.set_copy_on_select(true);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.press('2');
    h.drag(8, 8, 16, 13, MouseButton::Left);
    assert!(h
        .app
        .clipboard
        .text
        .as_deref()
        .expect("a copy")
        .contains("┌───┐"));
    // Re-selecting the same cells yields the same text; the clipboard is left alone.
    let n = h.app.clipboard.copies;
    h.drag(8, 8, 16, 13, MouseButton::Left);
    assert_eq!(h.app.clipboard.copies, n);
}

#[test]
fn deleting_a_drawing_asks_first_and_creates_a_fresh_one_afterwards() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.render();
    std::thread::sleep(std::time::Duration::from_millis(25));
    h.app.tick();
    let doomed = h.app.current_path().to_path_buf();
    assert!(doomed.exists());

    let (x, y) = h.label("≡");
    h.click(x, y);
    let lines = h.frame();
    let y_delete = lines
        .iter()
        .position(|l| l.contains("[delete]"))
        .expect("[delete]");
    let x_delete = char_find(&lines[y_delete], "[").expect("[") as i32 + 1;
    h.click(x_delete, y_delete as i32);
    assert!(h
        .text_frame()
        .contains("delete \"test\"? this can't be undone."));

    h.press('y');
    h.render();
    assert!(!doomed.exists(), "the file is gone");
    assert_eq!(h.committed_len(), 0, "and a fresh drawing is open");
}

#[test]
fn imported_text_follows_the_pointer_and_commits_on_a_click() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.app.start_placing("+--+\n|  |\n+--+");
    h.click(20, 15);
    // The inner spaces are skipped, so ten glyphs land.
    assert_eq!(h.committed_len(), 10);
    assert!(
        h.app.editor.canvas.scratch.is_empty(),
        "the preview layer is gone"
    );
}

#[test]
fn the_save_export_prompt_writes_a_file() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.key(KeyCode::Char('e'), KeyModifiers::CONTROL);

    let lines = h.frame();
    let y = lines
        .iter()
        .position(|l| l.contains("[save…]"))
        .expect("[save…]");
    let x = char_find(&lines[y], "[save…]").expect("[save…]") as i32 + 1;
    h.click(x, y as i32);
    assert!(h.text_frame().contains("save export"));

    let out = h.dir.join("diagram.txt");
    h.key(KeyCode::Char('u'), KeyModifiers::CONTROL);
    h.type_text(out.to_str().expect("utf-8 path"));
    h.key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(
        std::fs::read_to_string(&out).expect("written"),
        "┌───┐\n│   │\n└───┘\n"
    );
}

#[test]
fn the_quit_prompt_lapses_after_three_seconds() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    std::thread::sleep(std::time::Duration::from_millis(3010));
    assert!(!h.text_frame().contains("press ctrl+q again to exit"));
    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert!(!h.app.should_quit);
    assert!(h.text_frame().contains("press ctrl+q again to exit"));
}

// ------------------------------------------------------------------ space to pan

#[test]
fn space_drag_pans_in_drawing_tools() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let origin = h.app.viewport.origin;
    h.press(' ');
    h.drag(50, 20, 40, 20, MouseButton::Left);
    assert_eq!(h.app.viewport.origin.x, origin.x + 10);
    assert_eq!(h.committed_len(), 0);
}

#[test]
fn in_text_mode_a_single_typed_space_does_not_pan_but_a_held_one_does() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('5');
    h.click(30, 20);
    h.type_text("a ");
    h.click(40, 25);
    assert_eq!(h.app.editor.text().cursor, Some(h.cell_at(40, 25)));
    h.type_text("b");
    let origin = h.app.viewport.origin;
    h.press(' ');
    h.press(' ');
    h.press(' ');
    h.drag(50, 20, 45, 20, MouseButton::Left);
    assert_eq!(h.app.viewport.origin.x, origin.x + 5);
    h.escape();
    assert_eq!(h.committed_len(), 2);
}

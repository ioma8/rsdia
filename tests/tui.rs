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
use rsdia::tui::painter::PanelId;
use rsdia::tui::theme::{palette, TerminalColors, ThemeName};
use rsdia::tui::toolbar::toolbar_row;

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

    /// Row the floating picker draws on for this screen.
    fn picker_row(&self) -> i32 {
        toolbar_row(self.buf.area().height as i32)
    }

    /// Screen position of a floating-toolbar label.
    fn label(&mut self, text: &str) -> (i32, i32) {
        let y = (self.picker_row() + 1) as usize;
        let lines = self.frame();
        (char_find(&lines[y], text).unwrap_or(0) as i32, y as i32)
    }

    fn open_menu(&mut self, label: &str) {
        let lines = self.frame();
        let x = char_find(&lines[0], label).expect("menu label") as i32;
        self.click(x, 0);
    }

    fn click_menu_item(&mut self, menu: &str, label: &str) {
        let panel = match menu {
            "File" => PanelId::FileMenu,
            "Edit" => PanelId::EditMenu,
            "View" => PanelId::ViewMenu,
            "Help" => PanelId::HelpMenu,
            _ => panic!("unknown menu {menu}"),
        };
        if self.app.panel != Some(panel) {
            self.open_menu(menu);
        }
        let lines = self.frame();
        let y = lines
            .iter()
            .position(|line| line.contains(label))
            .expect(label);
        let x = char_find(&lines[y], label).expect(label) as i32;
        self.click(x, y as i32);
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

#[test]
fn msedit_chrome_has_menus_line_numbers_and_blue_status() {
    use ratatui::style::Color;
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let frame = h.frame();
    for label in ["File", "Edit", "View", "Help"] {
        assert!(frame[0].contains(label), "{}", frame[0]);
    }
    assert!(frame[5].starts_with("  5 │"), "{}", frame[5]);
    h.click(2, 5);
    assert_eq!(h.committed_len(), 0, "the gutter is not canvas input");

    // No terminal answered here, so the whole chrome must stay on the stand-in
    // scheme: freezing the MS Edit greys would leave a dark canvas wearing them.
    assert_eq!(
        frame[0].char_indices().count(),
        120,
        "the bar spans the screen"
    );
    let status = h.buf.cell((10, 39)).expect("status cell").style();
    assert_eq!(status.bg, Some(Color::Rgb(0x81, 0xa1, 0xc1)));
    assert_eq!(status.fg, Some(Color::Black));
    assert_eq!(
        h.buf.cell((1, 0)).expect("menu strip").style().bg,
        Some(Color::Rgb(0x2e, 0x34, 0x40)),
        "the menu strip is the stand-in background, not a grey"
    );

    let edit_x = frame[0].find("Edit").expect("Edit menu") as i32;
    h.click(edit_x, 0);
    assert!(h.text_frame().contains("Undo"));
    assert!(h.text_frame().contains("Paste"));
}

#[test]
fn the_gutter_numbers_document_rows_so_panning_scrolls_them() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let row_number = |h: &mut Harness| -> i32 {
        h.frame()[5]
            .chars()
            .take(3)
            .collect::<String>()
            .trim()
            .parse()
            .expect("a number")
    };
    let before = row_number(&mut h);
    h.app.viewport.pan(0, 4);
    assert_eq!(row_number(&mut h), before + 4);
}

// ------------------------------------------------------------------ toolbar

#[test]
fn the_floating_toolbar_contains_only_tools_near_the_canvas_bottom() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let top = h.picker_row();
    let lines = h.frame();
    let row = &lines[(top + 1) as usize];
    for tool in ["box", "select", "arrow", "line", "text", "eraser"] {
        assert!(row.contains(tool), "{row}");
    }
    for menu_action in ["export", "help", "⚙", "⟲", "≡"] {
        assert!(
            !row.contains(menu_action),
            "duplicated {menu_action}: {row}"
        );
    }
    assert!(lines[top as usize].contains("┌"));
    assert!(lines[(top + 2) as usize].contains("┘"));
    assert!(
        lines[(top + 3) as usize].contains("rsdia"),
        "the picker sits on the status bar"
    );
}

#[test]
fn compact_menus_and_tiny_toolbar_fit_narrow_terminals() {
    let mut h = Harness::new(30, 40, Layer::new(), None);
    let row = (toolbar_row(40) + 1) as usize;
    assert!(h.frame()[row].contains("box sel arw lin txt ers"));

    let mut h = Harness::new(18, 40, Layer::new(), None);
    let frame = h.frame();
    assert!(frame[row].contains("b s a l t e"));
    // The mnemonic labels themselves, not the full ones clipped by the buffer.
    for (mnemonic, x) in [('F', 2), ('E', 6), ('V', 10), ('H', 14)] {
        assert_eq!(frame[0].chars().nth(x), Some(mnemonic), "{}", frame[0]);
    }
    assert!(!frame[0].contains("File"), "{}", frame[0]);
}

#[test]
fn a_tall_panel_leaves_the_floating_toolbar_intact() {
    let mut h = Harness::new(80, 24, Layer::new(), None);
    for i in 0..25 {
        h.app
            .store
            .create(&format!("drawing {i}"))
            .expect("created");
    }
    h.click_menu_item("File", "Drawings");
    assert!(h.text_frame().contains("[delete]"), "the panel is open");
    let top = toolbar_row(24) as usize;
    let lines = h.frame();
    assert!(lines[top].starts_with(" 20 │"), "{}", lines[top]);
    assert!(lines[top].contains('┌'), "torn: {}", lines[top]);
    assert!(lines[top + 1].contains("box select arrow line text eraser"));
    assert!(lines[top + 2].contains('┘'), "torn: {}", lines[top + 2]);
    // ...and the picker is still clickable.
    let (x, y) = h.label("line");
    h.click(x, y);
    assert_eq!(h.app.tool(), rsdia::core::editor::ToolId::Line);
}

#[test]
fn a_narrow_dropdown_keeps_its_border() {
    let mut h = Harness::new(18, 40, Layer::new(), None);
    h.open_menu("H");
    let lines = h.frame();
    // The label is shortened, but the panel's own frame survives the clamp.
    assert!(
        lines[1].starts_with('┌') && lines[1].ends_with('┐'),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].starts_with('│') && lines[2].ends_with('│'),
        "{}",
        lines[2]
    );
    assert!(
        lines[3].starts_with('└') && lines[3].ends_with('┘'),
        "{}",
        lines[3]
    );
    assert!(lines[2].contains("Keyboard"), "{}", lines[2]);
}

#[test]
fn menu_entries_grey_out_and_do_nothing_without_a_subject() {
    use ratatui::style::Color;
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let disabled = palette(ThemeName::Terminal, None).disabled;

    // Nothing drawn: cut and copy have no selection to work on.
    h.open_menu("Edit");
    let lines = h.frame();
    let copy_row = lines.iter().position(|l| l.contains("Copy")).expect("Copy");
    let copy_x = char_find(&lines[copy_row], "Copy").expect("Copy") as u16;
    assert_eq!(
        h.buf
            .cell((copy_x, copy_row as u16))
            .expect("cell")
            .style()
            .fg,
        Some(disabled)
    );
    h.click(copy_x as i32, copy_row as i32);
    assert!(
        h.app.clipboard.text.is_none(),
        "an inert entry does nothing"
    );
    assert_eq!(h.app.panel, Some(PanelId::EditMenu), "and stays open");

    // With a selection behind it, Copy is live and copies.
    h.escape();
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.press('2');
    h.drag(8, 8, 16, 13, MouseButton::Left);
    h.open_menu("Edit");
    let lines = h.frame();
    let copy_row = lines.iter().position(|l| l.contains("Copy")).expect("Copy");
    let copy_x = char_find(&lines[copy_row], "Copy").expect("Copy") as u16;
    assert_ne!(
        h.buf
            .cell((copy_x, copy_row as u16))
            .expect("cell")
            .style()
            .fg,
        Some(disabled)
    );
    h.click(copy_x as i32, copy_row as i32);
    assert!(h
        .app
        .clipboard
        .text
        .as_deref()
        .is_some_and(|t| t.contains('┌')));
    assert_eq!(h.app.panel, None);

    // The terminal theme falls back to the stand-in scheme, so the grey must be a
    // concrete colour rather than `Reset`.
    assert_ne!(disabled, Color::Reset);
}

#[test]
fn a_dropdown_hangs_off_its_label_however_it_is_clicked() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let label_x = char_find(&h.frame()[0], "File").expect("File") as i32;
    let dropdown_x = |h: &mut Harness| {
        h.frame()[1]
            .chars()
            .position(|c| c == '┌')
            .expect("a panel")
    };
    h.click(label_x, 0);
    let first = dropdown_x(&mut h);
    h.escape();
    h.click(label_x + 4, 0);
    assert_eq!(
        dropdown_x(&mut h),
        first,
        "the click position must not move it"
    );
}

#[test]
fn the_menu_bar_walks_with_alt_mnemonics_and_arrows() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    for (mnemonic, panel) in [
        ('f', PanelId::FileMenu),
        ('e', PanelId::EditMenu),
        ('v', PanelId::ViewMenu),
        ('h', PanelId::HelpMenu),
    ] {
        h.key(KeyCode::Char(mnemonic), KeyModifiers::ALT);
        assert_eq!(h.app.panel, Some(panel), "alt+{mnemonic}");
    }

    // Left and right walk the bar and wrap around it.
    h.key(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(h.app.panel, Some(PanelId::FileMenu));
    h.key(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(h.app.panel, Some(PanelId::EditMenu));
    h.key(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(h.app.panel, Some(PanelId::FileMenu));
    h.key(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(h.app.panel, Some(PanelId::HelpMenu), "wraps to the end");

    // Esc closes, and only then does esc reach the canvas again.
    h.key(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(h.app.panel, None);
}

#[test]
fn enter_runs_the_highlighted_menu_entry() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    assert_eq!(h.committed_len(), 12);

    // alt+e highlights Undo, and enter runs it.
    h.key(KeyCode::Char('e'), KeyModifiers::ALT);
    h.key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(h.committed_len(), 0, "undo ran");
    assert_eq!(h.app.panel, None, "and the menu closed");

    // Redo is the next row, and is still greyed out at this point.
    h.key(KeyCode::Char('e'), KeyModifiers::ALT);
    h.key(KeyCode::Down, KeyModifiers::empty());
    h.key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(h.committed_len(), 12);
}

#[test]
fn arrow_navigation_skips_disabled_entries_and_stops_at_the_ends() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let p = palette(ThemeName::Terminal, None);
    let highlighted = |h: &mut Harness, label: &str| {
        let lines = h.frame();
        let row = lines
            .iter()
            .position(|l| l.contains(label))
            .expect("the row");
        let x = char_find(&lines[row], label).expect("the label") as u16;
        h.buf.cell((x, row as u16)).expect("cell").style().bg == Some(p.menu_active_bg)
    };

    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.key(KeyCode::Char('e'), KeyModifiers::ALT);
    assert!(
        highlighted(&mut h, "Undo"),
        "the first row starts highlighted"
    );
    assert!(!highlighted(&mut h, "Redo"));
    assert!(!highlighted(&mut h, "Cut"), "no selection to cut");

    // Nothing is selectable between Undo and Paste, so down lands on Paste.
    h.key(KeyCode::Down, KeyModifiers::empty());
    assert!(highlighted(&mut h, "Paste"));
    assert!(!highlighted(&mut h, "Undo"));

    // Down at the end and up at the start stay put.
    h.key(KeyCode::Down, KeyModifiers::empty());
    assert!(highlighted(&mut h, "Paste"));
    h.key(KeyCode::Up, KeyModifiers::empty());
    h.key(KeyCode::Up, KeyModifiers::empty());
    assert!(
        highlighted(&mut h, "Undo"),
        "up stops at the first enabled row"
    );
}

#[test]
fn an_open_menu_swallows_plain_keys_but_not_the_global_shortcuts() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let tool = h.app.tool();
    h.key(KeyCode::Char('f'), KeyModifiers::ALT);
    assert_eq!(h.app.panel, Some(PanelId::FileMenu));

    for c in ['b', 'r', '2', '?', 'x', 'p'] {
        h.press(c);
        assert_eq!(h.app.tool(), tool, "{c} must not reach the canvas");
    }
    assert_eq!(h.app.panel, Some(PanelId::FileMenu), "nor close the menu");
    assert_eq!(h.committed_len(), 0);
    assert!(h.app.clipboard.text.is_none());

    // ctrl chords are global and still work while the menu is open.
    h.key(KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert!(h.text_frame().contains("saved"));
}

#[test]
fn hovering_another_menu_label_switches_the_open_dropdown() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.open_menu("File");
    assert_eq!(h.app.panel, Some(PanelId::FileMenu));
    assert!(h.text_frame().contains("Drawings"));
    let x = char_find(&h.frame()[0], "View").expect("View") as i32;
    h.mouse(MouseEventKind::Moved, x, 0, KeyModifiers::empty());
    h.render();
    assert_eq!(h.app.panel, Some(PanelId::ViewMenu));
    assert!(h.text_frame().contains("Recenter canvas"));
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
fn there_is_no_title_above_the_menu_and_the_status_bar_names_the_product() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let lines = h.frame();
    assert!(!lines[0].contains("rsdia"));
    assert!(lines[lines.len() - 1].contains("rsdia"));
}

#[test]
fn top_menus_open_dropdowns_then_their_panels_close_normally() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.open_menu("Edit");
    let edit = h.text_frame();
    assert!(edit.contains("Undo") && edit.contains("Ctrl+Z"));
    assert!(edit.contains("┌"));
    h.escape();
    h.open_menu("Help");
    assert!(h.text_frame().contains("Keyboard shortcuts"));
    h.escape();

    h.open_menu("View");
    assert!(h.text_frame().contains("Settings"));
    assert_eq!(h.app.panel, Some(PanelId::ViewMenu));
    h.click_menu_item("View", "Settings");
    let frame = h.frame();
    assert!(frame.iter().any(|line| line.contains("lattice")));
    assert!(frame.iter().any(|line| line.contains("theme:")));
    h.escape();
    assert!(!h.text_frame().contains("lattice"));

    h.open_menu("File");
    assert!(h.text_frame().contains("Drawings"));
    h.click_menu_item("File", "Drawings");
    assert!(h.text_frame().contains("[new]"));
    h.click(5, 30);
    assert_eq!(h.app.panel, None);
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

    h.click_menu_item("Edit", "Undo");
    assert_eq!(h.committed_len(), 0);
    h.click_menu_item("Edit", "Redo");
    assert_eq!(h.committed_len(), 26);
}

#[test]
fn drags_starting_on_the_toolbar_are_ignored() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    let (x, y) = h.label("box");
    h.drag(x, y, x + 5, 20, MouseButton::Left);
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

    // ...but not while an overlay owns the screen, however the pointer sits.
    h.key(KeyCode::Char('f'), KeyModifiers::ALT);
    let origin = h.app.viewport.origin;
    h.mouse(MouseEventKind::ScrollDown, 50, 20, KeyModifiers::empty());
    assert_eq!(
        h.app.viewport.origin.y, origin.y,
        "a menu freezes the canvas"
    );
    h.escape();
    h.key(KeyCode::Char('o'), KeyModifiers::CONTROL);
    h.mouse(MouseEventKind::ScrollDown, 50, 20, KeyModifiers::empty());
    assert_eq!(h.app.viewport.origin.y, origin.y, "so does a panel");
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
    h.press('y');
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
fn y_x_p_are_copy_cut_and_paste() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.drag(10, 10, 14, 12, MouseButton::Left);
    h.press('2');
    h.drag(8, 8, 16, 13, MouseButton::Left);
    h.press('y');
    assert!(h
        .app
        .clipboard
        .text
        .as_deref()
        .expect("a copy")
        .contains("┌───┐"));
    h.press('x');
    assert_eq!(h.committed_len(), 0);
    h.press('p');
    assert_eq!(h.committed_len(), 12, "the box is back");
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
    h.click_menu_item("File", "Drawings");
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

    h.click_menu_item("File", "Drawings");
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
fn help_opens_from_the_menu_and_from_the_question_mark() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('?');
    assert_eq!(h.app.panel, Some(PanelId::Help));
    let frame = h.text_frame();
    assert!(frame.contains("switch tool"));
    assert!(!frame.to_lowercase().contains("asciiflow"));
    assert!(!frame.contains("github.com"));
    h.escape();
    assert_eq!(h.app.panel, None);

    h.click_menu_item("Help", "Keyboard shortcuts");
    assert_eq!(h.app.panel, Some(PanelId::Help));
}

#[test]
fn recentering_centers_the_drawing_in_the_strip_above_the_picker() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    // An 11x27 box, deliberately drawn away from the middle.
    h.drag(60, 4, 70, 30, MouseButton::Left);
    h.app.recenter();
    h.render();

    let lines = h.frame();
    let picker = toolbar_row(40) as usize;
    // Column 5 onwards skips the gutter's own rule.
    let drawn: Vec<usize> = (1..picker)
        .filter(|r| lines[*r].chars().skip(5).any(|c| "┌└─│".contains(c)))
        .collect();
    let first = *drawn.first().expect("the box is on screen");
    let last = *drawn.last().expect("the box is on screen");
    assert_eq!(drawn.len(), 27, "all of the box is visible");
    assert!(last < picker - 1, "and clear of the picker");
    let (above, below) = (first - 1, picker - 1 - last);
    assert!(
        above.abs_diff(below) <= 1,
        "centered in the strip, got {above} above and {below} below"
    );
}

#[test]
fn the_view_menu_recenters_a_panned_canvas() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    // The picker's own frame contains `┌`, so look only at the canvas rows.
    let box_visible = |h: &mut Harness| {
        let top = toolbar_row(40) as usize;
        h.frame()[1..top].iter().any(|line| line.contains('┌'))
    };
    h.drag(10, 10, 20, 16, MouseButton::Left);
    assert!(box_visible(&mut h));

    h.app.viewport.pan(0, 40);
    assert!(!box_visible(&mut h), "the box is off-screen");
    h.click_menu_item("View", "Recenter canvas");
    h.render();
    assert!(box_visible(&mut h), "recenter brings it back");
    assert_eq!(h.app.panel, None, "and the menu closed");
}

#[test]
fn notices_sit_where_the_hint_does() {
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
}

#[test]
fn bare_q_does_not_quit_and_ctrl_q_does() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('q');
    assert!(!h.app.should_quit);
    h.key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert!(h.app.should_quit);
}

#[test]
fn ctrl_c_quits_instead_of_copying_even_mid_edit() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.press('t');
    h.click(12, 12);
    h.press('h');
    h.key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(h.app.should_quit);
    assert_eq!(h.app.clipboard.text, None, "ctrl+c is not a copy");

    // ...and a dialog does not eat it either.
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.app.new_drawing();
    h.key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert!(h.app.should_quit);
}

#[test]
fn the_copy_on_select_toggle_shows_and_flips() {
    let mut h = Harness::new(120, 40, Layer::new(), None);
    h.click_menu_item("View", "Settings");
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

    h.click_menu_item("File", "Drawings");
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

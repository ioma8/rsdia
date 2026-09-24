//! Draws the screenshots on the project page.
//!
//! `cargo run --example shots` paints the real app into a `ratatui::Buffer` and
//! writes `docs/shots/*.svg`: one run of cells per style, so the page shows the
//! actual UI rather than a mock-up.

use std::fmt::Write as _;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use rsdia::core::editor::{Editor, ToolId};
use rsdia::core::layer::Layer;
use rsdia::core::tools::tool::{Key, Mods};
use rsdia::core::vector::{units, Pos};
use rsdia::storage::config::{GridStyle, DEFAULT_CONFIG};
use rsdia::storage::drawings::DrawingStore;
use rsdia::tui::app::{App, AppOptions, Clipboard, OpenDrawing};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 26;
const FONT_SIZE: f64 = 15.0;
const CELL_WIDTH: f64 = 9.0;
const CELL_HEIGHT: f64 = 18.0;

const PAGE_BG: Color = Color::Rgb(0x2e, 0x34, 0x40);

const fn at(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn box_at(editor: &mut Editor, from: Pos, to: Pos) {
    editor.set_tool(ToolId::Box);
    editor.down(from, Mods::NONE);
    editor.move_to(to, Mods::NONE);
    editor.up();
}

fn arrow(editor: &mut Editor, from: Pos, to: Pos) {
    editor.set_tool(ToolId::Arrow);
    editor.down(from, Mods::NONE);
    editor.move_to(to, Mods::NONE);
    editor.up();
}

fn text(editor: &mut Editor, at: Pos, message: &str) {
    editor.set_tool(ToolId::Text);
    editor.down(at, Mods::NONE);
    editor.up();
    for c in message.chars() {
        editor.key(Key::Char(c), Mods::NONE);
    }
    editor.flush();
}

/// The diagram the shots show, drawn with the tools themselves.
fn diagram() -> Layer {
    let mut editor = Editor::new();
    box_at(&mut editor, at(2, 1), at(17, 4));
    box_at(&mut editor, at(23, 1), at(38, 5));
    box_at(&mut editor, at(23, 9), at(38, 12));
    arrow(&mut editor, at(17, 2), at(23, 2));
    arrow(&mut editor, at(30, 5), at(30, 9));
    text(&mut editor, at(3, 2), "draw");
    text(&mut editor, at(24, 2), "snap + move");
    text(&mut editor, at(24, 10), "export");
    text(
        &mut editor,
        at(2, 14),
        "one static binary - no runtime, no server, no account",
    );
    editor.canvas.committed.clone()
}

fn make_app(layer: Layer) -> App {
    let dir = std::env::temp_dir().join("rsdia-shots");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a writable temp dir");
    let store = DrawingStore::new(dir.join("drawings"));
    let path = store.path_for("diagram");
    store.save(&path, "diagram", &layer).expect("saved");
    let mut config = DEFAULT_CONFIG;
    // Dots rather than the lattice: the lattice glyph is missing from some fonts.
    config.grid = GridStyle::Dots;
    App::new(AppOptions {
        store,
        config,
        config_path: dir.join("config.json"),
        drawing: OpenDrawing {
            path,
            name: "diagram".to_string(),
            layer,
        },
        clipboard: Clipboard::memory(),
        term: None,
        autosave_ms: None,
    })
}

const fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn draw(app: &mut App) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, WIDTH, HEIGHT));
    app.tick();
    app.paint(&mut buf);
    buf
}

fn hex(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".to_string(),
        Color::White => "#ffffff".to_string(),
        _ => "#808080".to_string(),
    }
}

fn escape(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// One styled run of a row: the background rectangle and the glyphs on top of it.
fn run(out: &mut String, x: u16, y: u16, text: &str, fg: Color, bg: Color, bold: bool) {
    let width = f64::from(units(text.chars().count())) * CELL_WIDTH;
    let (left, top) = (f64::from(x) * CELL_WIDTH, f64::from(y) * CELL_HEIGHT);
    if bg != PAGE_BG {
        let _ = write!(
            out,
            r#"<rect x="{left}" y="{top}" width="{width}" height="{CELL_HEIGHT}" fill="{}"/>"#,
            hex(bg)
        );
    }
    if text.trim().is_empty() {
        return;
    }
    let _ = write!(
        out,
        r#"<text x="{left}" y="{}" fill="{}" textLength="{width}" lengthAdjust="spacing"{}>"#,
        CELL_HEIGHT.mul_add(0.75, top),
        hex(fg),
        if bold { r#" font-weight="600""# } else { "" }
    );
    escape(text, out);
    out.push_str("</text>");
}

fn svg(buf: &Buffer) -> String {
    let area = buf.area();
    let (w, h) = (
        f64::from(area.width) * CELL_WIDTH,
        f64::from(area.height) * CELL_HEIGHT,
    );
    let mut out = String::new();
    let _ = write!(
        out,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}""#,
            r#" viewBox="0 0 {w} {h}" font-size="{font}""#,
            r#" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace">"#,
            r#"<rect width="{w}" height="{h}" fill="{bg}"/>"#
        ),
        w = w,
        h = h,
        font = FONT_SIZE,
        bg = hex(PAGE_BG)
    );
    for y in 0..area.height {
        let mut x = 0;
        while x < area.width {
            let cell = buf.cell((x, y)).expect("inside the buffer");
            let (fg, bg) = (cell.fg, cell.bg);
            let bold = cell.modifier.contains(Modifier::BOLD);
            let start = x;
            let mut text = String::new();
            while x < area.width {
                let next = buf.cell((x, y)).expect("inside the buffer");
                if next.fg != fg || next.bg != bg || next.modifier != cell.modifier {
                    break;
                }
                text.push(next.symbol().chars().next().unwrap_or(' '));
                x += 1;
            }
            run(&mut out, start, y, &text, fg, bg, bold);
        }
    }
    out.push_str("</svg>\n");
    out
}

fn write_shot(name: &str, buf: &Buffer) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/shots");
    std::fs::create_dir_all(&dir).expect("docs/shots is writable");
    let path = dir.join(format!("{name}.svg"));
    std::fs::write(&path, svg(buf)).expect("the shot is writable");
    path
}

fn main() {
    let layer = diagram();
    println!(
        "--- drawing ---\n{}",
        rsdia::core::text::layer_to_text(&layer, None, false)
    );

    let mut app = make_app(layer.clone());
    let path = write_shot("canvas", &draw(&mut app));
    println!("wrote {}", path.display());

    let mut app = make_app(layer.clone());
    app.on_key(&key(KeyCode::Char('f'), KeyModifiers::ALT));
    let path = write_shot("menu", &draw(&mut app));
    println!("wrote {}", path.display());

    let mut app = make_app(layer.clone());
    app.on_key(&key(KeyCode::Char('e'), KeyModifiers::CONTROL));
    let path = write_shot("export", &draw(&mut app));
    println!("wrote {}", path.display());

    let mut app = make_app(layer);
    app.on_key(&key(KeyCode::Char('?'), KeyModifiers::NONE));
    let path = write_shot("help", &draw(&mut app));
    println!("wrote {}", path.display());
}

//! The terminal app: one full-screen surface that paints the canvas, toolbar,
//! popovers and dialogs, and routes mouse/keyboard input.
//!
//! Everything the terminal provides arrives as a crossterm event and everything
//! the app draws goes into a `ratatui::Buffer`, so the whole app can be driven
//! headlessly by the tests.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::clipboard::CopyToClipboard;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crossterm::execute;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;

use crate::core::canvas::{Canvas, Revision};
use crate::core::editor::{Editor, ToolId, TOOL_IDS};
use crate::core::export::{export_text, ExportConfig};
use crate::core::layer::Layer;
use crate::core::text::{text_size, text_to_layer};
use crate::core::tools::tool::{HoverHint, Key, Mods};
use crate::core::vector::Pos;
use crate::storage::config::{save_config, Config, GridStyle};
use crate::storage::drawings::{now_iso, slugify, DrawingInfo, DrawingStore};
use crate::tui::canvas_view::{render_canvas, CanvasViewState, Viewport};
use crate::tui::host::{ConfirmKind, Dialog, Host, InputDialog, InputKind};
use crate::tui::input::{alt_digit, ctrl_char, is_ctrl, is_shift, printable, tool_key};
use crate::tui::painter::{hotspot_at, in_rect, Action, Hotspot, ItemId, Painter, PanelId, Rect};
use crate::tui::popovers::{
    render_dialog, render_export, render_files, render_help, render_menu, render_settings,
};
use crate::tui::theme::{palette, Palette, TerminalColors, ThemeName};
use crate::tui::toolbar::{layout_toolbar, render_toolbar, ToolbarHost};

pub struct OpenDrawing {
    pub path: PathBuf,
    pub name: String,
    pub layer: Layer,
    pub created_at: String,
}

const CHIP_MS: u64 = 1500;
/// How long a first ctrl+q stays armed, waiting for the second one.
const QUIT_CONFIRM_MS: u64 = 3000;
const TOAST_MS: u64 = 2500;
const SPACE_PAN_MS: u64 = 1000;

#[derive(Clone, Copy)]
enum Mode {
    None,
    Draw { last: Pos },
    Pan { sx: i32, sy: i32, origin: Pos },
}

fn tool_hint(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Box => "drag to draw a box",
        ToolId::Select => "drag to select or move · del erases · ctrl+c/x/v",
        ToolId::Arrow => "drag to draw an arrow · press f to flip",
        ToolId::Line => "drag to draw a line · press f to flip",
        ToolId::Text => "click to place the cursor, then type",
        ToolId::Eraser => "drag to erase",
    }
}

fn tool_shortcut(c: char) -> Option<ToolId> {
    match c {
        'r' => Some(ToolId::Box),
        'v' => Some(ToolId::Select),
        'a' => Some(ToolId::Arrow),
        'l' => Some(ToolId::Line),
        't' => Some(ToolId::Text),
        'e' => Some(ToolId::Eraser),
        _ => None,
    }
}

fn panel_for(id: ItemId) -> Option<PanelId> {
    match id {
        ItemId::Files => Some(PanelId::Files),
        ItemId::Export => Some(PanelId::Export),
        ItemId::Settings => Some(PanelId::Settings),
        ItemId::Help => Some(PanelId::Help),
        ItemId::Menu => Some(PanelId::Menu),
        _ => None,
    }
}

/// System clipboard with an internal fallback. `system: false` keeps everything in
/// memory, which is what the tests use.
pub struct Clipboard {
    pub copied: Vec<String>,
    internal: Option<String>,
    system: bool,
}

impl Clipboard {
    pub fn new(system: bool) -> Self {
        Self {
            copied: Vec::new(),
            internal: None,
            system,
        }
    }

    pub fn memory() -> Self {
        Self::new(false)
    }

    fn local(&self) -> bool {
        self.system
            && std::env::var_os("SSH_TTY").is_none()
            && std::env::var_os("SSH_CONNECTION").is_none()
    }

    pub fn copy(&mut self, text: &str) {
        self.copied.push(text.to_string());
        self.internal = Some(text.to_string());
        if !self.system {
            return;
        }
        let _ = execute!(std::io::stdout(), CopyToClipboard::to_clipboard_from(text));
        if self.local() && cfg!(target_os = "macos") {
            let _ = run_with("pbcopy", Some(text));
        }
    }

    /// Seeds the internal clipboard: the fallback a headless run pastes from.
    pub fn set_text(&mut self, text: &str) {
        self.internal = Some(text.to_string());
    }

    pub fn paste(&mut self) -> Option<String> {
        if self.local() {
            let text = if cfg!(target_os = "macos") {
                run_with("pbpaste", None)
            } else {
                run_with("wl-paste", None).or_else(|| run_with("xclip", None))
            };
            if let Some(text) = text {
                return Some(text);
            }
        }
        self.internal.clone()
    }
}

fn run_with(cmd: &str, input: Option<&str>) -> Option<String> {
    use std::process::Stdio;
    let mut command = Command::new(cmd);
    if cmd == "xclip" {
        command.args(["-selection", "clipboard", "-o"]);
    } else if cmd == "wl-paste" {
        command.arg("--no-newline");
    }
    command.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(text.as_bytes());
    }
    let output = child.wait_with_output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

pub struct AppOptions {
    pub store: DrawingStore,
    pub config: Config,
    pub config_path: PathBuf,
    pub drawing: OpenDrawing,
    pub clipboard: Clipboard,
    /// The terminal's colors, if startup already read them.
    pub term: Option<TerminalColors>,
    /// Autosave debounce in ms.
    pub autosave_ms: Option<u64>,
}

pub struct App {
    pub editor: Editor,
    pub viewport: Viewport,
    pub config: Config,
    pub drawings: Vec<DrawingInfo>,
    pub panel: Option<PanelId>,
    pub clipboard: Clipboard,
    pub store: DrawingStore,
    pub config_path: PathBuf,
    pub pal: Palette,
    pub term: Option<TerminalColors>,
    pub drawing: OpenDrawing,
    pub width: i32,
    pub height: i32,
    pub hotspots: Vec<Hotspot>,
    pub chrome: Vec<Rect>,
    pub hint: HoverHint,
    pub should_quit: bool,

    panel_anchor: i32,
    dialog: Option<Dialog>,
    toast: Option<(String, Instant)>,
    chips_until: Instant,
    /// After this moment a ctrl+c quits; a first ctrl+c arms it.
    quit_armed: Option<Instant>,
    /// The last text sent to the clipboard, so re-selecting the same cells does not resend it.
    last_copy: Option<String>,
    pressed: Option<(Action, i32, i32)>,
    mode: Mode,
    hover: Option<(i32, i32)>,
    flip_toggle: bool,
    last_mods: Mods,
    /// Space presses in a row; terminals rarely report releases, so holding is
    /// inferred from repeats.
    space_run: (u32, Instant),
    placing: Option<Layer>,
    recenter_soon: bool,
    save_at: Option<Instant>,
    dirty: bool,
    autosave_ms: u64,
    revision: Revision,
}

impl App {
    pub fn new(opts: AppOptions) -> Self {
        let AppOptions {
            store,
            config,
            config_path,
            drawing,
            clipboard,
            term,
            autosave_ms,
        } = opts;
        let pal = palette(config.theme, term.as_ref());
        let mut app = Self {
            editor: Editor::with_canvas(Canvas::with_committed(drawing.layer.clone())),
            viewport: Viewport::default(),
            config,
            drawings: Vec::new(),
            panel: None,
            clipboard,
            store,
            config_path,
            pal,
            term,
            drawing,
            width: 0,
            height: 0,
            hotspots: Vec::new(),
            chrome: Vec::new(),
            hint: HoverHint::Default,
            should_quit: false,
            panel_anchor: 0,
            dialog: None,
            toast: None,
            chips_until: Instant::now(),
            quit_armed: None,
            last_copy: None,
            pressed: None,
            mode: Mode::None,
            hover: None,
            flip_toggle: false,
            last_mods: Mods::NONE,
            space_run: (0, Instant::now()),
            placing: None,
            recenter_soon: true,
            save_at: None,
            dirty: false,
            autosave_ms: autosave_ms.unwrap_or(500),
            revision: Revision::default(),
        };
        app.revision = app.editor.canvas.revision();
        app
    }

    // ---------------------------------------------------------------- state

    pub fn tool(&self) -> ToolId {
        self.editor.tool()
    }

    pub fn can_undo(&self) -> bool {
        self.editor.canvas.can_undo()
            || (self.editor.text_entry() && !self.editor.canvas.scratch.is_empty())
    }

    pub fn can_redo(&self) -> bool {
        self.editor.canvas.can_redo()
    }

    pub fn show_chips(&self) -> bool {
        self.panel == Some(PanelId::Help) || Instant::now() < self.chips_until
    }

    pub fn drawing_name(&self) -> &str {
        &self.drawing.name
    }

    pub fn current_path(&self) -> &Path {
        &self.drawing.path
    }

    pub fn export_preview(&self) -> String {
        let text = export_text(&self.editor.canvas.committed, &self.config.export);
        if text.is_empty() {
            "(empty drawing)".to_string()
        } else {
            text
        }
    }

    fn apply_theme(&mut self) {
        self.pal = palette(self.config.theme, self.term.as_ref());
    }

    /// Polls the canvas revision counters, replacing the JS `onChange` listeners.
    pub fn poll_canvas(&mut self) {
        let revision = self.editor.canvas.revision();
        if revision == self.revision {
            return;
        }
        if revision.committed != self.revision.committed {
            self.schedule_save();
        }
        self.revision = revision;
    }

    /// One frame-tick of bookkeeping: due autosaves and the toast/chip clocks.
    pub fn tick(&mut self) {
        self.poll_canvas();
        if self.save_at.is_some_and(|at| Instant::now() >= at) {
            self.save();
        }
    }

    pub fn toast(&mut self, text: &str) {
        self.toast = Some((
            text.to_string(),
            Instant::now() + Duration::from_millis(TOAST_MS),
        ));
    }

    // ---------------------------------------------------------------- saving

    fn schedule_save(&mut self) {
        self.dirty = true;
        self.save_at = Some(Instant::now() + Duration::from_millis(self.autosave_ms));
    }

    pub fn save(&mut self) {
        self.save_at = None;
        let drawing = &self.drawing;
        match self.store.save(
            &drawing.path,
            &drawing.name,
            &self.editor.canvas.committed,
            &drawing.created_at,
        ) {
            Ok(_) => self.dirty = false,
            Err(e) => self.toast(&format!("save failed: {e}")),
        }
    }

    fn persist_config(&self) {
        save_config(&self.config, &self.config_path);
    }

    /// Commits pending edits and writes everything to disk.
    pub fn shutdown(&mut self) {
        self.editor.cancel_gesture();
        self.editor.flush();
        if self.dirty || self.save_at.is_some() {
            self.save();
        }
        self.config.last_drawing = Some(self.drawing.path.to_string_lossy().to_string());
        self.persist_config();
    }

    /// Copies a finished selection. Off by default; the `settings` panel turns it on.
    fn auto_copy(&mut self) {
        if !self.config.copy_on_select || self.tool() != ToolId::Select {
            return;
        }
        let text = self.editor.copy_selection();
        // Re-selecting or moving the same cells yields the same text; leave the clipboard alone.
        let Some(text) = text else { return };
        if Some(&text) == self.last_copy.as_ref() {
            return;
        }
        self.last_copy = Some(text.clone());
        self.clipboard.copy(&text);
    }

    /// First ctrl+q arms the prompt in the status bar; a second one within the window quits.
    fn arm_quit(&mut self) {
        let now = Instant::now();
        if self.quit_armed.is_some_and(|until| now < until) {
            return self.quit();
        }
        self.quit_armed = Some(now + Duration::from_millis(QUIT_CONFIRM_MS));
    }

    pub fn quit(&mut self) {
        self.shutdown();
        self.should_quit = true;
    }

    // ---------------------------------------------------------------- painting

    pub fn paint(&mut self, buf: &mut Buffer) {
        let area = buf.area();
        self.width = area.width as i32;
        self.height = area.height as i32;
        let layout = layout_toolbar(self.width);
        if self.recenter_soon {
            self.recenter_soon = false;
            self.viewport
                .recenter(&self.editor, self.width, self.height, layout.bottom + 1);
        }
        let mut p = Painter::new(buf, self.pal, self.hover);
        let hover_cell = self.hover.map(|(x, y)| self.viewport.to_canvas(x, y));
        render_canvas(
            &mut p,
            &CanvasViewState {
                editor: &self.editor,
                viewport: self.viewport,
                grid: self.config.grid,
                hover_cell,
                hover_hint: self.hint,
                cursor_on: true,
            },
        );
        render_toolbar(&mut p, self, &layout);
        if let Some(panel) = self.panel {
            let anchor = self.panel_anchor;
            let bottom = layout.bottom;
            match panel {
                PanelId::Files => render_files(&mut p, self, anchor, bottom),
                PanelId::Export => render_export(&mut p, self, anchor, bottom),
                PanelId::Settings => render_settings(&mut p, self, anchor, bottom),
                PanelId::Help => render_help(&mut p, self, anchor, bottom),
                PanelId::Menu => render_menu(&mut p, self, anchor, bottom),
            }
        }
        self.paint_status(&mut p);
        if let Some(dialog) = self.dialog.as_ref() {
            let before = p.hotspots.len();
            render_dialog(&mut p, dialog);
            // The dialog is modal: only its own buttons are live.
            p.hotspots.truncate(before);
        }
        self.hotspots = std::mem::take(&mut p.hotspots);
        self.chrome = std::mem::take(&mut p.chrome);
    }

    fn paint_status(&mut self, p: &mut Painter) {
        let pal = p.pal;
        let y = self.height - 1;
        if y < 5 {
            return;
        }
        p.fill(
            Rect {
                x: 0,
                y,
                w: self.width,
                h: 1,
            },
            pal.bg,
        );
        let mut hint = tool_hint(self.tool());
        if self.placing.is_some() {
            hint = "click to place the imported text · esc cancels";
        } else if self.editor.text_entry() {
            hint = "typing · enter: new line · esc: done · ctrl+z: undo keystroke";
        } else if self.editor.drawing
            && (self.tool() == ToolId::Arrow || self.tool() == ToolId::Line)
        {
            hint = if self.flip_toggle {
                "press f to flip · flipped"
            } else {
                "press f to flip"
            };
        }
        // Every transient notice shares one slot: the hint's place, in the prompt's
        // color. The quit prompt outranks a toast, since it is the one the next
        // keystroke acts on.
        let now = Instant::now();
        let toast = self
            .toast
            .as_ref()
            .filter(|(_, until)| now < *until)
            .map(|(text, _)| text.as_str());
        let message = if self.quit_armed.is_some_and(|until| now < until) {
            Some("press ctrl+q again to exit")
        } else {
            toast
        };
        let right = format!(
            "{}{}  ·  rsdia",
            self.drawing.name,
            if self.dirty { " •" } else { "" }
        );
        let rx = 0.max(self.width - right.chars().count() as i32 - 1);
        let (text, fg) = match message {
            Some(m) => (m, pal.warning),
            None => (hint, pal.muted),
        };
        p.text_clipped(1, y, text, fg, pal.bg, Modifier::empty(), rx - 2);
        p.text(rx, y, &right, pal.muted, pal.bg);
        p.chrome.push(Rect {
            x: 0,
            y,
            w: self.width,
            h: 1,
        });
    }

    // ---------------------------------------------------------------- mouse

    fn mouse_mods(&self, e: &MouseEvent) -> Mods {
        let m = Mods::new(
            e.modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL),
            e.modifiers.contains(crossterm::event::KeyModifiers::ALT),
        );
        m.flipped(m.flip != self.flip_toggle)
    }

    /// Space is held: once outside text entry, or repeating while typing.
    fn space_held(&self) -> bool {
        let need = if self.editor.text_entry() { 2 } else { 1 };
        Instant::now() < self.space_run.1 && self.space_run.0 >= need
    }

    fn over_chrome(&self, x: i32, y: i32) -> bool {
        self.chrome.iter().any(|r| in_rect(*r, x, y))
    }

    pub fn on_mouse(&mut self, e: &MouseEvent) {
        let (x, y) = (e.column as i32, e.row as i32);
        match e.kind {
            MouseEventKind::ScrollDown => self.on_scroll(e, 0, 1),
            MouseEventKind::ScrollUp => self.on_scroll(e, 0, -1),
            MouseEventKind::ScrollLeft => self.on_scroll(e, -2, 0),
            MouseEventKind::ScrollRight => self.on_scroll(e, 2, 0),
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.on_move(e),
            MouseEventKind::Down(button) => self.on_down(e, button),
            MouseEventKind::Up(_) => self.on_up(x, y),
        }
    }

    fn on_scroll(&mut self, e: &MouseEvent, dx: i32, dy: i32) {
        let (x, y) = (e.column as i32, e.row as i32);
        if self.dialog.is_some() {
            return;
        }
        if self.panel.is_some() && self.over_chrome(x, y) {
            return;
        }
        self.viewport.pan(dx, dy);
        self.refresh_hover(x, y);
    }

    fn refresh_hover(&mut self, x: i32, y: i32) {
        self.hover = Some((x, y));
        let cell = self.viewport.to_canvas(x, y);
        if self.placing.is_some() {
            self.show_placing(cell);
        }
        self.hint = if self.tool() == ToolId::Select && !self.over_chrome(x, y) {
            let m = Mods::NONE;
            self.editor.hover_hint(cell, m)
        } else {
            HoverHint::Default
        };
    }

    fn on_move(&mut self, e: &MouseEvent) {
        let (x, y) = (e.column as i32, e.row as i32);
        let mods = self.mouse_mods(e);
        match self.mode {
            Mode::Pan { sx, sy, origin } => {
                self.viewport.origin = Pos::new(origin.x - (x - sx), origin.y - (y - sy));
            }
            Mode::Draw { last } => {
                let cell = self.viewport.to_canvas(x, y);
                self.last_mods = mods;
                if cell != last {
                    self.mode = Mode::Draw { last: cell };
                    self.editor.move_to(cell, mods);
                }
            }
            Mode::None => {}
        }
        self.refresh_hover(x, y);
    }

    fn on_down(&mut self, e: &MouseEvent, button: MouseButton) {
        let (x, y) = (e.column as i32, e.row as i32);
        self.hover = Some((x, y));
        let action = hotspot_at(&self.hotspots, x, y);
        self.pressed = action.map(|a| (a, x, y));
        if action.is_some() || self.dialog.is_some() {
            return;
        }
        if self.over_chrome(x, y) {
            return;
        }
        if self.panel.is_some() {
            // A click outside a popover only closes it.
            self.close_panel();
            return;
        }
        if !matches!(self.mode, Mode::None) {
            return;
        }
        let cell = self.viewport.to_canvas(x, y);
        let middle = button == MouseButton::Middle;
        if middle || (button == MouseButton::Left && self.space_held()) {
            if button == MouseButton::Left && self.editor.text_entry() {
                // The held space was typed into the text; take it back.
                for _ in 0..self.space_run.0 {
                    if !self.editor.text_last_typed_space() {
                        break;
                    }
                    self.editor.text_undo_keystroke();
                }
            }
            self.mode = Mode::Pan {
                sx: x,
                sy: y,
                origin: self.viewport.origin,
            };
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        if self.placing.is_some() {
            self.show_placing(cell);
            self.editor.canvas.commit_scratch();
            self.placing = None;
            self.toast("imported");
            return;
        }
        self.flip_toggle = false;
        self.last_mods = self.mouse_mods(e);
        self.mode = Mode::Draw { last: cell };
        let m = self.last_mods;
        self.editor.down(cell, m);
    }

    fn on_up(&mut self, x: i32, y: i32) {
        let pressed = self.pressed.take();
        if let Some((action, px, _)) = pressed {
            if hotspot_at(&self.hotspots, x, y) == Some(action) {
                self.activate(action, px);
            }
            return;
        }
        if matches!(self.mode, Mode::Draw { .. }) {
            self.editor.up();
            self.auto_copy();
        }
        self.mode = Mode::None;
        self.flip_toggle = false;
        self.refresh_hover(x, y);
    }

    // ---------------------------------------------------------------- keyboard

    pub fn on_key(&mut self, k: &KeyEvent) {
        // Terminals that report key releases (kitty protocol) say when a held space
        // ended; everything else is inferred from repeats.
        if k.kind == crossterm::event::KeyEventKind::Release {
            if k.code == KeyCode::Char(' ') {
                self.space_run = (0, Instant::now());
            }
            return;
        }
        self.handle_key(k);
    }

    fn handle_key(&mut self, k: &KeyEvent) {
        if self.dialog.is_some() {
            return self.dialog_key(k);
        }

        // Global shortcuts.
        if is_ctrl(k, 'q') {
            return self.arm_quit();
        }
        if is_ctrl(k, 'c') {
            return self.copy_selection(false);
        }
        if is_ctrl(k, 'z') {
            if is_shift(k) {
                self.redo();
            } else {
                self.undo();
            }
            return;
        }
        if is_ctrl(k, 'y') {
            self.redo();
            return;
        }
        if is_ctrl(k, 's') {
            self.editor.flush();
            self.save();
            self.toast("saved");
            return;
        }
        if is_ctrl(k, 'e') {
            let anchor = self.anchor_of(ItemId::Export);
            self.toggle_panel(PanelId::Export, anchor);
            return;
        }
        if is_ctrl(k, 'o') {
            let anchor = self.anchor_of(ItemId::Files);
            self.toggle_panel(PanelId::Files, anchor);
            return;
        }
        if is_ctrl(k, 'x') {
            return self.copy_selection(true);
        }
        if is_ctrl(k, 'v') {
            if let Some(text) = self.clipboard.paste() {
                self.paste_text(&text);
            }
            return;
        }
        if k.code == KeyCode::Esc {
            return self.escape();
        }
        if let Some(digit) = alt_digit(k) {
            return self.set_tool(TOOL_IDS[digit - 1]);
        }

        let key = tool_key(k);

        // Mid-drag: `f` flips line/arrow/select elbows; other keys go to the tool.
        if self.editor.drawing && matches!(self.mode, Mode::Draw { .. }) {
            if key == Some(Key::Char('f')) {
                self.flip_toggle = !self.flip_toggle;
                self.last_mods = self.last_mods.flipped(!self.last_mods.flip);
                if self.tool() == ToolId::Select {
                    if let Mode::Draw { last } = self.mode {
                        let m = self.last_mods;
                        self.editor.move_to(last, m);
                    }
                } else {
                    let m = self.last_mods;
                    self.editor.key(Key::Char('f'), m);
                }
                return;
            }
            if let Some(key) = key {
                let m = self.last_mods;
                self.editor.key(key, m);
            }
            return;
        }

        if k.code == KeyCode::Char(' ') {
            let now = Instant::now();
            let count = if now < self.space_run.1 {
                self.space_run.0 + 1
            } else {
                1
            };
            self.space_run = (count, now + Duration::from_millis(SPACE_PAN_MS));
            if !self.editor.text_entry() {
                return;
            }
        } else {
            self.space_run = (0, Instant::now());
        }

        if self.editor.text_entry() {
            if let Some(key) = key {
                self.editor.key(key, Mods::NONE);
            }
            return;
        }

        let ch = printable(k);
        if let Some(c @ '1'..='6') = ch {
            return self.set_tool(TOOL_IDS[c as usize - '1' as usize]);
        }
        if let Some(tool) = ch.and_then(tool_shortcut) {
            return self.set_tool(tool);
        }
        if ch == Some('?') {
            let anchor = self.anchor_of(ItemId::Help);
            return self.toggle_panel(PanelId::Help, anchor);
        }

        if self.tool() == ToolId::Select {
            if let Some(key) = key {
                self.editor.key(key, Mods::NONE);
            }
        }
    }

    fn escape(&mut self) {
        if self.panel.is_some() {
            return self.close_panel();
        }
        if self.placing.is_some() {
            self.placing = None;
            self.editor.canvas.clear_scratch();
            return;
        }
        if matches!(self.mode, Mode::Draw { .. }) {
            self.editor.cancel_gesture();
            self.mode = Mode::None;
            return;
        }
        if self.editor.text_entry() {
            return self.editor.flush();
        }
        if self.editor.canvas.selection.is_some() {
            self.editor.select_cleanup();
        }
    }

    fn dialog_key(&mut self, k: &KeyEvent) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        if k.code == KeyCode::Esc {
            return self.close_dialog();
        }
        if k.code == KeyCode::Enter {
            return self.submit_dialog();
        }
        if dialog.input().is_none() {
            if k.code == KeyCode::Char('y') {
                return self.submit_dialog();
            }
            if k.code == KeyCode::Char('n') {
                self.close_dialog();
            }
            return;
        }
        if let Some(d) = self.dialog.as_mut().and_then(Dialog::input_mut) {
            let mut chars: Vec<char> = d.value.chars().collect();
            match k.code {
                KeyCode::Backspace => {
                    if d.cursor > 0 {
                        d.cursor -= 1;
                        chars.remove(d.cursor);
                    }
                }
                KeyCode::Delete => {
                    if d.cursor < chars.len() {
                        chars.remove(d.cursor);
                    }
                }
                KeyCode::Left => d.cursor = d.cursor.saturating_sub(1),
                KeyCode::Right => d.cursor = (d.cursor + 1).min(chars.len()),
                KeyCode::Home => d.cursor = 0,
                KeyCode::End => d.cursor = chars.len(),
                _ => match ctrl_char(k) {
                    Some('a') => d.cursor = 0,
                    Some('e') => d.cursor = chars.len(),
                    Some('u') => {
                        chars.drain(0..d.cursor);
                        d.cursor = 0;
                    }
                    _ => {
                        if let Some(ch) = printable(k) {
                            chars.insert(d.cursor, ch);
                            d.cursor += 1;
                        }
                    }
                },
            }
            d.value = chars.into_iter().collect();
            d.error = None;
        }
    }

    pub fn on_paste(&mut self, text: &str) {
        if self.dialog.as_ref().and_then(Dialog::input).is_some() {
            let insert: String = text.split(['\r', '\n']).next().unwrap_or("").to_string();
            if let Some(d) = self.dialog.as_mut().and_then(Dialog::input_mut) {
                let mut chars: Vec<char> = d.value.chars().collect();
                let added = insert.chars().count();
                for (i, ch) in insert.chars().enumerate() {
                    chars.insert(d.cursor + i, ch);
                }
                d.cursor += added;
                d.value = chars.into_iter().collect();
            }
        } else if self.editor.text_entry() {
            for ch in text.replace("\r\n", "\n").chars() {
                let key = if ch == '\n' {
                    Key::Enter
                } else {
                    Key::Char(ch)
                };
                self.editor.key(key, Mods::NONE);
            }
        } else if self.dialog.is_none() {
            self.paste_text(text);
        }
    }

    // ---------------------------------------------------------------- actions

    pub fn undo(&mut self) {
        self.editor.undo();
    }

    pub fn redo(&mut self) {
        self.editor.redo();
    }

    pub fn set_tool(&mut self, id: ToolId) {
        self.editor.set_tool(id);
        self.hint = HoverHint::Default;
        self.chips_until = Instant::now() + Duration::from_millis(CHIP_MS);
    }

    fn anchor_of(&self, id: ItemId) -> i32 {
        layout_toolbar(self.width)
            .items
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.x)
            .unwrap_or(1)
    }

    fn toggle_panel(&mut self, id: PanelId, anchor_x: i32) {
        if self.panel == Some(id) {
            return self.close_panel();
        }
        if id == PanelId::Files {
            self.drawings = self.store.list();
        }
        self.panel = Some(id);
        self.panel_anchor = anchor_x;
    }

    pub fn close_panel(&mut self) {
        self.panel = None;
    }

    fn activate(&mut self, action: Action, x: i32) {
        match action {
            Action::Toolbar(id) | Action::MenuEntry(id) => self.activate_item(id, x),
            Action::FilesOpen(i) => {
                if let Some(path) = self.drawings.get(i).map(|d| d.path.clone()) {
                    self.open_drawing(&path);
                }
            }
            Action::FilesNew => self.new_drawing(),
            Action::FilesRename => self.rename_drawing(),
            Action::FilesFork => self.fork_drawing(),
            Action::FilesImport => self.import_text(),
            Action::FilesClear => self.clear_drawing(),
            Action::FilesDelete => self.delete_drawing(),
            Action::ExportCharset(c) => self.set_export(ExportConfig {
                characters: c,
                ..self.config.export
            }),
            Action::ExportFence => {
                let fenced = !self.config.export.fenced;
                self.set_export(ExportConfig {
                    fenced,
                    ..self.config.export
                });
            }
            Action::ExportWrapper(w) => self.set_export(ExportConfig {
                wrapper: w,
                ..self.config.export
            }),
            Action::ExportCopy => self.copy_export(),
            Action::ExportSave => self.save_export(),
            Action::SettingsGrid(g) => self.set_grid(g),
            Action::SettingsTheme(t) => self.set_theme(t),
            Action::SettingsCopyOnSelect => self.set_copy_on_select(!self.config.copy_on_select),
            Action::SettingsRecenter => self.recenter(),
            Action::DialogOk => self.submit_dialog(),
            Action::DialogCancel => self.close_dialog(),
        }
    }

    fn activate_item(&mut self, id: ItemId, anchor_x: i32) {
        match id {
            ItemId::Quit => self.quit(),
            ItemId::Undo => self.undo(),
            ItemId::Redo => self.redo(),
            ItemId::Tool(tool) => {
                self.set_tool(tool);
                self.panel = None;
            }
            other => {
                let Some(panel) = panel_for(other) else {
                    return;
                };
                let anchor = if panel == PanelId::Menu {
                    anchor_x
                } else {
                    self.anchor_of(other)
                };
                self.toggle_panel(panel, anchor);
            }
        }
    }

    fn copy_selection(&mut self, cut: bool) {
        if self.tool() != ToolId::Select {
            return;
        }
        let Some(text) = self.editor.copy_selection() else {
            return;
        };
        self.last_copy = Some(text.clone());
        self.clipboard.copy(&text);
        if cut {
            self.editor.erase_selection();
        }
        self.toast(if cut {
            "cut to clipboard"
        } else {
            "copied to clipboard"
        });
    }

    fn paste_text(&mut self, text: &str) {
        let size = text_size(text);
        let at = if self.tool() == ToolId::Select && self.editor.has_selection() {
            self.editor.selection_top_left().unwrap_or_default()
        } else {
            self.viewport
                .to_canvas((self.width - size.x) / 2, (self.height - size.y) / 2)
        };
        self.editor.cancel_gesture();
        self.set_tool(ToolId::Select);
        self.editor.paste(text, at);
        self.toast("pasted");
    }

    // dialogs

    pub fn open_dialog(&mut self, d: Dialog) {
        self.dialog = Some(d);
    }

    pub fn close_dialog(&mut self) {
        self.dialog = None;
    }

    fn submit_dialog(&mut self) {
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        match dialog {
            Dialog::Confirm(confirm) => {
                self.dialog = None;
                if confirm.kind == ConfirmKind::DeleteDrawing {
                    self.confirm_delete();
                }
            }
            Dialog::Input(input) => {
                let value = input.value.trim().to_string();
                match self.submit_input(input.kind, &value) {
                    Some(error) => {
                        if let Some(d) = self.dialog.as_mut().and_then(Dialog::input_mut) {
                            d.error = Some(error);
                        }
                    }
                    None => self.dialog = None,
                }
            }
        }
    }

    fn prompt(&mut self, kind: InputKind, title: &str, label: &str, value: &str) {
        self.open_dialog(Dialog::Input(InputDialog {
            kind,
            title: title.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            cursor: value.chars().count(),
            error: None,
        }));
    }

    fn name_error(&self, name: &str, except: Option<&str>) -> Option<String> {
        if name.is_empty() {
            return Some("name can't be empty".to_string());
        }
        let matches_except = except.is_some_and(|e| slugify(e) == slugify(name));
        if !matches_except && self.store.exists(name) {
            return Some("a drawing with that name exists".to_string());
        }
        None
    }

    // files

    pub fn open_drawing(&mut self, path: &Path) {
        if path == self.drawing.path {
            return self.close_panel();
        }
        match self.store.load(path) {
            Ok((file, layer)) => self.switch_to(OpenDrawing {
                path: path.to_path_buf(),
                name: file.name,
                layer,
                created_at: file.created_at,
            }),
            Err(e) => self.toast(&format!("can't open: {e}")),
        }
    }

    fn switch_to(&mut self, d: OpenDrawing) {
        self.editor.cancel_gesture();
        self.editor.flush();
        if self.dirty || self.save_at.is_some() {
            self.save();
        }
        self.placing = None;
        self.drawing = d;
        self.editor
            .set_canvas(Canvas::with_committed(self.drawing.layer.clone()));
        self.revision = self.editor.canvas.revision();
        self.config.last_drawing = Some(self.drawing.path.to_string_lossy().to_string());
        self.persist_config();
        self.panel = None;
        self.recenter_soon = true;
    }

    pub fn new_drawing(&mut self) {
        self.prompt(
            InputKind::NewDrawing,
            "new drawing",
            "name",
            &self.store.unique_name("untitled"),
        );
    }

    pub fn rename_drawing(&mut self) {
        let name = self.drawing.name.clone();
        self.prompt(InputKind::RenameDrawing, "rename drawing", "name", &name);
    }

    pub fn fork_drawing(&mut self) {
        let base = format!("{} copy", self.drawing.name);
        self.prompt(
            InputKind::ForkDrawing,
            "fork drawing",
            "name",
            &self.store.unique_name(&base),
        );
    }

    pub fn delete_drawing(&mut self) {
        self.open_dialog(Dialog::Confirm(crate::tui::host::ConfirmDialog {
            kind: ConfirmKind::DeleteDrawing,
            title: "delete drawing".to_string(),
            message: format!("delete \"{}\"? this can't be undone.", self.drawing.name),
            yes: "delete".to_string(),
        }));
    }

    fn confirm_delete(&mut self) {
        // Settle pending edits first so autosave can't recreate the file.
        self.editor.cancel_gesture();
        self.editor.flush();
        self.save_at = None;
        self.dirty = false;
        self.store.delete(&self.drawing.path);
        if let Some(next) = self.store.list().into_iter().next() {
            if let Ok((file, layer)) = self.store.load(&next.path) {
                self.switch_to(OpenDrawing {
                    path: next.path,
                    name: file.name,
                    layer,
                    created_at: file.created_at,
                });
            }
        } else {
            let name = self.store.unique_name("untitled");
            self.create_and_switch(&name);
        }
        self.drawings = self.store.list();
        self.toast("deleted");
    }

    fn create_and_switch(&mut self, name: &str) {
        match self.store.create(name) {
            Ok(path) => self.switch_to(OpenDrawing {
                path,
                name: name.to_string(),
                layer: Layer::new(),
                created_at: now_iso(),
            }),
            Err(e) => self.toast(&format!("save failed: {e}")),
        }
    }

    pub fn clear_drawing(&mut self) {
        self.editor.cancel_gesture();
        self.editor.flush();
        self.editor.select_cleanup();
        self.editor.canvas.clear();
        self.panel = None;
        self.toast("cleared · ctrl+z to undo");
    }

    pub fn import_text(&mut self) {
        self.prompt(InputKind::ImportText, "import text", "file", "");
    }

    /// Imported text follows the pointer as scratch until a click commits it.
    pub fn start_placing(&mut self, text: &str) {
        let layer = text_to_layer(text, Pos::default());
        if layer.is_empty() {
            return self.toast("nothing to import");
        }
        self.editor.cancel_gesture();
        self.editor.flush();
        self.panel = None;
        self.placing = Some(layer);
        let size = text_size(text);
        let center = self
            .viewport
            .to_canvas((self.width - size.x) / 2, (self.height - size.y) / 2);
        self.show_placing(center);
    }

    fn show_placing(&mut self, at: Pos) {
        let Some(placing) = self.placing.as_ref() else {
            return;
        };
        let mut scratch = Layer::new();
        for (p, v) in placing.entries() {
            scratch.set(p.add(at), v);
        }
        self.editor.canvas.set_scratch(scratch);
    }

    /// The dialog submit handlers, one arm per prompt.
    fn submit_input(&mut self, kind: InputKind, value: &str) -> Option<String> {
        match kind {
            InputKind::NewDrawing => {
                if let Some(err) = self.name_error(value, None) {
                    return Some(err);
                }
                self.create_and_switch(value);
                None
            }
            InputKind::RenameDrawing => {
                if value == self.drawing.name {
                    return None;
                }
                if let Some(err) = self.name_error(value, Some(&self.drawing.name.clone())) {
                    return Some(err);
                }
                self.editor.flush();
                let drawing = &self.drawing;
                match self.store.rename(
                    &drawing.path,
                    value,
                    &self.editor.canvas.committed,
                    &drawing.created_at,
                ) {
                    Ok(path) => {
                        self.drawing.path = path.clone();
                        self.drawing.name = value.to_string();
                        self.dirty = false;
                        self.drawings = self.store.list();
                        self.config.last_drawing = Some(path.to_string_lossy().to_string());
                        self.persist_config();
                        None
                    }
                    Err(e) => Some(format!("can't rename: {e}")),
                }
            }
            InputKind::ForkDrawing => {
                if let Some(err) = self.name_error(value, None) {
                    return Some(err);
                }
                self.editor.flush();
                let layer = self.editor.canvas.committed.clone();
                let path = self.store.path_for(value);
                let created_at = now_iso();
                match self.store.save(&path, value, &layer, &created_at) {
                    Ok(_) => {
                        self.switch_to(OpenDrawing {
                            path,
                            name: value.to_string(),
                            layer,
                            created_at,
                        });
                        self.toast(&format!("forked to {value}"));
                        None
                    }
                    Err(e) => Some(format!("can't write: {e}")),
                }
            }
            InputKind::ImportText => {
                let path = expand_path(value);
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        self.start_placing(&text);
                        None
                    }
                    Err(_) => Some("can't read that file".to_string()),
                }
            }
            InputKind::SaveExport => {
                let path = expand_path(value);
                let text = format!(
                    "{}\n",
                    export_text(&self.editor.canvas.committed, &self.config.export)
                );
                match std::fs::write(&path, text) {
                    Ok(()) => {
                        self.toast(&format!("saved {}", path.display()));
                        None
                    }
                    Err(e) => Some(format!("can't write: {e}")),
                }
            }
        }
    }

    // export

    pub fn set_export(&mut self, export: ExportConfig) {
        self.config.export = export;
        self.persist_config();
    }

    pub fn copy_export(&mut self) {
        let text = export_text(&self.editor.canvas.committed, &self.config.export);
        self.clipboard.copy(&text);
        self.toast("copied to clipboard");
    }

    pub fn save_export(&mut self) {
        let default = format!("{}.txt", slugify(&self.drawing.name));
        self.prompt(InputKind::SaveExport, "save export", "file", &default);
    }

    // settings

    pub fn set_grid(&mut self, g: GridStyle) {
        self.config.grid = g;
        self.persist_config();
    }

    pub fn set_copy_on_select(&mut self, on: bool) {
        self.config.copy_on_select = on;
        self.persist_config();
    }

    pub fn set_theme(&mut self, t: ThemeName) {
        self.config.theme = t;
        self.persist_config();
        self.apply_theme();
    }

    pub fn recenter(&mut self) {
        self.recenter_soon = true;
    }
}

/// A leading `~` expands to the home directory, as the JS prompt handlers did.
fn expand_path(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix('~') {
        if rest.is_empty() || rest.starts_with('/') {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            return PathBuf::from(format!("{home}{rest}"));
        }
    }
    PathBuf::from(value)
}

impl Host for App {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn drawings(&self) -> &[DrawingInfo] {
        &self.drawings
    }

    fn current_path(&self) -> &Path {
        &self.drawing.path
    }

    fn drawing_name(&self) -> &str {
        &self.drawing.name
    }

    fn panel(&self) -> Option<PanelId> {
        self.panel
    }

    fn export_preview(&self) -> String {
        App::export_preview(self)
    }
}

impl ToolbarHost for App {
    fn tool(&self) -> ToolId {
        App::tool(self)
    }

    fn panel(&self) -> Option<PanelId> {
        self.panel
    }

    fn can_undo(&self) -> bool {
        App::can_undo(self)
    }

    fn can_redo(&self) -> bool {
        App::can_redo(self)
    }

    fn show_chips(&self) -> bool {
        App::show_chips(self)
    }
}

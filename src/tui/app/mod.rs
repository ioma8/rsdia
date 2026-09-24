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
use crate::core::tools::tool::{Key, Mods};
use crate::core::vector::Pos;
use crate::storage::config::{save_config, Config, GridStyle};
use crate::storage::drawings::{slugify, DrawingInfo, DrawingStore};
use crate::tui::canvas_view::{render_canvas, CanvasViewState, Viewport};
use crate::tui::host::{ConfirmKind, Dialog, Host, InputDialog, InputKind};
use crate::tui::input::{alt_digit, ctrl_char, is_ctrl, is_shift, printable, tool_key};
use crate::tui::painter::{hotspot_at, in_rect, Action, Hotspot, ItemId, Painter, PanelId, Rect};
use crate::tui::popovers::{
    render_dialog, render_export, render_files, render_help, render_menu, render_settings,
};
use crate::tui::theme::{palette, Palette, TerminalColors, ThemeName};
use crate::tui::toolbar::{layout_toolbar, render_toolbar};

pub struct OpenDrawing {
    pub path: PathBuf,
    pub name: String,
    pub layer: Layer,
}

const CHIP_MS: u64 = 1500;
/// How long a first ctrl+q stays armed, waiting for the second one.
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
    /// The text last put on the clipboard; also the fallback a headless run pastes from.
    pub text: Option<String>,
    /// How many copies reached the clipboard, so a test can prove a repeat was skipped.
    pub copies: u32,
    system: bool,
}

impl Clipboard {
    pub fn new(system: bool) -> Self {
        Self {
            text: None,
            copies: 0,
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
        self.copies += 1;
        self.text = Some(text.to_string());
        if !self.system {
            return;
        }
        let _ = execute!(std::io::stdout(), CopyToClipboard::to_clipboard_from(text));
        if self.local() && cfg!(target_os = "macos") {
            let _ = run_with("pbcopy", Some(text));
        }
    }

    /// Seeds the internal clipboard, as if something had been copied elsewhere.
    pub fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_string());
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
        self.text.clone()
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
    /// The pointer rests on something the active tool would grab.
    pub hover_is_target: bool,
    pub should_quit: bool,

    panel_anchor: i32,
    dialog: Option<Dialog>,
    toast: Option<(String, Instant)>,
    chips_until: Instant,
    /// After this moment a ctrl+c quits; a first ctrl+c arms it.
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

mod actions;
mod events;
mod paint;

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
            hover_is_target: false,
            should_quit: false,
            panel_anchor: 0,
            dialog: None,
            toast: None,
            chips_until: Instant::now(),
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
        match self
            .store
            .save(&drawing.path, &drawing.name, &self.editor.canvas.committed)
        {
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
    pub fn quit(&mut self) {
        self.shutdown();
        self.should_quit = true;
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

    fn tool(&self) -> ToolId {
        App::tool(self)
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

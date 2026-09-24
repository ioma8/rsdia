//! The terminal app: one full-screen surface that paints the canvas, toolbar,
//! popovers and dialogs, and routes mouse/keyboard input.
//!
//! Everything the terminal provides arrives as a crossterm event and everything
//! the app draws goes into a `ratatui::Buffer`, so the whole app can be driven
//! headlessly by the tests.

use crate::core::canvas::{Canvas, Revision};
use crate::core::editor::{Editor, ToolId};
use crate::core::export::export_text;
use crate::core::layer::Layer;
use crate::core::tools::tool::Mods;
use crate::core::vector::Pos;
use crate::storage::config::{save_config, Config};
use crate::storage::drawings::{DrawingInfo, DrawingStore};
use crate::tui::canvas_view::Viewport;
use crate::tui::host::{Dialog, Host};
use crate::tui::painter::{Action, Hotspot, ItemId, PanelId, Rect};
use crate::tui::theme::{palette, Palette, TerminalColors};
use crossterm::clipboard::CopyToClipboard;
use crossterm::execute;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub struct OpenDrawing {
    pub path: PathBuf,
    pub name: String,
    pub layer: Layer,
}

/// Below this the menu bar, the canvas, the tool picker and the status bar can no
/// longer all be drawn, so the app says so rather than showing torn chrome.
const MIN_WIDTH: i32 = 16;
const MIN_HEIGHT: i32 = 6;
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

const fn tool_hint(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Box => "drag to draw a box",
        ToolId::Select => "drag to select or move · del erases · y x p copy/cut/paste",
        ToolId::Arrow => "drag to draw an arrow · press f to flip",
        ToolId::Line => "drag to draw a line · press f to flip",
        ToolId::Text => "click to place the cursor, then type",
        ToolId::Eraser => "drag to erase",
    }
}

const fn tool_shortcut(c: char) -> Option<ToolId> {
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

/// The four menu-bar dropdowns, as opposed to the tool panels they open.
const fn is_menu(panel: PanelId) -> bool {
    matches!(
        panel,
        PanelId::FileMenu | PanelId::EditMenu | PanelId::ViewMenu | PanelId::HelpMenu
    )
}

const fn panel_for(id: ItemId) -> Option<PanelId> {
    match id {
        ItemId::Files => Some(PanelId::Files),
        ItemId::Export => Some(PanelId::Export),
        ItemId::Settings => Some(PanelId::Settings),
        ItemId::Help => Some(PanelId::Help),
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
    #[must_use]
    pub const fn new(system: bool) -> Self {
        Self {
            text: None,
            copies: 0,
            system,
        }
    }

    #[must_use]
    pub const fn memory() -> Self {
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

/// Where the pointer is, and whether it rests on something the active tool would
/// grab — which the canvas paints as a highlight.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pointer {
    pub at: Option<(i32, i32)>,
    pub on_target: bool,
}

/// The drawing's persistence state: whether it changed, when autosave is due, and
/// how often to schedule it.
#[derive(Clone, Copy, Debug)]
struct Save {
    dirty: bool,
    due: Option<Instant>,
    every_ms: u64,
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
    /// Where the pointer is, and whether it rests on something the tool would grab.
    pub pointer: Pointer,
    pub should_quit: bool,

    panel_anchor: i32,
    /// Highlighted row of the open dropdown, for keyboard navigation.
    menu_index: usize,
    /// Selected drawing in the files list; `ListState` scrolls it into view.
    list_selection: Option<usize>,
    /// First export preview line shown, so a long drawing can be read through.
    preview_top: usize,
    dialog: Option<Dialog>,
    toast: Option<(String, Instant)>,
    chips_until: Instant,
    /// The last text sent to the clipboard, so re-selecting the same cells does not resend it.
    last_copy: Option<String>,
    pressed: Option<(Action, i32, i32)>,
    mode: Mode,
    flip_toggle: bool,
    last_mods: Mods,
    /// Space presses in a row; terminals rarely report releases, so holding is
    /// inferred from repeats.
    space_run: (u32, Instant),
    placing: Option<Layer>,
    recenter_soon: bool,
    /// What has changed since the last write, and when the next one is due.
    save: Save,
    revision: Revision,
}

mod actions;
mod events;
mod paint;

impl App {
    #[must_use]
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
            pointer: Pointer::default(),
            should_quit: false,
            panel_anchor: 0,
            menu_index: 0,
            list_selection: None,
            preview_top: 0,
            dialog: None,
            toast: None,
            chips_until: Instant::now(),
            last_copy: None,
            pressed: None,
            mode: Mode::None,
            flip_toggle: false,
            last_mods: Mods::NONE,
            space_run: (0, Instant::now()),
            placing: None,
            recenter_soon: true,
            save: Save {
                dirty: false,
                due: None,
                every_ms: autosave_ms.unwrap_or(500),
            },
            revision: Revision::default(),
        };
        app.revision = app.editor.canvas.revision();
        app
    }

    // ---------------------------------------------------------------- state

    #[must_use]
    pub const fn tool(&self) -> ToolId {
        self.editor.tool()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.editor.canvas.can_undo()
            || (self.editor.text_entry() && !self.editor.canvas.scratch.is_empty())
    }

    #[must_use]
    pub const fn can_redo(&self) -> bool {
        self.editor.canvas.can_redo()
    }

    #[must_use]
    pub const fn has_selection(&self) -> bool {
        self.editor.has_selection()
    }

    #[must_use]
    pub const fn menu_index(&self) -> usize {
        self.menu_index
    }

    #[must_use]
    pub const fn list_selection(&self) -> Option<usize> {
        self.list_selection
    }

    #[must_use]
    pub const fn preview_top(&self) -> usize {
        self.preview_top
    }

    #[must_use]
    pub fn show_chips(&self) -> bool {
        self.panel == Some(PanelId::Help) || Instant::now() < self.chips_until
    }

    #[must_use]
    pub fn drawing_name(&self) -> &str {
        &self.drawing.name
    }

    #[must_use]
    pub fn current_path(&self) -> &Path {
        &self.drawing.path
    }

    #[must_use]
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
        if self.save.due.is_some_and(|at| Instant::now() >= at) {
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
        self.save.dirty = true;
        self.save.due = Some(Instant::now() + Duration::from_millis(self.save.every_ms));
    }

    pub fn save(&mut self) {
        self.save.due = None;
        let drawing = &self.drawing;
        match self
            .store
            .save(&drawing.path, &drawing.name, &self.editor.canvas.committed)
        {
            Ok(()) => self.save.dirty = false,
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
        if self.save.dirty || self.save.due.is_some() {
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
        Self::export_preview(self)
    }

    fn tool(&self) -> ToolId {
        Self::tool(self)
    }

    fn can_undo(&self) -> bool {
        Self::can_undo(self)
    }

    fn can_redo(&self) -> bool {
        Self::can_redo(self)
    }

    fn has_selection(&self) -> bool {
        Self::has_selection(self)
    }

    fn menu_index(&self) -> usize {
        Self::menu_index(self)
    }

    fn list_selection(&self) -> Option<usize> {
        Self::list_selection(self)
    }

    fn preview_top(&self) -> usize {
        Self::preview_top(self)
    }

    fn show_chips(&self) -> bool {
        Self::show_chips(self)
    }
}

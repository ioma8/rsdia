//! Routes input to the active tool. The terminal-free counterpart of
//! ASCIIFlow's store + controller: tests drive it directly.

use super::canvas::Canvas;
use super::tools::box_tool::BoxTool;
use super::tools::eraser::EraserTool;
use super::tools::line_tool::LineTool;
use super::tools::select::SelectTool;
use super::tools::text_tool::TextTool;
use super::tools::tool::{HoverHint, Key, Mods, Tool};
use super::vector::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolId {
    Box,
    Select,
    Arrow,
    Line,
    Text,
    Eraser,
}

pub const TOOL_IDS: [ToolId; 6] = [
    ToolId::Box,
    ToolId::Select,
    ToolId::Arrow,
    ToolId::Line,
    ToolId::Text,
    ToolId::Eraser,
];

impl ToolId {
    pub fn name(self) -> &'static str {
        match self {
            ToolId::Box => "box",
            ToolId::Select => "select",
            ToolId::Arrow => "arrow",
            ToolId::Line => "line",
            ToolId::Text => "text",
            ToolId::Eraser => "eraser",
        }
    }
}

pub struct Editor {
    pub canvas: Canvas,
    tool_id: ToolId,
    box_tool: BoxTool,
    select_tool: SelectTool,
    arrow_tool: LineTool,
    line_tool: LineTool,
    text_tool: TextTool,
    eraser_tool: EraserTool,
    /// A pointer gesture is in progress on the canvas.
    pub drawing: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        Self::with_canvas(Canvas::new())
    }

    pub fn with_canvas(canvas: Canvas) -> Self {
        Self {
            canvas,
            tool_id: ToolId::Box,
            box_tool: BoxTool::default(),
            select_tool: SelectTool::default(),
            arrow_tool: LineTool::new(true),
            line_tool: LineTool::new(false),
            text_tool: TextTool::default(),
            eraser_tool: EraserTool::default(),
            drawing: false,
        }
    }

    pub fn tool(&self) -> ToolId {
        self.tool_id
    }

    pub fn select(&self) -> &SelectTool {
        &self.select_tool
    }

    pub fn text(&self) -> &TextTool {
        &self.text_tool
    }

    // The app never reaches into the tools; these six wrappers keep the canvas and
    // the tool borrowed disjointly.

    pub fn select_cleanup(&mut self) {
        let canvas = &mut self.canvas;
        self.select_tool.cleanup(canvas);
    }

    pub fn has_selection(&self) -> bool {
        self.select_tool.has_selection(&self.canvas)
    }

    pub fn selection_top_left(&self) -> Option<Pos> {
        self.select_tool.select_box.map(|b| b.top_left())
    }

    pub fn copy_selection(&self) -> Option<String> {
        self.select_tool.copy_selection(&self.canvas)
    }

    pub fn erase_selection(&mut self) -> bool {
        let canvas = &mut self.canvas;
        self.select_tool.erase_selection(canvas)
    }

    pub fn paste(&mut self, text: &str, at: Pos) {
        let canvas = &mut self.canvas;
        self.select_tool.paste(canvas, text, at);
    }

    pub fn text_undo_keystroke(&mut self) -> bool {
        let canvas = &mut self.canvas;
        self.text_tool.undo_keystroke(canvas)
    }

    pub fn text_last_typed_space(&self) -> bool {
        self.text_tool.last_typed_space()
    }

    /// The active tool plus the canvas, borrowed disjointly.
    fn tool_mut(&mut self) -> (&mut dyn Tool, &mut Canvas) {
        let canvas = &mut self.canvas;
        let tool: &mut dyn Tool = match self.tool_id {
            ToolId::Box => &mut self.box_tool,
            ToolId::Select => &mut self.select_tool,
            ToolId::Arrow => &mut self.arrow_tool,
            ToolId::Line => &mut self.line_tool,
            ToolId::Text => &mut self.text_tool,
            ToolId::Eraser => &mut self.eraser_tool,
        };
        (tool, canvas)
    }

    /// Printable keys belong to the text tool rather than shortcuts.
    pub fn text_entry(&self) -> bool {
        self.tool_id == ToolId::Text && self.text_tool.editing()
    }

    pub fn set_tool(&mut self, id: ToolId) {
        if id == self.tool_id {
            return;
        }
        self.cancel_gesture();
        let (tool, canvas) = self.tool_mut();
        tool.cleanup(canvas);
        self.tool_id = id;
    }

    /// Swaps in another drawing's canvas.
    pub fn set_canvas(&mut self, canvas: Canvas) {
        self.cancel_gesture();
        let (tool, current) = self.tool_mut();
        tool.cleanup(current);
        self.canvas = canvas;
    }

    pub fn down(&mut self, p: Pos, m: Mods) {
        self.drawing = true;
        let (tool, canvas) = self.tool_mut();
        tool.start(canvas, p, m);
    }

    pub fn move_to(&mut self, p: Pos, m: Mods) {
        if !self.drawing {
            return;
        }
        let (tool, canvas) = self.tool_mut();
        tool.move_to(canvas, p, m);
    }

    pub fn up(&mut self) {
        if !self.drawing {
            return;
        }
        self.drawing = false;
        let (tool, canvas) = self.tool_mut();
        tool.end(canvas);
    }

    /// Aborts an in-progress drag without committing.
    pub fn cancel_gesture(&mut self) {
        if !self.drawing {
            return;
        }
        self.drawing = false;
        self.canvas.clear_scratch();
        let (tool, canvas) = self.tool_mut();
        tool.cleanup(canvas);
    }

    pub fn key(&mut self, key: Key, m: Mods) -> bool {
        let (tool, canvas) = self.tool_mut();
        tool.handle_key(canvas, key, m)
    }

    pub fn hover_hint(&self, p: Pos, m: Mods) -> HoverHint {
        match self.tool_id {
            ToolId::Box => self.box_tool.hover_hint(&self.canvas, p, m),
            ToolId::Select => self.select_tool.hover_hint(&self.canvas, p, m),
            ToolId::Arrow => self.arrow_tool.hover_hint(&self.canvas, p, m),
            ToolId::Line => self.line_tool.hover_hint(&self.canvas, p, m),
            ToolId::Text => self.text_tool.hover_hint(&self.canvas, p, m),
            ToolId::Eraser => self.eraser_tool.hover_hint(&self.canvas, p, m),
        }
    }

    /// Undo: a live text session loses its last keystroke first.
    pub fn undo(&mut self) -> bool {
        if self.drawing {
            return false;
        }
        if self.text_entry() && self.text_undo_keystroke() {
            return true;
        }
        if self.text_entry() {
            self.flush();
        }
        self.canvas.undo()
    }

    pub fn redo(&mut self) -> bool {
        if self.drawing {
            return false;
        }
        if self.text_entry() {
            self.flush();
        }
        self.canvas.redo()
    }

    /// Commits pending tool state (a text session) before save/switch.
    pub fn flush(&mut self) {
        if self.tool_id == ToolId::Text {
            self.text_tool.commit(&mut self.canvas);
        }
    }
}

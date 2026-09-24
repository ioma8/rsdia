//! What popovers and dialogs need from the app.
//!
//! Popovers only read; every mutation happens through a [`crate::tui::painter::Action`]
//! that the app interprets after the frame. That replaces the JS closures.

use std::path::Path;

use crate::core::editor::{Editor, ToolId};
use crate::storage::config::Config;
use crate::storage::drawings::DrawingInfo;
use crate::tui::painter::PanelId;

/// Everything the panels and the toolbar read. Popovers never mutate.
pub trait Host {
    fn editor(&self) -> &Editor;
    fn config(&self) -> &Config;
    fn drawings(&self) -> &[DrawingInfo];
    fn current_path(&self) -> &Path;
    fn drawing_name(&self) -> &str;
    fn panel(&self) -> Option<PanelId>;
    fn export_preview(&self) -> String;

    // The toolbar's share.
    fn tool(&self) -> ToolId;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    /// The select tool holds a live selection, so cut and copy are meaningful.
    fn has_selection(&self) -> bool;
    /// Row of the open dropdown that keyboard navigation has highlighted.
    fn menu_index(&self) -> usize;
    /// Selected row of the drawings list, which the list widget scrolls into view.
    fn list_selection(&self) -> Option<usize>;
    /// First preview line shown in the export panel.
    fn preview_top(&self) -> usize;
    fn show_chips(&self) -> bool;
}

/// Which prompt an input dialog is asking for; the app owns the handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    NewDrawing,
    RenameDrawing,
    ForkDrawing,
    ImportText,
    SaveExport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDialog {
    pub kind: InputKind,
    pub title: String,
    pub label: String,
    pub value: String,
    pub cursor: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub yes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dialog {
    Input(InputDialog),
    Confirm(ConfirmDialog),
}

impl ConfirmDialog {
    /// The one thing a confirmation does today: delete the open drawing.
    #[must_use]
    pub fn delete_drawing(name: &str) -> Self {
        Self {
            title: "delete drawing".to_string(),
            message: format!("delete \"{name}\"? this can't be undone."),
            yes: "delete".to_string(),
        }
    }
}

impl Dialog {
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Input(d) => &d.title,
            Self::Confirm(d) => &d.title,
        }
    }

    #[must_use]
    pub const fn input(&self) -> Option<&InputDialog> {
        match self {
            Self::Input(d) => Some(d),
            Self::Confirm(_) => None,
        }
    }

    pub const fn input_mut(&mut self) -> Option<&mut InputDialog> {
        match self {
            Self::Input(d) => Some(d),
            Self::Confirm(_) => None,
        }
    }
}

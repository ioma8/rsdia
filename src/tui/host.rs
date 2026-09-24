//! What popovers and dialogs need from the app.
//!
//! Popovers only read; every mutation happens through a [`crate::tui::painter::Action`]
//! that the app interprets after the frame. That replaces the JS closures.

use std::path::Path;

use crate::core::editor::Editor;
use crate::storage::config::Config;
use crate::storage::drawings::DrawingInfo;
use crate::tui::painter::PanelId;

pub trait Host {
    fn editor(&self) -> &Editor;
    fn config(&self) -> &Config;
    fn drawings(&self) -> &[DrawingInfo];
    fn current_path(&self) -> &Path;
    fn drawing_name(&self) -> &str;
    fn panel(&self) -> Option<PanelId>;
    fn export_preview(&self) -> String;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmKind {
    DeleteDrawing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmDialog {
    pub kind: ConfirmKind,
    pub title: String,
    pub message: String,
    pub yes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dialog {
    Input(InputDialog),
    Confirm(ConfirmDialog),
}

impl Dialog {
    pub fn title(&self) -> &str {
        match self {
            Dialog::Input(d) => &d.title,
            Dialog::Confirm(d) => &d.title,
        }
    }

    pub fn input(&self) -> Option<&InputDialog> {
        match self {
            Dialog::Input(d) => Some(d),
            Dialog::Confirm(_) => None,
        }
    }

    pub fn input_mut(&mut self) -> Option<&mut InputDialog> {
        match self {
            Dialog::Input(d) => Some(d),
            Dialog::Confirm(_) => None,
        }
    }
}

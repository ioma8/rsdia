//! What the app does once a click, key or dialog lands: panels, dialogs,
//! files, export and settings.

use super::{expand_path, is_menu, panel_for, App, OpenDrawing, CHIP_MS};
use crate::core::canvas::Canvas;
use crate::core::editor::ToolId;
use crate::core::export::{export_text, ExportConfig};
use crate::core::layer::Layer;
use crate::core::text::{text_size, text_to_layer};
use crate::core::vector::{index, units, Pos};
use crate::storage::config::GridStyle;
use crate::storage::drawings::slugify;
use crate::tui::host::{ConfirmDialog, Dialog, InputDialog, InputKind};
use crate::tui::painter::{Action, ItemId, PanelId};
use crate::tui::popovers::{menu_entries, menu_entry_disabled};
use crate::tui::theme::ThemeName;
use crate::tui::toolbar::{menu_anchor, menu_panels};
use std::path::Path;
use std::time::{Duration, Instant};

impl App {
    pub fn undo(&mut self) {
        self.editor.undo();
    }

    pub fn redo(&mut self) {
        self.editor.redo();
    }

    pub fn set_tool(&mut self, id: ToolId) {
        self.editor.set_tool(id);
        self.pointer.on_target = false;
        self.chips_until = Instant::now() + Duration::from_millis(CHIP_MS);
    }

    /// A dropdown hangs off its menu label, wherever on the label it was clicked;
    /// other panels come from a toolbar item and keep the clicked position.
    pub(crate) fn anchor_for(&self, panel: PanelId, clicked_x: i32) -> i32 {
        if is_menu(panel) {
            menu_anchor(panel, self.width)
        } else {
            clicked_x
        }
    }

    pub(crate) fn toggle_panel(&mut self, id: PanelId, anchor_x: i32) {
        if self.panel == Some(id) {
            return self.close_panel();
        }
        if id == PanelId::Files {
            self.drawings = self.store.list();
            // Open focused on the drawing you are already in.
            let current = self
                .drawings
                .iter()
                .position(|d| d.path == self.drawing.path);
            self.list_selection = current.or(Some(0)).filter(|_| !self.drawings.is_empty());
        }
        if id == PanelId::Export {
            self.preview_top = 0;
        }
        if is_menu(id) {
            self.menu_index = 0;
        }
        self.panel = Some(id);
        self.panel_anchor = anchor_x;
    }

    /// Opens a dropdown by mnemonic; already open leaves its highlight alone.
    pub(crate) fn open_menu(&mut self, panel: PanelId) {
        if self.panel == Some(panel) {
            return;
        }
        let anchor = self.anchor_for(panel, 0);
        self.toggle_panel(panel, anchor);
    }

    /// Moves the drawings list selection by `step`, clamped to the list.
    pub(crate) fn move_list_selection(&mut self, step: i32) {
        if self.drawings.is_empty() {
            self.list_selection = None;
            return;
        }
        let last = units(self.drawings.len()) - 1;
        let at = units(self.list_selection.unwrap_or(0)) + step;
        self.list_selection = Some(index(at.clamp(0, last)));
    }

    /// Selects the first or last drawing.
    pub(crate) const fn select_list_edge(&mut self, end: bool) {
        if self.drawings.is_empty() {
            return;
        }
        let last = self.drawings.len() - 1;
        self.list_selection = Some(if end { last } else { 0 });
    }

    /// Scrolls the export preview by `step` lines.
    pub(crate) fn scroll_preview(&mut self, step: i32) {
        let last = self.export_preview().lines().count().saturating_sub(1);
        let at = index(units(self.preview_top) + step);
        self.preview_top = at.min(last);
    }

    /// Opens the selected drawing, if there is one.
    pub(crate) fn open_selected_drawing(&mut self) {
        let Some(i) = self.list_selection else {
            return;
        };
        if let Some(path) = self.drawings.get(i).map(|d| d.path.clone()) {
            self.open_drawing(&path);
        }
    }

    /// Walks the menu bar to the next or previous dropdown, wrapping.
    pub(crate) fn step_menu(&mut self, panel: PanelId, step: i32) {
        let panels = menu_panels();
        let Some(at) = panels.iter().position(|p| *p == panel) else {
            return;
        };
        let next = (units(at) + step).rem_euclid(units(panels.len()));
        self.open_menu(panels[index(next)]);
    }

    /// Moves the highlight, skipping entries with nothing to act on and stopping
    /// at the ends of the menu.
    pub(crate) fn move_menu_cursor(&mut self, step: i32) {
        let Some(panel) = self.panel.filter(|p| is_menu(*p)) else {
            return;
        };
        let entries = menu_entries(panel);
        let mut at = units(self.menu_index) + step;
        while at >= 0 {
            let next = index(at);
            if next >= entries.len() {
                return;
            }
            let (id, _, _) = entries[next];
            if !menu_entry_disabled(self, id) {
                self.menu_index = next;
                return;
            }
            at += step;
        }
    }

    /// Runs the highlighted dropdown entry through the same path a click takes.
    pub(crate) fn run_menu_entry(&mut self) {
        let Some(panel) = self.panel.filter(|p| is_menu(*p)) else {
            return;
        };
        let Some((id, _, _)) = menu_entries(panel).get(self.menu_index).copied() else {
            return;
        };
        if !menu_entry_disabled(self, id) {
            let anchor = self.panel_anchor;
            self.activate(Action::MenuEntry(id), anchor);
        }
    }

    pub const fn close_panel(&mut self) {
        self.panel = None;
    }

    pub(crate) fn activate(&mut self, action: Action, x: i32) {
        match action {
            Action::Panel(panel) => {
                let anchor = self.anchor_for(panel, x);
                self.toggle_panel(panel, anchor);
            }
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
            ItemId::Undo => {
                self.undo();
                self.close_panel();
            }
            ItemId::Redo => {
                self.redo();
                self.close_panel();
            }
            ItemId::Copy => {
                self.copy_selection(false);
                self.close_panel();
            }
            ItemId::Cut => {
                self.copy_selection(true);
                self.close_panel();
            }
            ItemId::Recenter => {
                self.recenter();
                self.close_panel();
            }
            ItemId::Paste => {
                if let Some(text) = self.clipboard.paste() {
                    self.paste_text(&text);
                }
                self.close_panel();
            }
            ItemId::Tool(tool) => {
                self.set_tool(tool);
                self.panel = None;
            }
            other => {
                let Some(panel) = panel_for(other) else {
                    return;
                };
                // A panel opened from an open dropdown stays under that dropdown;
                // otherwise it lands on whatever was clicked.
                let anchor = if self.panel.is_some_and(is_menu) {
                    self.panel_anchor
                } else {
                    anchor_x
                };
                self.toggle_panel(panel, anchor);
            }
        }
    }

    pub(crate) fn copy_selection(&mut self, cut: bool) {
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

    pub(crate) fn paste_text(&mut self, text: &str) {
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

    pub(crate) fn submit_dialog(&mut self) {
        let Some(dialog) = self.dialog.clone() else {
            return;
        };
        match dialog {
            Dialog::Confirm(_) => {
                self.dialog = None;
                self.confirm_delete();
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
            Ok((name, layer)) => self.switch_to(OpenDrawing {
                path: path.to_path_buf(),
                name,
                layer,
            }),
            Err(e) => self.toast(&format!("can't open: {e}")),
        }
    }

    fn switch_to(&mut self, d: OpenDrawing) {
        self.editor.cancel_gesture();
        self.editor.flush();
        if self.save.dirty || self.save.due.is_some() {
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
        self.open_dialog(Dialog::Confirm(ConfirmDialog::delete_drawing(
            &self.drawing.name,
        )));
    }

    fn confirm_delete(&mut self) {
        // Settle pending edits first so autosave can't recreate the file.
        self.editor.cancel_gesture();
        self.editor.flush();
        self.save.due = None;
        self.save.dirty = false;
        self.store.delete(&self.drawing.path);
        if let Some(next) = self.store.list().into_iter().next() {
            if let Ok((name, layer)) = self.store.load(&next.path) {
                self.switch_to(OpenDrawing {
                    path: next.path,
                    name,
                    layer,
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

    pub(crate) fn show_placing(&mut self, at: Pos) {
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
                match self
                    .store
                    .rename(&drawing.path, value, &self.editor.canvas.committed)
                {
                    Ok(path) => {
                        self.drawing.path.clone_from(&path);
                        self.drawing.name = value.to_string();
                        self.save.dirty = false;
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
                match self.store.save(&path, value, &layer) {
                    Ok(()) => {
                        self.switch_to(OpenDrawing {
                            path,
                            name: value.to_string(),
                            layer,
                        });
                        self.toast(&format!("forked to {value}"));
                        None
                    }
                    Err(e) => Some(format!("can't write: {e}")),
                }
            }
            InputKind::ImportText => {
                let path = expand_path(value);
                std::fs::read_to_string(&path).map_or_else(
                    |_| Some("can't read that file".to_string()),
                    |text| {
                        self.start_placing(&text);
                        None
                    },
                )
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

    pub const fn recenter(&mut self) {
        self.recenter_soon = true;
    }
}

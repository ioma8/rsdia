//! What the app does once a click, key or dialog lands: panels, dialogs,
//! files, export and settings.

use super::*;

impl App {
    pub fn undo(&mut self) {
        self.editor.undo();
    }

    pub fn redo(&mut self) {
        self.editor.redo();
    }

    pub fn set_tool(&mut self, id: ToolId) {
        self.editor.set_tool(id);
        self.hover_is_target = false;
        self.chips_until = Instant::now() + Duration::from_millis(CHIP_MS);
    }

    pub(crate) fn anchor_of(&self, id: ItemId) -> i32 {
        layout_toolbar(self.width)
            .items
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.x)
            .unwrap_or(1)
    }

    pub(crate) fn toggle_panel(&mut self, id: PanelId, anchor_x: i32) {
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

    pub(crate) fn activate(&mut self, action: Action, x: i32) {
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
                match self.store.save(&path, value, &layer) {
                    Ok(_) => {
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

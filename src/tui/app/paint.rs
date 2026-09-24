//! Drawing a frame: the whole screen, then the status line.

use super::*;

impl App {
    pub fn paint(&mut self, buf: &mut Buffer) {
        let area = buf.area();
        self.width = area.width as i32;
        self.height = area.height as i32;
        let layout = layout_toolbar(self.width, self.height);
        // The canvas strip between the menu bar and the floating picker: the drawing
        // is centred inside it and every overlay is confined to it, so neither the
        // bar nor the picker can be covered by a drawing or a panel.
        let strip = Rect {
            x: 0,
            y: MENU_Y + 1,
            w: self.width,
            h: (layout.y - MENU_Y - 1).max(1),
        };
        if self.recenter_soon {
            self.recenter_soon = false;
            self.viewport.recenter(&self.editor, strip);
        }
        let mut p = Painter::new(buf, self.pal, self.hover);
        let hover_cell = self.hover.map(|(x, y)| self.viewport.to_canvas(x, y));
        render_canvas(
            &mut p,
            &CanvasViewState {
                editor: &self.editor,
                viewport: self.viewport,
                grid: self.config.grid,
                top: MENU_Y + 1,
                hover_cell,
                hover_is_target: self.hover_is_target,
                cursor_on: true,
            },
        );
        render_menubar(&mut p, self);
        // The picker is painted after the overlays, so whatever a panel contains
        // cannot tear it.
        if let Some(panel) = self.panel {
            let anchor = self.panel_anchor;
            match panel {
                PanelId::Files => render_files(&mut p, self, anchor, strip),
                PanelId::Export => render_export(&mut p, self, anchor, strip),
                PanelId::Settings => render_settings(&mut p, self, anchor, strip),
                PanelId::Help => render_help(&mut p, self, anchor, strip),
                PanelId::FileMenu => render_file_menu(&mut p, self, anchor, strip),
                PanelId::EditMenu => render_edit_menu(&mut p, self, anchor, strip),
                PanelId::ViewMenu => render_view_menu(&mut p, self, anchor, strip),
                PanelId::HelpMenu => render_help_menu(&mut p, self, anchor, strip),
            }
        }
        render_toolbar(&mut p, self, &layout);
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
            pal.status_bg,
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
        // A transient notice takes the hint's place while it lasts.
        let now = Instant::now();
        let message = self
            .toast
            .as_ref()
            .filter(|(_, until)| now < *until)
            .map(|(text, _)| text.as_str());
        let right = format!(
            "{}{}  ·  rsdia",
            self.drawing.name,
            if self.dirty { " •" } else { "" }
        );
        let rx = 0.max(self.width - right.chars().count() as i32 - 1);
        let text = message.unwrap_or(hint);
        p.text_clipped(
            1,
            y,
            text,
            pal.status_fg,
            pal.status_bg,
            Modifier::empty(),
            rx - 2,
        );
        p.text(rx, y, &right, pal.status_fg, pal.status_bg);
        p.chrome.push(Rect {
            x: 0,
            y,
            w: self.width,
            h: 1,
        });
    }
}

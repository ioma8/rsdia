//! Drawing a frame: the whole screen, then the status line.

use super::*;

impl App {
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
                hover_is_target: self.hover_is_target,
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
}

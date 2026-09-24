//! Drawing a frame: the whole screen, then the status line.

use super::{tool_hint, App, MIN_HEIGHT, MIN_WIDTH};
use crate::core::editor::ToolId;
use crate::core::vector::wide;
use crate::tui::canvas_view::{render_canvas, CanvasViewState};
use crate::tui::painter::{from_area, Painter, PanelId, Rect};
use crate::tui::popovers::{
    render_dialog, render_edit_menu, render_export, render_file_menu, render_files, render_help,
    render_help_menu, render_settings, render_view_menu,
};
use crate::tui::toolbar::{layout_toolbar, render_menubar, render_toolbar, tool_color, MENU_Y};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::time::Instant;

impl App {
    pub fn paint(&mut self, buf: &mut Buffer) {
        let area = buf.area();
        self.width = i32::from(area.width);
        self.height = i32::from(area.height);
        self.hotspots.clear();
        self.chrome.clear();
        if self.width < MIN_WIDTH || self.height < MIN_HEIGHT {
            // Below this the bar, the canvas and the picker cannot all fit, and
            // drawing them anyway leaves torn borders. Say so instead.
            let pal = self.pal;
            let mut p = Painter::new(buf, pal, None);
            let line = Line::from(format!(
                "terminal too small — need at least {MIN_WIDTH}x{MIN_HEIGHT}"
            ))
            .centered()
            .style(Style::new().fg(pal.warning).bg(pal.bg));
            let r = Rect {
                x: 0,
                y: self.height / 2,
                w: self.width,
                h: 1,
            };
            p.paragraph(r, line, pal.bg);
            return;
        }
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
        let mut p = Painter::new(buf, self.pal, self.pointer.at);
        let hover_cell = self.pointer.at.map(|(x, y)| self.viewport.to_canvas(x, y));
        render_canvas(
            &mut p,
            &CanvasViewState {
                editor: &self.editor,
                viewport: self.viewport,
                grid: self.config.grid,
                top: MENU_Y + 1,
                hover_cell,
                hover_is_target: self.pointer.on_target,
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

    fn paint_status(&self, p: &mut Painter) {
        let pal = p.pal;
        let y = self.height - 1;
        let row = Rect {
            x: 0,
            y,
            w: self.width,
            h: 1,
        };
        p.fill(row, pal.status_bg);
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
            "{}{} · rsdia",
            self.drawing.name,
            if self.save.dirty { " •" } else { "" }
        );
        // Two spans that divide the row between them: the hint grows or shrinks with
        // the terminal, the name keeps its width. `Layout` is the idiomatic split.
        let [left, right_area] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(wide(right.chars().count()) + 2),
        ])
        .areas(p.area(row));
        let tool = self.tool();
        p.paragraph(
            from_area(left),
            Line::from(vec![
                Span::styled(
                    format!(" {} ", tool.name()),
                    Style::new()
                        .fg(pal.status_bg)
                        .bg(tool_color(&pal, tool))
                        .bold(),
                ),
                Span::styled(
                    format!(" {} ", message.unwrap_or(hint)),
                    Style::new().fg(pal.status_fg),
                ),
                Span::styled("· ?: help", Style::new().fg(pal.muted)),
            ]),
            pal.status_bg,
        );
        p.paragraph(
            from_area(right_area),
            Line::from(format!("{right} "))
                .right_aligned()
                .style(Style::new().fg(pal.status_fg)),
            pal.status_bg,
        );
        p.chrome.push(Rect {
            x: 0,
            y,
            w: self.width,
            h: 1,
        });
    }
}

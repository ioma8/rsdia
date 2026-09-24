//! Drawing helpers over a ratatui buffer, plus click hotspots.
//!
//! ratatui emits no click event either, so a click is a down and an up on the same
//! hotspot — the same rule the OpenTUI front end used.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};

use crate::core::editor::ToolId;
use crate::core::export::{Charset, Wrapper};
use crate::storage::config::GridStyle;
use crate::tui::theme::{Palette, ThemeName};

/// What a click runs. Hotspots hold data, not closures: the app interprets the
/// action after the frame, so painting never borrows the app mutably.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Panel(PanelId),
    Toolbar(ItemId),
    MenuEntry(ItemId),
    FilesOpen(usize),
    FilesNew,
    FilesRename,
    FilesFork,
    FilesImport,
    FilesClear,
    FilesDelete,
    ExportCharset(Charset),
    ExportFence,
    ExportWrapper(Wrapper),
    ExportCopy,
    ExportSave,
    SettingsGrid(GridStyle),
    SettingsTheme(ThemeName),
    SettingsCopyOnSelect,
    SettingsRecenter,
    DialogOk,
    DialogCancel,
}

/// A toolbar entry. `quit` has no place on the bar; it is a menu-only entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemId {
    Menu,
    Files,
    Export,
    Undo,
    Redo,
    Settings,
    Help,
    Quit,
    Copy,
    Cut,
    Paste,
    Recenter,
    Tool(ToolId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelId {
    Files,
    Export,
    Settings,
    Help,
    FileMenu,
    EditMenu,
    ViewMenu,
    HelpMenu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn in_rect(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hotspot {
    pub action: Action,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub fn hotspot_at(hotspots: &[Hotspot], x: i32, y: i32) -> Option<Action> {
    // Later hotspots are drawn on top.
    hotspots
        .iter()
        .rev()
        .find(|h| {
            in_rect(
                Rect {
                    x: h.x,
                    y: h.y,
                    w: h.w,
                    h: h.h,
                },
                x,
                y,
            )
        })
        .map(|h| h.action)
}

/// Button styling. A `Btn::default()` is a plain, enabled label.
#[derive(Clone, Copy, Debug, Default)]
pub struct Btn {
    pub fg: Option<Color>,
    pub active: bool,
    pub active_color: Option<Color>,
    pub disabled: bool,
    /// Highlighted by keyboard navigation: drawn as the inverse of the strip, the
    /// way the selected menu-bar entry is.
    pub selected: bool,
}

impl Btn {
    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn active(mut self, yes: bool) -> Self {
        self.active = yes;
        self
    }

    pub fn active_color(mut self, c: Color) -> Self {
        self.active = true;
        self.active_color = Some(c);
        self
    }

    pub fn disabled(mut self, yes: bool) -> Self {
        self.disabled = yes;
        self
    }
}

pub struct Painter<'a> {
    buf: &'a mut Buffer,
    pub pal: Palette,
    pub width: i32,
    pub height: i32,
    /// Pointer position, for hover styling.
    pub hover: Option<(i32, i32)>,
    pub hotspots: Vec<Hotspot>,
    /// Screen areas covered by UI chrome; canvas drags may not start there.
    pub chrome: Vec<Rect>,
}

impl<'a> Painter<'a> {
    pub fn new(buf: &'a mut Buffer, pal: Palette, hover: Option<(i32, i32)>) -> Self {
        let (width, height) = {
            let area = buf.area();
            (area.width as i32, area.height as i32)
        };
        Self {
            buf,
            pal,
            width,
            height,
            hover,
            hotspots: Vec::new(),
            chrome: Vec::new(),
        }
    }

    pub fn cell(&mut self, x: i32, y: i32, sym: &str, fg: Color, bg: Color, modifier: Modifier) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        if let Some(cell) = self.buf.cell_mut((x as u16, y as u16)) {
            cell.set_symbol(sym)
                .set_style(Style::default().fg(fg).bg(bg).add_modifier(modifier));
        }
    }

    /// Writes single-width text, clipped to the screen. Returns the x after the text.
    pub fn text(&mut self, x: i32, y: i32, s: &str, fg: Color, bg: Color) -> i32 {
        self.text_clipped(x, y, s, fg, bg, Modifier::empty(), self.width)
    }

    // Immediate-mode cell writer: the parameters are the buffer's own
    // (x, y, text, fg, bg, modifier, clip).
    #[allow(clippy::too_many_arguments)]
    pub fn text_clipped(
        &mut self,
        mut x: i32,
        y: i32,
        s: &str,
        fg: Color,
        bg: Color,
        modifier: Modifier,
        max_x: i32,
    ) -> i32 {
        for ch in s.chars() {
            if x >= max_x {
                break;
            }
            self.cell(x, y, &ch.to_string(), fg, bg, modifier);
            x += 1;
        }
        x
    }

    pub fn fill(&mut self, r: Rect, bg: Color) {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                self.cell(x, y, " ", bg, bg, Modifier::empty());
            }
        }
    }

    /// Bordered panel; registers the area as chrome.
    pub fn panel(&mut self, r: Rect, border: Color, bg: Color) {
        self.fill(r, bg);
        let x1 = r.x + r.w - 1;
        let y1 = r.y + r.h - 1;
        for x in r.x + 1..x1 {
            self.cell(x, r.y, "─", border, bg, Modifier::empty());
            self.cell(x, y1, "─", border, bg, Modifier::empty());
        }
        for y in r.y + 1..y1 {
            self.cell(r.x, y, "│", border, bg, Modifier::empty());
            self.cell(x1, y, "│", border, bg, Modifier::empty());
        }
        self.cell(r.x, r.y, "┌", border, bg, Modifier::empty());
        self.cell(x1, r.y, "┐", border, bg, Modifier::empty());
        self.cell(r.x, y1, "└", border, bg, Modifier::empty());
        self.cell(x1, y1, "┘", border, bg, Modifier::empty());
        self.chrome.push(r);
    }

    pub fn is_hover(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.hover
            .is_some_and(|(hx, hy)| in_rect(Rect { x, y, w, h }, hx, hy))
    }

    /// A clickable label. Hover brightens it; `active` bolds it in its own color.
    /// Returns the x after the label.
    pub fn button(&mut self, action: Action, x: i32, y: i32, label: &str, b: Btn) -> i32 {
        let w = label.chars().count() as i32;
        let mut bg = self.pal.tb_bg;
        let mut fg = b.fg.unwrap_or(self.pal.tb_label);
        let mut modifier = Modifier::empty();
        let hovered = !b.selected && !b.disabled && self.is_hover(x, y, w, 1);
        if b.disabled {
            fg = self.pal.disabled;
        } else if b.selected {
            fg = self.pal.menu_active_fg;
            bg = self.pal.menu_active_bg;
            modifier = Modifier::BOLD;
        } else if b.active {
            fg = b.active_color.unwrap_or(self.pal.text);
            modifier = Modifier::BOLD;
        }
        if hovered {
            fg = match b.fg {
                Some(c) if !b.active => c,
                _ => self.pal.tb_hover,
            };
            bg = self.pal.tb_hover_bg;
        }
        let end = self.text_clipped(x, y, label, fg, bg, modifier, self.width);
        if !b.disabled {
            self.hotspots.push(Hotspot {
                action,
                x,
                y,
                w,
                h: 1,
            });
        }
        end
    }
}

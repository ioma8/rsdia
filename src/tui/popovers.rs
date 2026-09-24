//! The overlay panels: files, export, settings, help, the narrow menu, and the
//! modal input/confirm dialog.

use ratatui::style::{Color, Modifier};

use crate::core::editor::ToolId;
use crate::core::export::{Charset, WRAPPERS};
use crate::storage::config::GRID_STYLES;
use crate::tui::host::{Dialog, Host};
use crate::tui::painter::{from_area, to_area, Action, Btn, ItemId, Painter, PanelId, Rect};
use crate::tui::theme::{Palette, ThemeName, THEME_CHOICES};
use crate::tui::toolbar::tool_color;
use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, ListItem, Row, Table, Widget};

// ------------------------------------------------------------------ common

/// Places a dropdown or panel under its anchor, clamped inside `area` — the strip
/// between the menu bar and the floating tool picker. Panels therefore never
/// overlap the picker, whatever their contents.
pub fn place_popover(anchor_x: i32, area: Rect, w: i32, h: i32) -> Rect {
    let want = Rect {
        x: (anchor_x - 2).max(area.x),
        y: area.y,
        w: w.max(1),
        h: h.max(1),
    };
    // `Rect::clamp` shrinks and repositions in one step, which is all a popover needs.
    from_area(to_area(want).clamp(to_area(area)))
}

/// Horizontal rule inside a panel.
pub fn rule(p: &mut Painter, r: Rect, y: i32) {
    let pal = p.pal;
    for x in r.x + 1..r.x + r.w - 1 {
        p.cell(x, y, "─", pal.tb_border, pal.tb_bg, Modifier::empty());
    }
    p.cell(r.x, y, "├", pal.tb_border, pal.tb_bg, Modifier::empty());
    p.cell(
        r.x + r.w - 1,
        y,
        "┤",
        pal.tb_border,
        pal.tb_bg,
        Modifier::empty(),
    );
}

pub struct FlowButton {
    pub action: Action,
    pub label: String,
    pub active: bool,
    pub fg: Option<Color>,
}

impl FlowButton {
    pub fn new(action: Action, label: impl Into<String>) -> Self {
        Self {
            action,
            label: label.into(),
            active: false,
            fg: None,
        }
    }

    pub fn active(mut self, yes: bool) -> Self {
        self.active = yes;
        self
    }

    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }
}

/// Lays out buttons left to right, wrapping inside `r`. Returns the next free row.
pub fn flow(p: &mut Painter, r: Rect, mut y: i32, buttons: &[FlowButton], lead: &str) -> i32 {
    let pal = p.pal;
    let left = r.x + 2;
    let right = r.x + r.w - 2;
    let mut x = left;
    if !lead.is_empty() {
        x = p.text(x, y, lead, pal.muted, pal.tb_bg);
    }
    let indent = x;
    for b in buttons {
        let len = b.label.chars().count() as i32;
        if x + len > right && x > indent {
            y += 1;
            x = indent;
        }
        let style = Btn {
            active: b.active,
            active_color: Some(pal.accent),
            fg: b.fg,
            ..Default::default()
        };
        x = p.button(b.action, x, y, &b.label, style) + 2;
    }
    y + 1
}

/// Height needed by [`flow`] for the same inputs.
pub fn flow_height(width: i32, labels: &[&str], lead: &str) -> i32 {
    let inner = width - 4 - lead.chars().count() as i32;
    let mut rows = 1;
    let mut x = 0;
    for l in labels {
        let len = l.chars().count() as i32;
        if x + len > inner && x > 0 {
            rows += 1;
            x = 0;
        }
        x += len + 2;
    }
    rows
}

fn truncate_end(name: &str, width: i32) -> String {
    let count = name.chars().count() as i32;
    if width <= 0 {
        String::new()
    } else if count <= width {
        name.to_string()
    } else {
        let mut out: String = name.chars().take((width - 1).max(0) as usize).collect();
        out.push('…');
        out
    }
}

// ------------------------------------------------------------------ files

const FILES_WIDTH: i32 = 50;

/// `files` popover: drawing list plus new / rename / fork / clear / delete / import.
pub fn render_files(p: &mut Painter, host: &dyn Host, anchor_x: i32, area: Rect) {
    let pal = p.pal;
    let actions = [
        FlowButton::new(Action::FilesNew, "[new]").fg(pal.accent),
        FlowButton::new(Action::FilesRename, "[rename]"),
        FlowButton::new(Action::FilesFork, "[fork]"),
        FlowButton::new(Action::FilesImport, "[import]"),
        FlowButton::new(Action::FilesClear, "[clear]").fg(pal.warning),
        FlowButton::new(Action::FilesDelete, "[delete]").fg(pal.danger),
    ];
    let labels: Vec<&str> = actions.iter().map(|a| a.label.as_str()).collect();
    let actions_rows = flow_height(FILES_WIDTH, &labels, "");
    let all = host.drawings();
    // Two borders + the list + the rule + the button rows must fit the strip exactly.
    let room = (area.h - 3 - actions_rows).max(1);
    let list_rows = (all.len() as i32).clamp(1, room);
    let selected = host.list_selection().filter(|i| *i < all.len());
    let footer = format!("{}/{}", selected.map_or(0, |i| i + 1), all.len());
    let h = 2 + list_rows + 1 + actions_rows;
    let r = place_popover(anchor_x, area, FILES_WIDTH, h);
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Files"), Some(&footer));

    let list = Rect {
        x: inner.x,
        y: inner.y,
        w: inner.w,
        h: list_rows,
    };
    if all.is_empty() {
        p.text(
            inner.x + 1,
            inner.y,
            "no drawings yet",
            pal.muted,
            pal.tb_bg,
        );
    } else {
        let size_w = all
            .iter()
            .map(|d| d.size.to_string().len())
            .max()
            .unwrap_or(1);
        let name_w = (list.w - size_w as i32 - 8).max(4);

        let current = host.current_path().to_path_buf();
        let items: Vec<ListItem> = all
            .iter()
            .map(|d| {
                let label = format!(
                    "{} {:<width$} {:>size_w$} cells",
                    if d.path == current { ">" } else { " " },
                    truncate_end(&d.name, name_w),
                    d.size,
                    width = name_w as usize
                );
                let fg = if d.path == current {
                    pal.accent
                } else {
                    pal.text
                };
                ListItem::new(Line::from(label).style(Style::new().fg(fg)))
            })
            .collect();
        p.list(list, items, selected);
        // A click anywhere on the row opens that drawing.
        for i in 0..all.len().min(list_rows as usize) {
            let row = Rect {
                x: list.x,
                y: list.y + i as i32,
                w: list.w,
                h: 1,
            };
            p.hotspot(Action::FilesOpen(i), row);
        }
    }

    let rule_y = inner.y + list_rows;
    rule(p, r, rule_y);
    flow(p, r, rule_y + 1, &actions, "");
}

// ------------------------------------------------------------------ export

/// `export` dialog: charset, comment wrapper, fence, preview, copy / save.
pub fn render_export(p: &mut Painter, host: &dyn Host, anchor_x: i32, area: Rect) {
    let pal = p.pal;
    let cfg = host.config().export;
    let preview: Vec<String> = host
        .export_preview()
        .split('\n')
        .map(|l| l.to_string())
        .collect();
    let preview_width = preview.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
    let w = p.width.min(64.max(preview_width + 4));
    let wrap_labels: Vec<&str> = WRAPPERS.iter().map(|(_, _, label)| *label).collect();
    let wrap_rows = flow_height(w, &wrap_labels, "wrap:       ");
    let chrome = wrap_rows + 6;
    let preview_rows = (area.h - chrome).max(1);
    let top = host.preview_top().min(preview.len().saturating_sub(1));
    let h = chrome + preview_rows;
    let r = place_popover(anchor_x, area, w, h);
    let footer = format!("{}/{} lines", (top + 1).min(preview.len()), preview.len());
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Export"), Some(&footer));

    let charset = [
        FlowButton::new(Action::ExportCharset(Charset::Extended), "extended")
            .active(cfg.characters == Charset::Extended),
        FlowButton::new(Action::ExportCharset(Charset::Basic), "basic")
            .active(cfg.characters == Charset::Basic),
        FlowButton::new(
            Action::ExportFence,
            if cfg.fenced {
                "[x] markdown fence"
            } else {
                "[ ] markdown fence"
            },
        )
        .active(cfg.fenced),
    ];
    let wrappers: Vec<FlowButton> = WRAPPERS
        .iter()
        .map(|(id, _, label)| {
            FlowButton::new(Action::ExportWrapper(*id), *label).active(cfg.wrapper == *id)
        })
        .collect();

    let mut y = inner.y;
    y = flow(p, r, y, &charset, "characters: ");
    y = flow(p, r, y, &wrappers, "wrap:       ");
    rule(p, r, y);
    y += 1;
    let lines: Vec<Line> = preview
        .iter()
        .skip(top)
        .take(preview_rows as usize)
        .map(|l| Line::from(format!(" {l}")).style(Style::new().fg(pal.fg)))
        .collect();
    p.list(
        Rect {
            x: inner.x,
            y,
            w: inner.w,
            h: preview_rows,
        },
        lines.into_iter().map(ListItem::new).collect(),
        None,
    );
    y += preview_rows;
    rule(p, r, y);
    y += 1;
    let x = p.button(
        Action::ExportCopy,
        inner.x + 1,
        y,
        "[copy to clipboard]",
        Btn::default().fg(pal.success),
    ) + 2;
    p.button(
        Action::ExportSave,
        x,
        y,
        "[save…]",
        Btn::default().fg(pal.accent),
    );
}

// ------------------------------------------------------------------ settings

const SETTINGS_WIDTH: i32 = 58;

fn settings_height() -> i32 {
    let themes: Vec<&str> = THEME_CHOICES.iter().map(|t| t.name()).collect();
    2 + 1 + flow_height(SETTINGS_WIDTH, &themes, "theme: ") + 1 + 1
}

/// `settings` popover: grid style, theme, copy-on-select, recenter.
pub fn render_settings(p: &mut Painter, host: &dyn Host, anchor_x: i32, area: Rect) {
    let r = place_popover(anchor_x, area, SETTINGS_WIDTH, settings_height());
    let pal = p.pal;
    p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Settings"), None);
    let cfg = host.config().clone();
    let mut y = r.y + 1;
    let grids: Vec<FlowButton> = GRID_STYLES
        .iter()
        .map(|g| FlowButton::new(Action::SettingsGrid(*g), g.name()).active(cfg.grid == *g))
        .collect();
    y = flow(p, r, y, &grids, "grid:  ");
    let themes: Vec<FlowButton> = THEME_CHOICES
        .iter()
        .map(|t| FlowButton::new(Action::SettingsTheme(*t), t.name()).active(cfg.theme == *t))
        .collect();
    y = flow(p, r, y, &themes, "theme: ");
    let copy = [FlowButton::new(
        Action::SettingsCopyOnSelect,
        if cfg.copy_on_select { "on" } else { "off" },
    )
    .active(cfg.copy_on_select)];
    y = flow(p, r, y, &copy, "copy on select: ");
    p.button(
        Action::SettingsRecenter,
        r.x + 2,
        y,
        "[recenter]",
        Btn::default().fg(pal.orange),
    );
}

// ------------------------------------------------------------------ help

pub const TOOL_HELP: [(ToolId, &str); 6] = [
    (ToolId::Box, "drag corner to corner"),
    (
        ToolId::Select,
        "drag to move boxes, words and selections; drag a line to resize, a line end to reshape",
    ),
    (
        ToolId::Arrow,
        "drag start to end. press f to flip the elbow",
    ),
    (ToolId::Line, "drag start to end. press f to flip the elbow"),
    (
        ToolId::Text,
        "click and type. enter: new line, esc: finish, arrows move",
    ),
    (ToolId::Eraser, "drag to erase"),
];

const SHORTCUTS: [(&str, &str); 14] = [
    (
        "1-6 / alt+1-6",
        "switch tool (box select arrow line text eraser)",
    ),
    ("r v a l t e", "switch tool by letter"),
    (
        "ctrl+z  ctrl+y",
        "undo / redo (ctrl+shift+z with kitty keys)",
    ),
    ("y  x  p  (ctrl+x ctrl+v)", "copy / cut / paste selection"),
    ("del  arrows", "erase / nudge selection"),
    ("f", "flip elbow while dragging a line or arrow"),
    (
        "scroll",
        "pan the canvas, or scroll the list an open panel shows",
    ),
    ("space+drag  middle-drag", "pan freely"),
    ("ctrl+o  ctrl+e  ctrl+s", "files / export / save now"),
    ("?  esc", "help / close popover, cancel, deselect"),
    (
        "j k  arrows  enter",
        "walk the drawings list and open the one selected",
    ),
    ("alt+f e v h", "open the File, Edit, View or Help menu"),
    (
        "arrows  enter",
        "walk the menu bar and its items, enter runs one",
    ),
    ("ctrl+c  ctrl+q", "quit (the drawing is saved first)"),
];

/// `help` popover: active-tool help and the shortcut table.
pub fn render_help(p: &mut Painter, host: &dyn Host, anchor_x: i32, area: Rect) {
    let pal = p.pal;
    let w = area.w.min(76);
    let h = 2 + 1 + 1 + SHORTCUTS.len() as i32 + 1;
    let r = place_popover(anchor_x, area, w, h);
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Help"), None);
    let tool = host.editor().tool();
    let help = TOOL_HELP
        .iter()
        .find(|(t, _)| *t == tool)
        .map(|(_, h)| *h)
        .unwrap_or("");
    p.paragraph(
        Rect {
            x: inner.x,
            y: inner.y,
            w: inner.w,
            h: 1,
        },
        Line::from(vec![
            Span::styled(
                format!("{}: ", tool.name()),
                Style::new().fg(tool_color(&pal, tool)).bold(),
            ),
            Span::styled(help, Style::new().fg(pal.text)),
        ]),
        pal.tb_bg,
    );
    let rule_y = inner.y + 1;
    rule(p, r, rule_y);
    // A table gives each column its own width and clips what does not fit.
    let rows: Vec<Row> = SHORTCUTS
        .iter()
        .map(|(keys, what)| {
            Row::new(vec![
                Cell::from(Line::from(*keys).style(Style::new().fg(pal.accent))),
                Cell::from(Line::from(*what).style(Style::new().fg(pal.text))),
            ])
        })
        .collect();
    let key_w = SHORTCUTS
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let table = Table::new(
        rows,
        [Constraint::Length(key_w as u16), Constraint::Min(20)],
    )
    .column_spacing(2)
    .style(Style::new().bg(pal.tb_bg));
    let table_area = Rect {
        x: inner.x,
        y: rule_y + 1,
        w: inner.w,
        h: (inner.y + inner.h - rule_y - 1).max(0),
    };
    table.render(p.area(table_area), p.buf_mut());
}

// ------------------------------------------------------------------ menus

const FILE_MENU: [(ItemId, &str, &str); 3] = [
    (ItemId::Files, "Drawings…", "Ctrl+O"),
    (ItemId::Export, "Export…", "Ctrl+E"),
    (ItemId::Quit, "Quit", "Ctrl+Q"),
];
const EDIT_MENU: [(ItemId, &str, &str); 5] = [
    (ItemId::Undo, "Undo", "Ctrl+Z"),
    (ItemId::Redo, "Redo", "Ctrl+Y"),
    (ItemId::Cut, "Cut", "X"),
    (ItemId::Copy, "Copy", "Y"),
    (ItemId::Paste, "Paste", "P"),
];
const VIEW_MENU: [(ItemId, &str, &str); 2] = [
    (ItemId::Settings, "Settings…", ""),
    (ItemId::Recenter, "Recenter canvas", ""),
];
const HELP_MENU: [(ItemId, &str, &str); 1] = [(ItemId::Help, "Keyboard shortcuts", "?")];

/// The entries of a menu-bar dropdown, in display order. Keyboard navigation and
/// the renderer read the same list, so they cannot drift apart.
pub fn menu_entries(panel: PanelId) -> &'static [(ItemId, &'static str, &'static str)] {
    match panel {
        PanelId::FileMenu => &FILE_MENU,
        PanelId::EditMenu => &EDIT_MENU,
        PanelId::ViewMenu => &VIEW_MENU,
        PanelId::HelpMenu => &HELP_MENU,
        _ => &[],
    }
}

/// Whether an entry has nothing to act on. Greying and keyboard navigation both
/// use this, so a skipped row is always a greyed row.
pub fn menu_entry_disabled(host: &dyn Host, id: ItemId) -> bool {
    match id {
        ItemId::Undo => !host.can_undo(),
        ItemId::Redo => !host.can_redo(),
        ItemId::Cut | ItemId::Copy => !host.has_selection(),
        _ => false,
    }
}

pub fn render_file_menu(p: &mut Painter, host: &dyn Host, x: i32, area: Rect) {
    render_dropdown(p, host, x, area, &FILE_MENU);
}

pub fn render_edit_menu(p: &mut Painter, host: &dyn Host, x: i32, area: Rect) {
    render_dropdown(p, host, x, area, &EDIT_MENU);
}

pub fn render_view_menu(p: &mut Painter, host: &dyn Host, x: i32, area: Rect) {
    render_dropdown(p, host, x, area, &VIEW_MENU);
}

pub fn render_help_menu(p: &mut Painter, host: &dyn Host, x: i32, area: Rect) {
    render_dropdown(p, host, x, area, &HELP_MENU);
}

fn render_dropdown(
    p: &mut Painter,
    host: &dyn Host,
    anchor_x: i32,
    area: Rect,
    entries: &[(ItemId, &str, &str)],
) {
    let pal = p.pal;
    let label_w = entries
        .iter()
        .map(|(_, label, _)| label.len())
        .max()
        .unwrap_or(0);
    let key_w = entries
        .iter()
        .map(|(_, _, key)| key.len())
        .max()
        .unwrap_or(0);
    let width = (label_w + key_w + 5) as i32;
    let r = place_popover(anchor_x, area, width, entries.len() as i32 + 2);
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, None, None);
    // A narrow terminal clamps the panel, so the label gives way first: it is
    // shortened with an ellipsis and the accelerator column is kept whole.
    let room = (inner.w - key_w as i32 - 3).max(1) as usize;
    let cursor = Some(host.menu_index()).filter(|i| {
        entries
            .get(*i)
            .is_some_and(|(id, _, _)| !menu_entry_disabled(host, *id))
    });
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, (id, label, key))| {
            let fg = match id {
                ItemId::Undo => pal.undo,
                ItemId::Redo => pal.redo,
                ItemId::Quit => pal.danger,
                _ => pal.text,
            };
            let text: String = format!(
                " {:<room$} {:>key_w$} ",
                truncate_end(label, room as i32),
                key
            )
            .chars()
            .take(inner.w.max(0) as usize)
            .collect();
            let mut style = Style::new().fg(if menu_entry_disabled(host, *id) {
                pal.disabled
            } else {
                fg
            });
            let row = Rect {
                x: inner.x,
                y: inner.y + i as i32,
                w: inner.w,
                h: 1,
            };
            if p.is_hover(row.x, row.y, row.w, row.h) {
                style = Style::new().fg(pal.tb_hover).bg(pal.tb_hover_bg);
            }
            ListItem::new(Line::from(text).style(style))
        })
        .collect();
    p.list(inner, items, cursor);
    for (i, (id, _, _)) in entries.iter().enumerate() {
        let row = Rect {
            x: inner.x,
            y: inner.y + i as i32,
            w: inner.w,
            h: 1,
        };
        if !menu_entry_disabled(host, *id) {
            p.hotspot(Action::MenuEntry(*id), row);
        }
    }
}

// ------------------------------------------------------------------ dialog

/// Modal input and confirm dialogs, centered on screen.
pub fn render_dialog(p: &mut Painter, d: &Dialog) {
    let pal = p.pal;
    let w = p.width.min(56);
    let h = if d.input().is_some() { 7 } else { 6 };
    // A modal sits dead centre, whatever the screen size.
    let r = from_area(
        to_area(Rect {
            x: 0,
            y: 0,
            w: p.width,
            h: p.height,
        })
        .centered(
            Constraint::Length(w.max(1) as u16),
            Constraint::Length(h.max(1) as u16),
        ),
    );
    // The block's title carries the prompt, so the body starts on the second row.
    p.titled_panel(r, pal.accent, pal.tb_bg, Some(d.title()), None);
    match d {
        Dialog::Input(input) => {
            let y = r.y + 2;
            let label = format!("{}: ", input.label);
            let label_end = p.text(r.x + 2, y, &label, pal.muted, pal.tb_bg);
            let field_w = r.x + r.w - 2 - label_end;
            // Scroll the field so the cursor stays visible.
            let start = 0.max(input.cursor as i32 - field_w + 1) as usize;
            let chars: Vec<char> = input.value.chars().collect();
            for i in 0..field_w {
                let index = start + i as usize;
                let ch = chars.get(index).copied().unwrap_or(' ');
                let modifier = if index == input.cursor {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                };
                p.cell(
                    label_end + i,
                    y,
                    &ch.to_string(),
                    pal.text,
                    pal.selection_bg,
                    modifier,
                );
            }
            if let Some(error) = &input.error {
                p.text_clipped(
                    r.x + 2,
                    y + 1,
                    error,
                    pal.danger,
                    pal.tb_bg,
                    Modifier::empty(),
                    r.x + r.w - 2,
                );
            }
        }
        Dialog::Confirm(confirm) => {
            p.text_clipped(
                r.x + 2,
                r.y + 2,
                &confirm.message,
                pal.text,
                pal.tb_bg,
                Modifier::empty(),
                r.x + r.w - 2,
            );
        }
    }
    let y = r.y + r.h - 2;
    let ok = match d {
        Dialog::Input(_) => "[ok]".to_string(),
        Dialog::Confirm(c) => format!("[{}]", c.yes),
    };
    let mut x = r.x + r.w - 4 - ok.chars().count() as i32 - "[cancel]".chars().count() as i32;
    let ok_fg = if matches!(d, Dialog::Confirm(_)) {
        pal.danger
    } else {
        pal.success
    };
    x = p.button(Action::DialogOk, x, y, &ok, Btn::default().fg(ok_fg)) + 2;
    p.button(Action::DialogCancel, x, y, "[cancel]", Btn::default());
}

/// Reads a palette for a theme name; free function so tests can build one without an app.
pub fn palette_for(theme: ThemeName, term: Option<&crate::tui::theme::TerminalColors>) -> Palette {
    crate::tui::theme::palette(theme, term)
}

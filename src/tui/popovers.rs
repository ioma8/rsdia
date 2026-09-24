//! The overlay panels: files, export, settings, help, the narrow menu, and the
//! modal input/confirm dialog.

use ratatui::style::{Color, Modifier};

use crate::core::editor::ToolId;
use crate::core::export::{Charset, WRAPPERS};
use crate::core::vector::{index, px, units, wide};
use crate::storage::config::GRID_STYLES;
use crate::tui::host::{Dialog, Host};
use crate::tui::painter::{from_area, to_area, Action, Btn, ItemId, Painter, PanelId, Rect};
use crate::tui::theme::THEME_CHOICES;
use crate::tui::toolbar::tool_color;
use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, ListItem, Row, Table, Widget};

// ------------------------------------------------------------------ common

/// Places a dropdown or panel under its anchor, clamped inside `area` — the strip
/// between the menu bar and the floating tool picker. Panels therefore never
/// overlap the picker, whatever their contents.
#[must_use]
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
    /// The same style a standalone button takes, so there is one way to colour one.
    pub btn: Btn,
}

impl FlowButton {
    #[must_use]
    pub fn new(action: Action, label: impl Into<String>) -> Self {
        Self {
            action,
            label: label.into(),
            btn: Btn::default(),
        }
    }

    /// `yes` marks the current choice, which is drawn bold in `accent`.
    #[must_use]
    pub const fn active(mut self, yes: bool, accent: Color) -> Self {
        self.btn.active_color = if yes { Some(accent) } else { None };
        self
    }

    #[must_use]
    pub const fn fg(mut self, c: Color) -> Self {
        self.btn.fg = Some(c);
        self
    }
}

/// Where a flow's next button goes: past the right edge means the next row.
/// Both the drawing pass and the height pass that sized the panel use this one rule.
const fn wrap(x: &mut i32, y: &mut i32, len: i32, right: i32, indent: i32) {
    if *x + len > right && *x > indent {
        *y += 1;
        *x = indent;
    }
}

/// Lays out buttons left to right, wrapping inside `r`. Returns the next free row.
pub fn flow(p: &mut Painter, r: Rect, mut y: i32, buttons: &[FlowButton], lead: &str) -> i32 {
    let pal = p.pal;
    let right = r.x + r.w - 2;
    let mut x = r.x + 2;
    if !lead.is_empty() {
        x = p.text(x, y, lead, pal.muted, pal.tb_bg);
    }
    let indent = x;
    for b in buttons {
        let len = units(b.label.chars().count());
        wrap(&mut x, &mut y, len, right, indent);
        x = p.button(b.action, x, y, &b.label, b.btn) + 2;
    }
    y + 1
}

/// Height needed by [`flow`] for the same buttons.
#[must_use]
pub fn flow_height(width: i32, buttons: &[FlowButton], lead: &str) -> i32 {
    let right = width - 4 - units(lead.chars().count());
    let (mut x, mut y) = (0, 0);
    for b in buttons {
        let len = units(b.label.chars().count());
        wrap(&mut x, &mut y, len, right, 0);
        x += len + 2;
    }
    y + 1
}

fn truncate_end(name: &str, width: i32) -> String {
    let count = units(name.chars().count());
    if width <= 0 {
        String::new()
    } else if count <= width {
        name.to_string()
    } else {
        let mut out: String = name.chars().take(index(width - 1)).collect();
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
    let actions_rows = flow_height(FILES_WIDTH, &actions, "");
    let all = host.drawings();
    // Two borders + the list + the rule + the button rows must fit the strip exactly.
    let room = (area.h - 3 - actions_rows).max(1);
    let list_rows = units(all.len().clamp(1, index(room)));
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
        let name_w = (list.w - units(size_w) - 8).max(4);

        let items: Vec<ListItem> = all
            .iter()
            .map(|d| {
                let current = d.path == host.current_path();
                let label = format!(
                    "{} {:<width$} {:>size_w$} cells",
                    if current { ">" } else { " " },
                    truncate_end(&d.name, name_w),
                    d.size,
                    width = index(name_w)
                );
                let fg = if current { pal.accent } else { pal.text };
                ListItem::new(Line::from(label).style(Style::new().fg(fg)))
            })
            .collect();
        p.list(list, items, selected);
        // A click anywhere on the row opens that drawing.
        for i in 0..all.len().min(index(list_rows)) {
            let row = Rect {
                x: list.x,
                y: list.y + units(i),
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
        .map(ToString::to_string)
        .collect();
    let preview_width = units(preview.iter().map(|l| l.chars().count()).max().unwrap_or(0));
    let width = p.width.min(64.max(preview_width + 4));
    let wrappers: Vec<FlowButton> = WRAPPERS
        .iter()
        .map(|(id, _, label)| {
            FlowButton::new(Action::ExportWrapper(*id), *label)
                .active(cfg.wrapper == *id, pal.accent)
        })
        .collect();
    let chrome = flow_height(width, &wrappers, "wrap:       ") + 6;
    let preview_rows = (area.h - chrome).max(1);
    let top = host.preview_top().min(preview.len().saturating_sub(1));
    let height = chrome + preview_rows;
    let panel = place_popover(anchor_x, area, width, height);
    let footer = format!("{}/{} lines", (top + 1).min(preview.len()), preview.len());
    let inner = p.titled_panel(
        panel,
        pal.tb_border,
        pal.tb_bg,
        Some("Export"),
        Some(&footer),
    );

    let charset = [
        FlowButton::new(Action::ExportCharset(Charset::Extended), "extended")
            .active(cfg.characters == Charset::Extended, pal.accent),
        FlowButton::new(Action::ExportCharset(Charset::Basic), "basic")
            .active(cfg.characters == Charset::Basic, pal.accent),
        FlowButton::new(
            Action::ExportFence,
            if cfg.fenced {
                "[at] markdown fence"
            } else {
                "[ ] markdown fence"
            },
        )
        .active(cfg.fenced, pal.accent),
    ];
    let mut row = inner.y;
    row = flow(p, panel, row, &charset, "characters: ");
    row = flow(p, panel, row, &wrappers, "wrap:       ");
    rule(p, panel, row);
    row += 1;
    let lines: Vec<ListItem> = preview
        .iter()
        .skip(top)
        .take(index(preview_rows))
        .map(|line| ListItem::new(Line::from(format!(" {line}")).style(Style::new().fg(pal.fg))))
        .collect();
    p.list(
        Rect {
            x: inner.x,
            y: row,
            w: inner.w,
            h: preview_rows,
        },
        lines,
        None,
    );
    row += preview_rows;
    rule(p, panel, row);
    row += 1;
    let at = p.button(
        Action::ExportCopy,
        inner.x + 1,
        row,
        "[copy to clipboard]",
        Btn::default().fg(pal.success),
    ) + 2;
    p.button(
        Action::ExportSave,
        at,
        row,
        "[save…]",
        Btn::default().fg(pal.accent),
    );
}

// ------------------------------------------------------------------ settings

const SETTINGS_WIDTH: i32 = 58;

/// `settings` popover: grid style, theme, copy-on-select, recenter.
pub fn render_settings(p: &mut Painter, host: &dyn Host, anchor_x: i32, area: Rect) {
    let pal = p.pal;
    let cfg = host.config();
    let grids: Vec<FlowButton> = GRID_STYLES
        .iter()
        .map(|g| {
            FlowButton::new(Action::SettingsGrid(*g), g.name()).active(cfg.grid == *g, pal.accent)
        })
        .collect();
    let themes: Vec<FlowButton> = THEME_CHOICES
        .iter()
        .map(|t| {
            FlowButton::new(Action::SettingsTheme(*t), t.name()).active(cfg.theme == *t, pal.accent)
        })
        .collect();
    // The panel is as tall as the rows its own buttons need.
    let h = 2 + 1 + flow_height(SETTINGS_WIDTH, &themes, "theme: ") + 1 + 1;
    let r = place_popover(anchor_x, area, SETTINGS_WIDTH, h);
    p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Settings"), None);
    let mut y = r.y + 1;
    y = flow(p, r, y, &grids, "grid:  ");
    y = flow(p, r, y, &themes, "theme: ");
    let copy = [FlowButton::new(
        Action::SettingsCopyOnSelect,
        if cfg.copy_on_select { "on" } else { "off" },
    )
    .active(cfg.copy_on_select, pal.accent)];
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

/// What the active tool does, in one line.
#[must_use]
pub const fn tool_help(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Box => "drag corner to corner",
        ToolId::Select => {
            "drag to move boxes, words and selections; drag a line to resize, a line end to reshape"
        }
        ToolId::Arrow | ToolId::Line => "drag start to end. press f to flip the elbow",
        ToolId::Text => "click and type. enter: new line, esc: finish, arrows move",
        ToolId::Eraser => "drag to erase",
    }
}

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
    let h = 2 + 1 + 1 + units(SHORTCUTS.len()) + 1;
    let r = place_popover(anchor_x, area, w, h);
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, Some("Help"), None);
    let tool = host.editor().tool();
    let help = tool_help(tool);
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
    let table = Table::new(rows, [Constraint::Length(wide(key_w)), Constraint::Min(20)])
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
#[must_use]
pub const fn menu_entries(panel: PanelId) -> &'static [(ItemId, &'static str, &'static str)] {
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
    let width = units(label_w + key_w + 5);
    let r = place_popover(anchor_x, area, width, units(entries.len()) + 2);
    let inner = p.titled_panel(r, pal.tb_border, pal.tb_bg, None, None);
    // A narrow terminal clamps the panel, so the label gives way first: it is
    // shortened with an ellipsis and the accelerator column is kept whole.
    let room = index(inner.w - units(key_w) - 3).max(1);
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
                truncate_end(label, units(room)),
                key
            )
            .chars()
            .take(index(inner.w.max(0)))
            .collect();
            let mut style = Style::new().fg(if menu_entry_disabled(host, *id) {
                pal.disabled
            } else {
                fg
            });
            let row = Rect {
                x: inner.x,
                y: inner.y + units(i),
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
            y: inner.y + units(i),
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
pub fn render_dialog(p: &mut Painter, dialog: &Dialog) {
    let pal = p.pal;
    let width = p.width.min(56);
    let height = if dialog.input().is_some() { 7 } else { 6 };
    // A modal sits dead centre, whatever the screen size.
    let panel = from_area(
        to_area(Rect {
            x: 0,
            y: 0,
            w: p.width,
            h: p.height,
        })
        .centered(
            Constraint::Length(px(width.max(1))),
            Constraint::Length(px(height.max(1))),
        ),
    );
    // The block's title carries the prompt, so the body starts on the second row.
    p.titled_panel(panel, pal.accent, pal.tb_bg, Some(dialog.title()), None);
    match dialog {
        Dialog::Input(input) => {
            let row = panel.y + 2;
            let label = format!("{}: ", input.label);
            let label_end = p.text(panel.x + 2, row, &label, pal.muted, pal.tb_bg);
            let field_w = panel.x + panel.w - 2 - label_end;
            // Scroll the field so the cursor stays visible.
            let start = index(0.max(units(input.cursor) - field_w + 1));
            let chars: Vec<char> = input.value.chars().collect();
            for i in 0..field_w {
                let index = start + index(i);
                let ch = chars.get(index).copied().unwrap_or(' ');
                let modifier = if index == input.cursor {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                };
                p.cell(
                    label_end + i,
                    row,
                    &ch.to_string(),
                    pal.text,
                    pal.selection_bg,
                    modifier,
                );
            }
            if let Some(error) = &input.error {
                let style = Style::new().fg(pal.danger).bg(pal.tb_bg);
                p.text_clipped(panel.x + 2, row + 1, error, style, panel.x + panel.w - 2);
            }
        }
        Dialog::Confirm(confirm) => {
            let style = Style::new().fg(pal.text).bg(pal.tb_bg);
            p.text_clipped(
                panel.x + 2,
                panel.y + 2,
                &confirm.message,
                style,
                panel.x + panel.w - 2,
            );
        }
    }
    let row = panel.y + panel.h - 2;
    let ok = match dialog {
        Dialog::Input(_) => "[ok]".to_string(),
        Dialog::Confirm(yes) => format!("[{}]", yes.yes),
    };
    let mut at =
        panel.x + panel.w - 4 - units(ok.chars().count()) - units("[cancel]".chars().count());
    let ok_fg = if matches!(dialog, Dialog::Confirm(_)) {
        pal.danger
    } else {
        pal.success
    };
    at = p.button(Action::DialogOk, at, row, &ok, Btn::default().fg(ok_fg)) + 2;
    p.button(Action::DialogCancel, at, row, "[cancel]", Btn::default());
}

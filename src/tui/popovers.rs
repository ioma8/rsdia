//! The overlay panels: files, export, settings, help, the narrow menu, and the
//! modal input/confirm dialog.

use ratatui::style::{Color, Modifier};

use crate::core::editor::ToolId;
use crate::core::export::{Charset, WRAPPERS};
use crate::storage::config::GRID_STYLES;
use crate::tui::host::{Dialog, Host};
use crate::tui::painter::{Action, Btn, ItemId, Painter, Rect};
use crate::tui::theme::{Palette, ThemeName, THEME_CHOICES};
use crate::tui::toolbar::tool_color;

// ------------------------------------------------------------------ common

/// Places a popover below its toolbar label, clamped to the screen.
pub fn place_popover(p: &Painter, anchor_x: i32, top: i32, w: i32, h: i32) -> Rect {
    let w = w.min(p.width);
    let h = h.min(3.max(p.height - top - 1));
    let x = 0.max((anchor_x - 2).min(p.width - w));
    Rect { x, y: top, w, h }
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
pub fn render_files(p: &mut Painter, host: &dyn Host, anchor_x: i32, top: i32) {
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
    let max_rows = (p.height - top - 6 - actions_rows).max(1) as usize;
    let rows = &all[..max_rows.min(all.len())];
    let h = 2 + 1.max(rows.len() as i32) + 1 + actions_rows;
    let r = place_popover(p, anchor_x, top, FILES_WIDTH, h);
    p.panel(r, pal.tb_border, pal.tb_bg);

    let mut y = r.y + 1;
    if rows.is_empty() {
        p.text(r.x + 2, y, "no drawings yet", pal.muted, pal.tb_bg);
        y += 1;
    }
    for (i, d) in rows.iter().enumerate() {
        let current = d.path == host.current_path();
        let size = format!("{} cells", d.size);
        let name_width = (r.w - 6 - size.chars().count() as i32).max(0);
        let name = truncate_end(&d.name, name_width);
        let label = format!(
            "{} {:<width$} {}",
            if current { ">" } else { " " },
            name,
            size,
            width = name_width as usize
        );
        let button_label: String = format!(" {label} ")
            .chars()
            .take((r.w - 2).max(0) as usize)
            .collect();
        let style = Btn {
            active: current,
            active_color: Some(pal.text),
            ..Default::default()
        };
        p.button(Action::FilesOpen(i), r.x + 1, y, &button_label, style);
        y += 1;
    }
    if all.len() > rows.len() {
        let more = format!(" +{} more ", all.len() - rows.len());
        p.text(r.x + r.w - 12, y - 1, &more, pal.muted, pal.tb_bg);
    }
    rule(p, r, y);
    y += 1;
    flow(p, r, y, &actions, "");
}

// ------------------------------------------------------------------ export

/// `export` dialog: charset, comment wrapper, fence, preview, copy / save.
pub fn render_export(p: &mut Painter, host: &dyn Host, anchor_x: i32, top: i32) {
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
    let chrome = 2 + 1 + wrap_rows + 1 + 1 + 1 + 1;
    let max_preview = (p.height - top - 1 - chrome).max(1) as usize;
    let shown = &preview[..max_preview.min(preview.len())];
    let h = chrome + shown.len() as i32;
    let r = place_popover(p, anchor_x, top, w, h);
    p.panel(r, pal.tb_border, pal.tb_bg);

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

    let mut y = r.y + 1;
    y = flow(p, r, y, &charset, "characters: ");
    y = flow(p, r, y, &wrappers, "wrap:       ");
    rule(p, r, y);
    y += 1;
    for line in shown {
        p.text_clipped(
            r.x + 2,
            y,
            line,
            pal.fg,
            pal.tb_bg,
            Modifier::empty(),
            r.x + r.w - 2,
        );
        y += 1;
    }
    if preview.len() > shown.len() {
        let more = format!(" +{} lines ", preview.len() - shown.len());
        p.text(r.x + r.w - 16, y - 1, &more, pal.muted, pal.tb_bg);
    }
    rule(p, r, y);
    y += 1;
    let x = p.button(
        Action::ExportCopy,
        r.x + 2,
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
pub fn render_settings(p: &mut Painter, host: &dyn Host, anchor_x: i32, top: i32) {
    let r = place_popover(p, anchor_x, top, SETTINGS_WIDTH, settings_height());
    let pal = p.pal;
    p.panel(r, pal.tb_border, pal.tb_bg);
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

const SHORTCUTS: [(&str, &str); 11] = [
    (
        "1-6 / alt+1-6",
        "switch tool (box select arrow line text eraser)",
    ),
    ("r v a l t e", "switch tool by letter"),
    (
        "ctrl+z  ctrl+y",
        "undo / redo (ctrl+shift+z with kitty keys)",
    ),
    (
        "ctrl+shift+c  ctrl+x  ctrl+v",
        "copy / cut / paste selection",
    ),
    ("del  arrows", "erase / nudge selection"),
    ("f", "flip elbow while dragging a line or arrow"),
    ("scroll", "pan vertically, or horizontally with a trackpad"),
    ("space+drag  middle-drag", "pan freely"),
    ("ctrl+o  ctrl+e  ctrl+s", "files / export / save now"),
    ("?  esc", "help / close popover, cancel, deselect"),
    ("ctrl+c  ctrl+q", "quit (the drawing is saved first)"),
];

/// `help` popover: active-tool help and the shortcut table.
pub fn render_help(p: &mut Painter, host: &dyn Host, anchor_x: i32, top: i32) {
    let pal = p.pal;
    let w = p.width.min(76);
    let h = 2 + 1 + 1 + SHORTCUTS.len() as i32 + 1;
    let r = place_popover(p, anchor_x, top, w, h);
    p.panel(r, pal.tb_border, pal.tb_bg);
    let tool = host.editor().tool();
    let mut y = r.y + 1;
    let heading = format!("{}: ", tool.name());
    let x = p.text_clipped(
        r.x + 2,
        y,
        &heading,
        tool_color(&pal, tool),
        pal.tb_bg,
        Modifier::BOLD,
        r.x + r.w - 2,
    );
    let help = TOOL_HELP
        .iter()
        .find(|(t, _)| *t == tool)
        .map(|(_, h)| *h)
        .unwrap_or("");
    p.text_clipped(
        x,
        y,
        help,
        pal.text,
        pal.tb_bg,
        Modifier::empty(),
        r.x + r.w - 2,
    );
    y += 1;
    rule(p, r, y);
    y += 1;
    let key_w = SHORTCUTS
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0) as i32
        + 2;
    for (k, description) in SHORTCUTS {
        p.text(r.x + 2, y, k, pal.accent, pal.tb_bg);
        p.text_clipped(
            r.x + 2 + key_w,
            y,
            description,
            pal.text,
            pal.tb_bg,
            Modifier::empty(),
            r.x + r.w - 2,
        );
        y += 1;
    }
}

// ------------------------------------------------------------------ menu

const MENU_ENTRIES: [(ItemId, &str); 7] = [
    (ItemId::Files, "files"),
    (ItemId::Export, "export"),
    (ItemId::Undo, "undo"),
    (ItemId::Redo, "redo"),
    (ItemId::Settings, "settings"),
    (ItemId::Help, "help"),
    (ItemId::Quit, "quit"),
];

/// `≡` menu for terminals too narrow for the full toolbar.
pub fn render_menu(p: &mut Painter, _host: &dyn Host, anchor_x: i32, top: i32) {
    let pal = p.pal;
    let r = place_popover(p, anchor_x, top, 18, MENU_ENTRIES.len() as i32 + 2);
    p.panel(r, pal.tb_border, pal.tb_bg);
    for (i, (id, label)) in MENU_ENTRIES.iter().enumerate() {
        let fg = match id {
            ItemId::Undo => Some(pal.undo),
            ItemId::Redo => Some(pal.redo),
            ItemId::Quit => Some(pal.danger),
            _ => None,
        };
        let text = format!(" {:<width$} ", label, width = (r.w - 4).max(0) as usize);
        p.button(
            Action::MenuEntry(*id),
            r.x + 1,
            r.y + 1 + i as i32,
            &text,
            Btn {
                fg,
                ..Default::default()
            },
        );
    }
}

// ------------------------------------------------------------------ dialog

/// Modal input and confirm dialogs, centered on screen.
pub fn render_dialog(p: &mut Painter, d: &Dialog) {
    let pal = p.pal;
    let w = p.width.min(56);
    let h = if d.input().is_some() { 7 } else { 6 };
    let r = Rect {
        x: (p.width - w) / 2,
        y: 0.max((p.height - h) / 2),
        w,
        h,
    };
    p.panel(r, pal.accent, pal.tb_bg);
    let title = format!(" {} ", d.title());
    p.text_clipped(
        r.x + 2,
        r.y,
        &title,
        pal.accent,
        pal.tb_bg,
        Modifier::BOLD,
        p.width,
    );
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

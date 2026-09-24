//! Floating toolbar: `≡ │ tools │ export │ ⟲ ⟳ │ ⚙ │ help`.

use ratatui::style::Color;

use crate::core::editor::{ToolId, TOOL_IDS};
use crate::tui::painter::{in_rect, Action, Btn, ItemId, Painter, PanelId, Rect};
use crate::tui::theme::Palette;

const fn item(id: ItemId, label: &'static str) -> Item {
    Item { id, label }
}

const fn tool(id: ToolId, label: &'static str) -> Item {
    Item {
        id: ItemId::Tool(id),
        label,
    }
}

#[derive(Clone, Copy, Debug)]
struct Item {
    id: ItemId,
    label: &'static str,
}

struct Form {
    name: &'static str,
    pad: i32,
    groups: &'static [&'static [Item]],
}

const FULL_GROUPS: [&[Item]; 6] = [
    &[item(ItemId::Files, "≡")],
    &[
        tool(ToolId::Box, "box"),
        tool(ToolId::Select, "select"),
        tool(ToolId::Arrow, "arrow"),
        tool(ToolId::Line, "line"),
        tool(ToolId::Text, "text"),
        tool(ToolId::Eraser, "eraser"),
    ],
    &[item(ItemId::Export, "export")],
    &[item(ItemId::Undo, "⟲"), item(ItemId::Redo, "⟳")],
    &[item(ItemId::Settings, "⚙")],
    &[item(ItemId::Help, "help")],
];

const COMPACT_GROUPS: [&[Item]; 6] = [
    &[item(ItemId::Files, "≡")],
    &[
        tool(ToolId::Box, "box"),
        tool(ToolId::Select, "sel"),
        tool(ToolId::Arrow, "arrow"),
        tool(ToolId::Line, "line"),
        tool(ToolId::Text, "text"),
        tool(ToolId::Eraser, "erase"),
    ],
    &[item(ItemId::Export, "exp")],
    &[item(ItemId::Undo, "⟲"), item(ItemId::Redo, "⟳")],
    &[item(ItemId::Settings, "⚙")],
    &[item(ItemId::Help, "?")],
];

const NARROW_GROUPS: [&[Item]; 2] = [
    &[item(ItemId::Menu, "≡")],
    &[
        tool(ToolId::Box, "box"),
        tool(ToolId::Select, "sel"),
        tool(ToolId::Arrow, "arw"),
        tool(ToolId::Line, "lin"),
        tool(ToolId::Text, "txt"),
        tool(ToolId::Eraser, "ers"),
    ],
];

const TINY_GROUPS: [&[Item]; 2] = [
    &[item(ItemId::Menu, "≡")],
    &[
        tool(ToolId::Box, "b"),
        tool(ToolId::Select, "s"),
        tool(ToolId::Arrow, "a"),
        tool(ToolId::Line, "l"),
        tool(ToolId::Text, "t"),
        tool(ToolId::Eraser, "e"),
    ],
];

/// Widest form first; smaller terminals get shorter labels.
const FORMS: [Form; 5] = [
    Form {
        name: "full",
        pad: 2,
        groups: &FULL_GROUPS,
    },
    Form {
        name: "compact",
        pad: 1,
        groups: &COMPACT_GROUPS,
    },
    Form {
        name: "tight",
        pad: 0,
        groups: &COMPACT_GROUPS,
    },
    Form {
        name: "narrow",
        pad: 1,
        groups: &NARROW_GROUPS,
    },
    Form {
        name: "tiny",
        pad: 0,
        groups: &TINY_GROUPS,
    },
];

fn form_width(f: &Form) -> i32 {
    let mut w = 2 + (f.groups.len() as i32 - 1);
    for g in f.groups {
        w += 2 * f.pad
            + g.iter()
                .map(|i| i.label.chars().count() as i32)
                .sum::<i32>()
            + (g.len() as i32 - 1);
    }
    w
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemSpan {
    pub id: ItemId,
    pub x: i32,
    pub w: i32,
}

#[derive(Clone, Debug)]
pub struct ToolbarLayout {
    pub form: &'static str,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    /// Row below the bar, where popovers open.
    pub bottom: i32,
    pub items: Vec<ItemSpan>,
}

pub const BAR_Y: i32 = 1;

pub fn layout_toolbar(screen_width: i32) -> ToolbarLayout {
    let form = FORMS
        .iter()
        .find(|f| form_width(f) <= screen_width)
        .unwrap_or_else(|| FORMS.last().expect("at least one form"));
    let w = form_width(form);
    let x = 0.max((screen_width - w) / 2);
    let mut items = Vec::new();
    let mut cx = x + 1;
    for (gi, g) in form.groups.iter().enumerate() {
        cx += form.pad;
        for (ii, entry) in g.iter().enumerate() {
            items.push(ItemSpan {
                id: entry.id,
                x: cx,
                w: entry.label.chars().count() as i32,
            });
            cx += entry.label.chars().count() as i32 + i32::from(ii + 1 < g.len());
        }
        cx += form.pad;
        if gi + 1 < form.groups.len() {
            cx += 1;
        }
    }
    ToolbarLayout {
        form: form.name,
        x,
        y: BAR_Y,
        w,
        bottom: BAR_Y + 3,
        items,
    }
}

pub fn tool_color(pal: &Palette, id: ToolId) -> Color {
    match id {
        ToolId::Box => pal.cyan,
        ToolId::Select => pal.success,
        ToolId::Arrow => pal.purple,
        ToolId::Line => pal.accent,
        ToolId::Text => pal.warning,
        ToolId::Eraser => pal.orange,
    }
}

/// What the toolbar needs from the app.
pub trait ToolbarHost {
    fn tool(&self) -> ToolId;
    fn panel(&self) -> Option<PanelId>;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    fn show_chips(&self) -> bool;
}

fn panel_for(id: ItemId) -> Option<PanelId> {
    match id {
        ItemId::Files => Some(PanelId::Files),
        ItemId::Export => Some(PanelId::Export),
        ItemId::Settings => Some(PanelId::Settings),
        ItemId::Help => Some(PanelId::Help),
        ItemId::Menu => Some(PanelId::Menu),
        _ => None,
    }
}

fn label_for(form: &Form, id: ItemId) -> &'static str {
    form.groups
        .iter()
        .flat_map(|g| g.iter())
        .find(|i| i.id == id)
        .map(|i| i.label)
        .unwrap_or("")
}

pub fn render_toolbar(p: &mut Painter, host: &dyn ToolbarHost, layout: &ToolbarLayout) {
    let pal = p.pal;
    let form = FORMS
        .iter()
        .find(|f| f.name == layout.form)
        .expect("the layout came from these forms");
    p.panel(
        Rect {
            x: layout.x,
            y: layout.y,
            w: layout.w,
            h: 3,
        },
        pal.tb_border,
        pal.tb_bg,
    );
    let row = layout.y + 1;

    // Group separators.
    let mut cx = layout.x + 1;
    for (gi, g) in form.groups.iter().enumerate() {
        cx += 2 * form.pad
            + g.iter()
                .map(|i| i.label.chars().count() as i32)
                .sum::<i32>()
            + (g.len() as i32 - 1);
        if gi + 1 < form.groups.len() {
            p.cell(
                cx,
                row,
                "│",
                pal.tb_border,
                pal.tb_bg,
                ratatui::style::Modifier::empty(),
            );
            cx += 1;
        }
    }

    for span in &layout.items {
        let label = label_for(form, span.id);
        let action = Action::Toolbar(span.id);
        match span.id {
            ItemId::Tool(tool_id) => {
                p.button(
                    action,
                    span.x,
                    row,
                    label,
                    Btn::default()
                        .active(host.tool() == tool_id)
                        .active_color(tool_color(&pal, tool_id)),
                );
            }
            ItemId::Undo => {
                p.button(
                    action,
                    span.x,
                    row,
                    label,
                    Btn::default().fg(pal.undo).disabled(!host.can_undo()),
                );
            }
            ItemId::Redo => {
                p.button(
                    action,
                    span.x,
                    row,
                    label,
                    Btn::default().fg(pal.redo).disabled(!host.can_redo()),
                );
            }
            other => {
                let panel = panel_for(other);
                let active = panel.is_some() && host.panel() == panel;
                p.button(action, span.x, row, label, Btn::default().active(active));
            }
        }
    }

    // Shortcut chips on the bottom border, under each tool (ASCIIFlow shows them while Alt is held).
    if host.show_chips() {
        for span in &layout.items {
            if let ItemId::Tool(tool_id) = span.id {
                let i = TOOL_IDS.iter().position(|t| *t == tool_id).unwrap_or(0);
                p.cell(
                    span.x,
                    layout.y + 2,
                    &(i + 1).to_string(),
                    pal.bg,
                    pal.warning,
                    ratatui::style::Modifier::BOLD,
                );
            }
        }
    }
}

/// The toolbar area, for click routing.
pub fn toolbar_contains(layout: &ToolbarLayout, x: i32, y: i32) -> bool {
    in_rect(
        Rect {
            x: layout.x,
            y: layout.y,
            w: layout.w,
            h: 3,
        },
        x,
        y,
    )
}

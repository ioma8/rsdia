//! Floating tool picker near the bottom of the canvas.

use ratatui::style::Color;

use crate::core::editor::{ToolId, TOOL_IDS};
use crate::tui::host::Host;
use crate::tui::painter::{Action, Btn, ItemId, Painter, PanelId, Rect};
use crate::tui::theme::Palette;

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
    pad: i32,
    items: &'static [Item],
}

const FULL: &[Item] = &[
    tool(ToolId::Box, "box"),
    tool(ToolId::Select, "select"),
    tool(ToolId::Arrow, "arrow"),
    tool(ToolId::Line, "line"),
    tool(ToolId::Text, "text"),
    tool(ToolId::Eraser, "eraser"),
];
const COMPACT: &[Item] = &[
    tool(ToolId::Box, "box"),
    tool(ToolId::Select, "sel"),
    tool(ToolId::Arrow, "arw"),
    tool(ToolId::Line, "lin"),
    tool(ToolId::Text, "txt"),
    tool(ToolId::Eraser, "ers"),
];
const TINY: &[Item] = &[
    tool(ToolId::Box, "b"),
    tool(ToolId::Select, "s"),
    tool(ToolId::Arrow, "a"),
    tool(ToolId::Line, "l"),
    tool(ToolId::Text, "t"),
    tool(ToolId::Eraser, "e"),
];

const FORMS: [Form; 3] = [
    Form {
        pad: 2,
        items: FULL,
    },
    Form {
        pad: 1,
        items: COMPACT,
    },
    Form {
        pad: 0,
        items: TINY,
    },
];

fn form_width(form: &Form) -> i32 {
    2 + form.pad * 2
        + form.items.iter().map(|i| i.label.len() as i32).sum::<i32>()
        + form.items.len() as i32
        - 1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ItemSpan {
    id: ItemId,
    label: &'static str,
    x: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct ToolbarLayout {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    items: Vec<ItemSpan>,
}

pub const MENU_Y: i32 = 0;

/// Menu entries: the label, the mnemonic used on terminals too narrow for it,
/// and the dropdown it opens.
const MENUS: [(&str, &str, PanelId); 4] = [
    ("File", "F", PanelId::FileMenu),
    ("Edit", "E", PanelId::EditMenu),
    ("View", "V", PanelId::ViewMenu),
    ("Help", "H", PanelId::HelpMenu),
];

fn menubar_width(compact: bool) -> i32 {
    1 + MENUS
        .iter()
        .map(|(full, short, _)| {
            let label = if compact { *short } else { *full };
            label.len() as i32 + 3
        })
        .sum::<i32>()
        - 1
}

/// Where `panel`'s menu label sits: `(label, x, width)`. One function so the bar
/// and the accelerators that open the same dropdowns can never disagree.
fn menu_entry(panel: PanelId, width: i32) -> Option<(&'static str, i32, i32)> {
    let compact = menubar_width(false) > width;
    let mut x = 1;
    for (full, short, entry) in MENUS {
        let label = if compact { short } else { full };
        let w = label.len() as i32 + 2;
        if entry == panel {
            return Some((label, x, w));
        }
        x += w + 1;
    }
    None
}

/// The x a popover anchored to `panel`'s label opens at.
pub(crate) fn menu_anchor(panel: PanelId, width: i32) -> i32 {
    menu_entry(panel, width).map_or(1, |(_, x, _)| x)
}

/// The dropdowns in bar order, so left/right can walk them.
pub(crate) fn menu_panels() -> [PanelId; MENUS.len()] {
    MENUS.map(|(_, _, panel)| panel)
}

/// The dropdown an `alt+<letter>` mnemonic opens.
pub(crate) fn menu_for_mnemonic(c: char) -> Option<PanelId> {
    let wanted = c.to_ascii_uppercase();
    MENUS
        .into_iter()
        .find(|(_, short, _)| short.starts_with(wanted))
        .map(|(_, _, panel)| panel)
}

pub(crate) fn render_menubar(p: &mut Painter, host: &dyn Host) {
    let pal = p.pal;
    p.fill(
        Rect {
            x: 0,
            y: MENU_Y,
            w: p.width,
            h: 1,
        },
        pal.menu_bg,
    );
    p.chrome.push(Rect {
        x: 0,
        y: MENU_Y,
        w: p.width,
        h: 1,
    });
    for (_, _, panel) in MENUS {
        let Some((label, x, w)) = menu_entry(panel, p.width) else {
            continue;
        };
        let active = host.panel() == Some(panel);
        let hovered = p.is_hover(x, MENU_Y, w, 1);
        // The selected entry inverts the strip, the way a tab bar marks its tab.
        let (fg, bg) = if active || hovered {
            (pal.menu_active_fg, pal.menu_active_bg)
        } else {
            (pal.menu_fg, pal.menu_bg)
        };
        p.fill(
            Rect {
                x,
                y: MENU_Y,
                w,
                h: 1,
            },
            bg,
        );
        let modifier = if active {
            ratatui::style::Modifier::BOLD
        } else {
            ratatui::style::Modifier::empty()
        };
        p.text_clipped(x + 1, MENU_Y, label, fg, bg, modifier, x + w);
        p.hotspots.push(crate::tui::painter::Hotspot {
            action: Action::Panel(panel),
            x,
            y: MENU_Y,
            w,
            h: 1,
        });
    }
}

/// The row the floating picker occupies: three rows sitting directly on the
/// status bar, floating over the canvas above it.
pub fn toolbar_row(screen_height: i32) -> i32 {
    (screen_height - 4).max(1)
}

pub(crate) fn layout_toolbar(screen_width: i32, screen_height: i32) -> ToolbarLayout {
    let form = FORMS
        .iter()
        .find(|form| form_width(form) <= screen_width)
        .unwrap_or_else(|| FORMS.last().expect("at least one form"));
    let w = form_width(form);
    let x = 0.max((screen_width - w) / 2);
    let y = toolbar_row(screen_height);
    let items = form
        .items
        .iter()
        .scan(x + 1 + form.pad, |cx, item| {
            let span = ItemSpan {
                id: item.id,
                label: item.label,
                x: *cx,
            };
            *cx += item.label.len() as i32 + 1;
            Some(span)
        })
        .collect();
    ToolbarLayout { x, y, w, items }
}

pub(crate) fn tool_color(pal: &Palette, id: ToolId) -> Color {
    match id {
        ToolId::Box => pal.cyan,
        ToolId::Select => pal.success,
        ToolId::Arrow => pal.purple,
        ToolId::Line => pal.accent,
        ToolId::Text => pal.warning,
        ToolId::Eraser => pal.orange,
    }
}

pub(crate) fn render_toolbar(p: &mut Painter, host: &dyn Host, layout: &ToolbarLayout) {
    let pal = p.pal;
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
    for span in &layout.items {
        let ItemId::Tool(tool_id) = span.id else {
            continue;
        };
        p.button(
            Action::Toolbar(span.id),
            span.x,
            row,
            span.label,
            Btn::default()
                .active(host.tool() == tool_id)
                .active_color(tool_color(&pal, tool_id)),
        );
    }

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

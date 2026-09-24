//! Input: mouse, keyboard and paste reach the app as crossterm events.

use super::*;

impl App {
    fn mouse_mods(&self, e: &MouseEvent) -> Mods {
        let m = Mods::new(
            e.modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL),
            e.modifiers.contains(crossterm::event::KeyModifiers::ALT),
        );
        m.flipped(m.flip != self.flip_toggle)
    }

    /// Space is held: once outside text entry, or repeating while typing.
    fn space_held(&self) -> bool {
        let need = if self.editor.text_entry() { 2 } else { 1 };
        Instant::now() < self.space_run.1 && self.space_run.0 >= need
    }

    fn over_chrome(&self, x: i32, y: i32) -> bool {
        self.chrome.iter().any(|r| in_rect(*r, x, y))
    }

    pub fn on_mouse(&mut self, e: &MouseEvent) {
        let (x, y) = (e.column as i32, e.row as i32);
        match e.kind {
            MouseEventKind::ScrollDown => self.on_scroll(e, 0, 1),
            MouseEventKind::ScrollUp => self.on_scroll(e, 0, -1),
            MouseEventKind::ScrollLeft => self.on_scroll(e, -2, 0),
            MouseEventKind::ScrollRight => self.on_scroll(e, 2, 0),
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.on_move(e),
            MouseEventKind::Down(button) => self.on_down(e, button),
            MouseEventKind::Up(_) => self.on_up(x, y),
        }
    }

    fn on_scroll(&mut self, e: &MouseEvent, dx: i32, dy: i32) {
        let (x, y) = (e.column as i32, e.row as i32);
        if self.dialog.is_some() {
            return;
        }
        if self.panel.is_some() && self.over_chrome(x, y) {
            return;
        }
        self.viewport.pan(dx, dy);
        self.refresh_hover(x, y);
    }

    fn refresh_hover(&mut self, x: i32, y: i32) {
        self.hover = Some((x, y));
        let cell = self.viewport.to_canvas(x, y);
        if self.placing.is_some() {
            self.show_placing(cell);
        }
        self.hover_is_target = self.tool() == ToolId::Select
            && !self.over_chrome(x, y)
            && self.editor.hover_is_target(cell, Mods::NONE);
    }

    fn on_move(&mut self, e: &MouseEvent) {
        let (x, y) = (e.column as i32, e.row as i32);
        let mods = self.mouse_mods(e);
        match self.mode {
            Mode::Pan { sx, sy, origin } => {
                self.viewport.origin = Pos::new(origin.x - (x - sx), origin.y - (y - sy));
            }
            Mode::Draw { last } => {
                let cell = self.viewport.to_canvas(x, y);
                self.last_mods = mods;
                if cell != last {
                    self.mode = Mode::Draw { last: cell };
                    self.editor.move_to(cell, mods);
                }
            }
            Mode::None => {}
        }
        self.refresh_hover(x, y);
    }

    fn on_down(&mut self, e: &MouseEvent, button: MouseButton) {
        let (x, y) = (e.column as i32, e.row as i32);
        self.hover = Some((x, y));
        let action = hotspot_at(&self.hotspots, x, y);
        self.pressed = action.map(|a| (a, x, y));
        if action.is_some() || self.dialog.is_some() {
            return;
        }
        if self.over_chrome(x, y) {
            return;
        }
        if self.panel.is_some() {
            // A click outside a popover only closes it.
            self.close_panel();
            return;
        }
        if !matches!(self.mode, Mode::None) {
            return;
        }
        let cell = self.viewport.to_canvas(x, y);
        let middle = button == MouseButton::Middle;
        if middle || (button == MouseButton::Left && self.space_held()) {
            if button == MouseButton::Left && self.editor.text_entry() {
                // The held space was typed into the text; take it back.
                for _ in 0..self.space_run.0 {
                    if !self.editor.text_last_typed_space() {
                        break;
                    }
                    self.editor.text_undo_keystroke();
                }
            }
            self.mode = Mode::Pan {
                sx: x,
                sy: y,
                origin: self.viewport.origin,
            };
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        if self.placing.is_some() {
            self.show_placing(cell);
            self.editor.canvas.commit_scratch();
            self.placing = None;
            self.toast("imported");
            return;
        }
        self.flip_toggle = false;
        self.last_mods = self.mouse_mods(e);
        self.mode = Mode::Draw { last: cell };
        let m = self.last_mods;
        self.editor.down(cell, m);
    }

    fn on_up(&mut self, x: i32, y: i32) {
        let pressed = self.pressed.take();
        if let Some((action, px, _)) = pressed {
            if hotspot_at(&self.hotspots, x, y) == Some(action) {
                self.activate(action, px);
            }
            return;
        }
        if matches!(self.mode, Mode::Draw { .. }) {
            self.editor.up();
            self.auto_copy();
        }
        self.mode = Mode::None;
        self.flip_toggle = false;
        self.refresh_hover(x, y);
    }

    pub fn on_key(&mut self, k: &KeyEvent) {
        // Terminals that report key releases (kitty protocol) say when a held space
        // ended; everything else is inferred from repeats.
        if k.kind == crossterm::event::KeyEventKind::Release {
            if k.code == KeyCode::Char(' ') {
                self.space_run = (0, Instant::now());
            }
            return;
        }
        self.handle_key(k);
    }

    fn handle_key(&mut self, k: &KeyEvent) {
        if self.dialog.is_some() {
            return self.dialog_key(k);
        }

        // Global shortcuts.
        if is_ctrl(k, 'q') {
            return self.arm_quit();
        }
        if is_ctrl(k, 'c') {
            return self.copy_selection(false);
        }
        if is_ctrl(k, 'z') {
            if is_shift(k) {
                self.redo();
            } else {
                self.undo();
            }
            return;
        }
        if is_ctrl(k, 'y') {
            self.redo();
            return;
        }
        if is_ctrl(k, 's') {
            self.editor.flush();
            self.save();
            self.toast("saved");
            return;
        }
        if is_ctrl(k, 'e') {
            let anchor = self.anchor_of(ItemId::Export);
            self.toggle_panel(PanelId::Export, anchor);
            return;
        }
        if is_ctrl(k, 'o') {
            let anchor = self.anchor_of(ItemId::Files);
            self.toggle_panel(PanelId::Files, anchor);
            return;
        }
        if is_ctrl(k, 'x') {
            return self.copy_selection(true);
        }
        if is_ctrl(k, 'v') {
            if let Some(text) = self.clipboard.paste() {
                self.paste_text(&text);
            }
            return;
        }
        if k.code == KeyCode::Esc {
            return self.escape();
        }
        if let Some(digit) = alt_digit(k) {
            return self.set_tool(TOOL_IDS[digit - 1]);
        }

        let key = tool_key(k);

        // Mid-drag: `f` flips line/arrow/select elbows; other keys go to the tool.
        if self.editor.drawing && matches!(self.mode, Mode::Draw { .. }) {
            if key == Some(Key::Char('f')) {
                self.flip_toggle = !self.flip_toggle;
                self.last_mods = self.last_mods.flipped(!self.last_mods.flip);
                if self.tool() == ToolId::Select {
                    if let Mode::Draw { last } = self.mode {
                        let m = self.last_mods;
                        self.editor.move_to(last, m);
                    }
                } else {
                    let m = self.last_mods;
                    self.editor.key(Key::Char('f'), m);
                }
                return;
            }
            if let Some(key) = key {
                let m = self.last_mods;
                self.editor.key(key, m);
            }
            return;
        }

        if k.code == KeyCode::Char(' ') {
            let now = Instant::now();
            let count = if now < self.space_run.1 {
                self.space_run.0 + 1
            } else {
                1
            };
            self.space_run = (count, now + Duration::from_millis(SPACE_PAN_MS));
            if !self.editor.text_entry() {
                return;
            }
        } else {
            self.space_run = (0, Instant::now());
        }

        if self.editor.text_entry() {
            if let Some(key) = key {
                self.editor.key(key, Mods::NONE);
            }
            return;
        }

        let ch = printable(k);
        if let Some(c @ '1'..='6') = ch {
            return self.set_tool(TOOL_IDS[c as usize - '1' as usize]);
        }
        if let Some(tool) = ch.and_then(tool_shortcut) {
            return self.set_tool(tool);
        }
        if ch == Some('?') {
            let anchor = self.anchor_of(ItemId::Help);
            return self.toggle_panel(PanelId::Help, anchor);
        }

        if self.tool() == ToolId::Select {
            if let Some(key) = key {
                self.editor.key(key, Mods::NONE);
            }
        }
    }

    fn escape(&mut self) {
        if self.panel.is_some() {
            return self.close_panel();
        }
        if self.placing.is_some() {
            self.placing = None;
            self.editor.canvas.clear_scratch();
            return;
        }
        if matches!(self.mode, Mode::Draw { .. }) {
            self.editor.cancel_gesture();
            self.mode = Mode::None;
            return;
        }
        if self.editor.text_entry() {
            return self.editor.flush();
        }
        if self.editor.canvas.selection.is_some() {
            self.editor.select_cleanup();
        }
    }

    fn dialog_key(&mut self, k: &KeyEvent) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        if k.code == KeyCode::Esc {
            return self.close_dialog();
        }
        if k.code == KeyCode::Enter {
            return self.submit_dialog();
        }
        if dialog.input().is_none() {
            if k.code == KeyCode::Char('y') {
                return self.submit_dialog();
            }
            if k.code == KeyCode::Char('n') {
                self.close_dialog();
            }
            return;
        }
        if let Some(d) = self.dialog.as_mut().and_then(Dialog::input_mut) {
            let mut chars: Vec<char> = d.value.chars().collect();
            match k.code {
                KeyCode::Backspace => {
                    if d.cursor > 0 {
                        d.cursor -= 1;
                        chars.remove(d.cursor);
                    }
                }
                KeyCode::Delete => {
                    if d.cursor < chars.len() {
                        chars.remove(d.cursor);
                    }
                }
                KeyCode::Left => d.cursor = d.cursor.saturating_sub(1),
                KeyCode::Right => d.cursor = (d.cursor + 1).min(chars.len()),
                KeyCode::Home => d.cursor = 0,
                KeyCode::End => d.cursor = chars.len(),
                _ => match ctrl_char(k) {
                    Some('a') => d.cursor = 0,
                    Some('e') => d.cursor = chars.len(),
                    Some('u') => {
                        chars.drain(0..d.cursor);
                        d.cursor = 0;
                    }
                    _ => {
                        if let Some(ch) = printable(k) {
                            chars.insert(d.cursor, ch);
                            d.cursor += 1;
                        }
                    }
                },
            }
            d.value = chars.into_iter().collect();
            d.error = None;
        }
    }

    pub fn on_paste(&mut self, text: &str) {
        if self.dialog.as_ref().and_then(Dialog::input).is_some() {
            let insert: String = text.split(['\r', '\n']).next().unwrap_or("").to_string();
            if let Some(d) = self.dialog.as_mut().and_then(Dialog::input_mut) {
                let mut chars: Vec<char> = d.value.chars().collect();
                let added = insert.chars().count();
                for (i, ch) in insert.chars().enumerate() {
                    chars.insert(d.cursor + i, ch);
                }
                d.cursor += added;
                d.value = chars.into_iter().collect();
            }
        } else if self.editor.text_entry() {
            for ch in text.replace("\r\n", "\n").chars() {
                let key = if ch == '\n' {
                    Key::Enter
                } else {
                    Key::Char(ch)
                };
                self.editor.key(key, Mods::NONE);
            }
        } else if self.dialog.is_none() {
            self.paste_text(text);
        }
    }
}

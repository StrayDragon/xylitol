use crate::keybindings::with_keybindings;
use crate::keys::decode_printable_key;
use crate::kill_ring::{KillRing, KillRingOptions};
use crate::tui::{CURSOR_MARKER, Component};
use crate::undo_stack::UndoStack;
use crate::utils::{is_whitespace_char, slice_by_column, visible_width};
use crate::word_navigation::{find_word_backward, find_word_forward};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
struct InputState {
    value: String,
    cursor: usize,
}

pub struct Input {
    value: String,
    cursor: usize,
    pub on_submit: Option<Box<dyn FnMut(String) + Send>>,
    pub on_escape: Option<Box<dyn Fn() + Send>>,
    focused: bool,
    paste_buffer: String,
    is_in_paste: bool,
    kill_ring: KillRing,
    last_action: Option<String>,
    undo_stack: UndoStack<InputState>,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            on_submit: None,
            on_escape: None,
            focused: false,
            paste_buffer: String::new(),
            is_in_paste: false,
            kill_ring: KillRing::new(),
            last_action: None,
            undo_stack: UndoStack::new(),
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: String) {
        self.cursor = std::cmp::min(self.cursor, value.len());
        self.value = value;
    }

    fn handle_paste(&mut self, paste_content: &str) {
        self.last_action = None;
        self.push_undo();
        let cleaned = paste_content
            .replace("\r\n", "")
            .replace(['\r', '\n'], "")
            .replace('\t', "    ");
        let left = self.value[..self.cursor].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}{}", left, cleaned, right);
        self.cursor += cleaned.len();
    }

    fn insert_char(&mut self, ch: &str) {
        if is_whitespace_char(ch.chars().next().unwrap_or(' '))
            || self.last_action.as_deref() != Some("type-word")
        {
            self.push_undo();
        }
        self.last_action = Some("type-word".to_string());
        let left = self.value[..self.cursor].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}{}", left, ch, right);
        self.cursor += ch.len();
    }

    fn handle_backspace(&mut self) {
        self.last_action = None;
        if self.cursor > 0 {
            self.push_undo();
            let before: Vec<&str> =
                UnicodeSegmentation::graphemes(&self.value[..self.cursor], true).collect();
            let last = before.last().copied().unwrap_or("");
            let grapheme_len = last.len();
            let left = self.value[..self.cursor - grapheme_len].to_string();
            let right = self.value[self.cursor..].to_string();
            self.value = format!("{}{}", left, right);
            self.cursor -= grapheme_len;
        }
    }

    fn handle_forward_delete(&mut self) {
        self.last_action = None;
        if self.cursor < self.value.len() {
            self.push_undo();
            let after: Vec<&str> =
                UnicodeSegmentation::graphemes(&self.value[self.cursor..], true).collect();
            let first = after.first().copied().unwrap_or("");
            let grapheme_len = first.len();
            let left = self.value[..self.cursor].to_string();
            let right = self.value[self.cursor + grapheme_len..].to_string();
            self.value = format!("{}{}", left, right);
        }
    }

    fn delete_to_line_start(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.push_undo();
        let deleted = self.value[..self.cursor].to_string();
        let was_kill = self.last_action.as_deref() == Some("kill");
        self.kill_ring.push(
            deleted,
            KillRingOptions {
                prepend: true,
                accumulate: was_kill,
            },
        );
        self.last_action = Some("kill".to_string());
        self.value = self.value[self.cursor..].to_string();
        self.cursor = 0;
    }

    fn delete_to_line_end(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.push_undo();
        let deleted = self.value[self.cursor..].to_string();
        let was_kill = self.last_action.as_deref() == Some("kill");
        self.kill_ring.push(
            deleted,
            KillRingOptions {
                prepend: false,
                accumulate: was_kill,
            },
        );
        self.last_action = Some("kill".to_string());
        self.value = self.value[..self.cursor].to_string();
    }

    fn delete_word_backwards(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let was_kill = self.last_action.as_deref() == Some("kill");
        self.push_undo();
        let old_cursor = self.cursor;
        self.move_word_backwards();
        let delete_from = self.cursor;
        self.cursor = old_cursor;
        let deleted = self.value[delete_from..self.cursor].to_string();
        self.kill_ring.push(
            deleted,
            KillRingOptions {
                prepend: true,
                accumulate: was_kill,
            },
        );
        self.last_action = Some("kill".to_string());
        let left = self.value[..delete_from].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}", left, right);
        self.cursor = delete_from;
    }

    fn delete_word_forward(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let was_kill = self.last_action.as_deref() == Some("kill");
        self.push_undo();
        let old_cursor = self.cursor;
        self.move_word_forwards();
        let delete_to = self.cursor;
        self.cursor = old_cursor;
        let deleted = self.value[self.cursor..delete_to].to_string();
        self.kill_ring.push(
            deleted,
            KillRingOptions {
                prepend: false,
                accumulate: was_kill,
            },
        );
        self.last_action = Some("kill".to_string());
        let left = self.value[..self.cursor].to_string();
        let right = self.value[delete_to..].to_string();
        self.value = format!("{}{}", left, right);
    }

    fn yank(&mut self) {
        let text = match self.kill_ring.peek() {
            Some(s) => s.to_string(),
            None => return,
        };
        self.push_undo();
        let left = self.value[..self.cursor].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}{}", left, text, right);
        self.cursor += text.len();
        self.last_action = Some("yank".to_string());
    }

    fn yank_pop(&mut self) {
        if self.last_action.as_deref() != Some("yank") || self.kill_ring.len() <= 1 {
            return;
        }
        self.push_undo();
        let prev = self.kill_ring.peek().map(|s| s.len()).unwrap_or(0);
        let left = self.value[..self.cursor - prev].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}", left, right);
        self.cursor -= prev;
        self.kill_ring.rotate();
        let text = self
            .kill_ring
            .peek()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let left = self.value[..self.cursor].to_string();
        let right = self.value[self.cursor..].to_string();
        self.value = format!("{}{}{}", left, text, right);
        self.cursor += text.len();
        self.last_action = Some("yank".to_string());
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(InputState {
            value: self.value.clone(),
            cursor: self.cursor,
        });
    }

    fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            self.value = state.value;
            self.cursor = state.cursor;
            self.last_action = None;
        }
    }

    fn move_word_backwards(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.last_action = None;
        self.cursor = find_word_backward(&self.value, self.cursor);
    }

    fn move_word_forwards(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.last_action = None;
        self.cursor = find_word_forward(&self.value, self.cursor);
    }
}

impl Component for Input {
    fn render(&mut self, width: usize) -> Vec<String> {
        let prompt = "> ";
        let available = width.saturating_sub(prompt.len());
        if available == 0 {
            return vec![prompt.to_string()];
        }

        let visible_text;
        let cursor_display;
        let total_width = visible_width(&self.value);

        if total_width < available {
            visible_text = self.value.clone();
            cursor_display = self.cursor;
        } else {
            let scroll_width = if self.cursor == self.value.len() {
                available.saturating_sub(1)
            } else {
                available
            };
            let cursor_col = visible_width(&self.value[..self.cursor]);

            if scroll_width > 0 {
                let half = scroll_width / 2;
                let start_col = if cursor_col < half {
                    0
                } else if cursor_col > total_width.saturating_sub(half) {
                    total_width.saturating_sub(scroll_width)
                } else {
                    cursor_col.saturating_sub(half)
                };
                visible_text = slice_by_column(&self.value, start_col, scroll_width);
                let before =
                    slice_by_column(&self.value, start_col, cursor_col.saturating_sub(start_col));
                cursor_display = before.len();
            } else {
                visible_text = String::new();
                cursor_display = 0;
            }
        }

        let cursor_grapheme = UnicodeSegmentation::graphemes(&visible_text[cursor_display..], true)
            .next()
            .unwrap_or(" ");

        let before = &visible_text[..cursor_display];
        let after_start = cursor_display + cursor_grapheme.len();
        let after = if after_start < visible_text.len() {
            &visible_text[after_start..]
        } else {
            ""
        };

        let marker = if self.focused { CURSOR_MARKER } else { "" };
        let cursor_char = format!("\x1b[7m{}\x1b[27m", cursor_grapheme);
        let text_with_cursor = format!("{}{}{}{}", before, marker, cursor_char, after);

        let vis_len = visible_width(&text_with_cursor);
        let padding = " ".repeat(available.saturating_sub(vis_len));
        let line = format!("{}{}{}", prompt, text_with_cursor, padding);
        vec![line]
    }

    fn handle_input(&mut self, data: &str) {
        if data.contains("\x1b[200~") {
            self.is_in_paste = true;
            self.paste_buffer.clear();
            let remaining = data.replace("\x1b[200~", "");
            if !remaining.is_empty() {
                self.handle_input(&remaining);
            }
            return;
        }

        if self.is_in_paste {
            self.paste_buffer.push_str(data);
            if let Some(end_pos) = self.paste_buffer.find("\x1b[201~") {
                let content = self.paste_buffer[..end_pos].to_string();
                let rest = self.paste_buffer[end_pos + 6..].to_string();
                self.paste_buffer.clear();
                self.is_in_paste = false;
                self.handle_paste(&content);
                if !rest.is_empty() {
                    self.handle_input(&rest);
                }
            }
            return;
        }

        if with_keybindings(|kb| kb.matches(data, "tui.select.cancel")) {
            if let Some(ref mut cb) = self.on_escape {
                cb();
            }
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.undo")) {
            self.undo();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.input.submit")) || data == "\n" {
            if let Some(ref mut cb) = self.on_submit {
                let value = std::mem::take(&mut self.value);
                self.cursor = 0;
                cb(value);
            }
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteCharBackward")) {
            self.handle_backspace();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteCharForward")) {
            self.handle_forward_delete();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteWordBackward")) {
            self.delete_word_backwards();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteWordForward")) {
            self.delete_word_forward();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteToLineStart")) {
            self.delete_to_line_start();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.deleteToLineEnd")) {
            self.delete_to_line_end();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.yank")) {
            self.yank();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.yankPop")) {
            self.yank_pop();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorLeft")) {
            self.last_action = None;
            if self.cursor > 0 {
                let before: Vec<&str> =
                    UnicodeSegmentation::graphemes(&self.value[..self.cursor], true).collect();
                let last = before.last().copied().unwrap_or("");
                self.cursor = self.cursor.saturating_sub(last.len().max(1));
            }
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorRight")) {
            self.last_action = None;
            if self.cursor < self.value.len() {
                let after: Vec<&str> =
                    UnicodeSegmentation::graphemes(&self.value[self.cursor..], true).collect();
                let first = after.first().copied().unwrap_or("");
                self.cursor += first.len().max(1);
            }
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorLineStart")) {
            self.last_action = None;
            self.cursor = 0;
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorLineEnd")) {
            self.last_action = None;
            self.cursor = self.value.len();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorWordLeft")) {
            self.move_word_backwards();
            return;
        }
        if with_keybindings(|kb| kb.matches(data, "tui.editor.cursorWordRight")) {
            self.move_word_forwards();
            return;
        }

        if let Some(ch) = decode_printable_key(data) {
            self.insert_char(&ch);
            return;
        }

        let has_control = data.chars().any(|c| {
            let code = c as u32;
            code < 32 || code == 0x7f || (0x80..=0x9f).contains(&code)
        });
        if !has_control && !data.is_empty() {
            self.insert_char(data);
        }
    }

    fn invalidate(&mut self) {}
}

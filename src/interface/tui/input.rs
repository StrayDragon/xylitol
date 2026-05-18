//! User input component for the TUI.
//!
//! Provides a text input area with cursor, history tracking, and
//! submission handling.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Maximum input line length.
const MAX_INPUT: usize = 1024;

/// Input component — single-line text entry.
pub(crate) struct InputComponent {
    /// Current input buffer.
    input: String,
    /// Cursor position within input (byte index).
    cursor: usize,
    /// History of submitted prompts (oldest first).
    history: Vec<String>,
    /// Current position in history navigation (None = fresh input).
    history_pos: Option<usize>,
    /// Whether the input is disabled (agent running).
    disabled: bool,
}

impl InputComponent {
    pub(crate) fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: None,
            disabled: false,
        }
    }

    /// Insert a character at cursor position.
    pub(crate) fn insert_char(&mut self, c: char) {
        if self.disabled || self.input.len() >= MAX_INPUT {
            return;
        }
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete character before cursor (backspace).
    pub(crate) fn delete_before(&mut self) {
        if self.disabled || self.cursor == 0 {
            return;
        }
        let prev = self.input[..self.cursor].char_indices().next_back();
        if let Some((idx, _c)) = prev {
            self.input.drain(idx..self.cursor);
            self.cursor = idx;
        }
    }

    /// Delete character at cursor (delete key).
    pub(crate) fn delete_at(&mut self) {
        if self.disabled || self.cursor >= self.input.len() {
            return;
        }
        let next = self.input[self.cursor..].char_indices().nth(1);
        let end = next
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.input.len());
        self.input.drain(self.cursor..end);
    }

    /// Move cursor left.
    pub(crate) fn cursor_left(&mut self) {
        if self.cursor > 0 {
            let prev = self.input[..self.cursor].char_indices().next_back();
            if let Some((idx, _)) = prev {
                self.cursor = idx;
            }
        }
    }

    /// Move cursor right.
    pub(crate) fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            let next = self.input[self.cursor..].char_indices().nth(1);
            if let Some((i, _)) = next {
                self.cursor += i;
            }
        }
    }

    /// Move cursor to beginning.
    pub(crate) fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end.
    pub(crate) fn cursor_end(&mut self) {
        self.cursor = self.input.len();
    }

    /// Navigate history backward (↑).
    pub(crate) fn history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_pos {
            None => {
                // Save current input and go to newest history entry.
                self.history_pos = Some(self.history.len() - 1);
            }
            Some(pos) if pos > 0 => {
                self.history_pos = Some(pos - 1);
            }
            _ => return,
        }
        let pos = self.history_pos.unwrap();
        self.input = self.history[pos].clone();
        self.cursor = self.input.len();
    }

    /// Navigate history forward (↓).
    pub(crate) fn history_forward(&mut self) {
        match self.history_pos {
            Some(pos) if pos + 1 < self.history.len() => {
                self.history_pos = Some(pos + 1);
                self.input = self.history[pos + 1].clone();
                self.cursor = self.input.len();
            }
            Some(_) => {
                // Back to fresh input.
                self.history_pos = None;
                self.input.clear();
                self.cursor = 0;
            }
            None => {}
        }
    }

    /// Submit the current input. Returns the prompt string if non-empty.
    pub(crate) fn submit(&mut self) -> Option<String> {
        if self.disabled {
            return None;
        }
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return None;
        }
        self.history.push(trimmed.clone());
        self.input.clear();
        self.cursor = 0;
        self.history_pos = None;
        Some(trimmed)
    }

    /// Enable/disable input.
    pub(crate) fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Render the input component.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        let title = if self.disabled {
            " Input (disabled while running) "
        } else {
            " Input "
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(if self.disabled {
                Color::DarkGray
            } else {
                Color::Green
            }));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show the input text.
        let display = if self.input.is_empty() && !self.disabled {
            "Type a message and press Enter..."
        } else {
            self.input.as_str()
        };

        let para = Paragraph::new(Line::from(display))
            .style(Style::default().fg(if self.disabled {
                Color::DarkGray
            } else {
                Color::White
            }))
            .wrap(Wrap { trim: false });
        frame.render_widget(para, inner);

        // Set cursor position.
        if !self.disabled {
            // Calculate visual cursor position (approximate).
            let visual_cx = self.input[..self.cursor]
                .chars()
                .map(|c| if c == '\t' { 4 } else { 1 })
                .sum::<usize>() as u16;
            frame.set_cursor_position(ratatui::layout::Position::new(
                inner.x + visual_cx.min(inner.width.saturating_sub(1)),
                inner.y,
            ));
        }
    }

    /// Access the current input text.
    pub(crate) fn input_text(&self) -> &str {
        &self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_delete() {
        let mut ic = InputComponent::new();
        ic.insert_char('a');
        ic.insert_char('b');
        ic.insert_char('c');
        assert_eq!(ic.input, "abc");
        ic.cursor_left();
        ic.delete_at();
        assert_eq!(ic.input, "ab");
    }

    #[test]
    fn test_submit_returns_text() {
        let mut ic = InputComponent::new();
        ic.insert_char('h');
        ic.insert_char('i');
        let result = ic.submit();
        assert_eq!(result.as_deref(), Some("hi"));
        assert!(ic.input.is_empty());
    }

    #[test]
    fn test_empty_submit_returns_none() {
        let mut ic = InputComponent::new();
        assert!(ic.submit().is_none());
    }

    #[test]
    fn test_history_navigation() {
        let mut ic = InputComponent::new();
        ic.insert_char('a');
        ic.submit();
        ic.insert_char('b');
        ic.submit();

        ic.history_back();
        assert_eq!(ic.input, "b");
        ic.history_back();
        assert_eq!(ic.input, "a");
        ic.history_forward();
        assert_eq!(ic.input, "b");
    }

    #[test]
    fn test_disabled_blocks_input() {
        let mut ic = InputComponent::new();
        ic.set_disabled(true);
        ic.insert_char('x');
        assert!(ic.input.is_empty());
        assert!(ic.submit().is_none());
    }

    #[test]
    fn test_cursor_movement() {
        let mut ic = InputComponent::new();
        ic.insert_char('a');
        ic.insert_char('b');
        ic.insert_char('c');
        ic.cursor_left();
        ic.cursor_left();
        ic.insert_char('X');
        assert_eq!(ic.input, "aXbc");
    }
}

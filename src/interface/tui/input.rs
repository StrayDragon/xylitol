//! Input component — multi-line prompt editor for the TUI.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use tui_textarea::TextArea;

use super::component::{Component, EventResult};
use super::event::{AppAction, TuiEvent};
use super::slash::Completer;

const MIN_HEIGHT: u16 = 3;
const MAX_HEIGHT: u16 = 15;

pub(crate) struct InputComponent {
    textarea: TextArea<'static>,
    dirty: bool,
    completer: Completer,
    history: Vec<String>,
    history_pos: Option<usize>,
    focused: bool,
}

impl InputComponent {
    pub(crate) fn new(completer: Completer) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message… (Enter to submit, Shift+Enter newline)");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));

        Self {
            textarea,
            dirty: true,
            completer,
            history: Vec::new(),
            history_pos: None,
            focused: false,
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    pub(crate) fn desired_height(&self) -> u16 {
        let n = self.textarea.lines().len() as u16;
        n.clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    pub(crate) fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
        self.history_pos = None;
    }

    pub(crate) fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.dirty = true;
        self.history_pos = None;
    }

    pub(crate) fn load_text(&mut self, text: &str) {
        self.set_text(text);
        self.dirty = true;
    }

    fn set_text(&mut self, text: &str) {
        self.textarea = TextArea::from(text.lines());
        self.textarea
            .set_placeholder_text("Type a message… (Enter to submit, Shift+Enter newline)");
        self.dirty = true;
        // Move cursor to end.
        self.textarea.move_cursor(tui_textarea::CursorMove::End);
    }

    fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    fn handle_history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_pos {
            None => self.history_pos = Some(self.history.len().saturating_sub(1)),
            Some(0) => return false,
            Some(pos) => self.history_pos = Some(pos.saturating_sub(1)),
        }
        let pos = self.history_pos.unwrap_or(0);
        let entry = self.history[pos].clone();
        self.set_text(&entry);
        true
    }

    fn handle_history_down(&mut self) -> bool {
        match self.history_pos {
            None => false,
            Some(pos) if pos + 1 < self.history.len() => {
                self.history_pos = Some(pos + 1);
                let entry = self.history[pos + 1].clone();
                self.set_text(&entry);
                true
            }
            Some(_) => {
                self.history_pos = None;
                self.set_text("");
                true
            }
        }
    }

    fn submit(&mut self) -> Option<String> {
        let text = self.text().trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.history.push(text.clone());
        self.history_pos = None;
        self.clear();
        Some(text)
    }

    fn is_slash_mode(&self) -> bool {
        let s = self.text();
        s.trim_start().starts_with('/')
    }

    fn try_complete(&mut self) -> bool {
        if !self.is_slash_mode() {
            return false;
        }
        let current = self.text();
        let completed = self.completer.complete(&current);
        if completed == current {
            return false;
        }
        self.set_text(&completed);
        true
    }
}

impl Component for InputComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .border_style(border_style)
            .title_style(border_style);
        self.textarea.set_block(block);
        frame.render_widget(&self.textarea, area);

        // Cursor positioning is approximated from (row,col) within the visible box.
        let inner = area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });
        let (row, col) = self.textarea.cursor();
        let x = inner
            .x
            .saturating_add(col as u16)
            .min(inner.right().saturating_sub(1));
        let y = inner
            .y
            .saturating_add(row as u16)
            .min(inner.bottom().saturating_sub(1));
        frame.set_cursor_position((x, y));

        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult {
        let TuiEvent::Key(key) = event else {
            return EventResult::default();
        };

        use crossterm::event::{KeyCode, KeyModifiers};

        // Shift+Enter inserts newline; Enter submits.
        if key.code == KeyCode::Enter {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                let input: tui_textarea::Input = (*key).into();
                if self.textarea.input(input) {
                    self.dirty = true;
                }
                return EventResult::consumed();
            }

            if let Some(text) = self.submit() {
                self.dirty = true;
                return EventResult::action(AppAction::RunPrompt(text));
            }
            return EventResult::consumed();
        }

        // History navigation on bare Up/Down.
        if key.code == KeyCode::Up && key.modifiers.is_empty() && self.handle_history_up() {
            return EventResult::consumed();
        }
        if key.code == KeyCode::Down && key.modifiers.is_empty() && self.handle_history_down() {
            return EventResult::consumed();
        }

        // Slash completion.
        if key.code == KeyCode::Tab && key.modifiers.is_empty() && self.try_complete() {
            return EventResult::consumed();
        }

        // Readline tweak: Ctrl+U kills to head-of-line (textarea default uses Ctrl+U for undo).
        if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.textarea.delete_line_by_head() {
                self.dirty = true;
            }
            return EventResult::consumed();
        }

        if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return EventResult::action(AppAction::ShowHistorySearch);
        }

        if key.code == KeyCode::Char('g') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return EventResult::action(AppAction::OpenEditor(self.text()));
        }

        // Default handling via tui-textarea.
        let input: tui_textarea::Input = (*key).into();
        if self.textarea.input(input) {
            self.dirty = true;
            return EventResult::consumed();
        }

        EventResult::default()
    }
}

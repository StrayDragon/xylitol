//! Chat component — scrollable message history with markdown rendering.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::agent::r#loop::{AgentError, AgentEvent};

use super::component::{Component, EventResult};
use super::event::TuiEvent;
use super::markdown::MarkdownRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct Message {
    role: Role,
    content: String,
    streaming: bool,
    cached_width: u16,
    cached_lines: Vec<Line<'static>>,
    dirty: bool,
}

impl Message {
    fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            streaming: false,
            cached_width: 0,
            cached_lines: Vec::new(),
            dirty: true,
        }
    }

    fn role_label(&self) -> (&'static str, Style) {
        match self.role {
            Role::User => (
                "user",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Assistant => (
                "assistant",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::System => (
                "system",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Error => (
                "error",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
        }
    }
}

pub(crate) struct ChatComponent {
    markdown: MarkdownRenderer,
    messages: Vec<Message>,
    /// Scroll offset measured from the bottom (0 = show newest).
    scroll_offset: u16,
    follow_tail: bool,
    focused: bool,
    dirty: bool,
}

impl ChatComponent {
    pub(crate) fn new(markdown: MarkdownRenderer) -> Self {
        Self {
            markdown,
            messages: Vec::new(),
            scroll_offset: 0,
            follow_tail: true,
            focused: false,
            dirty: true,
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    pub(crate) fn set_theme(&mut self, theme: &str) -> bool {
        if !self.markdown.set_theme(theme) {
            return false;
        }

        // Invalidate markdown caches.
        for msg in &mut self.messages {
            msg.cached_width = 0;
            msg.dirty = true;
        }
        self.dirty = true;
        true
    }

    pub(crate) fn add_user_message(&mut self, text: &str) {
        self.messages
            .push(Message::new(Role::User, text.to_string()));
        self.scroll_to_bottom();
        self.dirty = true;
    }

    pub(crate) fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.follow_tail = true;
        self.dirty = true;
    }

    pub(crate) fn scroll_wheel_up(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    pub(crate) fn scroll_wheel_down(&mut self, n: u16) {
        self.scroll_down(n);
    }

    fn append_assistant_delta(&mut self, delta: &str) {
        if let Some(last) = self.messages.last_mut()
            && last.role == Role::Assistant
            && last.streaming
        {
            last.content.push_str(delta);
            last.dirty = true;
        } else {
            let mut m = Message::new(Role::Assistant, delta.to_string());
            m.streaming = true;
            self.messages.push(m);
        }

        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn finish_streaming(&mut self) {
        if let Some(last) = self.messages.last_mut()
            && last.role == Role::Assistant
        {
            last.streaming = false;
            last.dirty = true;
        }
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn show_tool_message(&mut self, line: String) {
        self.messages.push(Message::new(Role::System, line));
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn show_error(&mut self, err: AgentError) {
        self.messages
            .push(Message::new(Role::Error, format!("{err}")));
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn scroll_up(&mut self, n: u16, max_scroll: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n).min(max_scroll);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    fn scroll_to_top(&mut self, max_scroll: u16) {
        self.scroll_offset = max_scroll;
        self.follow_tail = false;
        self.dirty = true;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.follow_tail = true;
        self.dirty = true;
    }

    fn render_lines(&mut self, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        for msg in &mut self.messages {
            let (label, style) = msg.role_label();
            let suffix = if msg.streaming { " ▍" } else { "" };
            lines.push(Line::from(Span::styled(format!("{label}{suffix}"), style)));

            if msg.dirty || msg.cached_width != width {
                msg.cached_width = width;
                msg.cached_lines = self.markdown.render(&msg.content, width);
                msg.dirty = false;
            }
            lines.extend(msg.cached_lines.clone());
            lines.push(Line::from(""));
        }

        lines
    }
}

impl Component for ChatComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(2);
        let mut lines = self.render_lines(inner_width);

        let view_height = area.height.saturating_sub(2);
        let total_lines = lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(view_height);
        let scroll = self.scroll_offset.min(max_scroll);
        let start = max_scroll.saturating_sub(scroll) as usize;
        let end = (start + view_height as usize).min(lines.len());

        let visible = if start < end {
            lines.drain(start..end).collect::<Vec<Line<'static>>>()
        } else {
            Vec::new()
        };

        let title = if max_scroll == 0 {
            " Chat ".to_string()
        } else {
            format!(
                " Chat  ({} / {}) ",
                max_scroll.saturating_sub(scroll),
                max_scroll
            )
        };

        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let para = Paragraph::new(visible)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style)
                    .title_style(border_style),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(para, area);
        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult {
        match event {
            TuiEvent::Agent(agent_event) => {
                match agent_event {
                    AgentEvent::TextDelta(delta) => self.append_assistant_delta(delta),
                    AgentEvent::ToolCallStart { name, args, .. } => {
                        self.show_tool_message(format!("Tool call: {name} {args}"));
                    }
                    AgentEvent::ToolCallEnd { id, result } => {
                        self.show_tool_message(format!("Tool result ({id}): {result}"));
                    }
                    AgentEvent::StepComplete { .. } => self.finish_streaming(),
                    AgentEvent::RepeatDetected { .. } => {
                        self.show_tool_message("Repeat detected — interrupted.".to_string());
                        self.finish_streaming();
                    }
                    AgentEvent::Error(err) => self.show_error(err.clone()),
                }
                return EventResult::consumed();
            }
            TuiEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};
                if !key.modifiers.is_empty() {
                    return EventResult::default();
                }

                // Scroll shortcuts.
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        // Max scroll is computed lazily in render; approximate here as "some".
                        // The exact clamp will happen in render.
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                        self.follow_tail = self.scroll_offset == 0;
                        self.dirty = true;
                        return EventResult::consumed();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down(1);
                        return EventResult::consumed();
                    }
                    KeyCode::Char('g') => {
                        // We'll clamp in render once we know max_scroll.
                        self.scroll_offset = u16::MAX;
                        self.follow_tail = false;
                        self.dirty = true;
                        return EventResult::consumed();
                    }
                    KeyCode::Char('G') => {
                        self.scroll_to_bottom();
                        return EventResult::consumed();
                    }
                    _ => {}
                }

                // Ctrl+L: clear screen (handled at app level) should not scroll chat.
                if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return EventResult::default();
                }
            }
            TuiEvent::Mouse(_) | TuiEvent::Tick | TuiEvent::Shutdown => {}
        }

        EventResult::default()
    }
}

//! Chat message display component for the TUI.
//!
//! Renders a scrollable list of messages (user + assistant) with streaming
//! text deltas and Markdown formatting.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::agent::r#loop::AgentEvent;

use super::markdown::MarkdownRenderer;

/// A single chat message.
#[derive(Debug, Clone)]
pub(crate) struct Message {
    /// "user" or "assistant".
    pub(crate) role: String,
    /// Accumulated text content.
    pub(crate) content: String,
    /// Whether this message is still receiving deltas.
    pub(crate) streaming: bool,
}

/// Chat component — scrollable message history.
pub(crate) struct ChatComponent {
    messages: Vec<Message>,
    scroll_offset: u16,
    needs_scroll: bool,
    markdown: MarkdownRenderer,
}

impl ChatComponent {
    pub(crate) fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            needs_scroll: false,
            markdown: MarkdownRenderer::default(),
        }
    }

    /// Handle an agent event.
    pub(crate) fn handle_event(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::TextDelta(delta) => {
                // Append to the last assistant message or create one.
                if let Some(last) = self.messages.last_mut()
                    && last.role == "assistant"
                {
                    last.content.push_str(delta);
                    last.streaming = true;
                    self.needs_scroll = true;
                    return;
                }
                // No assistant message yet — start one.
                self.messages.push(Message {
                    role: "assistant".into(),
                    content: delta.clone(),
                    streaming: true,
                });
                self.needs_scroll = true;
            }
            AgentEvent::ToolCallStart { name, .. } => {
                // Show tool invocation in the chat area.
                let msg = format!("\n\n*[Tool: {name} — running...]*\n");
                if let Some(last) = self.messages.last_mut()
                    && last.role == "assistant"
                {
                    last.content.push_str(&msg);
                }
            }
            AgentEvent::StepComplete { .. } => {
                // Finalize the current assistant message.
                if let Some(last) = self.messages.last_mut()
                    && last.role == "assistant"
                {
                    last.streaming = false;
                }
                self.needs_scroll = true;
            }
            _ => {}
        }
    }

    /// Add a user message.
    pub(crate) fn add_user_message(&mut self, text: &str) {
        self.messages.push(Message {
            role: "user".into(),
            content: text.to_string(),
            streaming: false,
        });
        self.needs_scroll = true;
    }

    /// Scroll up by `n` lines.
    pub(crate) fn scroll_up(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.needs_scroll = false;
    }

    /// Scroll down by `n` lines.
    pub(crate) fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.needs_scroll = false;
    }

    /// Reset scroll to bottom.
    pub(crate) fn scroll_bottom(&mut self) {
        self.scroll_offset = 0;
        self.needs_scroll = false;
    }

    /// Render the chat component into the given area.
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Build rendered lines from all messages.
        let mut all_lines: Vec<Line<'_>> = Vec::new();

        for msg in &self.messages {
            // Role label.
            let role_style = match msg.role.as_str() {
                "user" => Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                "assistant" => Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
                _ => Style::default().fg(Color::Gray),
            };
            let suffix = if msg.streaming { " ▍" } else { "" };
            all_lines.push(Line::from(ratatui::text::Span::styled(
                format!("{}{}", msg.role, suffix),
                role_style,
            )));

            // Message content as markdown-rendered lines.
            let content_lines = self
                .markdown
                .render(&msg.content, area.width.saturating_sub(2));
            all_lines.extend(content_lines);

            // Spacer between messages.
            all_lines.push(Line::from(""));
        }

        // Compute scroll offset.
        let total_lines = all_lines.len() as u16;
        let view_height = area.height.saturating_sub(2); // borders
        let max_scroll = total_lines.saturating_sub(view_height);

        if self.needs_scroll {
            self.scroll_offset = 0; // always show latest
            self.needs_scroll = false;
        }

        let scroll = self.scroll_offset.min(max_scroll);

        // Visible slice.
        let start = scroll as usize;
        let end = (scroll + view_height).min(total_lines) as usize;
        let visible: Vec<Line<'_>> = all_lines
            .into_iter()
            .skip(start)
            .take(end - start)
            .collect();

        let para = Paragraph::new(visible)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Chat ")
                    .title_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(para, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_user_message() {
        let mut cc = ChatComponent::new();
        cc.add_user_message("hello");
        assert_eq!(cc.messages.len(), 1);
        assert_eq!(cc.messages[0].content, "hello");
    }

    #[test]
    fn test_text_delta_appends() {
        let mut cc = ChatComponent::new();
        cc.handle_event(&AgentEvent::TextDelta("Hello ".into()));
        cc.handle_event(&AgentEvent::TextDelta("world".into()));
        assert_eq!(cc.messages.len(), 1);
        assert_eq!(cc.messages[0].content, "Hello world");
        assert!(cc.messages[0].streaming);
    }

    #[test]
    fn test_step_complete_stops_streaming() {
        let mut cc = ChatComponent::new();
        cc.handle_event(&AgentEvent::TextDelta("hi".into()));
        cc.handle_event(&AgentEvent::StepComplete {
            step: 1,
            summary: "done".into(),
        });
        assert!(!cc.messages[0].streaming);
    }
}

//! Tool output panel — shows tool calls/results separate from chat.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::agent::r#loop::{AgentError, AgentEvent};

use super::component::{Component, EventResult};
use super::event::TuiEvent;

pub(crate) struct ToolPanelComponent {
    lines: Vec<String>,
    dirty: bool,
    focused: bool,
}

impl ToolPanelComponent {
    pub(crate) fn new() -> Self {
        Self {
            lines: Vec::new(),
            dirty: true,
            focused: false,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.lines.clear();
        self.dirty = true;
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    fn push(&mut self, line: String) {
        const MAX: usize = 400;
        self.lines.push(line);
        if self.lines.len() > MAX {
            let drop_n = self.lines.len().saturating_sub(MAX);
            self.lines.drain(0..drop_n);
        }
        self.dirty = true;
    }

    fn show_error(&mut self, err: &AgentError) {
        self.push(format!("error: {err}"));
    }
}

impl Component for ToolPanelComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let view_height = area.height.saturating_sub(2) as usize;
        let start = self.lines.len().saturating_sub(view_height);
        let visible = self.lines[start..]
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect::<Vec<_>>();

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
                    .title(" Tools ")
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
                    AgentEvent::ToolCallStart { name, args, .. } => {
                        self.push(format!("call: {name} {args}"));
                    }
                    AgentEvent::ToolCallEnd { id, result } => {
                        self.push(format!("result ({id}): {result}"));
                    }
                    AgentEvent::Error(err) => self.show_error(err),
                    AgentEvent::RepeatDetected { .. } => self.push("repeat detected".to_string()),
                    AgentEvent::TextDelta(_) | AgentEvent::StepComplete { .. } => {}
                }
                return EventResult::consumed();
            }
            TuiEvent::Key(_) | TuiEvent::Mouse(_) | TuiEvent::Tick | TuiEvent::Shutdown => {}
        }

        EventResult::default()
    }
}

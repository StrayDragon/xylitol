//! Tool execution output component for the TUI.
//!
//! Displays tool call status and results in a dedicated panel.

use std::collections::HashMap;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// State of a tool invocation.
#[derive(Debug, Clone)]
pub(crate) enum ToolCallState {
    /// Tool is currently executing.
    Running,
    /// Tool completed successfully.
    Success,
    /// Tool failed with an error.
    Failed(String),
}

/// A single tool call tracked in the output panel.
#[derive(Debug, Clone)]
pub(crate) struct ToolCallEntry {
    /// Tool name.
    pub(crate) name: String,
    /// Current state.
    pub(crate) state: ToolCallState,
    /// Result summary (truncated).
    pub(crate) result_summary: Option<String>,
}

/// Tool output component — shows the tool calls for the current step.
pub(crate) struct ToolOutputComponent {
    /// Tool calls indexed by ID.
    calls: HashMap<String, ToolCallEntry>,
    /// Display order.
    order: Vec<String>,
    /// Whether the panel is expanded/collapsed.
    collapsed: bool,
}

impl ToolOutputComponent {
    pub(crate) fn new() -> Self {
        Self {
            calls: HashMap::new(),
            order: Vec::new(),
            collapsed: false,
        }
    }

    /// Toggle collapsed state.
    pub(crate) fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Handle a tool-related agent event.
    pub(crate) fn handle_tool_start(&mut self, id: String, name: String) {
        self.calls.insert(
            id.clone(),
            ToolCallEntry {
                name: name.clone(),
                state: ToolCallState::Running,
                result_summary: None,
            },
        );
        self.order.push(id);
    }

    /// Handle tool completion.
    pub(crate) fn handle_tool_end(&mut self, id: String, result: &serde_json::Value) {
        if let Some(entry) = self.calls.get_mut(&id) {
            let summary = match result {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let truncated = if summary.len() > 200 {
                format!("{}...", &summary[..200])
            } else {
                summary
            };
            entry.state = ToolCallState::Success;
            entry.result_summary = Some(truncated);
        }
    }

    /// Clear all tool calls.
    pub(crate) fn clear(&mut self) {
        self.calls.clear();
        self.order.clear();
    }

    /// Render the tool output panel.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        if self.calls.is_empty() {
            return;
        }

        let title = if self.collapsed {
            format!(" Tools ({}) [collapsed] ", self.calls.len())
        } else {
            format!(" Tools ({}) ", self.calls.len())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Magenta));

        if self.collapsed {
            frame.render_widget(block, area);
            return;
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'_>> = Vec::new();
        for id in &self.order {
            if let Some(entry) = self.calls.get(id) {
                let (icon, state_style) = match &entry.state {
                    ToolCallState::Running => ("◌", Style::default().fg(Color::Yellow)),
                    ToolCallState::Success => ("✓", Style::default().fg(Color::Green)),
                    ToolCallState::Failed(_) => ("✗", Style::default().fg(Color::Red)),
                };

                lines.push(Line::from(vec![
                    ratatui::text::Span::styled(format!(" {} ", icon), state_style),
                    ratatui::text::Span::styled(
                        &entry.name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                if let Some(ref summary) = entry.result_summary {
                    lines.push(Line::from(ratatui::text::Span::styled(
                        format!("   {}", summary),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_lifecycle() {
        let mut tc = ToolOutputComponent::new();
        tc.handle_tool_start("t1".into(), "read_file".into());
        assert!(matches!(
            tc.calls.get("t1").unwrap().state,
            ToolCallState::Running
        ));

        tc.handle_tool_end("t1".into(), &serde_json::json!("file content"));
        assert!(matches!(
            tc.calls.get("t1").unwrap().state,
            ToolCallState::Success
        ));
    }

    #[test]
    fn test_clear() {
        let mut tc = ToolOutputComponent::new();
        tc.handle_tool_start("t1".into(), "ls".into());
        tc.clear();
        assert!(tc.calls.is_empty());
    }
}

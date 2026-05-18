//! Approval overlay component for the TUI.
//!
//! Displays a command/tool approval prompt and collects the user's decision.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Approval result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalDecision {
    Allow,
    Deny,
    AllowOnce,
    DenyOnce,
}

/// Approval overlay — yes/no prompt for tool execution approval.
pub(crate) struct ApprovalOverlay {
    /// Tool name being approved.
    tool_name: String,
    /// Tool arguments (formatted).
    tool_args: String,
    /// Whether the overlay is active.
    active: bool,
    /// Current selection (0 = allow, 1 = deny, etc.).
    selected: usize,
    /// Result of the approval, if decided.
    decision: Option<ApprovalDecision>,
}

const OPTIONS: &[(&str, ApprovalDecision)] = &[
    (" Allow ", ApprovalDecision::Allow),
    (" Deny ", ApprovalDecision::Deny),
    (" Allow Once ", ApprovalDecision::AllowOnce),
    (" Deny Once ", ApprovalDecision::DenyOnce),
];

impl ApprovalOverlay {
    pub(crate) fn new() -> Self {
        Self {
            tool_name: String::new(),
            tool_args: String::new(),
            active: false,
            selected: 0,
            decision: None,
        }
    }

    /// Show the approval prompt for a tool call.
    pub(crate) fn prompt(&mut self, tool_name: &str, tool_args: &str) {
        self.tool_name = tool_name.to_string();
        self.tool_args = tool_args.to_string();
        self.active = true;
        self.selected = 0;
        self.decision = None;
    }

    /// Move selection up.
    pub(crate) fn select_prev(&mut self) {
        if self.active {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Move selection down.
    pub(crate) fn select_next(&mut self) {
        if self.active && self.selected + 1 < OPTIONS.len() {
            self.selected += 1;
        }
    }

    /// Confirm the current selection.
    pub(crate) fn confirm(&mut self) -> Option<ApprovalDecision> {
        if !self.active {
            return None;
        }
        let decision = OPTIONS[self.selected].1;
        self.decision = Some(decision);
        self.active = false;
        Some(decision)
    }

    /// Cancel the overlay.
    pub(crate) fn cancel(&mut self) {
        self.active = false;
        self.decision = None;
    }

    /// Check if the overlay is active.
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Get the decision, if made.
    pub(crate) fn decision(&self) -> Option<ApprovalDecision> {
        self.decision
    }

    /// Render the approval overlay.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Overlay — centered, ~60% width, ~40% height.
        let overlay_w = (area.width as f32 * 0.6) as u16;
        let overlay_h = (area.height as f32 * 0.4) as u16;
        let x = (area.width - overlay_w) / 2;
        let y = (area.height - overlay_h) / 2;
        let overlay_rect = Rect::new(x, y, overlay_w.max(40), overlay_h.max(10));

        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Tool Approval ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Rgb(20, 15, 5)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        // Tool info.
        let mut lines = Vec::new();
        lines.push(Line::from(ratatui::text::Span::styled(
            format!("Tool: {}", self.tool_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(ratatui::text::Span::styled(
            &self.tool_args,
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(ratatui::text::Span::styled(
            "Approve this tool call?",
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));

        // Options.
        for (i, (label, _)) in OPTIONS.iter().enumerate() {
            let selected = i == self.selected;
            let prefix = if selected { " > " } else { "   " };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(ratatui::text::Span::styled(
                format!("{}{}", prefix, label),
                style,
            )));
        }

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_flow() {
        let mut ao = ApprovalOverlay::new();
        assert!(!ao.is_active());

        ao.prompt("read_file", "path = \"/etc/passwd\"");
        assert!(ao.is_active());

        ao.select_next();
        ao.select_next();
        let decision = ao.confirm();
        assert_eq!(decision, Some(ApprovalDecision::AllowOnce));
        assert!(!ao.is_active());
    }

    #[test]
    fn test_cancel() {
        let mut ao = ApprovalOverlay::new();
        ao.prompt("exec", "cmd");
        ao.cancel();
        assert!(!ao.is_active());
        assert!(ao.decision().is_none());
    }
}

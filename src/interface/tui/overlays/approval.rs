//! Approval overlay component (modal) for tool execution.

use tokio::sync::oneshot;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

#[cfg(feature = "ui-review")]
use crate::interface::diff_review::types::{DiffHunk, DiffLineKind};

use super::super::approval::ApprovalDecision;
use super::super::component::{Component, EventResult, OverlayAction};
use super::super::event::TuiEvent;

const OPTIONS: &[(&str, ApprovalDecision)] = &[
    ("Allow", ApprovalDecision::Allow),
    ("Deny", ApprovalDecision::Deny),
    ("Allow Once", ApprovalDecision::AllowOnce),
    ("Deny Once", ApprovalDecision::DenyOnce),
];

pub(crate) struct ApprovalOverlay {
    call_id: String,
    tool_name: String,
    tool_args: String,
    #[cfg(feature = "ui-review")]
    diff_hunks: Vec<DiffHunk>,
    selected: usize,
    decision_tx: Option<oneshot::Sender<ApprovalDecision>>,
    dirty: bool,
}

impl ApprovalOverlay {
    pub(crate) fn prompt(
        call_id: String,
        tool_name: String,
        args: serde_json::Value,
        #[cfg(feature = "ui-review")] diff_hunks: Vec<DiffHunk>,
        decision_tx: oneshot::Sender<ApprovalDecision>,
    ) -> Self {
        let tool_args = match serde_json::to_string_pretty(&args) {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(error = %err, "Failed to format tool args for approval overlay.");
                args.to_string()
            }
        };

        Self {
            call_id,
            tool_name,
            tool_args,
            #[cfg(feature = "ui-review")]
            diff_hunks,
            selected: 0,
            decision_tx: Some(decision_tx),
            dirty: true,
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.dirty = true;
    }

    fn select_next(&mut self) {
        if self.selected + 1 < OPTIONS.len() {
            self.selected += 1;
            self.dirty = true;
        }
    }

    fn send_and_dismiss(&mut self, decision: ApprovalDecision) -> EventResult {
        if let Some(tx) = self.decision_tx.take()
            && tx.send(decision).is_err()
        {
            tracing::warn!(
                tool = self.tool_name,
                call_id = self.call_id,
                "Approval overlay failed to send decision (receiver dropped)."
            );
        }

        EventResult {
            consumed: true,
            action: None,
            overlay: Some(OverlayAction::Dismiss),
        }
    }

    #[cfg(feature = "ui-review")]
    fn diff_lines(&self, width: u16, max_lines: usize) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        let clip_w = width.saturating_sub(1);

        for hunk in &self.diff_hunks {
            out.push(Line::from(Span::styled(
                format!(
                    "@@ -{},{} +{},{} @@ {}",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count, hunk.file
                ),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )));

            for line in &hunk.lines {
                let prefix = line.kind.prefix();
                let content = line.content.trim_end_matches('\n');
                let (fg, bg) = match line.kind {
                    DiffLineKind::Add => (Color::Green, Color::Rgb(10, 40, 10)),
                    DiffLineKind::Delete => (Color::Red, Color::Rgb(40, 10, 10)),
                    DiffLineKind::Context => (Color::Gray, Color::Reset),
                };

                let mut text = String::new();
                text.push_str(prefix);
                text.push_str(content);
                if clip_w > 0 && text.chars().count() as u16 > clip_w {
                    text = text.chars().take(clip_w as usize).collect();
                }

                let style = if bg == Color::Reset {
                    Style::default().fg(fg)
                } else {
                    Style::default().fg(fg).bg(bg)
                };
                out.push(Line::from(Span::styled(text, style)));
                if out.len() >= max_lines {
                    return out;
                }
            }

            out.push(Line::from(""));
            if out.len() >= max_lines {
                return out;
            }
        }

        out
    }
}

impl Component for ApprovalOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Overlay — centered, ~60% width, ~45% height.
        let overlay_w = ((area.width as f32) * 0.65) as u16;
        let overlay_h = ((area.height as f32) * 0.55) as u16;
        let x = area
            .x
            .saturating_add((area.width.saturating_sub(overlay_w)) / 2);
        let y = area
            .y
            .saturating_add((area.height.saturating_sub(overlay_h)) / 2);
        let overlay_rect = Rect::new(x, y, overlay_w.max(44), overlay_h.max(12));

        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Tool Approval ")
            .border_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Rgb(20, 15, 5)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(4),
                    Constraint::Length(8), // options always visible
                ]
                .as_ref(),
            )
            .split(inner);

        // ── Top: tool details + diff preview ──────────────────────────
        let mut top_lines = Vec::<Line<'static>>::new();
        top_lines.push(Line::from(Span::styled(
            format!("Tool: {}", self.tool_name),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        top_lines.push(Line::from(Span::styled(
            format!("Call: {}", self.call_id),
            Style::default().fg(Color::DarkGray),
        )));
        top_lines.push(Line::from(""));

        // Arguments (dim).
        for l in self.tool_args.lines().take(6) {
            top_lines.push(Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
        if self.tool_args.lines().count() > 6 {
            top_lines.push(Line::from(Span::styled(
                "…",
                Style::default().fg(Color::DarkGray),
            )));
        }

        #[cfg(feature = "ui-review")]
        if !self.diff_hunks.is_empty() {
            top_lines.push(Line::from(""));
            top_lines.push(Line::from(Span::styled(
                "Proposed diff:",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )));
            top_lines.extend(self.diff_lines(chunks[0].width, chunks[0].height as usize));
        }

        let top_para = Paragraph::new(top_lines).wrap(Wrap { trim: false });
        frame.render_widget(top_para, chunks[0]);

        // ── Bottom: options ───────────────────────────────────────────
        let mut opt_lines = Vec::<Line<'static>>::new();
        opt_lines.push(Line::from(Span::styled(
            "Approve this tool call?",
            Style::default().fg(Color::White),
        )));
        opt_lines.push(Line::from(""));

        for (i, (label, _)) in OPTIONS.iter().enumerate() {
            let selected = i == self.selected;
            let prefix = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            opt_lines.push(Line::from(Span::styled(format!("{prefix}{label}"), style)));
        }
        opt_lines.push(Line::from(""));
        opt_lines.push(Line::from(Span::styled(
            "↑/↓ or j/k, Enter to confirm, Esc to deny once",
            Style::default().fg(Color::DarkGray),
        )));

        let opt_para = Paragraph::new(opt_lines).wrap(Wrap { trim: false });
        frame.render_widget(opt_para, chunks[1]);

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

        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                EventResult::consumed()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                EventResult::consumed()
            }
            KeyCode::Enter => {
                let decision = OPTIONS
                    .get(self.selected)
                    .map(|(_, d)| *d)
                    .unwrap_or(ApprovalDecision::DenyOnce);
                self.send_and_dismiss(decision)
            }
            KeyCode::Esc => self.send_and_dismiss(ApprovalDecision::DenyOnce),
            _ => EventResult::default(),
        }
    }
}

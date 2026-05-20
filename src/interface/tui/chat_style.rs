//! Codex-style transcript formatting helpers.

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

use super::chat::Message;

pub(crate) const LIVE_PREFIX_COLS: u16 = 2;

pub(crate) fn prefix_user_lines(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let prefix_first = Span::from("› ").bold().dim();
    let prefix_rest = Span::from("  ");
    for (idx, mut line) in lines.into_iter().enumerate() {
        if idx == 0 {
            line.spans.insert(0, prefix_first.clone());
        } else {
            line.spans.insert(0, prefix_rest.clone());
        }
        out.push(line);
    }
    out
}

pub(crate) fn prefix_assistant_lines(
    lines: Vec<Line<'static>>,
    streaming: bool,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();

    // Keep assistant mostly un-prefixed; codex uses live-cell prefixes mostly for user and tool.
    // We still show a subtle streaming caret on the last line when streaming.
    for line in lines {
        out.push(line);
    }

    if streaming && let Some(last) = out.last_mut() {
        last.spans
            .push(Span::styled(" ▍", Style::default().fg(Color::Reset).dim()));
    }

    out
}

pub(crate) fn format_user_message(
    message: &Message,
    rendered_lines: Vec<Line<'static>>,
) -> Vec<Line<'static>> {
    let _ = message;
    prefix_user_lines(rendered_lines)
}

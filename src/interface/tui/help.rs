//! Help overlay for the TUI.
//!
//! Displays available keyboard shortcuts.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

const HELP_TEXT: &[(&str, &str)] = &[
    ("Enter", "Submit prompt"),
    ("Ctrl+C", "Interrupt agent"),
    ("Ctrl+D", "Quit TUI"),
    ("Ctrl+L", "Clear chat"),
    ("Ctrl+R", "Toggle diff preview"),
    ("Ctrl+S", "Snapshot session"),
    ("Tab", "Cycle focus"),
    ("j / ↓", "Scroll down"),
    ("k / ↑", "Scroll up"),
    ("g", "Scroll to top"),
    ("G", "Scroll to bottom"),
    ("?", "Toggle this help"),
];

/// Help overlay — keyboard shortcut reference.
pub(crate) struct HelpOverlay {
    visible: bool,
}

impl HelpOverlay {
    pub(crate) fn new() -> Self {
        Self { visible: false }
    }

    /// Toggle visibility.
    pub(crate) fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible.
    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render the help overlay.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let overlay_w = (area.width as f32 * 0.55) as u16;
        let overlay_h = (HELP_TEXT.len() as u16 + 4).min(area.height.saturating_sub(4));
        let x = (area.width - overlay_w) / 2;
        let y = (area.height - overlay_h) / 2;
        let overlay_rect = Rect::new(x, y, overlay_w.max(40), overlay_h.max(10));

        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Keyboard Shortcuts ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Rgb(15, 15, 25)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        let mut lines = Vec::new();
        // Header.
        lines.push(Line::from(vec![ratatui::text::Span::styled(
            format!("  {:<20} {}", "Key", "Action"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        for (key, desc) in HELP_TEXT {
            lines.push(Line::from(vec![
                ratatui::text::Span::styled(
                    format!("  {:<20} ", key),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                ratatui::text::Span::styled(*desc, Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(ratatui::text::Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )));

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle() {
        let mut ho = HelpOverlay::new();
        assert!(!ho.is_visible());
        ho.toggle();
        assert!(ho.is_visible());
        ho.toggle();
        assert!(!ho.is_visible());
    }
}

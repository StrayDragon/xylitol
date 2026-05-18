//! Diff preview component for the TUI.
//!
//! Shows a unified diff view for reviewing code changes inline.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Diff kind for line-level highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffKind {
    Add,
    Delete,
    Context,
}

/// A single diff line.
#[derive(Debug, Clone)]
struct DiffLine {
    kind: DiffKind,
    content: String,
}

/// Diff preview component — shows a unified diff overlay.
pub(crate) struct DiffPreviewComponent {
    /// Lines of the current diff.
    lines: Vec<DiffLine>,
    /// Whether the preview is visible.
    visible: bool,
    /// Scroll offset.
    scroll: u16,
}

impl DiffPreviewComponent {
    pub(crate) fn new() -> Self {
        Self {
            lines: Vec::new(),
            visible: false,
            scroll: 0,
        }
    }

    /// Set the diff content from a unified diff string.
    pub(crate) fn set_diff(&mut self, diff_text: &str) {
        self.lines = diff_text
            .lines()
            .map(|l| {
                let kind = if l.starts_with('+') && !l.starts_with("+++") {
                    DiffKind::Add
                } else if l.starts_with('-') && !l.starts_with("---") {
                    DiffKind::Delete
                } else {
                    DiffKind::Context
                };
                DiffLine {
                    kind,
                    content: l.to_string(),
                }
            })
            .collect();
        self.scroll = 0;
    }

    /// Toggle visibility.
    pub(crate) fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible.
    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    /// Scroll the diff view.
    pub(crate) fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub(crate) fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    /// Render the diff preview as an overlay.
    pub(crate) fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible || self.lines.is_empty() {
            return;
        }

        // Overlay area — centered, 80% width and height.
        let overlay_w = (area.width as f32 * 0.85) as u16;
        let overlay_h = (area.height as f32 * 0.8) as u16;
        let x = (area.width - overlay_w) / 2;
        let y = (area.height - overlay_h) / 2;
        let overlay_rect = Rect::new(x, y, overlay_w, overlay_h);

        // Clear background.
        frame.render_widget(Clear, overlay_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Diff Preview ")
            .title_style(Style::default().fg(Color::LightMagenta))
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));
        let inner = block.inner(overlay_rect);
        frame.render_widget(block, overlay_rect);

        // Render visible lines with syntax coloring.
        let view_h = inner.height as usize;
        let start = self.scroll as usize;
        let end = (start + view_h).min(self.lines.len());

        let lines: Vec<Line<'_>> = self.lines[start..end]
            .iter()
            .map(|dl| {
                let style = match dl.kind {
                    DiffKind::Add => Style::default().fg(Color::Green).bg(Color::Rgb(10, 40, 10)),
                    DiffKind::Delete => Style::default().fg(Color::Red).bg(Color::Rgb(40, 10, 10)),
                    DiffKind::Context => Style::default().fg(Color::Gray),
                };
                Line::from(ratatui::text::Span::styled(&dl.content, style))
            })
            .collect();

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff() {
        let mut dp = DiffPreviewComponent::new();
        dp.set_diff("--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n");
        assert_eq!(dp.lines.len(), 5);
    }

    #[test]
    fn test_toggle() {
        let mut dp = DiffPreviewComponent::new();
        assert!(!dp.is_visible());
        dp.toggle();
        assert!(dp.is_visible());
        dp.toggle();
        assert!(!dp.is_visible());
    }
}
